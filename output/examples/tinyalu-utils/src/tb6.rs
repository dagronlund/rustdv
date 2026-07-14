//! Testbench 6.0 components (Chapters 33–34): single-purpose components
//! communicating through channels and analysis ports.

use std::cell::RefCell;
use std::collections::HashMap;
use std::pin::Pin;
use std::rc::Rc;

use rustdv::prelude::*;

use crate::tb2::Tester;
use crate::{alu_prediction, CmdTuple, Ops, TinyAluBfm};

/// The command as the stimulus side creates it.
pub type Cmd = (u8, u8, Ops);

// Chapter 33, Figure 2: The tester writes to a Sender, not to the BFM
pub struct TesterComp<T: Tester + 'static> {
    tx: Sender<Cmd>,
    tester: Option<T>,
}

impl<T: Tester + 'static> TesterComp<T> {
    pub fn new(tx: Sender<Cmd>, tester: T) -> TesterComp<T> {
        TesterComp { tx, tester: Some(tester) }
    }
}

impl<T: Tester + 'static> Component for TesterComp<T> {
    fn start(&mut self, ctx: &mut RunCtx) {
        let tx = self.tx.clone();
        let mut tester = self.tester.take().expect("tester started twice");
        let obj = ctx.raise_objection("tester stimulus");
        spawn_named(
            async move {
                for op in Ops::ALL {
                    let (aa, bb) = tester.get_operands();
                    tx.send((aa, bb, op)).await.expect("driver dropped");
                }
                // twenty clocks: let the last operations drain through the
                // channel, the BFM queue, and the DUT (pyuvm waited ten;
                // our pipeline is one buffered stage deeper)
                Timer::ns(200).await;
                drop(obj);
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

// Chapter 33, Figure 3: The Driver: from the Receiver to the pins
#[derive(rustdv::Component)]
pub struct Driver {
    bfm: Rc<TinyAluBfm>,
    rx: Option<Receiver<Cmd>>,
}

impl Driver {
    pub fn new(bfm: Rc<TinyAluBfm>, rx: Receiver<Cmd>) -> Driver {
        Driver { bfm, rx: Some(rx) }
    }
}

impl Component for Driver {
    fn start(&mut self, _ctx: &mut RunCtx) {
        let bfm = self.bfm.clone();
        let rx = self.rx.take().expect("driver started twice");
        spawn_named(
            async move {
                bfm.reset().await;
                bfm.start_tasks();
                while let Ok((aa, bb, op)) = rx.recv().await {
                    bfm.send_op(aa, bb, op).await;
                }
            },
            "driver.run",
        );
    }
}

// Chapter 33, Figure 5: One Monitor type; the function is the argument
pub type GetFn<T> = fn(Rc<TinyAluBfm>) -> Pin<Box<dyn std::future::Future<Output = T>>>;

pub struct Monitor<T: 'static> {
    name: &'static str,
    bfm: Rc<TinyAluBfm>,
    get: GetFn<T>,
    ap: AnalysisPort<T>,
}

impl<T: std::fmt::Debug + 'static> Monitor<T> {
    pub fn new(
        name: &'static str,
        bfm: Rc<TinyAluBfm>,
        get: GetFn<T>,
        ap: AnalysisPort<T>,
    ) -> Monitor<T> {
        Monitor { name, bfm, get, ap }
    }
}

impl<T: std::fmt::Debug + 'static> Component for Monitor<T> {
    fn start(&mut self, _ctx: &mut RunCtx) {
        let (name, bfm, get, ap) = (self.name, self.bfm.clone(), self.get, self.ap.clone());
        spawn_named(
            async move {
                loop {
                    let datum = get(bfm.clone()).await;
                    log::info(&format!("{name}: {datum:?}"));
                    ap.write(&datum);
                }
            },
            "monitor.run",
        );
    }
}

impl<T: std::fmt::Debug + 'static> ComponentNode for Monitor<T> {
    fn node_name(&self) -> &'static str {
        "Monitor"
    }
    fn visit_children(&mut self, _f: &mut dyn FnMut(&str, &mut dyn ComponentNode)) {}
}

// Chapter 33, Figure 6: The two BFM methods, as passable functions
pub fn get_cmd(bfm: Rc<TinyAluBfm>) -> Pin<Box<dyn std::future::Future<Output = CmdTuple>>> {
    Box::pin(async move { bfm.get_cmd().await })
}

pub fn get_result(bfm: Rc<TinyAluBfm>) -> Pin<Box<dyn std::future::Future<Output = u64>>> {
    Box::pin(async move { bfm.get_result().await })
}

// Chapter 33, Figure 7: Coverage is a Subscriber and a Component
struct CovCollector {
    cvg: HashMap<Ops, usize>,
}

impl Subscriber<CmdTuple> for CovCollector {
    fn write(&mut self, cmd: &CmdTuple) {
        if let Some(op) = Ops::from_u64(cmd.2) {
            *self.cvg.entry(op).or_insert(0) += 1;
        }
    }
}

pub struct Coverage {
    collector: Rc<RefCell<CovCollector>>,
}

impl Coverage {
    /// connect: subscribes to the command analysis port.
    pub fn new(cmd_ap: &AnalysisPort<CmdTuple>) -> Coverage {
        let collector = Rc::new(RefCell::new(CovCollector { cvg: HashMap::new() }));
        cmd_ap.connect(collector.clone());
        Coverage { collector }
    }
}

impl Component for Coverage {
    fn check(&mut self, errors: &mut CheckSink) {
        let cvg = &self.collector.borrow().cvg;
        if Ops::ALL.iter().any(|op| !cvg.contains_key(op)) {
            errors.error("Functional coverage error: missed operations".to_string());
        } else {
            log::info("Covered all operations");
        }
    }
}

impl ComponentNode for Coverage {
    fn node_name(&self) -> &'static str {
        "Coverage"
    }
    fn visit_children(&mut self, _f: &mut dyn FnMut(&str, &mut dyn ComponentNode)) {}
}

// Chapter 33, Figure 8: The Scoreboard drains its analysis FIFOs in check
#[derive(rustdv::Component)]
pub struct Scoreboard {
    cmd_fifo: AnalysisFifo<CmdTuple>,
    result_fifo: AnalysisFifo<u64>,
}

impl Scoreboard {
    pub fn new(cmd_fifo: AnalysisFifo<CmdTuple>, result_fifo: AnalysisFifo<u64>) -> Scoreboard {
        Scoreboard { cmd_fifo, result_fifo }
    }
}

impl Component for Scoreboard {
    fn check(&mut self, errors: &mut CheckSink) {
        while let Some(cmd) = self.cmd_fifo.try_get() {
            let (aa, bb, op_int) = cmd;
            let op = Ops::from_u64(op_int).expect("illegal op captured");
            let Some(actual) = self.result_fifo.try_get() else {
                errors.error(format!("Missing result for command {cmd:?}"));
                break;
            };
            let actual = actual as u16;
            let prediction = alu_prediction(aa as u8, bb as u8, op);
            if actual == prediction {
                log::info(&format!("PASSED: {aa:02x} {op:?} {bb:02x} = {actual:04x}"));
            } else {
                errors.error(format!(
                    "FAILED: {aa:02x} {op:?} {bb:02x} = {actual:04x} - predicted {prediction:04x}"
                ));
            }
        }
    }
}
