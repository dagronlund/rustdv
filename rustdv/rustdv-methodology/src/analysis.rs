//! Analysis: one write, every subscriber hears it (D86–D88; pyuvm
//! `uvm_analysis_port`, `uvm_subscriber`, `uvm_tlm_analysis_fifo`).
//!
//! # Analysis is not a queue
//!
//! [`AnalysisFifo`] and [`TlmFifo`](crate::TlmFifo) share three letters and
//! nothing else. A `TlmFifo` is a *queue*: one consumer takes each item, the
//! producer blocks when it is full, and the item is gone once taken. An
//! `AnalysisFifo` is a *broadcast*, and **it has no queue at all**: `write`
//! calls every subscriber and returns. Nothing is stored, so a write with no
//! subscribers is not buffered for later — it is simply gone, which is legal
//! and is what a monitor nobody listens to should cost. Do not reach for one
//! expecting the other.
//!
//! To keep the traffic, *subscribe and keep it*: a subscriber's `write` puts
//! the item wherever that component wants it — a `Vec`, an unbounded
//! `TlmFifo`, a comparison against a prediction. The hub is not the memory;
//! the subscriber is.
//!
//! # Why delivery is synchronous
//!
//! A monitor writes a transaction and moves on within the same simulation
//! instant — the time wheel must not turn because a scoreboard was listening.
//! So `write` is not `async`: the publisher's call runs every subscriber's
//! handler and returns.
//!
//! That is only possible because a subscriber shares its **state** rather than
//! itself. A handler needs `&mut` its data, and no component can hand out
//! `&mut self` to a sibling — so the data lives in a
//! [`RustdvShared`](crate::RustdvShared), the component keeps one handle, and
//! the port gets another. An earlier design queued items and delivered them
//! later; "later" is exactly what analysis must not do.
//!
//! # One connection idiom (Ray, 2026-07-24)
//!
//! The UVM broadcasts straight from a source's analysis port to subscribers.
//! rustdv's components are erased, so neither side can reach the other, and
//! analysis gets a **hub** for the same reason put/get has a FIFO: a concrete
//! `#[component(fifo)]` child that the parent owns and can wire.
//!
//! ```ignore
//! self.analysis_fifo.pub_export().connect(&self.mon, Monitor::AP);
//! self.analysis_fifo.sub_export().connect(&self.sb, Scoreboard::INPUT);
//! self.analysis_fifo.sub_export().connect(&self.cov, Coverage::INPUT);
//! ```
//!
//! Several subscribers on one `sub_export()` is what makes it a broadcast.
//! This is a deliberate divergence from IEEE 1800.2 — which we are not
//! implementing — and one idiom to learn beats two.

use std::cell::RefCell;
use std::rc::Rc;

use crate::component::{Component, ComponentNode};
use crate::fifo::TlmFifo;
use crate::port::{bind_or_panic, sink_of, PortName, PortOwner, PublishIf, SinkHandle};

// ===========================================================================
// The legacy direct port (pre-hub; still used by testbenches not yet rebuilt)
// ===========================================================================

/// Port of `uvm_subscriber`'s abstract `write` (mapping row 41).
///
/// Superseded by [`WriteSink`](crate::WriteSink), which is the same idea with
/// the sharing worked out; this stays until the Part IV testbenches are
/// rebuilt on the hub.
pub trait Subscriber<T> {
    fn write(&mut self, item: &T);
}

/// 1-to-many broadcast, connected directly rather than through a hub.
pub struct AnalysisPort<T> {
    subs: Rc<RefCell<Vec<Rc<RefCell<dyn Subscriber<T>>>>>>,
}

impl<T> Clone for AnalysisPort<T> {
    fn clone(&self) -> Self {
        AnalysisPort { subs: self.subs.clone() }
    }
}

impl<T> Default for AnalysisPort<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> AnalysisPort<T> {
    pub fn new() -> AnalysisPort<T> {
        AnalysisPort { subs: Rc::new(RefCell::new(Vec::new())) }
    }

    pub fn connect(&self, sub: Rc<RefCell<dyn Subscriber<T>>>) {
        self.subs.borrow_mut().push(sub);
    }

    /// Broadcast: non-blocking, fire-and-forget (pyuvm 12.2.8).
    pub fn write(&self, item: &T) {
        for sub in self.subs.borrow().iter() {
            sub.borrow_mut().write(item);
        }
    }

    pub fn subscriber_count(&self) -> usize {
        self.subs.borrow().len()
    }
}

impl<T: Clone + 'static> AnalysisPort<T> {
    /// Attach a buffering FIFO to this broadcast (pyuvm
    /// `uvm_tlm_analysis_fifo`) and hand it back to the caller.
    ///
    /// The FIFO is an ordinary unbounded [`TlmFifo`] — **not** an
    /// [`AnalysisFifo`], which keeps nothing (D90). A subscriber that wants to
    /// pull the stream at its own pace owns the storage; the broadcast does
    /// not. Unbounded, so a write never blocks the publisher.
    pub fn connect_fifo(&self) -> TlmFifo<T> {
        let fifo = TlmFifo::unbounded();
        let adapter = Rc::new(RefCell::new(FifoAdapter { fifo: fifo.handle() }));
        self.connect(adapter);
        fifo
    }
}

