//! Chapter 25: uvm_env testbench 4.0 — components in an environment.
//!
//!     sim-common/run_sim.sh ch25_uvm_env_testbench_4_0 tinyalu \
//!         sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use rustdv::prelude::*;
use tinyalu_utils::tb2::{MaxTester, RandomTester, Tester};
use tinyalu_utils::{alu_prediction, CmdTuple, Ops, TinyAluBfm};

rustdv::vpi_bootstrap!();

// Chapter 25, Figure 2: The tester as a component — start() is its run phase
pub struct TesterComp<T: Tester + 'static> {
    bfm: Rc<TinyAluBfm>,
    tester: Option<T>,
}

impl<T: Tester + 'static> TesterComp<T> {
    pub fn new(bfm: Rc<TinyAluBfm>, tester: T) -> TesterComp<T> {
        TesterComp { bfm, tester: Some(tester) }
    }
}

impl<T: Tester + 'static> Component for TesterComp<T> {
    fn start(&mut self, ctx: &mut RustdvCtx) {
        let bfm = self.bfm.clone();
        let mut tester = self.tester.take().expect("tester started twice");
        let obj = ctx.raise_objection("tester stimulus");
        spawn_named(
            async move {
                tester.execute(&bfm).await;
                drop(obj); // stimulus done: release the run phase
            },
            "tester.run",
        );
    }
}

impl<T: Tester + 'static> ComponentNode for TesterComp<T> {
    fn node_name(&self) -> &'static str {
        "TesterComp"
    }
    fn visit_children(&mut self, _f: &mut dyn FnMut(&str, &mut dyn ComponentNode)) {}
}

// Chapter 25, Figure 4: The Scoreboard as a component
pub struct Scoreboard {
    bfm: Rc<TinyAluBfm>,
    cmds: Rc<RefCell<Vec<CmdTuple>>>,
    results: Rc<RefCell<Vec<u64>>>,
    cvg: HashSet<Ops>,
}

impl Scoreboard {
    pub fn new(bfm: Rc<TinyAluBfm>) -> Scoreboard {
        Scoreboard {
            bfm,
            cmds: Rc::new(RefCell::new(Vec::new())),
            results: Rc::new(RefCell::new(Vec::new())),
            cvg: HashSet::new(),
        }
    }
}

impl Component for Scoreboard {
    // Chapter 25, Figure 5: start() launches the monitoring tasks
    fn start(&mut self, _ctx: &mut RustdvCtx) {
        let (bfm, cmds) = (self.bfm.clone(), self.cmds.clone());
        spawn_named(
            async move {
                loop {
                    let cmd = bfm.get_cmd().await;
                    cmds.borrow_mut().push(cmd);
                }
            },
            "scoreboard.get_cmd",
        );
        let (bfm, results) = (self.bfm.clone(), self.results.clone());
        spawn_named(
            async move {
                loop {
                    let result = bfm.get_result().await;
                    results.borrow_mut().push(result);
                }
            },
            "scoreboard.get_result",
        );
    }

    // Chapter 25, Figure 6: Checking results in the check phase
    fn check(&mut self, errors: &mut CheckSink) {
        let mut results = self.results.borrow_mut();
        for cmd in self.cmds.borrow().iter() {
            let (aa, bb, op_int) = *cmd;
            let op = Ops::from_u64(op_int).expect("illegal op captured");
            self.cvg.insert(op);
            let actual = results.remove(0) as u16;
            let prediction = alu_prediction(aa as u8, bb as u8, op);
            if actual == prediction {
                log::info(&format!("PASSED: {aa:02x} {op:?} {bb:02x} = {actual:04x}"));
            } else {
                errors.error(format!(
                    "FAILED: {aa:02x} {op:?} {bb:02x} = {actual:04x} - predicted {prediction:04x}"
                ));
            }
        }
        if Ops::ALL.iter().any(|op| !self.cvg.contains(op)) {
            errors.error("Functional coverage error: missed operations".to_string());
        } else {
            log::info("Covered all operations");
        }
    }
}

impl ComponentNode for Scoreboard {
    fn node_name(&self) -> &'static str {
        "Scoreboard"
    }
    fn visit_children(&mut self, _f: &mut dyn FnMut(&str, &mut dyn ComponentNode)) {}
}

// Chapter 25, Figure 8: The environment: a struct whose fields are the testbench
#[derive(rustdv::Component)]
pub struct AluEnv<T: Tester + 'static> {
    #[component(child)]
    tester: TesterComp<T>,
    #[component(child)]
    scoreboard: Scoreboard,
}

impl<T: Tester + 'static> AluEnv<T> {
    pub fn new(bfm: Rc<TinyAluBfm>, tester: T) -> AluEnv<T> {
        // build: construct the children; connect: hand them the BFM.
        AluEnv {
            tester: TesterComp::new(bfm.clone(), tester),
            scoreboard: Scoreboard::new(bfm),
        }
    }
}

impl<T: Tester + 'static> Component for AluEnv<T> {}

// Chapter 25, Figure 9: The shared test body: build the env, run the lifecycle
async fn run_env_test<T: Tester + 'static>(
    ctx: &RustdvCtx,
    tester: T,
) -> Result<(), TestError> {
    Clock::new(&ctx.dut().signal("clk")?, SimDuration::ns(10)).start();
    let bfm = Rc::new(TinyAluBfm::new(&ctx.dut())?);
    bfm.reset().await;
    bfm.start_tasks();

    let mut env = AluEnv::new(bfm, tester);

    let mut run_ctx = RustdvCtx::new();
    start_all(&mut env, &mut run_ctx);
    run_ctx.all_objections_dropped().await;

    run_extract_check_report(&mut env).map_err(TestError::from)
}

// Chapter 25, Figure 10: The tests build the right environment
#[rustdv::test]
async fn random_test(ctx: RustdvCtx) -> Result<(), TestError> {
    // Run with random operands
    run_env_test(&ctx, RandomTester { rng: ctx.rng() }).await
}

#[rustdv::test]
async fn max_test(ctx: RustdvCtx) -> Result<(), TestError> {
    // Run with max operands
    run_env_test(&ctx, MaxTester).await
}
