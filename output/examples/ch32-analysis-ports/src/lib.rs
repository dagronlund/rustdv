//! Chapter 32: Analysis ports — 1-to-many, never blocking.
//!
//!     sim-common/run_sim.sh ch32_analysis_ports playground

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use rustdv::prelude::*;
use tinyalu_utils::Ops;

rustdv::vpi_bootstrap!();

// Chapter 32, Figure 1: A Subscriber counts what it sees

struct OpCounter {
    counts: HashMap<Ops, u32>,
}

impl OpCounter {
    fn new() -> OpCounter {
        OpCounter { counts: HashMap::new() }
    }

    fn report(&self) {
        let mut ops: Vec<_> = self.counts.iter().collect();
        ops.sort_by_key(|(op, _)| **op as u8);
        let s: Vec<String> = ops.iter().map(|(op, n)| format!("{op:?}={n}")).collect();
        log::info(&format!("coverage: {}", s.join(" ")));
    }
}

impl Subscriber<Ops> for OpCounter {
    fn write(&mut self, item: &Ops) {
        *self.counts.entry(*item).or_insert(0) += 1;
    }
}

// Chapter 32, Figure 2: One write, every subscriber hears it
#[rustdv::test]
async fn fan_out_test(_ctx: RustdvCtx) -> Result<(), TestError> {
    let ap: AnalysisPort<Ops> = AnalysisPort::new();

    let coverage = Rc::new(RefCell::new(OpCounter::new()));
    let logger_sub = Rc::new(RefCell::new(OpLogger));
    ap.connect(coverage.clone());
    ap.connect(logger_sub);
    log::info(&format!("{} subscribers connected", ap.subscriber_count()));

    for op in [Ops::Add, Ops::Mul, Ops::Add] {
        ap.write(&op); // non-blocking, no matter how many listen
    }
    coverage.borrow().report();
    Ok(())
}

// Chapter 32, Figure 3: A second subscriber, three lines long
struct OpLogger;

impl Subscriber<Ops> for OpLogger {
    fn write(&mut self, item: &Ops) {
        log::info(&format!("saw {item:?}"));
    }
}

// Chapter 32, Figure 5: An AnalysisFifo turns broadcast into a stream
#[rustdv::test]
async fn analysis_fifo_test(_ctx: RustdvCtx) -> Result<(), TestError> {
    let ap: AnalysisPort<Ops> = AnalysisPort::new();
    let fifo: AnalysisFifo<Ops> = ap.connect_fifo();

    ap.write(&Ops::Xor);
    ap.write(&Ops::And);
    log::info(&format!("fifo holds {} items", fifo.len()));

    // A task drains the fifo at its own pace — write() never waited for it.
    let first = fifo.get().await;
    let second = fifo.get().await;
    log::info(&format!("drained {first:?} then {second:?}"));
    Ok(())
}

// Chapter 32, Figure 6: Zero subscribers is not an error
#[rustdv::test]
async fn no_subscribers_test(_ctx: RustdvCtx) -> Result<(), TestError> {
    let ap: AnalysisPort<Ops> = AnalysisPort::new();
    ap.write(&Ops::Add); // fire and forget: nobody listening, nobody hurt
    log::info("wrote to a port with no subscribers — fine");
    Ok(())
}