struct FifoAdapter<T: Clone + 'static> {
    fifo: TlmFifo<T>,
}

impl<T: Clone + 'static> Subscriber<T> for FifoAdapter<T> {
    fn write(&mut self, item: &T) {
        // Unbounded, so this cannot fail and cannot block. The clone is the
        // price of keeping a copy of something the publisher still owns.
        let _ = self.fifo.try_put(item.clone());
    }
}

// ===========================================================================
// The hub
// ===========================================================================

/// What every handle to one hub points at. A subscriber list, and nothing
/// else — there is no queue here by design (D90).
struct HubInner<T: 'static> {
    subs: RefCell<Vec<Rc<dyn SinkHandle<T>>>>,
}

impl<T: 'static> HubInner<T> {
    /// Hand the item to every subscriber, in connection order, and return.
    ///
    /// No queue, no clone of the item, no yield: subscribers see a `&T` and
    /// take from it what they want to keep.
    fn broadcast(&self, item: &T) {
        // Cloned out of the RefCell first: a subscriber's handler is free to
        // do anything, and this loop must not hold a borrow while it runs.
        let subs: Vec<Rc<dyn SinkHandle<T>>> = self.subs.borrow().clone();
        for sub in subs {
            sub.deliver(item);
        }
    }
}

impl<T: 'static> PublishIf<T> for HubInner<T> {
    fn write(&self, item: &T) {
        self.broadcast(item);
    }
}

/// The publish side of a hub: connect it to a source's
/// [`PublishPort`](crate::PublishPort).
pub struct PublishExport<T: 'static> {
    inner: Rc<HubInner<T>>,
}

impl<T: 'static> PublishExport<T> {
    pub fn connect(&self, owner: &dyn PortOwner, name: PortName<dyn PublishIf<T>>) {
        bind_or_panic(owner, name, self.inner.clone() as Rc<dyn PublishIf<T>>);
    }
}

/// The subscribe side of a hub. Connect as many subscribers to it as you
/// like — that is what makes the write a broadcast.
pub struct SubscribeExport<T: 'static> {
    inner: Rc<HubInner<T>>,
}

impl<T: 'static> SubscribeExport<T> {
    /// Take the subscriber's sink and add it to the broadcast list.
    ///
    /// Unlike a put/get connect, nothing is written *into* the port: the
    /// subscriber already put its sink there with `on_write`, and the hub
    /// collects it. Broadcast runs the other way, so the wiring does too.
    pub fn connect(&self, owner: &dyn PortOwner, name: PortName<dyn SinkHandle<T>>) {
        match sink_of(owner, name) {
            Ok(sink) => self.inner.subs.borrow_mut().push(sink),
            Err(e) => panic!("{e}"),
        }
    }
}

/// A broadcast hub: one publisher in, every subscriber out.
///
/// **It holds no items.** `write` calls each subscriber and returns; if nobody
/// is subscribed the datum is gone (D90). A component that needs to keep the
/// traffic subscribes and keeps it — in a `Vec`, or in an unbounded `TlmFifo`
/// it owns, if it wants to pull on its own schedule.
///
/// Declare it as a child with `#[component(fifo)]`, like a `TlmFifo`, then hand
/// out its exports in `connect`: [`pub_export`](Self::pub_export) for the
/// source, [`sub_export`](Self::sub_export) for each listener.
pub struct AnalysisFifo<T: 'static> {
    inner: Rc<HubInner<T>>,
}

impl<T: 'static> Clone for AnalysisFifo<T> {
    /// Another handle to the *same* hub.
    fn clone(&self) -> Self {
        AnalysisFifo { inner: self.inner.clone() }
    }
}

impl<T: 'static> Default for AnalysisFifo<T> {
    fn default() -> Self {
        AnalysisFifo::new()
    }
}

impl<T: 'static> AnalysisFifo<T> {
    pub fn new() -> AnalysisFifo<T> {
        AnalysisFifo { inner: Rc::new(HubInner { subs: RefCell::new(Vec::new()) }) }
    }

    /// The publish side, for a source's `PublishPort`.
    pub fn pub_export(&self) -> PublishExport<T> {
        PublishExport { inner: self.inner.clone() }
    }

    /// The subscribe side, for a subscriber's `SubscribePort`. Connect
    /// several; each one sees every item.
    pub fn sub_export(&self) -> SubscribeExport<T> {
        SubscribeExport { inner: self.inner.clone() }
    }

    /// Broadcast an item, as the owner of the hub rather than through a port.
    pub fn write(&self, item: &T) {
        self.inner.broadcast(item);
    }

    /// How many subscribers are listening. Zero is legal.
    pub fn subscriber_count(&self) -> usize {
        self.inner.subs.borrow().len()
    }
}

// A hub is a component: it appears in the hierarchy and its phases are no-ops.
impl<T: 'static> Component for AnalysisFifo<T> {}

impl<T: 'static> ComponentNode for AnalysisFifo<T> {
    fn node_name(&self) -> &'static str {
        "AnalysisFifo"
    }
    fn children_mut(&mut self) -> Vec<(String, &mut (dyn ComponentNode + 'static))> {
        Vec::new()
    }
}
