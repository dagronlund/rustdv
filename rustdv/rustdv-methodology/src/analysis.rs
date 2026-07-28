//! Analysis: one write, every subscriber hears it (D86–D88; pyuvm
//! `uvm_analysis_port`, `uvm_subscriber`, `uvm_tlm_analysis_fifo`).
//!
//! # Analysis is not a queue
//!
//! [`AnalysisFifo`] and [`TlmFifo`](crate::TlmFifo) share three letters and
//! nothing else. A `TlmFifo` is a *queue*: one consumer takes each item, the
//! producer blocks when it is full, and the item is gone once taken. An
//! `AnalysisFifo` is a *broadcast*: every subscriber sees every item, nothing
//! is consumed, nobody blocks, and a write with no subscribers is legal.
//! Do not reach for one expecting the other.
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

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use rustdv_sim::queue::Queue;

use crate::component::{Component, ComponentNode};
use crate::fifo::GetExport;
use crate::port::{bind_or_panic, sink_of, GetIf, PortName, PortOwner, PublishIf, SinkHandle};

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
    /// Attach a buffering analysis FIFO (pyuvm `uvm_tlm_analysis_fifo`).
    pub fn connect_fifo(&self) -> AnalysisFifo<T> {
        let fifo = AnalysisFifo::new();
        fifo.start_buffering();
        let adapter = Rc::new(RefCell::new(FifoAdapter { fifo: fifo.clone() }));
        self.connect(adapter);
        fifo
    }
}

struct FifoAdapter<T: Clone + 'static> {
    fifo: AnalysisFifo<T>,
}

impl<T: Clone + 'static> Subscriber<T> for FifoAdapter<T> {
    fn write(&mut self, item: &T) {
        self.fifo.write(item);
    }
}

// ===========================================================================
// The hub
// ===========================================================================

/// What every handle to one hub points at.
struct HubInner<T: Clone + 'static> {
    subs: RefCell<Vec<Rc<dyn SinkHandle<T>>>>,
    /// The pull side (`get_export`). Unbounded, so a write never blocks.
    q: Queue<T>,
    /// Buffer only if somebody is pulling. A hub used purely for broadcast
    /// would otherwise grow a queue nobody ever drains.
    buffering: Cell<bool>,
}

impl<T: Clone + 'static> HubInner<T> {
    fn broadcast(&self, item: &T) {
        // Cloned out of the RefCell first: a subscriber's handler is free to
        // do anything, and this loop must not hold a borrow while it runs.
        let subs: Vec<Rc<dyn SinkHandle<T>>> = self.subs.borrow().clone();
        for sub in subs {
            sub.deliver(item);
        }
        if self.buffering.get() {
            // Unbounded, so this cannot fail and cannot block.
            let _ = self.q.try_put(item.clone());
        }
    }
}

impl<T: Clone + 'static> PublishIf<T> for HubInner<T> {
    fn write(&self, item: &T) {
        self.broadcast(item);
    }
}

impl<T: Clone + 'static> GetIf<T> for HubInner<T> {
    fn get(&self) -> Pin<Box<dyn Future<Output = T> + '_>> {
        Box::pin(self.q.get())
    }
    fn try_get(&self) -> Option<T> {
        self.q.try_get()
    }
    fn can_get(&self) -> bool {
        !self.q.is_empty()
    }
}

/// The publish side of a hub: connect it to a source's
/// [`PublishPort`](crate::PublishPort).
pub struct PublishExport<T: Clone + 'static> {
    inner: Rc<HubInner<T>>,
}

impl<T: Clone + 'static> PublishExport<T> {
    pub fn connect(&self, owner: &dyn PortOwner, name: PortName<dyn PublishIf<T>>) {
        bind_or_panic(owner, name, self.inner.clone() as Rc<dyn PublishIf<T>>);
    }
}

/// The subscribe side of a hub. Connect as many subscribers to it as you
/// like — that is what makes the write a broadcast.
pub struct SubscribeExport<T: Clone + 'static> {
    inner: Rc<HubInner<T>>,
}

impl<T: Clone + 'static> SubscribeExport<T> {
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

/// A broadcast hub: one publisher in, every subscriber out — and, for anyone
/// who would rather pull than be pushed, a buffered stream.
///
/// Declare it as a child with `#[component(fifo)]`, like a `TlmFifo`, then
/// hand out its exports in `connect`. Three accessors, three jobs:
/// [`pub_export`](Self::pub_export), [`sub_export`](Self::sub_export),
/// [`get_export`](Self::get_export).
pub struct AnalysisFifo<T: Clone + 'static> {
    inner: Rc<HubInner<T>>,
}

impl<T: Clone + 'static> Clone for AnalysisFifo<T> {
    /// Another handle to the *same* hub.
    fn clone(&self) -> Self {
        AnalysisFifo { inner: self.inner.clone() }
    }
}

impl<T: Clone + 'static> Default for AnalysisFifo<T> {
    fn default() -> Self {
        AnalysisFifo::new()
    }
}

impl<T: Clone + 'static> AnalysisFifo<T> {
    pub fn new() -> AnalysisFifo<T> {
        AnalysisFifo {
            inner: Rc::new(HubInner {
                subs: RefCell::new(Vec::new()),
                q: Queue::unbounded(),
                buffering: Cell::new(false),
            }),
        }
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

    /// The pull side, for a component's `GetPort` — the port of
    /// `uvm_tlm_analysis_fifo`. Asking for it starts the buffering; a hub
    /// nobody pulls from keeps no queue.
    pub fn get_export(&self) -> GetExport<T> {
        self.start_buffering();
        GetExport::from_iface(self.inner.clone() as Rc<dyn GetIf<T>>)
    }

    pub(crate) fn start_buffering(&self) {
        self.inner.buffering.set(true);
    }

    /// Broadcast an item, as the owner of the hub rather than through a port.
    pub fn write(&self, item: &T) {
        self.inner.broadcast(item);
    }

    /// How many subscribers are listening.
    pub fn subscriber_count(&self) -> usize {
        self.inner.subs.borrow().len()
    }

    // --- the buffered stream, read directly by the hub's owner ------------

    pub async fn get(&self) -> T {
        self.start_buffering();
        self.inner.q.get().await
    }
    pub fn try_get(&self) -> Option<T> {
        self.inner.q.try_get()
    }
    pub fn len(&self) -> usize {
        self.inner.q.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.q.is_empty()
    }
}

// A hub is a component: it appears in the hierarchy and its phases are no-ops.
impl<T: Clone + 'static> Component for AnalysisFifo<T> {}

impl<T: Clone + 'static> ComponentNode for AnalysisFifo<T> {
    fn node_name(&self) -> &'static str {
        "AnalysisFifo"
    }
    fn children_mut(&mut self) -> Vec<(String, &mut (dyn ComponentNode + 'static))> {
        Vec::new()
    }
}
