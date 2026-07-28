//! Chapter 34: Connections — testbench 6.0, the pieces wired with TLM.
//!
//!     sim-common/run_sim.sh ch34_connections_testbench_6_0 tinyalu \
//!         sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
//!
//! ============================================================================
//! ASPIRATIONAL — the target API (D1/D2). This is the payoff chapter: put/get
//! (Chapter 31) and analysis broadcast (Chapter 32) wired into one working
//! TinyALU testbench, all connections resolved by path through the registry so
//! no parent reaches into an erased child.
//! ============================================================================
//!
//! ## Architecture (the book's 6.0)
//!
//! ```text
//!   Tester --put--> [cmd_fifo] --get--> Driver --> BFM --> DUT
//!
//!   CmdMonitor --pub--> [cmd_bus] --sub--> Scoreboard
//!                            \----sub----> Coverage
//!
//!   ResultMonitor --pub--> [result_bus] --sub--> Scoreboard
//! ```
//!
//! One idiom throughout: a concrete FIFO between the two components, a named
//! export, `connect(component, PORT_NAME)`. `TlmFifo` carries point-to-point
//! traffic; `AnalysisFifo` brokers a broadcast — several subscribers connect
//! to the same `sub_export()`.
//!
//! The Tester generates commands and *puts* them; the Driver *gets* them and
//! drives the BFM. Two monitors watch the bus and *broadcast* what they see;
//! the Scoreboard and Coverage *subscribe*. The BFM is the ambient singleton
//! (D57), the RTL self-clocks (D42).
//!
//! ## The Rust win worth noting (D20): multiple analysis inputs, no macros
//!
//! The Scoreboard needs two analysis streams — commands and results. SV cannot
//! give one class two `write` methods, so it needs the `uvm_analysis_imp_decl`
//! macros. Rust just implements `Subscriber<CmdTuple>` *and* `Subscriber<u64>`
//! on the same struct — two `write`s, distinguished by type. No macros, no imp
//! classes (the point pyuvm makes with multiple inheritance, made with traits).
//!
//! ## Depends on concurrent run + real objections (D56/D60)
//!
//! Tester, Driver, and both monitors have run phases that must run at once (the
//! Driver blocks on an empty cmd_fifo until the Tester puts). This is the same
//! increment Chapter 31 forces: `run_all` spawns each `run`, the phase ends on
//! objection consensus. Written assuming that has landed.

use std::collections::HashSet;

use rustdv::prelude::*;
use tinyalu_utils::{alu_prediction, CmdTuple, Ops, TinyAluBfm};

rustdv::vpi_bootstrap!();

// A command the tester hands the driver: operands + operation.
type Command = (u8, u8, Ops);

// ===========================================================================
// Stimulus: Tester -> cmd_fifo -> Driver
// ===========================================================================

// Chapter 34, Figure 1: The Tester puts commands into a FIFO.
#[derive(Component, Default)]
struct Tester {
    #[port(put)]
    cmd_port: PutPort<Command>,
    rng: Option<Rng>,
}

impl Component for Tester {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        self.rng = Some(ctx.rng());
    }

    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("stimulus");
        let rng = self.rng.as_mut().expect("build ran");
        for op in Ops::ALL {
            self.cmd_port.put((rng.u8(), rng.u8(), op)).await;
        }
        // two dummy ops flush the pipeline so the last real result lands
        self.cmd_port.put((0, 0, Ops::Add)).await;
        self.cmd_port.put((0, 0, Ops::Add)).await;
        Ok(())
    }
}

// Chapter 34, Figure 2: The Driver gets commands and drives the BFM.
#[derive(Component, Default)]
struct Driver {
    #[port(get)]
    cmd_port: GetPort<Command>,
}

impl Component for Driver {
    async fn run(&mut self, _ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let bfm = TinyAluBfm::get();
        bfm.reset().await;
        loop {
            let (aa, bb, op) = self.cmd_port.get().await; // blocks until a command
            bfm.send_op(aa, bb, op).await;
        }
    }
}

// ===========================================================================
// Observation: monitors broadcast, subscribers collect
// ===========================================================================

// Chapter 34, Figure 3: The command monitor watches the bus and broadcasts.
#[derive(Component, Default)]
struct CmdMonitor {
    #[port(analysis)]
    ap: AnalysisPort<CmdTuple>,
}

impl Component for CmdMonitor {
    async fn run(&mut self, _ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let bfm = TinyAluBfm::get();
        loop {
            let cmd = bfm.get_cmd().await;
            self.ap.write(&cmd);
        }
    }
}

// Chapter 34, Figure 4: The result monitor broadcasts results.
#[derive(Component, Default)]
struct ResultMonitor {
    #[port(analysis)]
    ap: AnalysisPort<u64>,
}

impl Component for ResultMonitor {
    async fn run(&mut self, _ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let bfm = TinyAluBfm::get();
        loop {
            let result = bfm.get_result().await;
            self.ap.write(&result);
        }
    }
}

// Chapter 34, Figure 5: The Scoreboard subscribes to BOTH streams.
//
// Two `Subscriber` impls, one per transaction type — the multiple-analysis-input
// pattern that needs no imp_decl macros (D20).
#[derive(Component, Default)]
struct Scoreboard {
    #[port(analysis)]
    cmd_in: AnalysisExport<CmdTuple>,
    #[port(analysis)]
    result_in: AnalysisExport<u64>,
    cmds: Vec<CmdTuple>,
    results: Vec<u64>,
    cvg: HashSet<Ops>,
}

