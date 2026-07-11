//! TinyALU testbench (design-doc §7): the worked example, end to end.
//!
//! Body shape (§7.2): build config (sequence + variation choices) →
//! construct env → start lifecycle → await sequence → drain → check.
//! `max_ops` differs from `random_ops` only in the sequence it starts —
//! the variation-point pattern replacing the factory override.

use std::rc::Rc;

use rustdv::prelude::*;

pub mod alu_bfm;
pub mod alu_item;
pub mod components;
pub mod env;
pub mod sequences;

use alu_bfm::TinyAluBfm;
use env::{AluEnv, AluEnvConfig};
use sequences::{MaxSeq, RandomSeq};

// Export the VPI entry points from this cdylib.
rustdv::vpi_bootstrap!();

/// Common test scaffolding: clock, BFM, reset, env. Returns (bfm, env).
async fn build_testbench(
    ctx: &TestCtx,
    enable_coverage: bool,
) -> Result<(Rc<TinyAluBfm>, AluEnv), TestError> {
    let dut = ctx.dut();
    let bfm = Rc::new(TinyAluBfm::new(&dut)?);
    Clock::new(bfm.clk(), SimDuration::ns(10)).start();
    bfm.start_tasks();
    bfm.reset().await;

    let config = AluEnvConfig { bfm: bfm.clone(), is_active: Active::Active, enable_coverage };
    let env = AluEnv::new(config);
    Ok((bfm, env))
}

/// Run a sequence through the env, drain, then extract/check/report.
async fn run_sequence(
    bfm: &Rc<TinyAluBfm>,
    env: &mut AluEnv,
    seq: &mut dyn Sequence<alu_item::AluCommand>,
    description: &str,
) -> Result<(), TestError> {
    let mut run_ctx = RunCtx::new();
    start_all(env, &mut run_ctx);

    {
        // Every stimulus task holds an objection guard (§7.3 convention 2).
        let _obj = run_ctx.raise_objection(description);
        env.sequencer().start(seq).await?;
        bfm.wait_idle().await;
    }
    run_ctx.all_objections_dropped().await;

    run_extract_check_report(env).map_err(TestError::from)
}

#[rustdv::test(timeout_time = 500, timeout_unit = "us")]
async fn random_ops(ctx: TestCtx) -> Result<(), TestError> {
    let (bfm, mut env) = build_testbench(&ctx, true).await?;
    let mut seq = RandomSeq { n_per_op: 5, rng: ctx.rng() };
    run_sequence(&bfm, &mut env, &mut seq, "random_ops sequence").await?;
    log::info("random_ops: sequence complete");
    Ok(())
}

#[rustdv::test(timeout_time = 500, timeout_unit = "us")]
async fn max_ops(ctx: TestCtx) -> Result<(), TestError> {
    let (bfm, mut env) = build_testbench(&ctx, true).await?;
    let mut seq = MaxSeq;
    run_sequence(&bfm, &mut env, &mut seq, "max_ops sequence").await?;
    log::info("max_ops: sequence complete");
    Ok(())
}
