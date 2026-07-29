//! Sequences: stimulus as a program, separate from the testbench structure.
//!
//! The handshake is pyuvm's, event for event
//! (`pyuvm/_s14_15_python_sequences.py`):
//!
//! 1. Sequence: `start_item` → enqueue; block until this item's turn.
//! 2. Driver: `get_next_item` → dequeue; grant; block until the sequence has
//!    filled it in.
//! 3. Sequence: sets the fields; `finish_item` → hand off; block until done.
//! 4. Driver: drives the DUT; `item_done(rsp)` → release the sequence.
//! 5. Sequence, if it wants the answer: `get_response(id)`.
//!
//! **The gap between steps 1 and 3 is the point** (D3). It is the window in
//! which the driver is committed and the values are not yet decided, which is
//! where late stimulus setting lives. SystemVerilog had `mailbox#(T)` and
//! built this two-phase rendezvous anyway; pyuvm simplified nearly everything
//! else about sequences and kept both phases.
//!
//! Identity lives in the [`SeqItem`] envelope, not in the user's data type
//! (D93/D98): a transaction stays a plain struct with derives.

use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use rustdv_sim::queue::Queue;
use rustdv_sim::sync::Event;
use rustdv_sim::log::Logger;
use rustdv_sim::{Rng, RustdvPath};

use crate::component::{Component, ComponentNode};
use crate::port::{bind_or_panic, PortName, PortOwner};

// ===========================================================================
// Identity
// ===========================================================================

/// A transaction's ticket. Assigned by the sequencer, carried in the
/// envelope, and echoed on the response so a sequence gets the answer to the
/// question it asked.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TxnId(pub u64);

impl fmt::Display for TxnId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// What a driver receives: the framework's id plus the user's plain payload.
///
/// SystemVerilog needs `rsp.set_id_info(req)` and pyuvm needs
/// `rsp.set_context(req)` to correlate a response with its request by hand,
/// and forgetting either is a run-time fatal. The id is in here, so there is
/// nothing to remember.
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
impl From<String> for SeqError {
    fn from(s: String) -> SeqError {
        SeqError(s)
    }
}
impl From<crate::config::ConfigError> for SeqError {
    fn from(e: crate::config::ConfigError) -> SeqError {
        SeqError(e.to_string())
    }
}

// ===========================================================================
// Internals
// ===========================================================================

/// Handshake state for one in-flight item. The events pyuvm stores on the
/// transaction itself live here, owned by the framework.
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

/// Responses, retrievable in order or by ticket (pyuvm's `ResponseQueue`).
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
            inner: Rc::new(RespInner {
                items: RefCell::new(Vec::new()),
                waiters: RefCell::new(Vec::new()),
            }),
        }
    }

    fn push(&self, id: TxnId, rsp: RSP) {
        self.inner.items.borrow_mut().push((id, rsp));
        for w in self.inner.waiters.borrow_mut().drain(..) {
            w.wake();
        }
    }

    /// `None` → whatever is next; `Some(id)` → that ticket's answer, however
    /// many others arrive first.
    pub fn get_response(&self, txn_id: Option<TxnId>) -> GetResponse<RSP> {
        GetResponse { inner: self.inner.clone(), txn_id }
    }

    /// Is it ready **yet**? Returns `None` rather than waiting.
    ///
    /// The non-blocking half of the pair, as `try_put`/`try_get` are to
    /// `put`/`get` and `try_next_item` is to `get_next_item`. A sequence with
    /// several requests outstanding needs it: blocking on one ticket forces
    /// the collection order back to the order they were issued, and hides the
    /// very thing an out-of-order responder is doing.
    pub fn try_get_response(&self, txn_id: Option<TxnId>) -> Option<RSP> {
        let mut items = self.inner.items.borrow_mut();
        let idx = match txn_id {
            None => (!items.is_empty()).then_some(0),
            Some(id) => items.iter().position(|(i, _)| *i == id),
        };
        idx.map(|i| items.remove(i).1)
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
            None => (!items.is_empty()).then_some(0),
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
    /// FIFO arbitration. SystemVerilog offers six modes and grab/lock;
    /// pyuvm has queue order and nothing else, and no pyuvm user has ever
    /// asked for the rest (D97).
    queue: Queue<Rc<ItemSlot<REQ>>>,
    next_id: Cell<u64>,
    responses: ResponseQueue<RSP>,
    /// The driver's current item: a second `get_next_item` without an
    /// intervening `item_done` is an error, as in pyuvm.
    current: RefCell<Option<Rc<ItemSlot<REQ>>>>,
}

