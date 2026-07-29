//! `TlmFifo<T>`: the UVM `uvm_tlm_fifo` — a *component* that **encapsulates**
//! a queue (pyuvm's Queue/Mailbox) so two components connect to the same FIFO
//! and neither learns the other exists (D17/D23/D24; pyuvm `uvm_tlm_fifo`,
//! default depth 1).
//!
//! This encapsulation is the FIFO's *point*, not a nice-to-have: it is what
//! makes the connection late-bound. Hierarchy visibility is a consequence;
//! decoupling is the reason. Do not treat the FIFO as optional plumbing.
//!
//! # Ports and exports
//!
//! A component that needs a FIFO it does not own declares a **port**
//! ([`PutPort`], [`GetPort`], [`PeekPort`]). The component that *owns* the
//! FIFO hands out **exports** and connects them:
//!
//! ```ignore
//! self.fifo.put_export().connect(&self.producer, Producer::PUT_PORT);
//! self.fifo.get_export().connect(&self.consumer, Consumer::GET_PORT);
//! ```
//!
//! The export always initiates, as in the UVM. See [`crate::port`] for why the
//! first argument works for an erased child and for `self` alike.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use rustdv_sim::queue::Queue;

use crate::component::{Component, ComponentNode};
use crate::port::{bind_or_panic, sink_of, GetIf, PeekIf, PortName, PortOwner, PutIf, SinkHandle};

// ===========================================================================
// The shared inside
// ===========================================================================

/// What every handle to one FIFO points at. The interfaces are implemented
/// here, not on [`TlmFifo`], so an export can hold an `Rc` to the queue that
/// outlives any particular handle.
struct FifoInner<T: 'static> {
    q: Queue<T>,
    size: Option<usize>,
    /// The built-in analysis taps (D23), the port of `uvm_tlm_fifo`'s
    /// `put_ap`/`get_ap`. Observation running alongside the data path: every
    /// tap sees every item, none of them consume anything, and nobody waits.
    put_taps: RefCell<Vec<Rc<dyn SinkHandle<T>>>>,
    get_taps: RefCell<Vec<Rc<dyn SinkHandle<T>>>>,
}

impl<T: 'static> FifoInner<T> {
    fn new(size: Option<usize>) -> FifoInner<T> {
        FifoInner {
            q: match size {
                Some(n) => Queue::new(Some(n)),
                None => Queue::unbounded(),
            },
            size,
            put_taps: RefCell::new(Vec::new()),
            get_taps: RefCell::new(Vec::new()),
        }
    }

    fn is_full(&self) -> bool {
        match self.size {
            None => false,
            Some(s) => self.q.len() >= s,
        }
    }

    /// Fire one set of taps. The list is copied out first so a subscriber's
    /// handler cannot deadlock against the borrow.
    fn tap(taps: &RefCell<Vec<Rc<dyn SinkHandle<T>>>>, item: &T) {
        let subs: Vec<Rc<dyn SinkHandle<T>>> = taps.borrow().clone();
        for sub in subs {
            sub.deliver(item);
        }
    }

    /// Put, tapping the item at the instant it is accepted.
    ///
    /// The wait and the handover are separate steps because `try_put` takes
    /// ownership: once the item is in the queue there is nothing left to show
    /// the taps. Nothing awaits between the two lines, so no other task can
    /// take the space in between.
    async fn put_tapped(&self, item: T) {
        self.q.wait_for_space().await;
        Self::tap(&self.put_taps, &item);
        let _ = self.q.try_put(item);
    }

    fn try_put_tapped(&self, item: T) -> Result<(), T> {
        if !self.q.has_space() {
            return Err(item);
        }
        Self::tap(&self.put_taps, &item);
        self.q.try_put(item)
    }

    fn tap_get(&self, item: Option<T>) -> Option<T> {
        if let Some(v) = &item {
            Self::tap(&self.get_taps, v);
        }
        item
    }
}

