//! Per-test singletons — the port of pyuvm's `Singleton` metaclass and
//! `uvm_root.clear_singletons()`.
//!
//! **Why this exists (D57).** From testbench 4.0 on, sibling components need
//! to reach the same object — the BFM above all. pyuvm writes
//! `class TinyAluBfm(metaclass=pyuvm.Singleton)` and every component calls
//! `TinyAluBfm()`; SystemVerilog writes
//! `uvm_config_db#(virtual tinyalu_bfm)::set(null, "*", "bfm", bfm)`. Both
//! statically-capable designs chose a *runtime* lookup rather than threading
//! the handle through constructors, which is D3's detection rule exactly.
//! rustdv's `build` phase takes no constructor arguments (D6), so an ambient
//! handle is not merely convenient here — it is the mechanism.
//!
//! **Per-test, not forever.** pyuvm's `run_test` clears singletons before
//! every test unless you ask it not to, so each test gets a fresh BFM with
//! empty queues. The runner does the same (D16's rule, applied beyond the
//! ConfigDb): a singleton is scoped to one test, never to the process.
//!
//! `Rc`, not `Arc`, and a `thread_local` store: the executor is documented
//! `!Send` and never leaves the sim thread (D11's reasoning).

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

thread_local! {
    static SINGLETONS: RefCell<HashMap<TypeId, Rc<dyn Any>>> =
        RefCell::new(HashMap::new());
}

/// The first call in a test constructs; every later call returns the same
/// object. Port of pyuvm's `Singleton.__call__`.
///
/// ```ignore
/// impl TinyAluBfm {
///     pub fn get() -> Rc<TinyAluBfm> {
///         singleton(|| TinyAluBfm::new(&top_module().unwrap()).unwrap())
///     }
/// }
/// ```
pub fn singleton<T: 'static>(init: impl FnOnce() -> T) -> Rc<T> {
    // The store is *not* borrowed across `init()`: a constructor is user
    // code and may reach for another singleton, which would otherwise be a
    // double borrow at runtime.
    let existing = SINGLETONS.with(|s| s.borrow().get(&TypeId::of::<T>()).cloned());
    if let Some(any) = existing {
        return any.downcast::<T>().expect("singleton registered under a mismatched type");
    }
    let value: Rc<T> = Rc::new(init());
    SINGLETONS.with(|s| {
        s.borrow_mut().insert(TypeId::of::<T>(), value.clone() as Rc<dyn Any>);
    });
    value
}

/// Is this singleton already built in the current test? Mostly for tests of
/// the framework itself.
pub fn singleton_exists<T: 'static>() -> bool {
    SINGLETONS.with(|s| s.borrow().contains_key(&TypeId::of::<T>()))
}

/// Drop every singleton — called by the runner between tests, matching
/// pyuvm's `run_test(..., keep_singletons=False)` default.
///
/// Objects still held by a running task survive until that task is
/// cancelled; this only clears the *registry*, so the next `singleton` call
/// constructs afresh.
pub fn clear_singletons() {
    SINGLETONS.with(|s| s.borrow_mut().clear());
}