// ===========================================================================
// The driver's side of the port
// ===========================================================================

/// What a sequencer's export offers a driver. Behind `dyn`, so the port can
/// hold it without knowing which sequencer it came from.
pub trait SeqItemIf<REQ: 'static, RSP: 'static>: 'static {
    fn get_next_item(&self) -> Pin<Box<dyn Future<Output = SeqItem<REQ>> + '_>>;
    /// Take an item **only if one is waiting**. The UVM's `try_next_item`
    /// (clause 15.2.1.2.2, present since 1.1d); pyuvm has no equivalent. A
    /// driver that must do something else on this clock edge cannot afford to
    /// block, and this is the call for it.
    fn try_next_item(&self) -> Option<SeqItem<REQ>>;
    fn item_done(&self, rsp: Option<RSP>);
    /// Answer a request whose `item_done` has already been called — the
    /// pipelined case, where the response is not ready when the sequencer is
    /// released. The UVM's `put_response`.
    fn put_response(&self, id: TxnId, rsp: RSP);
}

impl<REQ: 'static, RSP: 'static> SeqItemIf<REQ, RSP> for SeqrInner<REQ, RSP> {
    fn get_next_item(&self) -> Pin<Box<dyn Future<Output = SeqItem<REQ>> + '_>> {
        Box::pin(async move {
            assert!(
                self.current.borrow().is_none(),
                "get_next_item called twice without item_done"
            );
            let slot = self.queue.get().await;
            slot.granted.set();
            slot.ready.wait().await;
            let payload = slot
                .payload
                .borrow_mut()
                .take()
                .expect("item ready but payload missing (rustdv bug)");
            let item = SeqItem { id: slot.id, payload };
            *self.current.borrow_mut() = Some(slot);
            item
        })
    }

    fn try_next_item(&self) -> Option<SeqItem<REQ>> {
        assert!(self.current.borrow().is_none(), "try_next_item called without item_done");
        // The sequence must already be waiting in `start_item` *and* have
        // filled the item in — otherwise there is nothing to hand over and we
        // must not block. `ready` is set by `finish_item`.
        let slot = self.queue.try_get()?;
        slot.granted.set();
        let taken = slot.payload.borrow_mut().take();
        let payload = match taken {
            Some(p) => p,
            None => {
                // Granted but not yet filled: put it back and try next edge.
                let _ = self.queue.try_put(slot);
                return None;
            }
        };
        let item = SeqItem { id: slot.id, payload };
        *self.current.borrow_mut() = Some(slot);
        Some(item)
    }

    fn item_done(&self, rsp: Option<RSP>) {
        let slot = self
            .current
            .borrow_mut()
            .take()
            .expect("item_done without get_next_item");
        if let Some(r) = rsp {
            self.responses.push(slot.id, r);
        }
        slot.done.set();
    }

    fn put_response(&self, id: TxnId, rsp: RSP) {
        self.responses.push(id, rsp);
    }
}

/// A driver's request for items. Declared `#[port(seq_item)]`.
pub type SeqItemPort<REQ, RSP = REQ> = crate::port::Port<dyn SeqItemIf<REQ, RSP>>;

/// The sequencer's side, handed to `connect`.
pub struct SeqItemExport<REQ: 'static, RSP: 'static> {
    iface: Rc<dyn SeqItemIf<REQ, RSP>>,
}

impl<REQ: 'static, RSP: 'static> SeqItemExport<REQ, RSP> {
    pub fn connect(&self, owner: &dyn PortOwner, name: PortName<dyn SeqItemIf<REQ, RSP>>) {
        bind_or_panic(owner, name, self.iface.clone());
    }

