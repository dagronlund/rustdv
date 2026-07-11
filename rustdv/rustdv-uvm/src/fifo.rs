//! `TlmFifo<T>`: a *component* wrapping a channel, for when the FIFO
//! should be visible in the hierarchy with size/used/flush (design-doc
//! §5.6; pyuvm uvm_tlm_fifo, default depth 1).

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
    fn visit_children(&mut self, _f: &mut dyn FnMut(&str, &mut dyn ComponentNode)) {}
}
