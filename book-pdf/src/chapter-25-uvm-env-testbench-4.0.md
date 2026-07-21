# Chapter 25: uvm_env Testbench: 4.0

Chapter 24 built the machinery; testbench 4.0 moves in. This version converts the 2.0 classes into real components and gathers them into an **environment** — the container that keeps a tester and its scoreboard together, because a tester without a scoreboard makes little sense. The earlier books did this in three steps and so do we: componentize the testers and scoreboard, instantiate them in an environment, instantiate the environment in tests.

> **In the UVM...** we made `BaseTester` a `uvm_component` that started the BFM in `start_of_simulation_phase()` and drove stimulus in an objection-guarded `run_phase()`; the `Scoreboard` launched its gathering tasks the same way and compared in `check_phase()`; `BaseEnv` built the scoreboard, `RandomEnv`/`MaxEnv` added the right tester; and `RandomTest`/`MaxTest` did nothing but build the right env.

## The tester becomes a component

The class diagram is the traditional one, with the inheritance arrows replaced by a type parameter:

```text
# Figure 1: The 4.0 structure

    AluEnv<T: Tester>                    ("BaseEnv / RandomEnv / MaxEnv")
    ├── tester: TesterComp<T>            ("BaseTester" + its variants)
    │       T = RandomTester | MaxTester (the Chapter 20 behaviors, unchanged)
    └── scoreboard: Scoreboard           (now a Component)
```

```rust
// Figure 2: The tester as a component — start() is its run phase

pub struct TesterComp<T: Tester + 'static> {
    bfm: Rc<TinyAluBfm>,
    tester: Option<T>,
}

impl<T: Tester + 'static> TesterComp<T> {
    pub fn new(bfm: Rc<TinyAluBfm>, tester: T) -> TesterComp<T> {
        TesterComp { bfm, tester: Some(tester) }
    }
}

impl<T: Tester + 'static> Component for TesterComp<T> {
    fn start(&mut self, ctx: &mut RustdvCtx) {
        let bfm = self.bfm.clone();
        let mut tester = self.tester.take().expect("tester started twice");
        let obj = ctx.raise_objection("tester stimulus");
        spawn_named(
            async move {
                tester.execute(&bfm).await;
                drop(obj); // stimulus done: release the run phase
            },
            "tester.run",
        );
    }
}
```

Read this against pyuvm's `BaseTester` and notice what each piece became. `run_phase()` became `start()` spawning a task; the objection guard raised in `start` *moves into* the task and drops when stimulus completes — `raise_objection`/`drop_objection` become the guard's lifetime, and its lifetime is the work's. The `RandomTester`/`MaxTester` split is no longer inheritance at the component level at all: `TesterComp<T>` is *generic over the tester behavior*, so `TesterComp<RandomTester>` and `TesterComp<MaxTester>` are the two "subclasses," manufactured by the compiler (Chapter 11, cashing its check). The behaviors themselves are unchanged from Chapter 20 — the same `Tester` trait, the same two implementors, imported from `tinyalu_utils`.

Two idioms here become Part IV furniture, so name them now. **The `Option::take` baton**: the spawned task needs to own the tester, but `start` only borrows `self` — so the tester rides in an `Option<T>`, and `start` takes it out, moving ownership into the task. Chapter 5's baton pass, exactly as the Interlude previewed. **Dependencies as constructor arguments**: the old testers conjured their BFM from a singleton or fetched a virtual interface from the config database; ours receives its `Rc<TinyAluBfm>` in `new()`. The UVM's traditional advice — components don't do real work in their constructors — inverts completely in rustdv: *the constructor is the build phase*, and taking dependencies there is not a violation of the methodology, it is the methodology.

The behaviors themselves cross over with a `use`, not a rewrite — the old promise ("RandomTester and MaxTester don't need to change") holds verbatim:

```rust
// Figure 3: The 2.0 testers don't need to change

use tinyalu_utils::tb2::{MaxTester, RandomTester, Tester};
```

One pyuvm figure has no Rust counterpart: `start_of_simulation_phase()` starting the BFM tasks. With no phase dispatcher, "before the run starts" is simply a line in the test (you will see it in figure 9), which is where a testbench-wide resource like the BFM belongs anyway.

