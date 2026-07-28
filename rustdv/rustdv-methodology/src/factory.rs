//! The factory (design-doc §15, D69–D75).
//!
//! rustdv has a factory, and it works as the UVM factory works (D74). It is
//! not a separate subsystem: a "maker" is an ordinary Rust value, so the
//! override table is the [`ConfigDb`](crate::config) and registration is a
//! link-time section like the test registry. From the user's chair there are
//! two constructors — `Foo::new_comp()` (fixed) and `Foo::create_comp()`
//! (overridable) — and `Factory::…` to install and inspect overrides.
//!
//! **How a `create_comp()` slot is overridden.** It is not resolved at the
//! call — a `create_comp()` builds the default type immediately and flags the
//! [`RustdvComp`] as factory-owned (D75). During the build walk, where the field
//! name and so the path are finally known, the framework asks each flagged
//! slot for its override (instance first, then type, by ConfigDb specificity,
//! D13) and swaps it in before descending.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;

use crate::component::{Component, ComponentNode, RustdvCtx};
use crate::config::ConfigDb;

/// A maker: builds a component with no arguments (its name and parent come
/// from the tree, D7). Non-capturing, so it is an ordinary `fn` pointer.
pub type Maker = fn() -> Box<dyn ComponentNode>;

// ===========================================================================
// RustdvComp — the child slot (D75)
// ===========================================================================

/// A slot that holds any component. It says nothing about position in the
/// tree and nothing about overridability; the *build line* decides that
/// (`new_comp()` fixed, `create_comp()` overridable). Every child field a
/// block may want to override is an `RustdvComp`.
#[derive(Default)]
pub struct RustdvComp {
    inner: Option<Box<dyn ComponentNode>>,
    /// Set by `create_comp()`; the walk checks flagged slots for an override.
    overridable: bool,
    /// The requested type's registered name, for the override lookup.
    requested: Option<&'static str>,
}

impl RustdvComp {
    /// A fixed slot: `new_comp()`. Never overridden.
    pub fn fixed(node: Box<dyn ComponentNode>) -> RustdvComp {
        RustdvComp { inner: Some(node), overridable: false, requested: None }
    }

    /// A factory slot: `create_comp()`. The default is built now and may be
    /// swapped for an override during the walk.
    pub fn overridable(node: Box<dyn ComponentNode>, requested: &'static str) -> RustdvComp {
        RustdvComp { inner: Some(node), overridable: true, requested: Some(requested) }
    }

    /// The held component, for the traversal. `None` before it is filled.
    /// The object lifetime is `'static` (a boxed component always is), which
    /// matches [`ComponentNode::children_mut`]'s element type.
    pub fn as_node_mut(&mut self) -> Option<&mut (dyn ComponentNode + 'static)> {
        self.inner.as_deref_mut()
    }

    /// Move the held component **out** of the slot, leaving it empty (D82b).
    ///
    /// This is what lets a parent's `run` be concurrent with its children's.
    /// While the box sits in the slot it is part of the parent, so `&mut
    /// parent` and `&mut child` overlap and cannot both exist. Once moved out
    /// it is an independent value with no borrow relationship to the parent,
    /// so both futures can be driven together.
    ///
    /// The slot is empty only for the duration of the run phase;
    /// [`put_node`] restores it before the post-run phases walk the tree.
    pub fn take_node(&mut self) -> Option<Box<dyn ComponentNode>> {
        self.inner.take()
    }

    /// Put a component taken by [`take_node`] back into the slot.
    pub fn put_node(&mut self, node: Box<dyn ComponentNode>) {
        self.inner = Some(node);
    }

    /// Called by the derive-generated resolver during the build walk, with
    /// this slot's field name. If flagged and an override applies at the
    /// slot's path, swap it in. The discarded default's phases never ran —
    /// resolution happens before the walk descends into the child.
    pub fn resolve(&mut self, ctx: &RustdvCtx, name: &str) {
        if !self.overridable {
            return;
        }
        let Some(req) = self.requested else { return };
        let path = if ctx.path().is_empty() {
            name.to_string()
        } else {
            format!("{}.{}", ctx.path(), name)
        };
        if let Some(ov) = Factory::lookup_override(req, &path) {
            self.inner = Some((ov.make)());
        }
        self.overridable = false;
    }
}

// ===========================================================================
// Overrides
// ===========================================================================

/// An installed override: what to build, and the target's name for the dump.
#[derive(Clone, Copy)]
pub struct Override {
    make: Maker,
    to: &'static str,
}

impl fmt::Debug for Override {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "-> {}", self.to)
    }
}

fn override_key(requested_name: &str) -> String {
    format!("__factory_override__{requested_name}")
}

// ===========================================================================
// Universal registration (D73) — a link-time section, like the test registry
//
// The section name `rustdv_comps` must be **≤ 16 bytes**: Mach-O caps section
// names at 16 characters, and rustc rejects a longer one only on Apple
// targets (ELF has no such limit, so Linux never complains). The original
// `rustdv_components` was 17 and broke the macOS build while the Linux VM
// stayed green. Keep any future section name short.
// ===========================================================================

/// One registered component: its name and its maker. Emitted by
/// `#[derive(Component)]` for every component, universally (D73).
pub struct ComponentReg {
    /// Accessor rather than a const string, so the derive can compute it
    /// from the type without `const` gymnastics.
    pub name: fn() -> &'static str,
    pub make: Maker,
}