    /// Bind a port value directly — for a component handed its export at
    /// construction rather than wired in a `connect` phase.
    pub fn connect_port(&self, port: &SeqItemPort<REQ, RSP>) {
        port.bind_iface(self.iface.clone());
    }
}

// ===========================================================================
// The sequencer — a component
// ===========================================================================

/// The sequencer: a queue of items feeding one driver.
///
/// It is a **component** — it has a path and appears in the hierarchy — and a
/// cheap `Clone` handle, so an env can file one in the ConfigDb for a test to
/// find (pyuvm's `"SEQR"` idiom). Clones share state.
pub struct Sequencer<REQ: 'static, RSP: 'static = REQ> {
    inner: Rc<SeqrInner<REQ, RSP>>,
}

impl<REQ, RSP> Clone for Sequencer<REQ, RSP> {
    fn clone(&self) -> Self {
        Sequencer { inner: self.inner.clone() }
    }
}

impl<REQ: 'static, RSP: 'static> fmt::Debug for Sequencer<REQ, RSP> {
    /// `ConfigDb` values must be `Debug` for its dump (D68). A sequencer has
    /// nothing worth dumping; say which object it is.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Sequencer")
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

    /// Another handle to the *same* sequencer — for the ConfigDb.
    pub fn handle(&self) -> Sequencer<REQ, RSP> {
        self.clone()
    }

    /// The driver-side endpoint, connected in the parent's `connect` phase.
    pub fn seq_item_export(&self) -> SeqItemExport<REQ, RSP> {
        SeqItemExport { iface: self.inner.clone() }
    }
}

// A sequencer is a component: it appears in the hierarchy and its phases are
// no-ops (D84's carve-out, as for `TlmFifo` and `AnalysisBus`).
impl<REQ: 'static, RSP: 'static> Component for Sequencer<REQ, RSP> {}

impl<REQ: 'static, RSP: 'static> ComponentNode for Sequencer<REQ, RSP> {
    fn node_name(&self) -> &'static str {
        "Sequencer"
    }
    fn children_mut(&mut self) -> Vec<(String, &mut (dyn ComponentNode + 'static))> {
        Vec::new()
    }
}

// ===========================================================================
// The sequence's side
// ===========================================================================

/// What a running sequence is handed. Its equivalent of a component's
/// [`RustdvCtx`]: it can log, it has a seeded RNG, and it knows its sequencer
/// — if it has one.
pub struct SeqCtx<REQ: 'static, RSP: 'static = REQ> {
    inner: Option<Rc<SeqrInner<REQ, RSP>>>,
    current: Option<Rc<ItemSlot<REQ>>>,
    name: &'static str,
    logger: Logger,
    seed: u64,
}

impl<REQ: 'static, RSP: 'static> SeqCtx<REQ, RSP> {
    fn new(inner: Option<Rc<SeqrInner<REQ, RSP>>>, name: &'static str, seed: u64) -> Self {
        SeqCtx {
            inner,
            current: None,
            name,
            logger: Logger::at(RustdvPath::root(name)),
            seed,
        }
    }

    /// The sequence's name — the type name unless one was set (D98). For
    /// reading only: nothing is looked up by it.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// A seeded RNG, so a run reproduces. pyuvm's sequences reach for the
    /// global `random` module and do not.
    pub fn rng(&self) -> Rng {
        Rng::new(self.seed)
    }

    pub fn info(&self, msg: &str) {
        self.logger.info(msg);
    }
    pub fn warning(&self, msg: &str) {
        self.logger.warning(msg);
    }
    pub fn error(&self, msg: &str) {
        self.logger.error(msg);
    }

    fn seqr(&self) -> Result<&Rc<SeqrInner<REQ, RSP>>, SeqError> {
        self.inner.as_ref().ok_or_else(|| {
            SeqError(format!(
                "{}: start_item in a virtual sequence — it was started without a sequencer",
                self.name
            ))
        })
    }

