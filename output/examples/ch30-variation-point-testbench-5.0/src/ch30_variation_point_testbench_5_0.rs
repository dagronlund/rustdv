//! Chapter 30: Variation-point testbench 5.0 — one env, two tests.
//!
//!     sim-common/run_sim.sh ch30_variation_point_testbench_5_0 tinyalu \
//!         sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
//!
//! Testbench 4.0 chose its tester with a **type parameter**: `AluEnv<T>`, and
//! `RandomTest`/`MaxTest` were type aliases (D28). That is a compile-time
//! decision — the environment is a different type in each test. Testbench 5.0
//! makes the same choice at **run time**, through the factory built in
//! Chapter 29: there is now *one* `AluEnv`, and each test overrides the tester
//! before building it. This is the port of the Python book's chapter 34, and
//! it is UVM working exactly as designed — `BaseTester.create()` in the env,
//! `set_type_override_by_type` in each test (Q16, resolved).
//!
//! Every child is an `AnyComp` slot, created with `new_comp()` when it is
//! fixed and `create_comp()` when it may be overridden — the build line, not
//! the field type, carries that choice (D75). Here the tester is the one
//! `create_comp()`; everything else is `new_comp()`. The results match
//! testbench 4.0 bit for bit — the same stimulus, chosen a different way.
//!
//! The BFM, `Ops` and `alu_prediction` come from `tinyalu_utils`; the
//! Scoreboard is re-shown from testbench 4.0 (D45), because the reader is
//! meant to see it unchanged while the tester's *selection* changes around it.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use rustdv::prelude::*;
use tinyalu_utils::{alu_prediction, CmdTuple, Ops, TinyAluBfm};

rustdv::vpi_bootstrap!();

// ===========================================================================
// The testers
// ===========================================================================

// Chapter 30, Figure 1: The stimulus every tester runs.
//
// Only the operands differ between testers — the Python book expresses this
// with an abstract `BaseTester.run_phase` and one overridden `get_operands`.
// Rust has no method inheritance, so the shared body is written once here and
// each tester passes in how it picks operands. The tester that the factory
// installs is what decides which closure runs.
async fn drive_stimulus(
    ctx: &mut RustdvCtx,
    mut get_operands: impl FnMut(&mut Rng) -> (u8, u8),
) -> Result<(), TestError> {
    let _obj = ctx.raise_objection("tester stimulus");
    let bfm = TinyAluBfm::get();
    let mut rng = ctx.rng();

    bfm.reset().await;

    for op in Ops::ALL {
        let (aa, bb) = get_operands(&mut rng);
        bfm.send_op(aa, bb, op).await;
    }
    // send two dummy operations to allow
    // the last real operation to complete
    bfm.send_op(0, 0, Ops::Add).await;
    bfm.send_op(0, 0, Ops::Add).await;
    Ok(())
}

// Chapter 30, Figure 2: The abstract base and the two testers that fill its
// slot.
//
// `BaseTester` is the type the environment names and the factory overrides —
// the analogue of the Python book's abstract `BaseTester`, which raises if it
// is ever run un-overridden. Here that is a `panic!`: a test that forgets its
// override builds a `BaseTester`, and running it is the bug. `RandomTester`
// and `MaxTester` are ordinary components; each differs only in the operands
// it sends. `#[derive(Component)]` registers all three, so any can stand in
// the tester slot by type override (Figures 4-5) or by name.
#[derive(Component, Default)]
struct BaseTester;

impl Component for BaseTester {
    async fn run(&mut self, _ctx: &mut RustdvCtx) -> Result<(), TestError> {
        panic!("BaseTester is abstract — override it with RandomTester or MaxTester");
    }
}

#[derive(Component, Default)]
struct RandomTester;

impl Component for RandomTester {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        drive_stimulus(ctx, |rng| (rng.u8(), rng.u8())).await
    }
}

#[derive(Component, Default)]
struct MaxTester;

impl Component for MaxTester {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        drive_stimulus(ctx, |_rng| (0xFF, 0xFF)).await
    }
}

// ===========================================================================
// The Scoreboard — copied from testbench 4.0 (unchanged)
// ===========================================================================

