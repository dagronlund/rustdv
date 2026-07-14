//! Chapter 34: Connections in testbench 6.0 — wiring by constructor.
//!
//!     sim-common/run_sim.sh ch34_connections_testbench_6_0 tinyalu \
//!         sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv

use std::rc::Rc;

use rustdv::prelude::*;
use tinyalu_utils::tb2::{MaxTester, RandomTester, Tester};
use tinyalu_utils::tb6::{get_cmd, get_result, Cmd, Coverage, Driver, Monitor, Scoreboard, TesterComp};
use tinyalu_utils::{CmdTuple, TinyAluBfm};

rustdv::vpi_bootstrap!();

// Chapter 34, Figure 1: The environment: every connection is an argument
#[derive(rustdv::Component)]
pub struct AluEnv<T: Tester + 'static> {
    #[component(child)]
    tester: TesterComp<T>,
    #[component(child)]
    driver: Driver,
    #[component(child)]
    cmd_mon: Monitor<CmdTuple>,
    #[component(child)]
    result_mon: Monitor<u64>,
    #[component(child)]
    scoreboard: Scoreboard,
    #[component(child)]
    coverage: Coverage,
}

impl<T: Tester + 'static> AluEnv<T> {
    pub fn new(bfm: Rc<TinyAluBfm>, tester: T) -> AluEnv<T> {
        // build: create the plumbing...
        let (cmd_tx, cmd_rx) = channel::<Cmd>(1);
        let cmd_ap: AnalysisPort<CmdTuple> = AnalysisPort::new();
        let result_ap: AnalysisPort<u64> = AnalysisPort::new();

        // connect: ...and hand each component its endpoints.
        AluEnv {
            tester: TesterComp::new(cmd_tx, tester),
            driver: Driver::new(bfm.clone(), cmd_rx),
            scoreboard: Scoreboard::new(cmd_ap.connect_fifo(), result_ap.connect_fifo()),
            coverage: Coverage::new(&cmd_ap),
            cmd_mon: Monitor::new("cmd_monitor", bfm.clone(), get_cmd, cmd_ap),
            result_mon: Monitor::new("result_monitor", bfm, get_result, result_ap),
        }
    }
}

impl<T: Tester + 'static> Component for AluEnv<T> {}

// Chapter 34, Figure 2: The test body — unchanged since 4.0
async fn run_test<T: Tester + 'static>(ctx: &TestCtx, tester: T) -> Result<(), TestError> {
    Clock::new(&ctx.dut().signal("clk")?, SimDuration::ns(10)).start();
    let bfm = Rc::new(TinyAluBfm::new(&ctx.dut())?);

    let mut env = AluEnv::new(bfm, tester);

    let mut run_ctx = RunCtx::new();
    start_all(&mut env, &mut run_ctx);
    run_ctx.all_objections_dropped().await;
    run_extract_check_report(&mut env).map_err(TestError::from)
}

#[rustdv::test]
async fn random_test(ctx: TestCtx) -> Result<(), TestError> {
    // Run with random operands
    run_test(&ctx, RandomTester { rng: ctx.rng() }).await
}

#[rustdv::test]
async fn max_test(ctx: TestCtx) -> Result<(), TestError> {
    // Run with maximum operands
    run_test(&ctx, MaxTester).await
}
