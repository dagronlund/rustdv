//! Chapter 32: Analysis ports — one write, every subscriber hears it.
//!
//!     sim-common/run_sim.sh ch32_analysis_ports playground
//!
//! ============================================================================
//! ASPIRATIONAL — the target API (D1/D2), written before the framework compiles
//! it. Replaces the pre-restoration `AnalysisPort::new()` / `ap.connect(rc)`
//! design with the component-and-registry model used across the TLM chapters.
//! ============================================================================
//!
//! ## The model (D17, D23, D24)
//!
//! Analysis is the *other* TLM shape: one-to-many broadcast, non-blocking, no
//! return value, no back-pressure. A source calls `ap.write(&item)` and every
//! connected subscriber's `write` runs; the source neither blocks nor learns
//! how many are listening (zero is legal). This is a different mechanism from
//! put/get (Chapter 31), not a mode of it — in pyuvm it is a separate class
//! whose `connect` appends to a subscriber list and whose `write` loops it.
//!
//! A subscriber implements `Subscriber<T>` (one `write` method) — the port of
//! `uvm_subscriber`. Its incoming endpoint is an `AnalysisExport<T>`, declared
//! with `#[port(analysis)]` so it registers by path like any port.
//!
//! ## One connection pattern for both TLM shapes (Ray, 2026-07-24)
//!
//! Put/get is wired by the concrete FIFO between the two components. Analysis
//! has no such intermediary in the UVM — a source's analysis port broadcasts
//! straight to subscribers — and since both components are erased
//! `RustdvComp`s, neither side can drive the call.
//!
//! **rustdv gives analysis a hub too.** An `AnalysisFifo` is a concrete
//! `#[component(fifo)]` child with two named export accessors:
//!
//! ```ignore
//! self.analysis_fifo.pub_export().connect(&self.mon, Monitor::PUB_PORT);
//! self.analysis_fifo.sub_export().connect(&self.sb,  Scoreboard::SUB_PORT);
//! ```
//!
//! `pub_export()` takes the publisher's analysis port; `sub_export()` takes a
//! subscriber's. Several subscribers connect to the same `sub_export()` — that
//! is what makes it broadcast. The pattern is now identical to Chapter 31's:
//! a concrete FIFO, a named export, `connect(component, PORT_NAME)`.
//!
//! This is a deliberate divergence: the UVM has no analysis FIFO in the path
//! (its `uvm_tlm_analysis_fifo` buffers a stream, it does not broker the
//! broadcast). We are not implementing IEEE 1800.2, and one connection idiom
//! for the reader to learn beats two.

use rustdv::prelude::*;

rustdv::vpi_bootstrap!();

// ===========================================================================
// Subscribers
// ===========================================================================

// Chapter 32, Figure 1: A subscriber counts what it sees.
//
// `Subscriber<T>` is one method, `write`. The incoming `AnalysisExport<T>` is
// the endpoint a source connects to; `#[port(analysis)]` registers it by path.
#[derive(Component, Default)]
struct Counter {
    #[port(analysis)]
    input: AnalysisExport<u32>,
    count: u32,
}

impl Subscriber<u32> for Counter {
    fn write(&mut self, _item: &u32, _ctx: &mut RustdvCtx) {
        self.count += 1;
    }
}

impl Component for Counter {
    fn report(&mut self, ctx: &mut RustdvCtx) {
        ctx.info(&format!("counted {} items", self.count));
    }
}

// Chapter 32, Figure 2: A second subscriber, logging each item — proof that one
// write reaches every listener.
#[derive(Component, Default)]
struct Logger {
    #[port(analysis)]
    input: AnalysisExport<u32>,
}

impl Subscriber<u32> for Logger {
    fn write(&mut self, item: &u32, ctx: &mut RustdvCtx) {
        ctx.info(&format!("logged {item}"));
    }
}

impl Component for Logger {}

// ===========================================================================
// The source and the broadcast
// ===========================================================================

