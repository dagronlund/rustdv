//! Chapter 31: Component communications — put, get, and peek through a FIFO.
//!
//!     sim-common/run_sim.sh ch31_component_communications playground
//!
//! ============================================================================
//! ASPIRATIONAL — the *target* API (D1/D2), written before the framework can
//! compile it. rustdv is to be changed to satisfy this, replacing the
//! pre-restoration `channel`/`Sender`/`Receiver` design (which "channels
//! replace TLM-1" — the destroyed model this branch exists to undo). Do not
//! wire this into the build until the framework satisfies it.
//! ============================================================================
//!
//! ## The model (D17–D24)
//!
//! TLM 1.0 point-to-point: a **port** in one component is the thing you *call*
//! (`put`, `get`, `peek`); it forwards to an **export** on a FIFO; the FIFO
//! owns a `Queue` and implements every capability against it. Two components
//! connect to the *same FIFO* and neither learns the other exists — the FIFO is
//! the decoupling point (§0.3.2). Each capability has a blocking form (waits on
//! full/empty) and a non-blocking form (`try_*`/`can_*`).
//!
//! ## Connection without reaching into an erased child (the registry model)
//!
//! A component is a factory `RustdvComp`, so a parent cannot reach `producer.put_port`
//! by field. Instead `#[port(...)]` makes each port **register itself** by
//! (component path, field name) during build, and connection is a registry
//! *lookup*: `fifo.put_export().connect((&self.producer, Producer::PUT_PORT))`.
//! `#[port(put)] put_port` also generates the `Producer::PUT_PORT` constant — a
//! typed `PortName<u32>` — so the name is typo-proof (a misspelled constant does
//! not compile) and the export's transaction type is checked against the port's
//! at the connect. The instance path is the only part resolved at runtime; the
//! constant's value and the registration key both come from the one field, so
//! they cannot drift. Nothing reaches into the child.
//!
//! The FIFO is a concrete `#[component(fifo)]` child (reachable to call
//! `put_export()` on) — the model closest to UVM, where the FIFO is a real
//! component with a path. It is a deliberate carve-out: a FIFO is plumbing,
//! never a factory-override target (like the BFM, D33). Every declared port must
//! be connected by end of elaboration or elaboration fails (D22).
//!
//! ## DEPENDENCY THIS EXAMPLE FORCES (read before building)
//!
//! Put/get through a size-1 FIFO **requires two run phases running at once**:
//! the producer blocks on a full FIFO and can only proceed once the consumer
//! drains it. Under the current sequential `run_all` (D56) this **deadlocks** —
//! the producer's `run` is awaited to completion before the consumer's ever
//! starts, and it never completes. Component Communications is therefore the
//! testbench D56/D60 anticipated: *"concurrent run phases + meaningful
//! objections arrive together... when a testbench first needs two long-running
//! run phases at once."* Building ch31 means undoing that deferral first:
//! `run_all` must spawn each `run` and the phase must end on objection
//! consensus, not on sequential completion. This example is written assuming
//! that increment has landed.

use rustdv::prelude::*;

rustdv::vpi_bootstrap!();

// ===========================================================================
// Blocking put / get / peek
// ===========================================================================

// Chapter 31, Figure 1: A producer holds a put port and blocks on a full FIFO.
//
// `#[port(put)]` registers `put_port` under this component's path + "put_port"
// so the env can wire it without reaching in. `PutPort<u32>` offers blocking
// `put().await` (here) and non-blocking `try_put`/`can_put` (Figure 6).
#[derive(Component, Default)]
struct Producer {
    #[port(put)]
    put_port: PutPort<u32>,
}

impl Component for Producer {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("producing");
        for n in 0..3 {
            self.put_port.put(n).await; // blocks while the FIFO is full
            ctx.info(&format!("put {n}"));
        }
        Ok(())
    }
}

// Chapter 31, Figure 2: A consumer that peeks, then gets.
//
// Two ports on one component, both wired to the same FIFO: `peek` returns the
// next item **without** consuming it (blocking until one is there), and `get`
// then consumes that same item. The assert makes the "peek does not consume"
// contract executable.
#[derive(Component, Default)]
struct Consumer {
    #[port(peek)]
    peek_port: PeekPort<u32>,
    #[port(get)]
    get_port: GetPort<u32>,
}

impl Component for Consumer {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("consuming");
        for _ in 0..3 {
            let seen = self.peek_port.peek().await; // blocks while empty; no consume
            ctx.info(&format!("peeked {seen}"));
            let got = self.get_port.get().await; // consumes the peeked item
            assert_eq!(seen, got, "peek must not consume the item");
            ctx.info(&format!("got {got}"));
        }
        Ok(())
    }
}

// Chapter 31, Figure 4: The env builds the two components and a FIFO, then wires
// them in `connect`.
//
// The producer and consumer are ordinary factory `RustdvComp` children. The FIFO
// is a concrete `#[component(fifo)]` child so its exports are reachable. Every
// `connect` is port -> export resolved by (path, name): the export never
// reaches into the erased child.
#[rustdv::test]
#[derive(Component, Default)]
struct PutGetPeekTest {
    #[component(child)]
    producer: RustdvComp,
    #[component(child)]
    consumer: RustdvComp,
    #[component(fifo)]
    fifo: TlmFifo<u32>,
}

