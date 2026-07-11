//! The sequencer handshake, preserved event-for-event from pyuvm
//! (design-doc §5.6; pyuvm: _s14_15_python_sequences.py):
//!
//! 1. Sequence: `start_item` → enqueue; block until granted.
//! 2. Driver: `get_next_item` → dequeue; grant; block until ready.
//! 3. Sequence: fills fields; `finish_item` → hand off; block until done.
//! 4. Driver: drives the DUT; `item_done(Some(rsp))` → release; response
//!    tagged with the envelope's txn id (replaces set_context, §5.1).
//! 5. Sequence (optionally): `get_response(txn_id)`.
//!
//! Identity lives in the infrastructure's [`SeqItem`] envelope, not the
//! user's plain data type (review-memo R1).

use std::cell::{Cell, RefCell};
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use rustdv_sim::queue::Queue;
use rustdv_sim::sync::Event;

/// Transaction identity for request/response correlation (design-doc §5.1).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TxnId(pub u64);

/// What the driver receives from `get_next_item()`: infrastructure-owned
/// id + the user's plain payload (design-doc §5.1 envelope).
pub struct SeqItem<REQ> {
    id: TxnId,
    payload: REQ,
}

impl<REQ> SeqItem<REQ> {
    pub fn txn_id(&self) -> TxnId {
        self.id
    }
    pub fn payload(&self) -> &REQ {
        &self.payload
    }
    pub fn payload_mut(&mut self) -> &mut REQ {
        &mut self.payload
    }
    pub fn into_payload(self) -> REQ {
        self.payload
    }
}

#[derive(Debug, Clone)]
pub struct SeqError(pub String);

impl fmt::Display for SeqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sequence error: {}", self.0)
    }
}
impl std::error::Error for SeqError {}
impl From<&str> for SeqError {
    fn from(s: &str) -> SeqError {
        SeqError(s.to_string())
    }
}

/// User-implemented sequence (design-doc §5.6). `body` returns a boxed
/// future because sequencers store sequences heterogeneously (the sole
/// residual of OQ-3).
pub trait Sequence<REQ: 'static, RSP: 'static = REQ> {
    fn body<'a>(
        &'a mut self,
        ctx: SeqCtx<REQ, RSP>,
    ) -> Pin<Box<dyn Future<Output = Result<(), SeqError>> + 'a>>;
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// Handshake state for one in-flight item. The events pyuvm stored on the
/// transaction itself (start/finish/item_ready conditions) live here, owned
/// by the infrastructure (§5.1).
struct ItemSlot<REQ> {
    id: TxnId,
    granted: Event,
    ready: Event,
    done: Event,
    payload: RefCell<Option<REQ>>,
}

struct RespInner<RSP> {
    items: RefCell<Vec<(TxnId, RSP)>>,
    waiters: RefCell<Vec<Waker>>,
}

/// FIFO-or-by-id response retrieval (pyuvm ResponseQueue, mapping row 39).
pub struct ResponseQueue<RSP> {
    inner: Rc<RespInner<RSP>>,
}

impl<RSP> Clone for ResponseQueue<RSP> {
    fn clone(&self) -> Self {
        ResponseQueue { inner: self.inner.clone() }
    }
}

impl<RSP> ResponseQueue<RSP> {
    fn new() -> ResponseQueue<RSP> {
        ResponseQueue {
            inner: Rc::new(RespInner { items: RefCell::new(Vec::new()), waiters: RefCell::new(Vec::new()) }),
        }
    }

    fn push(&self, id: TxnId, rsp: RSP) {
        self.inner.items.borrow_mut().push((id, rsp));
        for w in self.inner.waiters.borrow_mut().drain(..) {
            w.wake();
        }
    }

    /// `None` → FIFO order; `Some(id)` → cherry-pick by txn id.
    pub fn get_response(&self, txn_id: Option<TxnId>) -> GetResponse<RSP> {
        GetResponse { inner: self.inner.clone(), txn_id }
    }
}

pub struct GetResponse<RSP> {
    inner: Rc<RespInner<RSP>>,
    txn_id: Option<TxnId>,
}

