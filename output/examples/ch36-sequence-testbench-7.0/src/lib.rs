//! Chapter 36: Sequence testbench 7.0.
//!
//!     sim-common/run_sim.sh ch36_sequence_testbench_7_0 tinyalu \
//!         sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv

use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use rustdv::prelude::*;
use tinyalu_utils::alu_item::AluCommand;
use tinyalu_utils::bfm7::TinyAluBfm;
use tinyalu_utils::env7::{AluEnv, AluEnvConfig};
use tinyalu_utils::Ops;

rustdv::vpi_bootstrap!();

// Chapter 36, Figure 5: RandomSeq — late generation at grant time
pub struct RandomSeq {
    pub n_per_op: usize,
    pub rng: Rng,
}

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

// Chapter 36, Figure 6: MaxSeq — same protocol, different data
pub struct MaxSeq;

impl Sequence<AluCommand> for MaxSeq {
    fn body<'a>(
        &'a mut self,
        mut ctx: SeqCtx<AluCommand>,
    ) -> Pin<Box<dyn Future<Output = Result<(), SeqError>> + 'a>> {
        Box::pin(async move {
            for op in Ops::ALL {
                let mut cmd = AluCommand { a: 0xFF, b: 0xFF, op };
                ctx.start_item(&mut cmd).await;
                ctx.finish_item(cmd).await?;
            }
            Ok(())
        })
    }
}

// Chapter 36, Figure 7: The test starts a sequence on the sequencer
async fn run_seq_test(
    ctx: &TestCtx,
    seq: &mut dyn Sequence<AluCommand>,
    description: &str,
) -> Result<(), TestError> {
    Clock::new(&ctx.dut().signal("clk")?, SimDuration::ns(10)).start();
    let bfm = Rc::new(TinyAluBfm::new(&ctx.dut())?);
    bfm.start_tasks();
    bfm.reset().await;

    let config = AluEnvConfig { bfm: bfm.clone(), is_active: Active::Active, enable_coverage: true };
    let mut env = AluEnv::new(config);

    let mut run_ctx = RunCtx::new();
    start_all(&mut env, &mut run_ctx);
    {
        let _obj = run_ctx.raise_objection(description);
        env.sequencer().start(seq).await?;
        bfm.wait_idle().await; // drain by knowledge, not by clock-counting
    }
    run_ctx.all_objections_dropped().await;

    run_extract_check_report(&mut env).map_err(TestError::from)
}

#[rustdv::test]
async fn random_test(ctx: TestCtx) -> Result<(), TestError> {
    // Run with random operands
    let mut seq = RandomSeq { n_per_op: 1, rng: ctx.rng() };
    run_seq_test(&ctx, &mut seq, "random_test sequence").await
}

#[rustdv::test]
async fn max_test(ctx: TestCtx) -> Result<(), TestError> {
    // Run with maximum operands
    let mut seq = MaxSeq;
    run_seq_test(&ctx, &mut seq, "max_test sequence").await
}