// Chapter 32, Figure 3: A source holds an analysis port and writes to it.
//
// `ap.write(&n)` returns immediately no matter how many subscribers listen —
// including none.
#[derive(Component, Default)]
struct NumberGen {
    #[port(analysis)]
    ap: AnalysisPort<u32>,
}

impl Component for NumberGen {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("generating");
        for n in 0..3 {
            self.ap.write(&n); // broadcast; non-blocking
            ctx.info(&format!("wrote {n}"));
        }
        Ok(())
    }
}

// Chapter 32, Figure 4: One publisher, two subscribers, one hub.
//
// The `AnalysisFifo` brokers the broadcast: the publisher's port connects to
// `pub_export()`, and every subscriber connects to the same `sub_export()`.
// Connecting two subscribers to one `sub_export()` is what makes the write
// fan out — and the wiring reads exactly like Chapter 31's put/get.
#[rustdv::test]
#[derive(Component, Default)]
struct BroadcastTest {
    #[component(child)]
    source: RustdvComp,
    #[component(child)]
    counter: RustdvComp,
    #[component(child)]
    logger: RustdvComp,
    #[component(fifo)]
    analysis_fifo: AnalysisFifo<u32>,
}

impl Component for BroadcastTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.source = NumberGen::new_comp();
        self.counter = Counter::new_comp();
        self.logger = Logger::new_comp();
        self.analysis_fifo = AnalysisFifo::new();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        self.analysis_fifo.pub_export().connect(&self.source, NumberGen::AP);
        self.analysis_fifo.sub_export().connect(&self.counter, Counter::INPUT);
        self.analysis_fifo.sub_export().connect(&self.logger, Logger::INPUT);
    }
}

// ===========================================================================
// Beyond the book
// ===========================================================================

// Chapter 32, Figure 6: A hub with no subscribers is legal (D22).
//
// Unlike a put/get port, an analysis `sub_export()` has min cardinality 0:
// broadcasting to nobody is a valid state, so this elaborates and runs clean.
// The publisher writes into the void. (`pub_export()` is still connected —
// a hub with no publisher would be a wiring mistake.)
#[rustdv::test]
#[derive(Component, Default)]
struct NoSubscribersTest {
    #[component(child)]
    source: RustdvComp,
    #[component(fifo)]
    analysis_fifo: AnalysisFifo<u32>,
}

impl Component for NoSubscribersTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.source = NumberGen::new_comp();
        self.analysis_fifo = AnalysisFifo::new();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        self.analysis_fifo.pub_export().connect(&self.source, NumberGen::AP);
        // no sub_export() connection — legal for analysis
    }
}

// Chapter 32, Figure 8: The same hub also buffers — pull instead of push.
//
// A subscriber that cannot keep up inside `write` (or wants to consume on its
// own schedule) reads the hub as a *stream* instead of subscribing to it: the
// same `AnalysisFifo` offers `get_export()`, so a component with a `GetPort`
// pulls items at its own pace. This is the port of `uvm_tlm_analysis_fifo`, and
// it is how a scoreboard collects a command stream while comparing at leisure.
// One hub, three accessors: `pub_export()`, `sub_export()`, `get_export()`.
#[rustdv::test]
#[derive(Component, Default)]
struct BufferedTest {
    #[component(child)]
    source: RustdvComp,
    #[component(child)]
    drainer: RustdvComp,
    #[component(fifo)]
    afifo: AnalysisFifo<u32>,
}

impl Component for BufferedTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.source = NumberGen::new_comp();
        self.drainer = Drainer::new_comp();
        self.afifo = AnalysisFifo::new();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        self.afifo.pub_export().connect(&self.source, NumberGen::AP);
        // pulled, not pushed: the drainer gets from the hub on its own schedule
        self.afifo.get_export().connect(&self.drainer, Drainer::GET_PORT);
    }
}

// A component that pulls the buffered stream at its own pace.
#[derive(Component, Default)]
struct Drainer {
    #[port(get)]
    get_port: GetPort<u32>,
}

impl Component for Drainer {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("draining");
        for _ in 0..3 {
            let n = self.get_port.get().await;
            ctx.info(&format!("drained {n}"));
        }
        Ok(())
    }
}
