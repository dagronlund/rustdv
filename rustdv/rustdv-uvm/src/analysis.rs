//! Analysis broadcast: 1-to-many, never blocks, zero-or-more subscribers —
//! a broadcast, not a queue (design-doc §5.6; pyuvm uvm_analysis_port).

use std::cell::RefCell;
use std::rc::Rc;

use rustdv_sim::queue::Queue;

/// Port of uvm_subscriber's abstract `write` (mapping row 41): pyuvm
/// enforces it with a runtime UVMFatalError; Rust enforces it at compile
/// time.
pub trait Subscriber<T> {
    fn write(&mut self, item: &T);
}

/// 1-to-many broadcast port. `write(&T)` clones only for subscribers that
/// need ownership (the fifo adapter).
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

    /// Connect a subscriber (the connect convention: called where
    /// construction happens, design-doc §5.3).
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
    /// Attach an unbounded analysis FIFO (pyuvm uvm_tlm_analysis_fifo).
    pub fn connect_fifo(&self) -> AnalysisFifo<T> {
        let q = Queue::unbounded();
        let adapter = Rc::new(RefCell::new(FifoAdapter { q: q.clone() }));
        self.connect(adapter);
        AnalysisFifo { q }
    }
}

struct FifoAdapter<T> {
    q: Queue<T>,
}

impl<T: Clone> Subscriber<T> for FifoAdapter<T> {
    fn write(&mut self, item: &T) {
        // Unbounded: try_put cannot fail.
        let _ = self.q.try_put(item.clone());
    }
}

/// Unbounded FIFO fed by an AnalysisPort — the scoreboard's inbox.
pub struct AnalysisFifo<T> {
    q: Queue<T>,
}

impl<T> Clone for AnalysisFifo<T> {
    fn clone(&self) -> Self {
        AnalysisFifo { q: self.q.clone() }
    }
}

impl<T> AnalysisFifo<T> {
    pub async fn get(&self) -> T {
        self.q.get().await
    }
    pub fn try_get(&self) -> Option<T> {
        self.q.try_get()
    }
    pub fn len(&self) -> usize {
        self.q.len()
    }
    pub fn is_empty(&self) -> bool {
        self.q.is_empty()
    }
}
