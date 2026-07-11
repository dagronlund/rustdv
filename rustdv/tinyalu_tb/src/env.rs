//! The environment: a plain struct of children — the ownership tree IS
//! the component tree (design-doc D5.2). Constructors do build (children)
//! and connect (channel endpoints as arguments) — review-memo R3.

use std::rc::Rc;

use rustdv::prelude::*;

use crate::alu_bfm::TinyAluBfm;
use crate::alu_item::AluCommand;
use crate::components::{CmdMonitor, Coverage, Driver, ResultMonitor, Scoreboard};

/// Typed config tree (design-doc §5.4): nesting mirrors the hierarchy;
/// the shared BFM is an Rc field; active/passive is an enum, so a passive
/// env simply has no driver — the illegal state is unrepresentable.
pub struct AluEnvConfig {
    pub bfm: Rc<TinyAluBfm>,
    pub is_active: Active,
    pub enable_coverage: bool,
}

#[derive(rustdv::Component)]
pub struct AluEnv {
    seqr: Sequencer<AluCommand>,
    #[component(child)]
    driver: Option<Driver>,
    #[component(child)]
    cmd_mon: CmdMonitor,
    #[component(child)]
    result_mon: ResultMonitor,
    #[component(child)]
    scoreboard: Scoreboard,
    #[component(child)]
    coverage: Option<Coverage>,
}

impl AluEnv {
    pub fn new(config: AluEnvConfig) -> AluEnv {
        // build: construct children bottom-up in one pass.
        let seqr: Sequencer<AluCommand> = Sequencer::new();
        let cmd_ap: AnalysisPort<AluCommand> = AnalysisPort::new();
        let result_ap = AnalysisPort::new();

        // connect: endpoints are constructor arguments; a missing
        // connection is a missing argument — a compile error.
        let cmd_fifo = cmd_ap.connect_fifo();
        let result_fifo = result_ap.connect_fifo();
        let scoreboard = Scoreboard::new(cmd_fifo, result_fifo);

        let coverage = config.enable_coverage.then(|| Coverage::new(&cmd_ap));

        let driver = match config.is_active {
            Active::Active => Some(Driver::new(config.bfm.clone(), seqr.seq_item_port())),
            Active::Passive => None,
        };

        let cmd_mon = CmdMonitor::new(config.bfm.clone(), cmd_ap);
        let result_mon = ResultMonitor::new(config.bfm, result_ap);

        AluEnv { seqr, driver, cmd_mon, result_mon, scoreboard, coverage }
    }

    /// Clonable sequencer handle for the test to start sequences on.
    pub fn sequencer(&self) -> Sequencer<AluCommand> {
        self.seqr.clone()
    }
}

impl Component for AluEnv {}