    /// Enqueue this item and block until its turn comes. Returns with the
    /// driver committed and waiting: set the fields **now**.
    pub async fn start_item(&mut self, _item: &mut REQ) -> Result<(), SeqError> {
        if self.current.is_some() {
            return Err(SeqError("start_item called twice without finish_item".into()));
        }
        let inner = self.seqr()?.clone();
        let id = TxnId(inner.next_id.get());
        inner.next_id.set(id.0 + 1);
        let slot = Rc::new(ItemSlot {
            id,
            granted: Event::new(),
            ready: Event::new(),
            done: Event::new(),
            payload: RefCell::new(None),
        });
        self.current = Some(slot.clone());
        inner.queue.put(slot.clone()).await;
        slot.granted.wait().await;
        Ok(())
    }

    /// Hand the (now filled) item over and block until the driver releases it.
    /// Returns the ticket, for [`get_response`](Self::get_response).
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

    /// Ask whether an answer is ready, without waiting. `None` takes whatever
    /// is next; `Some(id)` looks for that ticket only.
    ///
    /// This is what a sequence with several requests outstanding polls with —
    /// the equivalent of checking the board for your number rather than
    /// standing at the counter.
    pub fn try_get_response(&mut self, txn_id: Option<TxnId>) -> Option<RSP> {
        let responses = self.seqr().ok()?.responses.clone();
        responses.try_get_response(txn_id)
    }

    /// Wait for an answer. `None` takes whatever is next; `Some(id)` waits for
    /// that ticket however many others arrive first.
    pub async fn get_response(&mut self, txn_id: Option<TxnId>) -> RSP {
        let responses = self.seqr().expect("get_response in a virtual sequence").responses.clone();
        responses.get_response(txn_id).await
    }
}

// ===========================================================================
// The Sequence trait
// ===========================================================================

/// A sequence: a program that produces stimulus.
///
/// Not a component — no place in the tree, no path, no phases. The request
/// and response types are **associated**, not parameters, so that
/// [`Factory::set_seq_override`](crate::Factory::set_seq_override) can pair
/// two sequences without being told them again.
pub trait Sequence: Sized + 'static {
    type Req: 'static;
    type Rsp: 'static;

    fn body(
        &mut self,
        ctx: &mut SeqCtx<Self::Req, Self::Rsp>,
    ) -> impl Future<Output = Result<(), SeqError>>;

    /// The name this sequence logs under (D98). Defaults to the type name, so
    /// nobody is forced to invent a label; override it when a run has two of
    /// the same type to tell apart. For reading only — nothing is looked up
    /// by it.
    fn seq_name(&self) -> &'static str {
        let full = std::any::type_name::<Self>();
        full.rsplit("::").next().unwrap_or(full)
    }

    /// Run this sequence on a sequencer, returning when it is done.
    fn start(
        &mut self,
        seqr: &Sequencer<Self::Req, Self::Rsp>,
    ) -> impl Future<Output = Result<(), SeqError>> {
        let inner = seqr.inner.clone();
        let name = self.seq_name();
        async move {
            let mut ctx = SeqCtx::new(Some(inner), name, current_seed());
            self.body(&mut ctx).await
        }
    }

    /// Run this sequence with **no** sequencer — a virtual sequence, which
    /// starts other sequences rather than sending items of its own. Calling
    /// `start_item` inside one is an error, as it is in pyuvm.
    fn start_virtual(&mut self) -> impl Future<Output = Result<(), SeqError>> {
        let name = self.seq_name();
        async move {
            let mut ctx = SeqCtx::new(None, name, current_seed());
            self.body(&mut ctx).await
        }
    }
}

thread_local! {
    static SEED: Cell<u64> = const { Cell::new(1) };
}

/// Called by the runner so sequences inherit the test's seed.
pub fn set_sequence_seed(seed: u64) {
    SEED.with(|s| s.set(seed));
}

fn current_seed() -> u64 {
    SEED.with(|s| s.get())
}

// ===========================================================================
// Boxing a sequence — what the factory returns (D99)
// ===========================================================================

/// Dyn-safe mirror of [`Sequence`], so a factory-created sequence can be held
/// in a variable. The same treatment `Component::run` needed (D48/D55).
pub trait DynSequence<REQ: 'static, RSP: 'static> {
    fn dyn_body<'a>(
        &'a mut self,
        ctx: &'a mut SeqCtx<REQ, RSP>,
    ) -> Pin<Box<dyn Future<Output = Result<(), SeqError>> + 'a>>;
    fn dyn_name(&self) -> &'static str;
}

