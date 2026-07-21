//! Chapter 38: get_response testbench 7.2 — transaction ids at work.
//!
//!     sim-common/run_sim.sh ch38_get_response_testbench_7_2 tinyalu \
//!         sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
//!
//! The env and driver are Chapter 37's (FibEnv/RspDriver, imported here
//! by copy from the ch37 crate would be circular; the pieces live in
//! this crate unchanged where needed).

use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use rustdv::prelude::*;
use tinyalu_utils::alu_item::{AluCommand, AluResult};
use tinyalu_utils::bfm7::TinyAluBfm;
use tinyalu_utils::tb7::{CmdMonitor, Scoreboard};
use tinyalu_utils::Ops;

rustdv::vpi_bootstrap!();

// --- Chapter 37's RspDriver, unchanged -----------------------------------
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
                    let result = bfm.get_result().await;
                    ap.write(&result);
                    port.item_done(Some(result));
                }
            },
            "rsp_driver",
        );
    }
}

// Chapter 38, Figure 1: finish_item returns the transaction's id
pub struct CherryPickSeq {
    pub rng: Rng,
}

impl Sequence<AluCommand, AluResult> for CherryPickSeq {
    fn body<'a>(
        &'a mut self,
        mut ctx: SeqCtx<AluCommand, AluResult>,
    ) -> Pin<Box<dyn Future<Output = Result<(), SeqError>> + 'a>> {
        Box::pin(async move {
            let mut ids: Vec<(TxnId, Ops)> = Vec::new();
            for op in Ops::ALL {
                let mut cmd = AluCommand { a: 0, b: 0, op };
                ctx.start_item(&mut cmd).await;
                cmd.a = self.rng.u8();
                cmd.b = self.rng.u8();
                let id = ctx.finish_item(cmd).await?;
                ids.push((id, op));
            }

            // Chapter 38, Figure 2: Cherry-picking responses by id,
            // in reverse order — arrival order no longer matters
            for (id, op) in ids.iter().rev() {
                let rsp = ctx.get_response(Some(*id)).await;
                log::info(&format!("response for {op:?} (txn {id:?}): {rsp:?}"));
            }
            Ok(())
        })
    }
}

// --- Chapter 37's env shape, reused ---------------------------------------
#[derive(rustdv::Component)]
pub struct RspEnv {
    seqr: Sequencer<AluCommand, AluResult>,
    #[component(child)]
    driver: RspDriver,
    #[component(child)]
    cmd_mon: CmdMonitor,
    #[component(child)]
    scoreboard: Scoreboard,
}

impl RspEnv {
    pub fn new(bfm: Rc<TinyAluBfm>) -> RspEnv {
        let seqr: Sequencer<AluCommand, AluResult> = Sequencer::new();
        let cmd_ap: AnalysisPort<AluCommand> = AnalysisPort::new();
        let result_ap: AnalysisPort<AluResult> = AnalysisPort::new();
        let scoreboard = Scoreboard::new(cmd_ap.connect_fifo(), result_ap.connect_fifo());
        let driver = RspDriver::new(bfm.clone(), result_ap, seqr.seq_item_port());
        let cmd_mon = CmdMonitor::new(bfm, cmd_ap);
        RspEnv { seqr, driver, cmd_mon, scoreboard }
    }

    pub fn sequencer(&self) -> Sequencer<AluCommand, AluResult> {
        self.seqr.clone()
    }
}

impl Component for RspEnv {}

// Chapter 38, Figure 3: The cherry-picking test
#[rustdv::test]
async fn cherry_pick_test(ctx: RustdvCtx) -> Result<(), TestError> {
    Clock::new(&ctx.dut().signal("clk")?, SimDuration::ns(10)).start();
    let bfm = Rc::new(TinyAluBfm::new(&ctx.dut())?);
    bfm.start_tasks();
    bfm.reset().await;

    let mut env = RspEnv::new(bfm.clone());

    let mut run_ctx = RustdvCtx::new();
    start_all(&mut env, &mut run_ctx);
    {
        let _obj = run_ctx.raise_objection("cherry-pick sequence");
        let mut seq = CherryPickSeq { rng: ctx.rng() };
        env.sequencer().start(&mut seq).await?;
        bfm.wait_idle().await;
    }
    run_ctx.all_objections_dropped().await;

    run_extract_check_report(&mut env).map_err(TestError::from)
}
