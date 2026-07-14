//! Chapter 30: Variation-point testbench 5.0 — one env, two tests.
//!
//!     sim-common/run_sim.sh ch30_variation_point_testbench_5_0 tinyalu \
//!         sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv

use std::rc::Rc;

use rustdv::prelude::*;
use tinyalu_utils::tb2::{MaxTester, RandomTester};
use tinyalu_utils::tb4::{Scoreboard, TesterComp};
use tinyalu_utils::TinyAluBfm;

rustdv::vpi_bootstrap!();

// Chapter 30, Figure 1: The slot's contract, and the maker that fills it

/// What must be true of anything standing in the tester slot.
pub trait TesterCompLike: ComponentNode {}
impl<T: ComponentNode> TesterCompLike for T {}

/// A stored constructor: give it the BFM, get a tester component.
pub type TesterMaker = Box<dyn FnOnce(Rc<TinyAluBfm>) -> Box<dyn TesterCompLike>>;

// Chapter 30, Figure 2: One environment with a designed variation point
pub struct AluEnvConfig {
    pub bfm: Rc<TinyAluBfm>,
    pub make_tester: TesterMaker,
}

pub struct AluEnv {
    tester: Box<dyn TesterCompLike>,
    scoreboard: Scoreboard,
}

impl AluEnv {
    pub fn new(config: AluEnvConfig) -> AluEnv {
        AluEnv {
            // The variation point: the env builds whatever the test sent.
            tester: (config.make_tester)(config.bfm.clone()),
            scoreboard: Scoreboard::new(config.bfm),
        }
    }
}

impl Component for AluEnv {}

impl ComponentNode for AluEnv {
    fn node_name(&self) -> &'static str {
        "AluEnv"
    }
    fn visit_children(&mut self, f: &mut dyn FnMut(&str, &mut dyn ComponentNode)) {
        f("tester", self.tester.as_mut());
        f("scoreboard", &mut self.scoreboard);
    }
}

// Chapter 30, Figure 3: The shared test body takes a config
async fn run_test(ctx: &TestCtx, make_tester: TesterMaker) -> Result<(), TestError> {
    Clock::new(&ctx.dut().signal("clk")?, SimDuration::ns(10)).start();
    let bfm = Rc::new(TinyAluBfm::new(&ctx.dut())?);
    bfm.reset().await;
    bfm.start_tasks();

    let mut env = AluEnv::new(AluEnvConfig { bfm, make_tester });

    let mut run_ctx = RunCtx::new();
    start_all(&mut env, &mut run_ctx);
    run_ctx.all_objections_dropped().await;
    run_extract_check_report(&mut env).map_err(TestError::from)
}

// Chapter 30, Figure 4: random_test picks its tester with three visible lines
#[rustdv::test]
async fn random_test(ctx: TestCtx) -> Result<(), TestError> {
    // Run with random operands
    let rng = ctx.rng();
    run_test(
        &ctx,
        Box::new(move |bfm| Box::new(TesterComp::new(bfm, RandomTester { rng }))),
    )
    .await
}

// Chapter 30, Figure 5: max_test differs only in the maker it sends
#[rustdv::test]
async fn max_test(ctx: TestCtx) -> Result<(), TestError> {
    // Run with maximum operands
    run_test(&ctx, Box::new(|bfm| Box::new(TesterComp::new(bfm, MaxTester)))).await
}