impl<RSP> Future for GetResponse<RSP> {
    type Output = RSP;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<RSP> {
        let mut items = self.inner.items.borrow_mut();
        let idx = match self.txn_id {
            None => {
                if items.is_empty() {
                    None
                } else {
                    Some(0)
                }
            }
            Some(id) => items.iter().position(|(i, _)| *i == id),
        };
        match idx {
            Some(i) => Poll::Ready(items.remove(i).1),
            None => {
                drop(items);
                self.inner.waiters.borrow_mut().push(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

struct SeqrInner<REQ: 'static, RSP: 'static> {
    /// FIFO arbitration (grab/lock/priority absent, as in pyuvm — [gap]).
    queue: Queue<Rc<ItemSlot<REQ>>>,
    next_id: Cell<u64>,
    responses: ResponseQueue<RSP>,
    /// The driver's current item; a second get_next_item without item_done
    /// is an error, same rule as pyuvm (UVMSequenceError).
    current: RefCell<Option<Rc<ItemSlot<REQ>>>>,
}

/// The sequencer component: a queue of sequences feeding one item channel
/// (design-doc §5.6). Clonable handle; clones share state.
pub struct Sequencer<REQ: 'static, RSP: 'static = REQ> {
    inner: Rc<SeqrInner<REQ, RSP>>,
}

impl<REQ, RSP> Clone for Sequencer<REQ, RSP> {
    fn clone(&self) -> Self {
        Sequencer { inner: self.inner.clone() }
    }
}

impl<REQ: 'static, RSP: 'static> Default for Sequencer<REQ, RSP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<REQ: 'static, RSP: 'static> Sequencer<REQ, RSP> {
    pub fn new() -> Sequencer<REQ, RSP> {
        Sequencer {
            inner: Rc::new(SeqrInner {
                queue: Queue::unbounded(),
                next_id: Cell::new(1),
                responses: ResponseQueue::new(),
                current: RefCell::new(None),
            }),
        }
    }

    /// The driver-side endpoint. Created once, passed to the driver's
    /// constructor (the connect convention, §5.3).
    pub fn seq_item_port(&self) -> SeqItemPort<REQ, RSP> {
        SeqItemPort { inner: self.inner.clone() }
    }

    /// Run a sequence to completion on this sequencer (port of
    /// uvm_sequence.start, mapping row 38).
    pub async fn start(&self, seq: &mut dyn Sequence<REQ, RSP>) -> Result<(), SeqError> {
        let ctx = SeqCtx { inner: self.inner.clone(), current: None };
        seq.body(ctx).await
    }
}

// ---------------------------------------------------------------------------
// Sequence side
// ---------------------------------------------------------------------------

/// Handed to a running sequence; knows its sequencer (design-doc §5.6).
pub struct SeqCtx<REQ: 'static, RSP: 'static = REQ> {
    inner: Rc<SeqrInner<REQ, RSP>>,
    current: Option<Rc<ItemSlot<REQ>>>,
}

impl<REQ: 'static, RSP: 'static> SeqCtx<REQ, RSP> {
    /// Enqueue and block until this item's turn arrives (the item's
    /// *start condition* in pyuvm).
    pub async fn start_item(&mut self, _item: &mut REQ) {
        assert!(
            self.current.is_none(),
            "start_item called twice without finish_item (pyuvm UVMSequenceError)"
        );
        let id = TxnId(self.inner.next_id.get());
        self.inner.next_id.set(id.0 + 1);
        let slot = Rc::new(ItemSlot {
            id,
            granted: Event::new(),
            ready: Event::new(),
            done: Event::new(),
            payload: RefCell::new(None),
        });
        self.current = Some(slot.clone());
        self.inner.queue.put(slot.clone()).await; // unbounded: immediate
        slot.granted.wait().await;
    }

    /// Hand off the (now filled) item; block until the driver calls
    /// item_done. Returns the envelope id for later get_response.
    pub async fn finish_item(&mut self, item: REQ) -> Result<TxnId, SeqError> {
        let slot = self
            .current
            .take()
            .ok_or_else(|| SeqError("finish_item without start_item".into()))?;
        *slot.payload.borrow_mut() = Some(item);
        slot.ready.set();
        slot.done.wait().await;
        Ok(slot.id)
    }

    /// FIFO-or-by-id response retrieval (pyuvm ResponseQueue semantics).
    pub async fn get_response(&mut self, txn_id: Option<TxnId>) -> RSP {
        self.inner.responses.get_response(txn_id).await
    }
}

// ---------------------------------------------------------------------------
// Driver side
// ---------------------------------------------------------------------------

/// Driver-side port (port of uvm_seq_item_port, mapping row 40).
pub struct SeqItemPort<REQ: 'static, RSP: 'static = REQ> {
    inner: Rc<SeqrInner<REQ, RSP>>,
}

impl<REQ: 'static, RSP: 'static> SeqItemPort<REQ, RSP> {
    /// Dequeue the next item: grant, wait for the sequence to fill it,
    /// return the envelope. Panics if called twice without item_done —
    /// same rule as pyuvm (UVMSequenceError → testbench bug → panic,
    /// per the failure taxonomy in §7.3).
    pub async fn get_next_item(&mut self) -> SeqItem<REQ> {
        assert!(
            self.inner.current.borrow().is_none(),
            "get_next_item called twice without item_done (pyuvm UVMSequenceError)"
        );
        let slot = self.inner.queue.get().await;
        slot.granted.set();
        slot.ready.wait().await;
        let payload = slot
            .payload
            .borrow_mut()
            .take()
            .expect("item ready but payload missing (rustdv bug)");
        let item = SeqItem { id: slot.id, payload };
        *self.inner.current.borrow_mut() = Some(slot);
        item
    }

    /// Release the sequence; optional response is tagged with the current
    /// envelope's txn id internally (replaces pyuvm set_context).
    pub fn item_done(&mut self, rsp: Option<RSP>) {
        let slot = self
            .inner
            .current
            .borrow_mut()
            .take()
            .expect("item_done without get_next_item (pyuvm UVMSequenceError)");
        if let Some(r) = rsp {
            self.inner.responses.push(slot.id, r);
        }
        slot.done.set();
    }

    pub async fn get_response(&mut self, txn_id: Option<TxnId>) -> RSP {
        self.inner.responses.get_response(txn_id).await
    }
}