fn sentinel_name() -> &'static str {
    "__rustdv_component_sentinel"
}
fn sentinel_make() -> Box<dyn ComponentNode> {
    panic!("the component-registry sentinel must never be built")
}

#[used]
#[cfg_attr(not(target_vendor = "apple"), link_section = "rustdv_comps")]
#[cfg_attr(target_vendor = "apple", link_section = "__DATA,rustdv_comps")]
static SENTINEL: &ComponentReg = &ComponentReg { name: sentinel_name, make: sentinel_make };

#[cfg(not(target_vendor = "apple"))]
extern "C" {
    static __start_rustdv_comps: u8;
    static __stop_rustdv_comps: u8;
}
#[cfg(target_vendor = "apple")]
extern "C" {
    #[link_name = "\x01section$start$__DATA$rustdv_comps"]
    static __start_rustdv_comps: u8;
    #[link_name = "\x01section$end$__DATA$rustdv_comps"]
    static __stop_rustdv_comps: u8;
}

fn collect_registry() -> HashMap<&'static str, Maker> {
    std::hint::black_box(SENTINEL.name);
    let mut map = HashMap::new();
    unsafe {
        let start = std::ptr::addr_of!(__start_rustdv_comps) as usize;
        let stop = std::ptr::addr_of!(__stop_rustdv_comps) as usize;
        let step = std::mem::size_of::<&ComponentReg>();
        let base = start as *const &'static ComponentReg;
        for i in 0..((stop - start) / step) {
            let reg = *base.add(i);
            let name = (reg.name)();
            if name != sentinel_name() {
                map.insert(name, reg.make);
            }
        }
    }
    map
}

thread_local! {
    /// Built once from the link-time section (the set of types does not
    /// change per test, unlike the override table).
    static REGISTRY: RefCell<Option<HashMap<&'static str, Maker>>> = const { RefCell::new(None) };
}

fn with_registry<R>(f: impl FnOnce(&HashMap<&'static str, Maker>) -> R) -> R {
    REGISTRY.with(|r| {
        let mut slot = r.borrow_mut();
        if slot.is_none() {
            *slot = Some(collect_registry());
        }
        f(slot.as_ref().unwrap())
    })
}

// ===========================================================================
// The facade
// ===========================================================================

/// The factory. Ambient, like the ConfigDb it is built on; every method is
/// an associated function.
pub struct Factory;

impl Factory {
    /// Build a component from its registered string name (D71). Overridable,
    /// like anything from the factory. A name that is not registered is a
    /// testbench bug and panics; the file-driven form (ch39) will return a
    /// `Result` instead.
    pub fn create_by_name(name: &str) -> RustdvComp {
        let make = with_registry(|reg| reg.get(name).copied());
        match make {
            Some(make) => {
                // `requested` needs a 'static name; recover the registry's
                // key so a by-name-created component can also be overridden.
                let stored = with_registry(|reg| reg.keys().find(|k| **k == name).copied());
                RustdvComp::overridable(make(), stored.expect("just found it"))
            }
            None => panic!("Factory::create_by_name: no component registered as \"{name}\""),
        }
    }

    /// Override every `From::create_comp()` with a `To`, testbench-wide
    /// (UVM `set_type_override_by_type`). Compile-checked: `To` must be a
    /// component.
    pub fn set_type_override<From, To>()
    where
        From: Component + ComponentNode + Default + 'static,
        To: Component + ComponentNode + Default + 'static,
    {
        Self::store_override(None, "*", From::comp_name(), To::comp_name(), || {
            Box::new(To::default())
        });
    }

    /// The same, by string name (UVM `set_type_override_by_name`). Not
    /// compile-checked; an unregistered `to` panics at this call.
    pub fn set_type_override_by_name(from: &str, to: &str) {
        let make = with_registry(|reg| reg.get(to).copied())
            .unwrap_or_else(|| panic!("Factory::set_type_override_by_name: \"{to}\" is not registered"));
        let to_static = with_registry(|reg| reg.keys().find(|k| **k == to).copied()).expect("just found it");
        Self::store_override(None, "*", from, to_static, make);
    }

    /// Override a single instance, addressed by its path relative to `ctx`
    /// (UVM `set_inst_override_by_type`). The path is a string because it
    /// names a component elsewhere in the tree — not a duplicate of a field
    /// name (D75).
    pub fn set_inst_override<From, To>(ctx: &RustdvCtx, path: &str)
    where
        From: Component + ComponentNode + Default + 'static,
        To: Component + ComponentNode + Default + 'static,
    {
        Self::store_override(Some(ctx), path, From::comp_name(), To::comp_name(), || {
            Box::new(To::default())
        });
    }

    fn store_override(
        ctx: Option<&RustdvCtx>,
        offset: &str,
        from_name: &str,
        to_name: &'static str,
        make: Maker,
    ) {
        ConfigDb::set(ctx, offset, &override_key(from_name), Override { make, to: to_name });
    }

    /// The override in force for `requested_name` at `abs_path`, if any.
    pub(crate) fn lookup_override(requested_name: &str, abs_path: &str) -> Option<Override> {
        ConfigDb::get::<Override>(None, abs_path, &override_key(requested_name)).ok()
    }

    /// Print the overrides in force (UVM `uvm_factory().print()`). It is the
    /// ConfigDb store, shown through the factory's window (D68).
    pub fn print() {
        rustdv_sim::log::info("Factory overrides:");
        for (path, from, to) in ConfigDb::factory_overrides() {
            rustdv_sim::log::info(&format!("  {path:<28}: {from} -> {to}"));
        }
    }
}
