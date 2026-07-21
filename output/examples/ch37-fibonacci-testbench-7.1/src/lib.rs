//! Chapter 37: Fibonacci testbench 7.1 — stimulus that needs answers.
//!
//!     sim-common/run_sim.sh ch37_fibonacci_testbench_7_1 tinyalu \
//!         sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv

use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use rustdv::prelude::*;
use tinyalu_utils::alu_item::{AluCommand, AluResult};
use tinyalu_utils::bfm7::TinyAluBfm;
use tinyalu_utils::tb7::{CmdMonitor, Scoreboard};
use tinyalu_utils::Ops;

rustdv::vpi_bootstrap!();

// Chapter 37, Figure 3: The 7.1 driver returns responses through item_done
#[derive(rustdv::Component)]
pub struct RspDriver {
    bfm: Rc<TinyAluBfm>,
    result_ap: AnalysisPort<AluResult>,
    seq_item_port: Option<SeqItemPort<AluCommand, AluResult>>,
}

impl RspDriver {
    pub fn new(
        bfm: Rc<TinyAluBfm>,
        result_ap: AnalysisPort<AluResult>,
        seq_item_port: SeqItemPort<AluCommand, AluResult>,
    ) -> RspDriver {
        RspDriver { bfm, result_ap, seq_item_port: Some(seq_item_port) }
    }
}

impl Component for RspDriver {
    fn start(&mut self, _ctx: &mut RustdvCtx) {
        let bfm = self.bfm.clone();
        let ap = self.result_ap.clone();
        let mut port = self.seq_item_port.take().expect("driver started twice");
        spawn_named(
            async move {
                loop {
                    let item = port.get_next_item().await;
                    bfm.send_op(item.payload().clone()).await;
                    // Wait for THIS operation's result before moving on...
                    let result = bfm.get_result().await;
                    log::info(&format!("driver: {result:?}"));
                    ap.write(&result);
                    // ...and hand it back to the sequence.
                    port.item_done(Some(result));
                }
            },
            "rsp_driver",
        );
    }
}

// Chapter 37, Figure 1: The Fibonacci sequence — each command needs
// the previous result
pub struct FibonacciSeq;

impl Sequence<AluCommand, AluResult> for FibonacciSeq {
    fn body<'a>(
        &'a mut self,
        mut ctx: SeqCtx<AluCommand, AluResult>,
    ) -> Pin<Box<dyn Future<Output = Result<(), SeqError>> + 'a>> {
        Box::pin(async move {
            let mut prev_num: u8 = 0;
            let mut cur_num: u8 = 1;
            let mut fib_list: Vec<u16> = vec![prev_num as u16, cur_num as u16];
            for _ in 0..7 {
                let mut cmd = AluCommand { a: 0, b: 0, op: Ops::Add };
                ctx.start_item(&mut cmd).await;
                cmd.a = prev_num;
                cmd.b = cur_num;
                ctx.finish_item(cmd).await?;
                // Chapter 37, Figure 2: the miracle, made explicit
                let rsp = ctx.get_response(None).await;
                fib_list.push(rsp.result);
                prev_num = cur_num;
                cur_num = rsp.result as u8;
            }
            log::info(&format!("Fibonacci Sequence: {fib_list:?}"));
            Ok(())
        })
    }
}

// Chapter 37, Figure 4: The 7.1 environment
#[derive(rustdv::Component)]
pub struct FibEnv {
    seqr: Sequencer<AluCommand, AluResult>,
    #[component(child)]
    driver: RspDriver,
    #[component(child)]
    cmd_mon: CmdMonitor,
    #[component(child)]
    scoreboard: Scoreboard,
}

impl FibEnv {
    pub fn new(bfm: Rc<TinyAluBfm>) -> FibEnv {
        let seqr: Sequencer<AluCommand, AluResult> = Sequencer::new();
        let cmd_ap: AnalysisPort<AluCommand> = AnalysisPort::new();
        let result_ap: AnalysisPort<AluResult> = AnalysisPort::new();

        let scoreboard = Scoreboard::new(cmd_ap.connect_fifo(), result_ap.connect_fifo());
        let driver = RspDriver::new(bfm.clone(), result_ap, seqr.seq_item_port());
        let cmd_mon = CmdMonitor::new(bfm, cmd_ap);

        FibEnv { seqr, driver, cmd_mon, scoreboard }
    }

    pub fn sequencer(&self) -> Sequencer<AluCommand, AluResult> {
        self.seqr.clone()
    }
}

impl Component for FibEnv {}

// Chapter 37, Figure 5: The Fibonacci test
#[rustdv::test]
async fn fibonacci_test(ctx: RustdvCtx) -> Result<(), TestError> {
    Clock::new(&ctx.dut().signal("clk")?, SimDuration::ns(10)).start();
    let bfm = Rc::new(TinyAluBfm::new(&ctx.dut())?);
    bfm.start_tasks();
    bfm.reset().await;

    let mut env = FibEnv::new(bfm.clone());

    let mut run_ctx = RustdvCtx::new();
    start_all(&mut env, &mut run_ctx);
    {
        let _obj = run_ctx.raise_objection("fibonacci sequence");
        env.sequencer().start(&mut FibonacciSeq).await?;
        bfm.wait_idle().await;
    }
    run_ctx.all_objections_dropped().await;

    run_extract_check_report(&mut env).map_err(TestError::from)
}
