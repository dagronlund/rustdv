# Interlude: The Complete TinyALU Testbench

> **In the UVM...** the destination was always the same summit: a TinyALU driven by sequences, checked by a scoreboard, measured by coverage, held together by the methodology — whether *The UVM Primer* built it in SystemVerilog or *Python for RTL Verification* built it in pyuvm, both ended at testbench 8.0. This interlude shows you that summit in Rust — the complete, running rustdv testbench — *before* the climb. Nothing here is pseudocode and nothing is a preview-shaped promise: every line below compiles, runs on Icarus Verilog, and finishes with `REGRESSION: PASS`.

Part I handed you fourteen chapters of language and kept saying they were load-bearing. This is the load. What follows is the actual TinyALU testbench from the rustdv repository — the code lives in the `tinyalu_tb` crate of the rustdv workspace, not in the examples tree, because it is not an exercise; it is the thing this book exists to teach you to build.

Read it the way you'd read the last page of a mystery you intend to enjoy properly later. You will understand more of it than you expect — that's Part I paying off — and the parts you don't understand yet each have a chapter with their name on it. A map of those chapters closes the interlude.

## The cast, reintroduced

Same DUT, same architecture. The TinyALU still takes two 8-bit operands and an op code, still raises `done`, still multiplies in three cycles what it adds in one. Around it stands the testbench you already know by role: a BFM that speaks the pin protocol, a driver fed by a sequencer, two monitors publishing what they see, a scoreboard predicting and comparing, coverage counting ops. What changed is the language underneath — and, as you're about to see, how much of the testbench's correctness moved from runtime discipline into the type system.

## Transactions: no base class, three derives

```rust
// Figure 1: The TinyALU transactions (tinyalu_tb/src/alu_item.rs)

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Ops {
    Add = 1,
    And = 2,
    Xor = 3,
    Mul = 4,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AluCommand {
    pub a: u8,
    pub b: u8,
    pub op: Ops,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AluResult {
    pub result: u16,
}
```

Chapter 7 gave you `Ops` as an enum with no integer pretense; Chapter 10 promised that derives would do the work of `uvm_sequence_item`'s dunder methods. Here is the whole of it: `Clone` is `do_copy`, `PartialEq` is `do_compare`, `Debug` is `convert2string`. There is no transaction base class in this testbench because there is nothing left for one to do.

The scoreboard's golden model is a free function over that plain data — which means it is testable with no simulator in sight:

```rust
// Figure 2: The predictor, and a unit test that needs no simulator

pub fn predict(cmd: &AluCommand) -> AluResult {
    let a = cmd.a as u16;
    let b = cmd.b as u16;
    let result = match cmd.op {
        Ops::Add => a + b,
        Ops::And => a & b,
        Ops::Xor => a ^ b,
        Ops::Mul => a * b,
    };
    AluResult { result }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predict_add_carries_into_bit8() {
        let r = predict(&AluCommand { a: 0xFF, b: 0xFF, op: Ops::Add });
        assert_eq!(r.result, 0x01FE);
    }
}
```

```text
$ cargo test
--
test alu_item::tests::predict_add_carries_into_bit8 ... ok
test result: ok. 4 passed; 0 failed
```

Chapter 14 called this capability — `cargo test` on pure testbench logic, milliseconds, no license checkout — something we would never stop using. The shipping testbench uses it from day one.

## The BFM: one owner of the pins, shared by handle

The `TinyAluBfm` plays exactly the role a BFM has always played in this testbench: it owns the typed signal handles and speaks the `start`/`done` handshake, so nothing else in the testbench touches a pin. Its surface is four async methods and a queue-fed state machine:

```rust
// Figure 3: The BFM's surface (tinyalu_tb/src/alu_bfm.rs, abridged)

pub struct TinyAluBfm {
    clk: LogicHandle,
    reset_n: LogicHandle,
    start: LogicHandle,
    done: LogicHandle,
    // ... A, B, op, result, and three queues
}

impl TinyAluBfm {
    pub fn new(dut: &HierarchyHandle) -> Result<TinyAluBfm, HandleError> {
        Ok(TinyAluBfm {
            clk: dut.signal("clk")?,
            reset_n: dut.signal("reset_n")?,
            // ...
        })
    }

    pub async fn reset(&self) { /* five falling edges, deassert, flush */ }
    pub async fn send_op(&self, cmd: AluCommand) { self.driver_q.put(cmd).await }
    pub async fn get_cmd(&self) -> AluCommand { self.cmd_q.get().await }
    pub async fn get_result(&self) -> AluResult { self.result_q.get().await }
}
```

