//! Chapter 23: uvm_test testbench 3.0.
//!
//!     sim-common/run_sim.sh ch23_uvm_test_testbench_3_0 tinyalu \
//!         sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
//!
//! The 2.0 classes now come from tinyalu_utils::tb2 — shared code
//! graduates to the shared crate.

use std::rc::Rc;

use rustdv::prelude::*;
use tinyalu_utils::tb2::{MaxTester, RandomTester, Scoreboard, Tester};
use tinyalu_utils::TinyAluBfm;

rustdv::vpi_bootstrap!();

// Chapter 23, Figure 1: The basic rustdv-UVM use model in hello_world
#[rustdv::test]
async fn hello_world_test(_ctx: TestCtx) -> Result<(), TestError> {
    let run_ctx = RunCtx::new();
    {
        let _obj = run_ctx.raise_objection("saying hello");
        log::info("Hello, world.");
    } // the guard drops here: the objection is released
    run_ctx.all_objections_dropped().await;
    Ok(())
}

// Chapter 23, Figure 4: base_test — the shared run phase of every test
async fn base_test(ctx: &TestCtx, tester: &mut impl Tester) -> Result<(), TestError> {
    let run_ctx = RunCtx::new();
    let _obj = run_ctx.raise_objection("base_test stimulus");

    Clock::new(&ctx.dut().signal("clk")?, SimDuration::ns(10)).start();
    let bfm = Rc::new(TinyAluBfm::new(&ctx.dut())?);
    let mut scoreboard = Scoreboard::new(bfm.clone());
    bfm.reset().await;
    bfm.start_tasks();
    scoreboard.start_tasks();

    tester.execute(&bfm).await;
    let passed = scoreboard.check_results();

    drop(_obj);
    run_ctx.all_objections_dropped().await;

    if passed {
        Ok(())
    } else {
        Err(TestError::from("scoreboard saw failing comparisons"))
    }
}

// Chapter 23, Figure 5: The tests build a tester and share base_test
#[rustdv::test]
async fn random_test(ctx: TestCtx) -> Result<(), TestError> {
    // Run with random operations
    base_test(&ctx, &mut RandomTester { rng: ctx.rng() }).await
}

#[rustdv::test]
async fn max_test(ctx: TestCtx) -> Result<(), TestError> {
    // Run with maximum operations
    base_test(&ctx, &mut MaxTester).await
}