## The scoreboard becomes a component

```rust
// Figure 4: The Scoreboard as a component

pub struct Scoreboard {
    bfm: Rc<TinyAluBfm>,
    cmds: Rc<RefCell<Vec<CmdTuple>>>,
    results: Rc<RefCell<Vec<u64>>>,
    cvg: HashSet<Ops>,
}
```

```rust
// Figure 5: start() launches the monitoring tasks

impl Component for Scoreboard {
    fn start(&mut self, _ctx: &mut RustdvCtx) {
        let (bfm, cmds) = (self.bfm.clone(), self.cmds.clone());
        spawn_named(
            async move {
                loop {
                    let cmd = bfm.get_cmd().await;
                    cmds.borrow_mut().push(cmd);
                }
            },
            "scoreboard.get_cmd",
        );
        let (bfm, results) = (self.bfm.clone(), self.results.clone());
        spawn_named(
            async move {
                loop {
                    let result = bfm.get_result().await;
                    results.borrow_mut().push(result);
                }
            },
            "scoreboard.get_result",
        );
    }
```

The gathering tasks are Chapter 20's, relocated into the lifecycle: `start` spawns them, and — pointedly — raises **no** objection. Monitors and scoreboards observe; they must never hold the test open, or no test could ever end. Only stimulus objects to ending the run phase. (pyuvm taught the same rule by convention; the code shape makes it visible — there is no guard in this `start`.)

```rust
// Figure 6: Checking results in the check phase

    fn check(&mut self, errors: &mut CheckSink) {
        let mut results = self.results.borrow_mut();
        for cmd in self.cmds.borrow().iter() {
            let (aa, bb, op_int) = *cmd;
            let op = Ops::from_u64(op_int).expect("illegal op captured");
            self.cvg.insert(op);
            let actual = results.remove(0) as u16;
            let prediction = alu_prediction(aa as u8, bb as u8, op);
            if actual == prediction {
                log::info(&format!("PASSED: {aa:02x} {op:?} {bb:02x} = {actual:04x}"));
            } else {
                errors.error(format!(
                    "FAILED: {aa:02x} {op:?} {bb:02x} = {actual:04x} - predicted {prediction:04x}"
                ));
            }
        }
        if Ops::ALL.iter().any(|op| !self.cvg.contains(op)) {
            errors.error("Functional coverage error: missed operations".to_string());
        } else {
            log::info("Covered all operations");
        }
    }
}
```

`check_results()` became `check()`, and the difference from every previous version is the *reporting channel*: no `passed` boolean threaded through returns, no `assert passed` at the end. Failures go into the `CheckSink`, the framework tallies every component's errors, and `run_extract_check_report` turns a non-empty sink into the test's `Err`. pyuvm's 4.0 scoreboard used `assert passed` inside `check_phase` — one component unilaterally ending the argument. The sink is the grown-up version: every scoreboard and coverage collector in the tree deposits its complaints, and the verdict is collective.

## The environment

pyuvm needed three classes and a `super().build_phase()` chain — `BaseEnv` building the scoreboard, `RandomEnv`/`MaxEnv` each adding their tester:

```text
# Figure 7: pyuvm's environment tower, and its Rust replacement

BaseEnv (builds scoreboard)              AluEnv<T: Tester>
├── RandomEnv (adds RandomTester)   ->     AluEnv<RandomTester>
└── MaxEnv (adds MaxTester)         ->     AluEnv<MaxTester>
```

Generics collapse the tower:

```rust
// Figure 8: The environment: a struct whose fields are the testbench

#[derive(rustdv::Component)]
pub struct AluEnv<T: Tester + 'static> {
    #[component(child)]
    tester: TesterComp<T>,
    #[component(child)]
    scoreboard: Scoreboard,
}

impl<T: Tester + 'static> AluEnv<T> {
    pub fn new(bfm: Rc<TinyAluBfm>, tester: T) -> AluEnv<T> {
        // build: construct the children; connect: hand them the BFM.
        AluEnv {
            tester: TesterComp::new(bfm.clone(), tester),
            scoreboard: Scoreboard::new(bfm),
        }
    }
}

impl<T: Tester + 'static> Component for AluEnv<T> {}
```