impl Subscriber<CmdTuple> for Scoreboard {
    fn write(&mut self, cmd: &CmdTuple, _ctx: &mut RustdvCtx) {
        self.cmds.push(*cmd);
    }
}

impl Subscriber<u64> for Scoreboard {
    fn write(&mut self, result: &u64, _ctx: &mut RustdvCtx) {
        self.results.push(*result);
    }
}

impl Component for Scoreboard {
    fn check(&mut self, ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        for (cmd, result) in self.cmds.iter().zip(self.results.iter()) {
            let (aa, bb, op_int) = *cmd;
            let op = Ops::from_u64(op_int).expect("legal op");
            self.cvg.insert(op);
            let actual = *result as u16;
            let prediction = alu_prediction(aa as u8, bb as u8, op);
            if actual == prediction {
                ctx.info(&format!("PASSED: {aa:02x} {op:?} {bb:02x} = {actual:04x}"));
            } else {
                errors.error(format!(
                    "FAILED: {aa:02x} {op:?} {bb:02x} = {actual:04x} - predicted {prediction:04x}"
                ));
            }
        }
        if Ops::ALL.iter().any(|op| !self.cvg.contains(op)) {
            errors.error("Functional coverage error: missed operations".to_string());
        } else {
            ctx.info("Covered all operations");
        }
    }
}

// Chapter 34, Figure 6: Coverage subscribes to the command stream only.
#[derive(Component, Default)]
struct Coverage {
    #[port(analysis)]
    cmd_in: AnalysisExport<CmdTuple>,
    cvg: HashSet<Ops>,
}

impl Subscriber<CmdTuple> for Coverage {
    fn write(&mut self, cmd: &CmdTuple, _ctx: &mut RustdvCtx) {
        if let Some(op) = Ops::from_u64(cmd.2) {
            self.cvg.insert(op);
        }
    }
}

impl Component for Coverage {
    fn report(&mut self, ctx: &mut RustdvCtx) {
        ctx.info(&format!("coverage saw {} of {} ops", self.cvg.len(), Ops::ALL.len()));
    }
}

// ===========================================================================
// The environment wires it all together
// ===========================================================================

// Chapter 34, Figure 7: build the components and the FIFOs; connect in one
// place. **Every connection is the same shape** — a concrete FIFO, a named
// export, and `connect(component, PORT_NAME)` — whether the traffic is
// point-to-point (`TlmFifo`) or broadcast (`AnalysisFifo`). Nothing reaches
// into an erased child; every endpoint resolves by path.
#[derive(Component, Default)]
struct AluEnv {
    #[component(child)]
    tester: RustdvComp,
    #[component(child)]
    driver: RustdvComp,
    #[component(child)]
    cmd_mon: RustdvComp,
    #[component(child)]
    result_mon: RustdvComp,
    #[component(child)]
    scoreboard: RustdvComp,
    #[component(child)]
    coverage: RustdvComp,
    #[component(fifo)]
    cmd_fifo: TlmFifo<Command>,
    #[component(fifo)]
    cmd_bus: AnalysisFifo<CmdTuple>, // the command broadcast, two subscribers
    #[component(fifo)]
    result_bus: AnalysisFifo<u64>, // the result broadcast, one subscriber
}

impl Component for AluEnv {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.tester = Tester::new_comp();
        self.driver = Driver::new_comp();
        self.cmd_mon = CmdMonitor::new_comp();
        self.result_mon = ResultMonitor::new_comp();
        self.scoreboard = Scoreboard::new_comp();
        self.coverage = Coverage::new_comp();
        self.cmd_fifo = TlmFifo::new(1);
        self.cmd_bus = AnalysisFifo::new();
        self.result_bus = AnalysisFifo::new();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        // stimulus: Tester --put--> cmd_fifo --get--> Driver
        self.cmd_fifo.put_export().connect(&self.tester, Tester::CMD_PORT);
        self.cmd_fifo.get_export().connect(&self.driver, Driver::CMD_PORT);

        // commands: CmdMonitor publishes; Scoreboard and Coverage subscribe
        self.cmd_bus.pub_export().connect(&self.cmd_mon, CmdMonitor::AP);
        self.cmd_bus.sub_export().connect(&self.scoreboard, Scoreboard::CMD_IN);
        self.cmd_bus.sub_export().connect(&self.coverage, Coverage::CMD_IN);

        // results: ResultMonitor publishes; only the Scoreboard subscribes
        self.result_bus.pub_export().connect(&self.result_mon, ResultMonitor::AP);
        self.result_bus.sub_export().connect(&self.scoreboard, Scoreboard::RESULT_IN);
    }

    fn start_of_simulation(&mut self, _ctx: &mut RustdvCtx) {
        TinyAluBfm::get().start_tasks();
    }
}

// Chapter 34, Figure 8: the test is just the env.
#[rustdv::test]
#[derive(Component, Default)]
struct AluTest {
    #[component(child)]
    env: RustdvComp,
}

impl Component for AluTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.env = AluEnv::new_comp();
    }
}
