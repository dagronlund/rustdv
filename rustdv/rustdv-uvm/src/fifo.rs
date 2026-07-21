//! `TlmFifo<T>`: the UVM `uvm_tlm_fifo` — a *component* that **encapsulates**
//! a queue (pyuvm's Queue/Mailbox) so two components connect to the same
//! FIFO and neither learns the other exists (design-doc §5.6, D17/D23/D24;
//! pyuvm uvm_tlm_fifo, default depth 1).
//!
//! This encapsulation is the FIFO's *point*, not a nice-to-have: it is what
//! makes the connection late-bound. Earlier framing here ("for when the
//! FIFO should be visible in the hierarchy") understated that — hierarchy
//! visibility is a consequence, decoupling is the reason. Do not treat the
//! FIFO as optional plumbing.

use rustdv_sim::queue::Queue;

use crate::component::{Component, ComponentNode};

pub struct TlmFifo<T: 'static> {
    q: Queue<T>,
    size: Option<usize>,
}

impl<T> TlmFifo<T> {
    /// `size = None` → unbounded; default UVM depth is 1.
    pub fn new(size: Option<usize>) -> TlmFifo<T> {
        TlmFifo { q: Queue::new(size), size }
    }

    pub fn size(&self) -> Option<usize> {
        self.size
    }
    pub fn used(&self) -> usize {
        self.q.len()
    }
    pub fn is_empty(&self) -> bool {
        self.q.is_empty()
    }
    pub fn is_full(&self) -> bool {
        match self.size {
            None => false,
            Some(s) => self.q.len() >= s,
        }
    }

    pub async fn put(&self, item: T) {
        self.q.put(item).await;
    }
    pub fn try_put(&self, item: T) -> Result<(), T> {
        self.q.try_put(item)
    }
    pub async fn get(&self) -> T {
        self.q.get().await
    }
    pub fn try_get(&self) -> Option<T> {
        self.q.try_get()
    }

    pub fn flush(&self) {
        while self.q.try_get().is_some() {}
    }

    /// Shared handle to the same FIFO (for wiring at construction).
    pub fn handle(&self) -> TlmFifo<T> {
        TlmFifo { q: self.q.clone(), size: self.size }
    }
}

impl<T> Component for TlmFifo<T> {}

impl<T> ComponentNode for TlmFifo<T> {
    fn node_name(&self) -> &'static str {
        "TlmFifo"
    }
    fn children_mut(&mut self) -> Vec<(String, &mut dyn ComponentNode)> {
        Vec::new()
    }
}