Three things to notice with Part I eyes. First, `dut.signal("clk")?` returns a `Result` — a typo'd signal name is an `Err` at time zero with the scope and name in the message, not an `AttributeError` forty minutes into elaboration (Chapter 9). Second, every method takes `&self`: the BFM's sharing is read-shaped from the outside, which is why — third — the testbench shares it as `Rc<TinyAluBfm>`, the exact pattern Chapter 13 previewed. One BFM, created by the test, a counted handle passed to each component that needs it, no `RefCell` anywhere.

Inside, the BFM runs the same three free-running loops its ancestors did — a driver state machine and two monitors, each watching falling clock edges:

```rust
// Figure 4: The BFM driver loop — the book's driver_bfm, in Rust (abridged)

spawn_named(
    async move {
        loop {
            clk.falling_edge().await;
            let st = start.get_binstr();
            let dn = done.get_binstr();
            if st == "0" && dn == "0" {
                if let Some(cmd) = q.try_get() {
                    a.set_u64(cmd.a as u64);
                    b.set_u64(cmd.b as u64);
                    op.set_u64(cmd.op.as_u64());
                    start.set_u64(1);
                }
            } else if st == "1" && dn == "1" {
                start.set_u64(0);
            }
        }
    },
    "bfm.driver",
);
```

`clk.falling_edge().await` is Part II's whole subject matter compressed into one expression — a future, an executor, a simulator callback (Chapters 15–17). The `set_u64` calls are scheduled writes, applied at the simulator's read-write phase exactly as cocotb applied them. And the `async move { loop { ... } }` block owns everything it captured, per Chapter 5 — this task cannot race another task for these locals, because no other task can even name them.

## Components: structs that opt into a lifecycle

Chapter 10's Figure 4 showed you the shape of the `Component` trait — default empty methods, override what you use. The driver overrides `start`:

```rust
// Figure 5: The driver (tinyalu_tb/src/components.rs)

#[derive(rustdv::Component)]
pub struct Driver {
    bfm: Rc<TinyAluBfm>,
    seq_item_port: Option<SeqItemPort<AluCommand>>,
}

impl Component for Driver {
    fn start(&mut self, _ctx: &mut RunCtx) {
        let bfm = self.bfm.clone();
        let mut port = self.seq_item_port.take().expect("Driver started twice");
        spawn_named(
            async move {
                loop {
                    let item = port.get_next_item().await;
                    bfm.send_op(item.payload().clone()).await;
                    port.item_done(None);
                }
            },
            "driver",
        );
    }
}
```

There is Chapter 11's `SeqItemPort<AluCommand>`, doing in earnest what Figure 6 previewed: the port's type is the contract, and a sequence of the wrong transaction type cannot be wired to this driver in code that compiles. The `get_next_item` / `item_done` handshake is pyuvm's, preserved event for event — Part IV walks it. One idiom is worth flagging now so it doesn't ambush you later: the port lives in an `Option`, and `start` *takes* it (`Option::take`, Chapter 9) to move it into the spawned task. Ownership of the port passes from the component to the task that uses it — the baton pass from Chapter 5, load-bearing at last.

The scoreboard overrides the other end of the lifecycle. It drains the two analysis FIFOs the monitors feed, predicts, and compares — with `PartialEq` doing the comparison policy, on the data, in the checker, where it belongs:

```rust
// Figure 6: The scoreboard's check phase (abridged)

impl Component for Scoreboard {
    fn check(&mut self, errors: &mut CheckSink) {
        loop {
            match (self.cmd_fifo.try_get(), self.result_fifo.try_get()) {
                (Some(cmd), Some(actual)) => {
                    let expected = predict(&cmd);
                    self.compared += 1;
                    if expected != actual {
                        errors.error(format!(
                            "scoreboard mismatch: {cmd:?} -> got {actual:?}, expected {expected:?}"
                        ));
                    }
                }
                (None, None) => break,
                (Some(cmd), None) => {
                    errors.error(format!("scoreboard: command {cmd:?} has no result"));
                }
                (None, Some(res)) => {
                    errors.error(format!("scoreboard: result {res:?} has no command"));
                }
            }
        }
    }

    fn report(&self) {
        log::info(&format!(
            "scoreboard: {} compared, {} mismatches",
            self.compared, self.mismatches
        ));
    }
}
```