There is the whole of Chapter 24, five lines at a time. Children are fields; `#[derive(Component)]` writes the traversal; the constructor is `build_phase` (children constructed) and `connect_phase` (the shared BFM handed to each — one `Rc` cloned, one moved, and the compiler would flag it if we cloned when nobody else needed it). `AluEnv<RandomTester>` *is* `RandomEnv`; `AluEnv<MaxTester>` *is* `MaxEnv`; the `super().build_phase()` chain has no translation because there is nothing to chain — the one constructor builds everything, always. And the empty `impl Component` line is the env telling the truth: a container has no behavior of its own; the lifecycle reaches its children through the derived traversal.

## The tests

```rust
// Figure 9: The shared test body: build the env, run the lifecycle

async fn run_env_test<T: Tester + 'static>(
    ctx: &RustdvCtx,
    tester: T,
) -> Result<(), TestError> {
    Clock::new(&ctx.dut().signal("clk")?, SimDuration::ns(10)).start();
    let bfm = Rc::new(TinyAluBfm::new(&ctx.dut())?);
    bfm.reset().await;
    bfm.start_tasks();

    let mut env = AluEnv::new(bfm, tester);

    let mut run_ctx = RustdvCtx::new();
    start_all(&mut env, &mut run_ctx);
    run_ctx.all_objections_dropped().await;

    run_extract_check_report(&mut env).map_err(TestError::from)
}
```

This is the testbench 4.0 skeleton that every remaining version elaborates: bring up the physical layer (clock, BFM, reset), construct the env, `start_all` (bottom-up: scoreboard's monitors before the tester's stimulus — the traversal order doing real work), await the objection consensus, then the post-run tail. The tester's objection is the only one raised, so the run ends when stimulus ends; the scoreboard's gathered lists are then checked in `check`.

```rust
// Figure 10: The tests build the right environment

#[rustdv::test]
async fn random_test(ctx: RustdvCtx) -> Result<(), TestError> {
    // Run with random operands
    run_env_test(&ctx, RandomTester { rng: ctx.rng() }).await
}

#[rustdv::test]
async fn max_test(ctx: RustdvCtx) -> Result<(), TestError> {
    // Run with max operands
    run_env_test(&ctx, MaxTester).await
}
```

pyuvm's tests built the right env in `build_phase`; ours build the right env by *argument* — same one-decision-per-test economy, and the generic machinery means `run_env_test` was compiled twice, once per tester type, each with every call devirtualized. The output is the familiar pair:

```text
# Figure 11: The tests running using environments
--
    145.00ns INFO     PASSED: c1 Add 67 = 0128
    145.00ns INFO     PASSED: 5e And 0b = 000a
    145.00ns INFO     PASSED: b9 Xor 80 = 0039
    145.00ns INFO     PASSED: a5 Mul 75 = 4b69
    145.00ns INFO     Covered all operations
    145.00ns INFO     random_test PASSED
    290.00ns INFO     PASSED: ff Add ff = 01fe
    290.00ns INFO     PASSED: ff And ff = 00ff
    290.00ns INFO     PASSED: ff Xor ff = 0000
    290.00ns INFO     PASSED: ff Mul ff = fe01
    290.00ns INFO     Covered all operations
    290.00ns INFO     max_test PASSED
```

## Summary

Testbench 4.0 componentized the pieces and gave them a home. `TesterComp<T>` wraps any `Tester` in the lifecycle — `start` spawns the stimulus task with the objection guard riding along, `Option::take` passing the baton — while the `Scoreboard` spawns unobjecting monitors in `start` and reports failures through the `CheckSink` in `check`, the collective verdict replacing per-component asserts. The environment is a two-field struct: children as fields, derive-generated traversal, constructor doing build-and-connect in one pass, with generics standing where pyuvm needed a three-class inheritance tower — `AluEnv<RandomTester>` simply *is* the random environment. Tests shrank to one decision each.

The structure is in place, and the next two chapters step off the version treadmill to tour the features that live on it — first logging, which pyuvm hung off the component hierarchy and rustdv must hang somewhere just as usable; then the configuration question, where this chapter's polite constructor arguments meet their loud pyuvm ancestor, the ConfigDB.