// Collects commands and results in start_of_simulation, compares them in
// check. It is not a variation point, so it is built the plain way and does
// not change between 4.0 and 5.0.
#[derive(Component, Default)]
struct Scoreboard {
    cmds: Rc<RefCell<Vec<CmdTuple>>>,
    results: Rc<RefCell<Vec<u64>>>,
    cvg: HashSet<Ops>,
}

impl Component for Scoreboard {
    fn start_of_simulation(&mut self, _ctx: &mut RustdvCtx) {
        let (bfm, cmds) = (TinyAluBfm::get(), self.cmds.clone());
        spawn_named(
            async move {
                loop {
                    let cmd = bfm.get_cmd().await;
                    cmds.borrow_mut().push(cmd);
                }
            },
            "scoreboard.get_cmds",
        );

        let (bfm, results) = (TinyAluBfm::get(), self.results.clone());
        spawn_named(
            async move {
                loop {
                    let result = bfm.get_result().await;
                    results.borrow_mut().push(result);
                }
            },
            "scoreboard.get_results",
        );
    }

    fn check(&mut self, ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        let mut results = self.results.borrow_mut();
        for cmd in self.cmds.borrow().iter() {
            let (aa, bb, op_int) = *cmd;
            let op = Ops::from_u64(op_int).expect("illegal op captured");
            self.cvg.insert(op);
            let actual = results.remove(0) as u16;
            let prediction = alu_prediction(aa as u8, bb as u8, op);
            if actual == prediction {
                ctx.info(&format!("PASSED: {aa:02x} {op:?} {bb:02x} = {actual:04x}"));
            } else {
                errors.error(format!(
                    "FAILED: {aa:02x} {op:?} {bb:02x} = {actual:04x} - predicted {prediction:04x}"
                ));
            }
        }

        if Ops::ALL.iter().any(|op| !self.cvg.contains(op)) {
            errors.error("Functional coverage error: missed operations".to_string());
        } else {
            ctx.info("Covered all operations");
        }
    }
}

// ===========================================================================
// The environment with a factory-built tester
// ===========================================================================

// Chapter 30, Figure 3: The environment builds its tester through the factory.
//
// Every child is an `AnyComp` slot; the build line decides fixed vs
// overridable (D75). The scoreboard is fixed — `new_comp()`. The tester is
// the variation point — `create_comp()`, so a test above can substitute a
// different tester without this code being edited or even knowing (D69). At
// 4.0 the choice was a type parameter; here it is one create_comp line.
//
// `start_of_simulation` starts the BFM's driver and monitor tasks — once, for
// the whole environment (the Python book's `AluEnv.start_of_simulation_phase`).
#[derive(Component, Default)]
struct AluEnv {
    #[component(child)]
    scoreboard: AnyComp,
    #[component(child)]
    tester: AnyComp,
}

impl Component for AluEnv {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.scoreboard = Scoreboard::new_comp();
        self.tester = BaseTester::create_comp();
    }

    fn start_of_simulation(&mut self, _ctx: &mut RustdvCtx) {
        TinyAluBfm::get().start_tasks();
    }
}

// ===========================================================================
// The tests
// ===========================================================================

// Chapter 30, Figure 4: random_test overrides BaseTester with RandomTester.
//
// The override is installed in the test's `build`, which runs before the walk
// reaches `env.tester` (build is top-down), so it is in force by the time the
// factory resolves that slot. The env is not edited between the two tests —
// only the override changes.
#[rustdv::test]
#[derive(Component, Default)]
struct RandomTest {
    #[component(child)]
    env: AnyComp,
}

impl Component for RandomTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        Factory::set_type_override::<BaseTester, RandomTester>();
        self.env = AluEnv::new_comp();
    }
}

// Chapter 30, Figure 5: max_test differs only in the tester it installs.
#[rustdv::test]
#[derive(Component, Default)]
struct MaxTest {
    #[component(child)]
    env: AnyComp,
}

impl Component for MaxTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        Factory::set_type_override::<BaseTester, MaxTester>();
        self.env = AluEnv::new_comp();
    }
}
