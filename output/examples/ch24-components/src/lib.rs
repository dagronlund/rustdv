//! Chapter 24: Components — the lifecycle and the ownership tree.
//!
//!     sim-common/run_sim.sh ch24_components playground
//!
//! No DUT needed: structure is the subject.

use rustdv::prelude::*;

rustdv::vpi_bootstrap!();

// Chapter 24, Figure 2: A component demonstrating the lifecycle methods
struct PhaseComp;

impl PhaseComp {
    fn new() -> PhaseComp {
        log::info("1 new() — the build convention");
        PhaseComp
    }
}

impl Component for PhaseComp {
    fn start(&mut self, _ctx: &mut RustdvCtx) {
        log::info("2 start");
    }
    fn extract(&mut self) {
        log::info("3 extract");
    }
    fn check(&mut self, _errors: &mut CheckSink) {
        log::info("4 check");
    }
    fn report(&self) {
        log::info("5 report");
    }
    fn final_phase(&self) {
        log::info("6 final_phase");
    }
}

impl ComponentNode for PhaseComp {
    fn node_name(&self) -> &'static str {
        "PhaseComp"
    }
    fn visit_children(&mut self, _f: &mut dyn FnMut(&str, &mut dyn ComponentNode)) {}
}

// Chapter 24, Figure 3: The test drives the lifecycle in order
#[rustdv::test]
async fn phase_test(_ctx: RustdvCtx) -> Result<(), TestError> {
    let mut comp = PhaseComp::new(); // build (and connect, had it children)

    let mut run_ctx = RustdvCtx::new();
    start_all(&mut comp, &mut run_ctx); // spawn free-running behavior
    run_ctx.all_objections_dropped().await; // the run "phase" is objection-gated

    run_extract_check_report(&mut comp).map_err(TestError::from)
}

// Chapter 24, Figure 5: A three-level hierarchy: children are fields
struct BottomComp;

impl Component for BottomComp {
    fn start(&mut self, ctx: &mut RustdvCtx) {
        let obj = ctx.raise_objection("bc run");
        spawn_named(
            async move {
                log::info("bc run phase");
                drop(obj);
            },
            "bc.run",
        );
    }
}

impl ComponentNode for BottomComp {
    fn node_name(&self) -> &'static str {
        "BottomComp"
    }
    fn visit_children(&mut self, _f: &mut dyn FnMut(&str, &mut dyn ComponentNode)) {}
}

#[derive(rustdv::Component)]
struct MiddleComp {
    #[component(child)]
    bc: BottomComp,
}

impl Component for MiddleComp {}

#[derive(rustdv::Component)]
struct TestTop {
    #[component(child)]
    mc: MiddleComp,
}

impl Component for TestTop {
    fn final_phase(&self) {
        log::info("final phase");
    }
}

// Chapter 24, Figure 6: Constructors are the build phase
#[rustdv::test]
async fn hierarchy_test(_ctx: RustdvCtx) -> Result<(), TestError> {
    // build: bottom-up, in one expression
    let mut top = TestTop { mc: MiddleComp { bc: BottomComp } };

    print_hierarchy(&mut top);

    let mut run_ctx = RustdvCtx::new();
    start_all(&mut top, &mut run_ctx);
    run_ctx.all_objections_dropped().await;

    run_extract_check_report(&mut top).map_err(TestError::from)
}
