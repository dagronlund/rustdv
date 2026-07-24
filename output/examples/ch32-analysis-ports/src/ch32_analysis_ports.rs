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
//! ## Connecting analysis has no FIFO hub — DESIGN POINT to confirm
//!
//! Put/get is wired by the concrete FIFO that sits between the two components
//! (`fifo.put_export().connect((&comp, C::PORT))`). Analysis has no such
//! intermediary: a source component's analysis port broadcasts straight to
//! subscriber components, and *both are erased* `RustdvComp`s. So there is no
//! concrete side to drive the call. This spec wires analysis with a registry
//! function that resolves *both* endpoints by path:
//!
//! ```ignore
//! Analysis::connect((&self.source, NumberGen::AP), (&self.counter, Counter::IN));
//! ```
//!
//! Open question for Ray: keep two connection forms (concrete-hub `connect` for
//! put/get, symmetric `Analysis::connect` for analysis), or unify everything to
//! the symmetric two-endpoint form? Put/get reads better with the FIFO driving;
//! analysis cannot use that shape. Left as two forms here, flagged.

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

// Chapter 32, Figure 4: One port, two subscribers. Each `write` reaches both.
#[rustdv::test]
#[derive(Component, Default)]
struct BroadcastTest {
    #[component(child)]
    source: RustdvComp,
    #[component(child)]
    counter: RustdvComp,
    #[component(child)]
    logger: RustdvComp,
}

impl Component for BroadcastTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.source = NumberGen::new_comp();
        self.counter = Counter::new_comp();
        self.logger = Logger::new_comp();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        // one source port -> many subscribers, wired by path (no FIFO hub)
        Analysis::connect((&self.source, NumberGen::AP), (&self.counter, Counter::INPUT));
        Analysis::connect((&self.source, NumberGen::AP), (&self.logger, Logger::INPUT));
    }
}

// ===========================================================================
// Beyond the book
// ===========================================================================

// Chapter 32, Figure 6: An analysis port with no subscribers is legal (D22).
//
// Unlike a put/get port (min cardinality 1 — Chapter 31, Figure 10), an
// analysis port has min cardinality 0: broadcasting to nobody is a valid state,
// so this test elaborates and runs clean. The source writes into the void.
#[rustdv::test]
#[derive(Component, Default)]
struct NoSubscribersTest {
    #[component(child)]
    source: RustdvComp,
}

impl Component for NoSubscribersTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.source = NumberGen::new_comp();
    }
    // No `connect` — and that is fine for analysis, unlike put/get.
}

// Chapter 32, Figure 8: An AnalysisFifo turns a broadcast into a pull stream.
//
// A subscriber that cannot keep up in `write` (or wants to consume on its own
// schedule) buffers the broadcast in an `AnalysisFifo` and `get`s from it — the
// port of `uvm_tlm_analysis_fifo`. This is how a scoreboard collects a command
// stream while comparing at its own pace. The AnalysisFifo is a concrete hub
// (like a `TlmFifo`), so it drives the connect: its analysis endpoint subscribes
// to the source's port, and its `get_export` feeds the consumer.
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
        // source port -> the fifo's analysis endpoint (fifo drives, by path)
        self.afifo.analysis_export().connect((&self.source, NumberGen::AP));
        // the fifo's get side -> the drainer's get port
        self.afifo.get_export().connect((&self.drainer, Drainer::GET_PORT));
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