Every arm of that `match` is a distinct end-of-test story — matched pair, clean exhaustion, orphaned command, orphaned result — and Chapter 4's exhaustiveness rule means forgetting one is a compile error. Coverage, meanwhile, is Chapter 10's `Subscriber<T>` trait earning its keep: a collector that counts ops as the command monitor broadcasts them, and errors in `check` if any op was never exercised.

## The environment: the ownership tree is the component tree

Chapter 13 ended by telling you rustdv barely needs smart pointers because the hierarchy is just structs owning structs. Here is that claim, in the shipping code:

```rust
// Figure 7: The environment and its config (tinyalu_tb/src/env.rs, abridged)

pub struct AluEnvConfig {
    pub bfm: Rc<TinyAluBfm>,
    pub is_active: Active,
    pub enable_coverage: bool,
}

#[derive(rustdv::Component)]
pub struct AluEnv {
    seqr: Sequencer<AluCommand>,
    #[component(child)]
    driver: Option<Driver>,
    #[component(child)]
    cmd_mon: CmdMonitor,
    #[component(child)]
    result_mon: ResultMonitor,
    #[component(child)]
    scoreboard: Scoreboard,
    #[component(child)]
    coverage: Option<Coverage>,
}

impl AluEnv {
    pub fn new(config: AluEnvConfig) -> AluEnv {
        // build: construct children bottom-up in one pass.
        let seqr: Sequencer<AluCommand> = Sequencer::new();
        let cmd_ap: AnalysisPort<AluCommand> = AnalysisPort::new();
        let result_ap = AnalysisPort::new();

        // connect: endpoints are constructor arguments; a missing
        // connection is a missing argument — a compile error.
        let cmd_fifo = cmd_ap.connect_fifo();
        let result_fifo = result_ap.connect_fifo();
        let scoreboard = Scoreboard::new(cmd_fifo, result_fifo);

        let coverage = config.enable_coverage.then(|| Coverage::new(&cmd_ap));

        let driver = match config.is_active {
            Active::Active => Some(Driver::new(config.bfm.clone(), seqr.seq_item_port())),
            Active::Passive => None,
        };

        let cmd_mon = CmdMonitor::new(config.bfm.clone(), cmd_ap);
        let result_mon = ResultMonitor::new(config.bfm, result_ap);

        AluEnv { seqr, driver, cmd_mon, result_mon, scoreboard, coverage }
    }
}
```

Read the field list first: children are fields, and the hierarchy is the struct tree — no parent pointers, no global component registry, no `Rc` cycles to leak. Then read the two comments in `new`, because they are the whole of pyuvm's `build_phase` and `connect_phase`: building *is* constructing your children; connecting *is* passing endpoints as arguments. A `ConfigDB` string path that might miss at runtime became `AluEnvConfig`, a typed struct — the wrong type in a field is now the compiler's problem, not seed 8,441's.

And look at what active/passive became. In pyuvm, an agent consulted a config value at build time and warned at runtime if it held garbage. Here, `is_active` is a two-variant enum (Chapter 7), and a passive environment simply *does not construct* a driver: `driver: Option<Driver>` (Chapter 9). The illegal state — a passive agent with a live driver — is not checked for. It is unrepresentable.

## Sequences and tests

The sequence protocol is pyuvm's, and the ordering the book taught — operands filled in *after* `start_item` grants, at the moment the driver is ready — survives intact:

```rust
// Figure 8: The random sequence (tinyalu_tb/src/sequences.rs, abridged)

impl Sequence<AluCommand> for RandomSeq {
    fn body<'a>(
        &'a mut self,
        mut ctx: SeqCtx<AluCommand>,
    ) -> Pin<Box<dyn Future<Output = Result<(), SeqError>> + 'a>> {
        Box::pin(async move {
            for _ in 0..self.n_per_op {
                for op in Ops::ALL {
                    let mut cmd = AluCommand { a: 0, b: 0, op };
                    ctx.start_item(&mut cmd).await;
                    // Late generation: fill at grant time.
                    cmd.a = self.rng.u8();
                    cmd.b = self.rng.u8();
                    ctx.finish_item(cmd).await?;
                }
            }
            Ok(())
        })
    }
}
```

A test is an `async fn` returning `Result<(), TestError>`, exactly as Chapter 9 promised, registered by an attribute where `@cocotb.test()` used a decorator (Part III explains the machinery):