impl<S: Sequence> DynSequence<S::Req, S::Rsp> for S {
    fn dyn_body<'a>(
        &'a mut self,
        ctx: &'a mut SeqCtx<S::Req, S::Rsp>,
    ) -> Pin<Box<dyn Future<Output = Result<(), SeqError>> + 'a>> {
        Box::pin(self.body(ctx))
    }
    fn dyn_name(&self) -> &'static str {
        self.seq_name()
    }
}

/// A slot holding any sequence with these request/response types — what
/// `create_seq()` returns, and what a component declares when the factory
/// chooses the type. The parallel of [`RustdvComp`](crate::RustdvComp).
pub struct RustdvSeq<REQ: 'static, RSP: 'static = REQ> {
    inner: Option<Box<dyn DynSequence<REQ, RSP>>>,
}

impl<REQ: 'static, RSP: 'static> Default for RustdvSeq<REQ, RSP> {
    fn default() -> Self {
        RustdvSeq { inner: None }
    }
}

impl<REQ: 'static, RSP: 'static> RustdvSeq<REQ, RSP> {
    pub fn new(seq: Box<dyn DynSequence<REQ, RSP>>) -> Self {
        RustdvSeq { inner: Some(seq) }
    }

    fn get(&mut self) -> Result<&mut Box<dyn DynSequence<REQ, RSP>>, SeqError> {
        self.inner.as_mut().ok_or_else(|| SeqError("an empty sequence slot".into()))
    }

    pub async fn start(&mut self, seqr: &Sequencer<REQ, RSP>) -> Result<(), SeqError> {
        let inner = seqr.inner.clone();
        let seq = self.get()?;
        let mut ctx = SeqCtx::new(Some(inner), seq.dyn_name(), current_seed());
        seq.dyn_body(&mut ctx).await
    }

    pub async fn start_virtual(&mut self) -> Result<(), SeqError> {
        let seq = self.get()?;
        let mut ctx = SeqCtx::new(None, seq.dyn_name(), current_seed());
        seq.dyn_body(&mut ctx).await
    }
}

// ===========================================================================
// The sequence factory (D80/D96) — a second registry, one factory
// ===========================================================================

type SeqOverrides = HashMap<TypeId, (TypeId, fn() -> Box<dyn Any>)>;

thread_local! {
    static SEQ_OVERRIDES: RefCell<SeqOverrides> = RefCell::new(HashMap::new());
}

/// Clear per test, as the ConfigDb and the component overrides are.
pub fn clear_seq_overrides() {
    SEQ_OVERRIDES.with(|o| o.borrow_mut().clear());
}

/// Install a sequence override: wherever `From::create_seq()` is called,
/// build a `To` instead.
///
/// This is the *object* half of the factory. UVM registers objects and
/// components separately (`uvm_object_utils` vs `uvm_component_utils`) and
/// creates them separately; so does rustdv. Two registries, one factory.
pub fn set_seq_override<From, To>()
where
    From: Sequence,
    To: Sequence<Req = From::Req, Rsp = From::Rsp> + Default,
{
    fn maker<To: Sequence + Default>() -> Box<dyn Any> {
        let boxed: Box<dyn DynSequence<To::Req, To::Rsp>> = Box::new(To::default());
        Box::new(boxed)
    }
    SEQ_OVERRIDES.with(|o| {
        o.borrow_mut()
            .insert(TypeId::of::<From>(), (TypeId::of::<To>(), maker::<To>));
    });
}

/// Build a sequence of this type, honouring any override installed for it.
pub fn create_seq<S>() -> RustdvSeq<S::Req, S::Rsp>
where
    S: Sequence + Default,
{
    let over = SEQ_OVERRIDES.with(|o| o.borrow().get(&TypeId::of::<S>()).map(|(_, m)| *m));
    match over {
        Some(make) => {
            let any = make();
            let boxed = any
                .downcast::<Box<dyn DynSequence<S::Req, S::Rsp>>>()
                .expect("sequence override built the wrong request/response types");
            RustdvSeq::new(*boxed)
        }
        None => RustdvSeq::new(Box::new(S::default())),
    }
}