impl<T: 'static> PutIf<T> for FifoInner<T> {
    fn put(&self, item: T) -> Pin<Box<dyn Future<Output = ()> + '_>> {
        Box::pin(self.put_tapped(item))
    }
    fn try_put(&self, item: T) -> Result<(), T> {
        self.try_put_tapped(item)
    }
    fn can_put(&self) -> bool {
        !self.is_full()
    }
}

impl<T: 'static> GetIf<T> for FifoInner<T> {
    fn get(&self) -> Pin<Box<dyn Future<Output = T> + '_>> {
        Box::pin(async move {
            let item = self.q.get().await;
            Self::tap(&self.get_taps, &item);
            item
        })
    }
    fn try_get(&self) -> Option<T> {
        let item = self.q.try_get();
        self.tap_get(item)
    }
    fn can_get(&self) -> bool {
        !self.q.is_empty()
    }
}

impl<T: Clone + 'static> PeekIf<T> for FifoInner<T> {
    fn peek(&self) -> Pin<Box<dyn Future<Output = T> + '_>> {
        Box::pin(self.q.peek())
    }
    fn try_peek(&self) -> Option<T> {
        self.q.try_peek()
    }
    fn can_peek(&self) -> bool {
        !self.q.is_empty()
    }
}

// ===========================================================================
// The exports
// ===========================================================================

/// The FIFO's put side, handed to a component that needs to put.
///
/// An export is a *value you connect*, not a value you keep: the usual life of
/// one is a single line in the parent's connect phase.
pub struct PutExport<T: 'static> {
    iface: Rc<dyn PutIf<T>>,
}

impl<T: 'static> PutExport<T> {
    /// Connect this export to `owner`'s port called `name`.
    ///
    /// `owner` is a child slot (`&self.producer`) or the connecting component
    /// itself (`self`) — both are `PortOwner`. `name` is the derive-generated
    /// constant, so it cannot be misspelled and cannot name a `get` port.
    pub fn connect(&self, owner: &dyn PortOwner, name: PortName<dyn PutIf<T>>) {
        bind_or_panic(owner, name, self.iface.clone());
    }
}

/// The FIFO's get side.
pub struct GetExport<T: 'static> {
    iface: Rc<dyn GetIf<T>>,
}

impl<T: 'static> GetExport<T> {
    pub fn connect(&self, owner: &dyn PortOwner, name: PortName<dyn GetIf<T>>) {
        bind_or_panic(owner, name, self.iface.clone());
    }
}

/// One of a FIFO's analysis taps (D23): connect a subscriber to watch the
/// traffic without joining the data path.
pub struct TapExport<T: 'static> {
    taps: Rc<FifoInner<T>>,
    on_put: bool,
}

impl<T: 'static> TapExport<T> {
    /// Add a subscriber to this tap. Same shape as every other connection —
    /// the export, the owner, the port name.
    pub fn connect(&self, owner: &dyn PortOwner, name: PortName<dyn SinkHandle<T>>) {
        match sink_of(owner, name) {
            Ok(sink) => {
                let list = if self.on_put { &self.taps.put_taps } else { &self.taps.get_taps };
                list.borrow_mut().push(sink);
            }
            Err(e) => panic!("{e}"),
        }
    }
}

/// The FIFO's peek side.
pub struct PeekExport<T: 'static> {
    iface: Rc<dyn PeekIf<T>>,
}

impl<T: 'static> PeekExport<T> {
    pub fn connect(&self, owner: &dyn PortOwner, name: PortName<dyn PeekIf<T>>) {
        bind_or_panic(owner, name, self.iface.clone());
    }
}

// ===========================================================================
// TlmFifo
// ===========================================================================

/// A bounded (or unbounded) FIFO that is also a component in the hierarchy.
///
/// Declare it as a child with `#[component(fifo)]` so it appears in the tree,
/// then hand out its exports in `connect`.
pub struct TlmFifo<T: 'static> {
    inner: Rc<FifoInner<T>>,
}

