//! Chapter 24: Components — the lifecycle and the ownership tree.
//!
//!     sim-common/run_sim.sh ch24_components playground
//!
//! No DUT needed: structure is the subject.
//!
//! ASPIRATIONAL — this file is the *target* API for the first half of the
//! chapter, written before the framework can compile it (D1/D2). rustdv is
//! being changed to satisfy it. It is the port of the Python book's
//! chapter 28 Figure 1, `PhaseTest(uvm_test)`.
//!
//! This is where the UVM's phase lifecycle comes back. The previous rustdv
//! pass had **destroyed** `build` and `connect` — demoted to "constructor
//! conventions" (review-memo R3) — leaving only five of the nine phases and
//! driving them by hand. D5/D6 restore them as real phase methods, and the
//! runner drives the whole sequence, exactly as pyuvm's phaser does. See
//! `output/.design-decisions.md`.

use rustdv::prelude::*;

rustdv::vpi_bootstrap!();

// ===========================================================================
// First half — running the phases
// ===========================================================================

// Chapter 24, Figure 1: A uvm_test demonstrating the phase methods.
//
// The test IS a component (`#[rustdv::test]` registers it). There is no free
// test function and no hand-rolled `start_all`: the runner drives every
// phase in order, the way `@pyuvm.test()` hands the class to the phaser.
//
// A component overrides only the phases it uses; the rest default to no-ops.
// Every phase receives the context, so its log line carries the component's
// path (D7): the output reads `[PhaseTest]`, the path the walk derived, not
// a hand-typed string that could lie.
//
// Order is pyuvm's, not SV UVM's (D34): build top-down, connect bottom-up,
// run bottom-up, and the elaboration and post-run phases top-down.

#[rustdv::test]
#[derive(Component, Default)]
struct PhaseTest;

impl Component for PhaseTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("1 build");
    }
    fn connect(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("2 connect");
    }
    fn end_of_elaboration(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("3 end_of_elaboration");
    }
    fn start_of_simulation(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("4 start_of_simulation");
    }
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("run");
        ctx.info("5 run");
        Ok(())
    }
    fn extract(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("6 extract");
    }
    fn check(&mut self, ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        let _ = errors;
        ctx.info("7 check");
    }
    fn report(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("8 report");
    }
    fn final_phase(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("9 final");
    }
}

// ===========================================================================
// Second half — building the hierarchy (TestTop -> mc -> bc)
// ===========================================================================
//
// NEXT, not yet written here. The Python book's Figures 4-6 build a
// three-level tree where each parent *creates its children in its own
// build phase* (`self.mc = MiddleComp("mc", self)`) and the phaser then
// recurses into them. That is D6's two-stage construction — children as
// `Option<T>`/`Vec<T>`, populated during `build` — and it is the real work
// of restoring late binding. It is deliberately left for the second half so
// the construction API is designed on purpose, not smuggled in with the
// phase list.
