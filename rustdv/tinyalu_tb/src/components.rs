//! Testbench components (design-doc §7.2, per the book's 6.0
//! architecture): the driver pulls from its typed SeqItemPort; monitors
//! publish on analysis ports; the scoreboard subscribes and checks in the
//! check phase; coverage is a Subscriber.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use rustdv::prelude::*;
use rustdv::RustdvCtx;

use crate::alu_bfm::TinyAluBfm;
use crate::alu_item::{predict, AluCommand, AluResult, Ops};

// ===========================================================================
// Driver
// ===========================================================================

/// Port of uvm_driver (mapping row 40): typed transactions end runtime
/// type errors at the driver boundary. RSP = REQ (no response path; the
/// result monitor observes results).
///
/// `#[component(no_factory)]` here is **transitional, not an endorsement**
/// (D75). It is *not* that this component cannot be factory-built; it is that
/// it has not been converted yet. It still takes its BFM and TLM endpoints as
/// constructor arguments (the R3 style). The conversion is: BFM via the
/// ConfigDb (the config_db exists precisely to bridge the factory's
/// no-argument signature, D57/D65), TLM endpoints via the connect phase
/// (D17–D24) — after which the opt-out comes off and these register like any
/// other component. Deferred to the TLM/sequence chapters, where the whole
/// §7 example converts at once. The other `no_factory` marks below share
/// this note.
#[derive(rustdv::Component)]
#[component(no_factory)]
pub struct Driver {
    bfm: Rc<TinyAluBfm>,
    seq_item_port: Option<SeqItemPort<AluCommand>>,
}

impl Driver {
    pub fn new(bfm: Rc<TinyAluBfm>, seq_item_port: SeqItemPort<AluCommand>) -> Driver {
        Driver { bfm, seq_item_port: Some(seq_item_port) }
    }
}

impl Component for Driver {
    fn start(&mut self, _ctx: &mut RustdvCtx) {
        let bfm = self.bfm.clone();
        let mut port = self.seq_item_port.take().expect("Driver started twice");
        spawn_named(
            async move {
                loop {
                    let item = port.get_next_item().await;
                    bfm.send_op(item.payload().clone()).await;
                    port.item_done(None);
                }
            },
            "driver",
        );
    }
}

// ===========================================================================
// Monitors
// ===========================================================================

#[derive(rustdv::Component)]
#[component(no_factory)]
pub struct CmdMonitor {
    bfm: Rc<TinyAluBfm>,
    ap: AnalysisPort<AluCommand>,
}

impl CmdMonitor {
    pub fn new(bfm: Rc<TinyAluBfm>, ap: AnalysisPort<AluCommand>) -> CmdMonitor {
        CmdMonitor { bfm, ap }
    }
}

impl Component for CmdMonitor {
    fn start(&mut self, _ctx: &mut RustdvCtx) {
        let bfm = self.bfm.clone();
        let ap = self.ap.clone();
        spawn_named(
            async move {
                loop {
                    let cmd = bfm.get_cmd().await;
                    log::info(&format!("cmd_monitor: {cmd:?}"));
                    ap.write(&cmd);
                }
            },
            "cmd_monitor",
        );
    }
}

#[derive(rustdv::Component)]
#[component(no_factory)]
pub struct ResultMonitor {
    bfm: Rc<TinyAluBfm>,
    ap: AnalysisPort<AluResult>,
}

impl ResultMonitor {
    pub fn new(bfm: Rc<TinyAluBfm>, ap: AnalysisPort<AluResult>) -> ResultMonitor {
        ResultMonitor { bfm, ap }
    }
}

impl Component for ResultMonitor {
    fn start(&mut self, _ctx: &mut RustdvCtx) {
        let bfm = self.bfm.clone();
        let ap = self.ap.clone();
        spawn_named(
            async move {
                loop {
                    let res = bfm.get_result().await;
                    log::info(&format!("result_monitor: {res:?}"));
                    ap.write(&res);
                }
            },
            "result_monitor",
        );
    }
}