impl Component for PutGetPeekTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.producer = Producer::new_comp();
        self.consumer = Consumer::new_comp();
        self.fifo = TlmFifo::new(1); // depth 1 — forces the producer to wait
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        self.fifo.put_export().connect((&self.producer, Producer::PUT_PORT));
        self.fifo.peek_export().connect((&self.consumer, Consumer::PEEK_PORT));
        self.fifo.get_export().connect((&self.consumer, Consumer::GET_PORT));
    }
}

// Expected transcript (both run phases concurrent, objection-gated):
//   put 0 / peeked 0 / got 0 / put 1 / peeked 1 / got 1 / put 2 / peeked 2 /
//   got 2   (interleaving may vary; every item is peeked before it is got, and
//   got exactly once)   REGRESSION: PASS

// ===========================================================================
// Non-blocking put / get
// ===========================================================================

// Chapter 31, Figure 6: A non-blocking producer never waits — `try_put` returns
// false when the FIFO is full, and the producer decides what to do (here, yield
// and retry).
#[derive(Component, Default)]
struct NbProducer {
    #[port(put)]
    put_port: PutPort<u32>,
}

impl Component for NbProducer {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("producing (nb)");
        for n in 0..3 {
            while !self.put_port.try_put(n) {
                ctx.info("FIFO full, retrying");
                Timer::ns(1).await;
            }
            ctx.info(&format!("put {n}"));
        }
        Ok(())
    }
}

// Chapter 31, Figure 7: A non-blocking consumer — `try_get` returns `None` when
// the FIFO is empty.
#[derive(Component, Default)]
struct NbConsumer {
    #[port(get)]
    get_port: GetPort<u32>,
}

impl Component for NbConsumer {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("consuming (nb)");
        let mut seen = 0;
        while seen < 3 {
            match self.get_port.try_get() {
                Some(n) => {
                    ctx.info(&format!("got {n}"));
                    seen += 1;
                }
                None => Timer::ns(1).await,
            }
        }
        Ok(())
    }
}

// Chapter 31, Figure 8: Same wiring, non-blocking components.
#[rustdv::test]
#[derive(Component, Default)]
struct NonBlockingTest {
    #[component(child)]
    producer: RustdvComp,
    #[component(child)]
    consumer: RustdvComp,
    #[component(fifo)]
    fifo: TlmFifo<u32>,
}

impl Component for NonBlockingTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.producer = NbProducer::new_comp();
        self.consumer = NbConsumer::new_comp();
        self.fifo = TlmFifo::new(1);
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        self.fifo.put_export().connect((&self.producer, NbProducer::PUT_PORT));
        self.fifo.get_export().connect((&self.consumer, NbConsumer::GET_PORT));
    }
}

// ===========================================================================
// Beyond the book — the checks a registry connection makes possible
// ===========================================================================

// Chapter 31, Figure 10: A port left unconnected is an elaboration error (D22).
//
// The book has no figure for this: pyuvm discovers a missing connection lazily,
// at first use, as a Python attribute error. rustdv sweeps the registry at the
// end of elaboration and reports every declared-but-unconnected port at once,
// naming its path — before any run phase starts. Here the producer's put port
// is never connected, so the test fails elaboration with `tlm_unconnected_port`.
#[rustdv::test(expect_error = "tlm_unconnected_port")]
#[derive(Component, Default)]
struct UnconnectedTest {
    #[component(child)]
    producer: RustdvComp,
    #[component(fifo)]
    fifo: TlmFifo<u32>,
}

impl Component for UnconnectedTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.producer = Producer::new_comp();
        self.fifo = TlmFifo::new(1);
    }
    // No `connect` — `Producer::put_port` is declared but never wired, so
    // elaboration fails and names `UnconnectedTest.producer.put_port`.
}

// Chapter 31, Figure 12: A FIFO's built-in analysis taps (D23).
//
// Every `TlmFifo` broadcasts each item it accepts on `put_ap` and each item it
// releases on `get_ap` — an analysis port (Chapter 32) with no extra wiring.
// This is how a scoreboard or coverage collector watches traffic flow through a
// FIFO without sitting in the data path. `TapWatcher` implements `Subscriber`
// (Chapter 32) and connects to the FIFO's `put_ap`.
#[rustdv::test]
#[derive(Component, Default)]
struct FifoTapTest {
    #[component(child)]
    producer: RustdvComp,
    #[component(child)]
    consumer: RustdvComp,
    #[component(child)]
    watcher: RustdvComp,
    #[component(fifo)]
    fifo: TlmFifo<u32>,
}

impl Component for FifoTapTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.producer = Producer::new_comp();
        self.consumer = Consumer::new_comp();
        self.watcher = TapWatcher::new_comp();
        self.fifo = TlmFifo::new(1);
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        self.fifo.put_export().connect((&self.producer, Producer::PUT_PORT));
        self.fifo.peek_export().connect((&self.consumer, Consumer::PEEK_PORT));
        self.fifo.get_export().connect((&self.consumer, Consumer::GET_PORT));
        // The tap is an analysis port on the FIFO; a subscriber connects to it
        // (Chapter 32's broadcast model), so the watcher sees every item put.
        self.fifo.put_ap().connect((&self.watcher, TapWatcher::TAP_IN));
    }
}

// A subscriber that logs everything the FIFO's put tap broadcasts.
#[derive(Component, Default)]
struct TapWatcher {
    #[port(analysis)]
    tap_in: AnalysisExport<u32>,
}

impl Subscriber<u32> for TapWatcher {
    fn write(&mut self, item: &u32, ctx: &mut RustdvCtx) {
        ctx.info(&format!("tap saw {item}"));
    }
}

impl Component for TapWatcher {}
