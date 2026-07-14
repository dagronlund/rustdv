//! Chapter 39: Virtual sequence testbench 8.0.
//!
//!     sim-common/run_sim.sh ch39_virtual_sequence_testbench_8_0 tinyalu \
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

// --- Chapter 36's sequences, unchanged ------------------------------------

pub struct RandomSeq {
    pub rng: Rng,
}

impl Sequence<AluCommand> for RandomSeq {
    fn body<'a>(
        &'a mut self,
        mut ctx: SeqCtx<AluCommand>,
    ) -> Pin<Box<dyn Future<Output = Result<(), SeqError>> + 'a>> {
        Box::pin(async move {
            for op in Ops::ALL {
                let mut cmd = AluCommand { a: 0, b: 0, op };
                ctx.start_item(&mut cmd).await;
                cmd.a = self.rng.u8();
                cmd.b = self.rng.u8();
                ctx.finish_item(cmd).await?;
            }
            Ok(())
        })
    }
}

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

// Chapter 39, Figure 1: A virtual sequence starts other sequences
pub struct TestAllSeq {
    pub seqr: Sequencer<AluCommand>,
    pub rng: Rng,
}

impl TestAllSeq {
    pub async fn body(&mut self) -> Result<(), SeqError> {
        // No start_item, no finish_item — no item context to call them on.
        let mut rand_seq = RandomSeq { rng: self.rng.clone() };
        let mut max_seq = MaxSeq;
        self.seqr.start(&mut rand_seq).await?;
        self.seqr.start(&mut max_seq).await?;
        Ok(())
    }
}

// Chapter 39, Figure 4: Running sub-sequences in parallel
pub struct TestAllParallelSeq {
    pub seqr: Sequencer<AluCommand>,
    pub rng: Rng,
}

impl TestAllParallelSeq {
    pub async fn body(&mut self) -> Result<(), SeqError> {
        let seqr_a = self.seqr.clone();
        let seqr_b = self.seqr.clone();
        let rng = self.rng.clone();
        let random_task = spawn_named(
            async move { seqr_a.start(&mut RandomSeq { rng }).await },
            "random_seq",
        );
        let max_task = spawn_named(async move { seqr_b.start(&mut MaxSeq).await }, "max_seq");
        let (r1, r2) = join2(random_task, max_task).await;
        r1.map_err(|e| SeqError(format!("{e:?}")))??;
        r2.map_err(|e| SeqError(format!("{e:?}")))??;
        Ok(())
    }
}

async fn build_and_run(
    ctx: &TestCtx,
    parallel: bool,
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
        let _obj = run_ctx.raise_objection("virtual sequence");
        if parallel {
            let mut vseq = TestAllParallelSeq { seqr: env.sequencer(), rng: ctx.rng() };
            vseq.body().await?;
        } else {
            let mut vseq = TestAllSeq { seqr: env.sequencer(), rng: ctx.rng() };
            vseq.body().await?;
        }
        bfm.wait_idle().await;
    }
    run_ctx.all_objections_dropped().await;

    run_extract_check_report(&mut env).map_err(TestError::from)
}

// Chapter 39, Figure 2: The test starts the virtual sequence
#[rustdv::test]
async fn test_all(ctx: TestCtx) -> Result<(), TestError> {
    build_and_run(&ctx, false).await
}

#[rustdv::test]
async fn test_all_parallel(ctx: TestCtx) -> Result<(), TestError> {
    build_and_run(&ctx, true).await
}