// ===========================================================================
// Scoreboard
// ===========================================================================

/// Scoreboards check in `check`, report in `report` (§7.3 convention 3).
/// Comparison policy lives here, not on the data type (review-memo R1):
/// the default comparator is `PartialEq` against the predictor's output.
#[derive(rustdv::Component)]
#[component(no_factory)]
pub struct Scoreboard {
    // Unbounded `TlmFifo`s, handed over by `AnalysisPort::connect_fifo()`.
    // The broadcast keeps nothing (D90) — the subscriber owns the storage.
    cmd_fifo: TlmFifo<AluCommand>,
    result_fifo: TlmFifo<AluResult>,
    compared: usize,
    mismatches: usize,
}

impl Scoreboard {
    pub fn new(cmd_fifo: TlmFifo<AluCommand>, result_fifo: TlmFifo<AluResult>) -> Scoreboard {
        Scoreboard { cmd_fifo, result_fifo, compared: 0, mismatches: 0 }
    }
}

impl Component for Scoreboard {
    fn check(&mut self, _ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        loop {
            match (self.cmd_fifo.try_get(), self.result_fifo.try_get()) {
                (Some(cmd), Some(actual)) => {
                    let expected = predict(&cmd);
                    self.compared += 1;
                    if expected != actual {
                        self.mismatches += 1;
                        log::info(&format!(
                            "scoreboard: in={cmd:?} out={actual:?} expected={expected:?} check=FAIL"
                        ));
                        errors.error(format!(
                            "scoreboard mismatch: {cmd:?} -> got {actual:?}, expected {expected:?}"
                        ));
                    } else {
                        log::info(&format!(
                            "scoreboard: in={cmd:?} out={actual:?} expected={expected:?} check=PASS"
                        ));
                    }
                }
                (None, None) => break,
                (Some(cmd), None) => {
                    errors.error(format!("scoreboard: command {cmd:?} has no result"));
                }
                (None, Some(res)) => {
                    errors.error(format!("scoreboard: result {res:?} has no command"));
                }
            }
        }
        if self.compared == 0 {
            errors.error("scoreboard: nothing was compared");
        }
    }

    fn report(&mut self, _ctx: &mut RustdvCtx) {
        log::info(&format!(
            "scoreboard: {} compared, {} mismatches",
            self.compared, self.mismatches
        ));
    }
}

// ===========================================================================
// Coverage
// ===========================================================================

struct CovCollector {
    seen: HashMap<Ops, usize>,
}

impl Subscriber<AluCommand> for CovCollector {
    fn write(&mut self, item: &AluCommand) {
        *self.seen.entry(item.op).or_insert(0) += 1;
    }
}

/// Functional coverage as a Subscriber (mapping row 41; book's Coverage
/// class): counts ops seen, errors in `check` if any op was never covered.
#[derive(rustdv::Component)]
#[component(no_factory)]
pub struct Coverage {
    collector: Rc<RefCell<CovCollector>>,
}

impl Coverage {
    /// `// connect:` subscribes to the command analysis port.
    pub fn new(cmd_ap: &AnalysisPort<AluCommand>) -> Coverage {
        let collector = Rc::new(RefCell::new(CovCollector { seen: HashMap::new() }));
        cmd_ap.connect(collector.clone());
        Coverage { collector }
    }
}

impl Component for Coverage {
    fn check(&mut self, _ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        let seen = &self.collector.borrow().seen;
        for op in Ops::ALL {
            if !seen.contains_key(&op) {
                errors.error(format!("coverage: op {op:?} was never exercised"));
            }
        }
    }

    fn report(&mut self, _ctx: &mut RustdvCtx) {
        let seen = &self.collector.borrow().seen;
        let mut parts: Vec<String> = Ops::ALL
            .iter()
            .map(|op| format!("{op:?}={}", seen.get(op).copied().unwrap_or(0)))
            .collect();
        parts.sort();
        log::info(&format!("coverage: {}", parts.join(" ")));
    }
}