```rust
// Figure 9: The tests (tinyalu_tb/src/lib.rs, abridged)

rustdv::vpi_bootstrap!();

#[rustdv::test(timeout_time = 500, timeout_unit = "us")]
async fn random_ops(ctx: TestCtx) -> Result<(), TestError> {
    let (bfm, mut env) = build_testbench(&ctx, true).await?;
    let mut seq = RandomSeq { n_per_op: 5, rng: ctx.rng() };
    run_sequence(&bfm, &mut env, &mut seq, "random_ops sequence").await?;
    log::info("random_ops: sequence complete");
    Ok(())
}

#[rustdv::test(timeout_time = 500, timeout_unit = "us")]
async fn max_ops(ctx: TestCtx) -> Result<(), TestError> {
    let (bfm, mut env) = build_testbench(&ctx, true).await?;
    let mut seq = MaxSeq;
    run_sequence(&bfm, &mut env, &mut seq, "max_ops sequence").await?;
    log::info("max_ops: sequence complete");
    Ok(())
}
```

`max_ops` differs from `random_ops` in one line — the sequence it starts. That is the factory's job done by ordinary values, the trade Chapter 12 set up when it taught you that closures and behavior are things you can hand around.

## Running it

One script builds the testbench as a shared library, compiles the DUT under Icarus, and hands both to the simulator. The testbench drives the DUT directly — there is no Verilog testbench file at all:

```text
# Figure 10: The regression, for real
$ sim/run_rustdv.sh
--
      0.00ns INFO     rustdv: found 2 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running random_ops (1/2)  [tinyalu_tb/src/lib.rs:67]
     75.00ns INFO     cmd_monitor: AluCommand { a: 193, b: 103, op: Add }
     75.00ns INFO     result_monitor: AluResult { result: 296 }
     95.00ns INFO     cmd_monitor: AluCommand { a: 94, b: 11, op: And }
     95.00ns INFO     result_monitor: AluResult { result: 10 }
    ...
    635.00ns INFO     scoreboard: 20 compared, 0 mismatches
    635.00ns INFO     coverage: Add=5 And=5 Mul=5 Xor=5
    635.00ns INFO     random_ops PASSED
    635.00ns INFO     running max_ops (2/2)  [tinyalu_tb/src/lib.rs:76]
    ...
    830.00ns INFO     scoreboard: 4 compared, 0 mismatches
    830.00ns INFO     coverage: Add=1 And=1 Mul=1 Xor=1
    830.00ns INFO     max_ops PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** random_ops                                   PASS         635.00      **
** max_ops                                      PASS         195.00      **
******************************************************************************
REGRESSION: PASS
```

The transcript should feel like coming home: sim-time-stamped log lines, a monitor narrating transactions, a scoreboard summary, cocotb's result table. The seed is printed so the run reproduces; set `RUSTDV_RANDOM_SEED` to chase a failure.

And because a checker you have never seen fail is a checker you should not trust, the repository's status log records the sabotage run: with the DUT's XOR deliberately corrupted into an OR, the same regression flags every affected transaction —

```text
# Figure 11: The same testbench, catching a planted bug
--
    830.00ns ERROR    scoreboard mismatch: AluCommand { a: 255, b: 255, op: Xor }
                      -> got AluResult { result: 255 }, expected AluResult { result: 0 }
    830.00ns ERROR    max_ops FAILED: 1 check failure(s): ...
REGRESSION: FAIL
```

— which is the last word on whether all those types left the testbench any teeth.

## What you already understand, and what's ahead

Tally what Part I let you read fluently just now: enums and `match` in the transactions and the scoreboard (Chapters 4, 7); ownership moving the port into the driver's task (Chapter 5); `Result` and `?` threading every fallible step (Chapter 9); traits and derives replacing the base classes (Chapter 10); `SeqItemPort<AluCommand>` (Chapter 11); a closure-shaped factory replacement (Chapter 12); one `Rc` where one resource is genuinely shared (Chapter 13); a workspace, a prelude, and simulator-free unit tests (Chapter 14).

What remains is the machinery you just took on faith, and it is exactly the rest of the book. `falling_edge().await` and the executor that resumes it: Chapters 15–17. The BFM built for real, testbenches 1.0 and 2.0: Chapters 18–20. `#[rustdv::test]`, `#[derive(Component)]`, and how tests get discovered with no import step: Chapter 21. The `Component` lifecycle, configs, channels, analysis ports, and the sequencer handshake, one pyuvm chapter at a time: Part IV. And the capstone — this testbench again, rebuilt by your own hands and grown to testbench 8.0: Part V.

You have seen the summit. Now we climb.
