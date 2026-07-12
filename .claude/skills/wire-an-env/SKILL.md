---
name: wire-an-env
description: Use when building rustdv testbench structure — transactions, components (driver/monitor/scoreboard/coverage), the environment, configuration, sequencer wiring, and the component lifecycle. Covers the derive(Component) rules, the Option::take idiom for moving ports into spawned tasks (the ownership fight every first attempt loses), and the sequencer handshake rules.
---

# Wire a rustdv environment

The component tree IS the ownership tree: children are struct fields; there
is no factory, no ConfigDB, no string paths. Reference:
`rustdv/tinyalu_tb/src/{components.rs,env.rs,sequences.rs}`.

## Transactions
Plain structs. `#[derive(Clone, Debug, PartialEq)]` replaces the whole
`uvm_object` surface. Op codes are enums (`#[repr(u8)]`, explicit values
matching the RTL encoding). The predictor is a free function over them —
unit-test it with plain `#[test]`, no simulator.

## Config, then env

```rust
pub struct MyEnvConfig {
    pub bfm: Rc<MyBfm>,        // the one genuinely shared resource
    pub is_active: Active,     // enum, not a string-keyed int
    pub enable_coverage: bool,
}

#[derive(rustdv::Component)]
pub struct MyEnv {
    seqr: Sequencer<MyCmd>,            // not a child — no lifecycle
    #[component(child)] driver: Option<Driver>,   // passive = no driver
    #[component(child)] monitor: Monitor,
    #[component(child)] scoreboard: Scoreboard,
    #[component(child)] coverage: Option<Coverage>,
}
```

Derive rules: non-generic structs with named fields only; children may be
`T`, `Option<T>`, or `Vec<T>`. The derive generates traversal only — you
still write `impl Component for MyEnv {}` (empty is fine; every lifecycle
method has a default body).

## Constructors do build AND connect

```rust
impl MyEnv {
    pub fn new(config: MyEnvConfig) -> MyEnv {
        // build: construct children bottom-up.
        let seqr = Sequencer::new();
        let cmd_ap = AnalysisPort::new();
        // connect: endpoints are constructor ARGUMENTS. A missing
        // connection is a missing argument — a compile error.
        let scoreboard = Scoreboard::new(cmd_ap.connect_fifo(), result_ap.connect_fifo());
        let coverage = config.enable_coverage.then(|| Coverage::new(&cmd_ap));
        let driver = match config.is_active {
            Active::Active => Some(Driver::new(config.bfm.clone(), seqr.seq_item_port())),
            Active::Passive => None,
        };
        // ...
    }
}
```

- Analysis fan-out: monitors own an `AnalysisPort<T>` and `write(&item)`;
  scoreboards read `AnalysisFifo<T>`s created by `connect_fifo()`;
  coverage implements `Subscriber<T>` and connects via
  `ap.connect(Rc<RefCell<dyn Subscriber<T>>>)`.

## The Option::take idiom (you WILL hit this)

`start(&mut self, ...)` spawns a `'static` task, but the task needs parts
of `self`. Borrowing `self` into the task does not compile. The idiom:
store movable parts in `Option`, take them at start:

```rust
fn start(&mut self, _ctx: &mut RunCtx) {
    let bfm = self.bfm.clone();                       // Rc: clone the handle
    let mut port = self.seq_item_port.take().expect("started twice");
    spawn_named(async move {                          // moves port + bfm
        loop {
            let item = port.get_next_item().await;
            bfm.send_op(item.payload().clone()).await;
            port.item_done(None);
        }
    }, "driver");
}
```

## Sequencer handshake rules (panics are deliberate)

- Sequence side: `start_item(&mut req).await` → fill fields (late
  generation) → `finish_item(req).await?`. Calling `start_item` twice
  without `finish_item` panics.
- Driver side: `get_next_item().await` → drive → `item_done(rsp)`.
  Calling `get_next_item` twice without `item_done` panics.
- Responses: `item_done(Some(rsp))` tags with the envelope's `TxnId`;
  retrieve with `get_response(None)` (FIFO) or `Some(txn_id)`.
- Per-test behavior change = start a different sequence. Deeper variation
  points = closure fields in the config struct (no factory exists).

## Lifecycle in the test body

```rust
let mut env = MyEnv::new(config);
let mut run_ctx = RunCtx::new();
start_all(&mut env, &mut run_ctx);            // bottom-up start
{
    let _obj = run_ctx.raise_objection("stimulus");   // RAII
    env.sequencer().start(&mut seq).await?;
    bfm.wait_idle().await;                    // drain! (see write-a-bfm)
}
run_ctx.all_objections_dropped().await;
run_extract_check_report(&mut env).map_err(TestError::from)
```

Convention: mismatches go in `check(&mut self, errors: &mut CheckSink)`
via `errors.error(...)`; human summaries go in `report(&self)`. The
runner kills all tasks a test spawned when the test ends — free-running
loops don't need shutdown logic.