impl<T: 'static> Default for TlmFifo<T> {
    /// Depth 1, the UVM default — and the depth that makes a producer and a
    /// consumer take turns, which is what most testbenches want.
    fn default() -> Self {
        TlmFifo::new(1)
    }
}

impl<T: 'static> TlmFifo<T> {
    /// A FIFO `size` items deep. Depth 1 is the UVM default and forces the
    /// producer to wait for the consumer.
    pub fn new(size: usize) -> TlmFifo<T> {
        TlmFifo { inner: Rc::new(FifoInner::new(Some(size))) }
    }

    /// A FIFO with no depth limit: a put never blocks.
    pub fn unbounded() -> TlmFifo<T> {
        TlmFifo { inner: Rc::new(FifoInner::new(None)) }
    }

    /// The declared depth; `None` for an unbounded FIFO.
    pub fn size(&self) -> Option<usize> {
        self.inner.size
    }
    /// How many items are in it now.
    pub fn used(&self) -> usize {
        self.inner.q.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.q.is_empty()
    }
    pub fn is_full(&self) -> bool {
        self.inner.is_full()
    }
    /// Throw away everything in the FIFO (UVM `flush`).
    pub fn flush(&self) {
        while self.inner.q.try_get().is_some() {}
    }

    // --- the exports ------------------------------------------------------

    /// The put side, to connect to a component's [`PutPort`](crate::PutPort).
    pub fn put_export(&self) -> PutExport<T> {
        PutExport { iface: self.inner.clone() }
    }

    /// The get side, to connect to a component's [`GetPort`](crate::GetPort).
    pub fn get_export(&self) -> GetExport<T> {
        GetExport { iface: self.inner.clone() }
    }

    /// The tap that fires as each item goes **in** (`uvm_tlm_fifo::put_ap`).
    pub fn put_ap(&self) -> TapExport<T> {
        TapExport { taps: self.inner.clone(), on_put: true }
    }

    /// The tap that fires as each item comes **out** (`uvm_tlm_fifo::get_ap`).
    pub fn get_ap(&self) -> TapExport<T> {
        TapExport { taps: self.inner.clone(), on_put: false }
    }

    // --- direct use, for the component that owns the FIFO -----------------
    //
    // The owner does not need a port: it holds the FIFO. These are the same
    // operations the exports offer, called without the indirection.

    pub async fn put(&self, item: T) {
        self.inner.put_tapped(item).await
    }
    pub fn try_put(&self, item: T) -> Result<(), T> {
        self.inner.try_put_tapped(item)
    }
    pub async fn get(&self) -> T {
        let item = self.inner.q.get().await;
        FifoInner::tap(&self.inner.get_taps, &item);
        item
    }
    pub fn try_get(&self) -> Option<T> {
        let item = self.inner.q.try_get();
        self.inner.tap_get(item)
    }

    /// A second handle to the *same* FIFO (for wiring at construction).
    pub fn handle(&self) -> TlmFifo<T> {
        TlmFifo { inner: self.inner.clone() }
    }
}

impl<T: Clone + 'static> TlmFifo<T> {
    /// The peek side, to connect to a [`PeekPort`](crate::PeekPort). Peek
    /// copies rather than removes, which is why it needs `T: Clone`.
    pub fn peek_export(&self) -> PeekExport<T> {
        PeekExport { iface: self.inner.clone() }
    }

    pub async fn peek(&self) -> T {
        self.inner.q.peek().await
    }
    pub fn try_peek(&self) -> Option<T> {
        self.inner.q.try_peek()
    }
}

// A FIFO is a component: it appears in the hierarchy, and its phases are
// no-ops (it has no children and nothing to run).
impl<T: 'static> Component for TlmFifo<T> {}

impl<T: 'static> ComponentNode for TlmFifo<T> {
    fn node_name(&self) -> &'static str {
        "TlmFifo"
    }
    fn children_mut(&mut self) -> Vec<(String, &mut (dyn ComponentNode + 'static))> {
        Vec::new()
    }
}
