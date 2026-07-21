//! Objections: distributed end-of-test consensus with RAII guards
//! (design-doc §5.3; pyuvm: ObjectionHandler, uvm_component.objection()).
//! Forgetting to drop is impossible; diagnostics (description, raise site)
//! are captured in the guard.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rustdv_sim::sync::Event;

struct ObjInner {
    count: Cell<usize>,
    drained: Event,
    raised_ever: Cell<bool>,
    active: RefCell<Vec<String>>,
}

#[derive(Clone)]
pub struct ObjectionRegistry {
    inner: Rc<ObjInner>,
}

impl ObjectionRegistry {
    #[allow(clippy::new_without_default)]
    pub fn new() -> ObjectionRegistry {
        ObjectionRegistry {
            inner: Rc::new(ObjInner {
                count: Cell::new(0),
                drained: Event::new(),
                raised_ever: Cell::new(false),
                active: RefCell::new(Vec::new()),
            }),
        }
    }

    pub fn raise(&self, description: &str) -> ObjectionGuard {
        let inner = self.inner.clone();
        inner.count.set(inner.count.get() + 1);
        inner.raised_ever.set(true);
        inner.drained.clear();
        inner.active.borrow_mut().push(description.to_string());
        ObjectionGuard { inner, description: description.to_string() }
    }

    pub fn count(&self) -> usize {
        self.inner.count.get()
    }

    /// Was an objection ever raised? The runner asks before awaiting
    /// consensus, so a Part II test that never objects is not scolded by
    /// `wait_all_dropped`'s pyuvm warning (D46: both front doors, one path).
    pub fn ever_raised(&self) -> bool {
        self.inner.raised_ever.get()
    }

    /// Objection report for timeout diagnostics (pyuvm ObjectionHandler).
    pub fn active(&self) -> Vec<String> {
        self.inner.active.borrow().clone()
    }

    pub async fn wait_all_dropped(&self) {
        if !self.inner.raised_ever.get() {
            // pyuvm's run_phase_complete warning path, ported as-is.
            rustdv_sim::log::warning(
                "all_objections_dropped awaited but no objection was ever raised",
            );
            return;
        }
        if self.inner.count.get() == 0 {
            return;
        }
        self.inner.drained.wait().await;
    }
}

/// RAII objection. Drop = drop_objection (mapping row 32).
pub struct ObjectionGuard {
    inner: Rc<ObjInner>,
    description: String,
}

impl Drop for ObjectionGuard {
    fn drop(&mut self) {
        let mut active = self.inner.active.borrow_mut();
        if let Some(pos) = active.iter().position(|d| *d == self.description) {
            active.remove(pos);
        }
        drop(active);
        let n = self.inner.count.get().saturating_sub(1);
        self.inner.count.set(n);
        if n == 0 {
            self.inner.drained.set();
        }
    }
}
