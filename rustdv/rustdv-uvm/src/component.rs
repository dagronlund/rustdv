//! Component lifecycle and hierarchy traversal (design-doc §5.2/§5.3).
//!
//! > **Superseded, being replaced — do not build on it.** What this file
//! > still implements is review-memo R3: build/connect as *constructor
//! > conventions* rather than phases, a component's `new(config, ...)`
//! > constructing its children (`// build:`) and taking channel endpoints
//! > as arguments (`// connect:`), with only the runtime lifecycle left as
//! > a trait.
//! >
//! > **D5 and D6 reverse that.** `build` (top-down) and `connect`
//! > (bottom-up) are being restored as real phases, because the gap
//! > between "a component exists" and "its children exist" is where all
//! > late binding lives — path-addressed configuration, factory
//! > overrides, TLM connection. Removing the phases removed the gap, and
//! > with it rustdv's ability to configure or override anything the user
//! > did not write. See `output/.design-decisions.md`; a future session
//! > must not read R3 here and take it for the intended design.
//!
//! **What has actually landed (step 4, D46–D49).** A test is a component:
//! [`Component`] gained an `async fn run`, and the context it receives is
//! the single universal [`RustdvCtx`] — the old `TestCtx` (runner) and
//! `RunCtx` merged. `build`/`connect`, `build_child` and two-stage
//! construction are **not here yet**; they arrive with ch24.

use rustdv_sim::handle::HierarchyHandle;
use rustdv_sim::log::Logger;
use rustdv_sim::rng::Rng;

use crate::error::TestError;
use crate::objection::{ObjectionGuard, ObjectionRegistry};

/// Agent activity (pyuvm's ConfigDB `is_active` int becomes an enum —
/// mapping row 42; illegal values are unrepresentable).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Active {
    Active,
    Passive,
}

/// Collector for `check`-phase failures (design-doc §5.3 signature).
#[derive(Default)]
pub struct CheckSink {
    errors: Vec<String>,
}

impl CheckSink {
    pub fn new() -> CheckSink {
        CheckSink::default()
    }
    pub fn error(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        rustdv_sim::log::error(&msg);
        self.errors.push(msg);
    }
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
    pub fn errors(&self) -> &[String] {
        &self.errors
    }
    pub fn into_result(self) -> Result<(), String> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(format!("{} check failure(s): {}", self.errors.len(), self.errors.join("; ")))
        }
    }
}

// ===========================================================================
// RustdvCtx — the one context (D47, which strikes D8)
// ===========================================================================

/// Everything a running testbench is handed: the DUT, randomization, the
/// objection registry, and the component's path.
///
/// **One type, not one per phase.** D8 wanted `BuildCtx`/`ConnectCtx`/
/// `RunCtx` so that `build_child` during run would fail to *compile*. D47
/// gives that up: Part II teaches a testbench with no components and
/// therefore no phases, and naming the type after a phase names a concept
/// the reader has not met. Phase-illegal operations are caught at run time,
/// as UVM catches them.
///
/// `Clone` is deliberate — the objection registry is `Rc`-shared, so a
/// clone objects to the same test. D9's per-node context, when the build
/// phase arrives, is that clone with the path extended.
#[derive(Clone)]
pub struct RustdvCtx {
    dut: HierarchyHandle,
    seed: u64,
    objections: ObjectionRegistry,
    logger: Logger,
}

impl RustdvCtx {
    /// Built by the runner, once per test, with `path` the test's
    /// registered name (D49 — UVM's fixed `uvm_test_top` is not ported).
    pub fn new(path: &str, dut: HierarchyHandle, seed: u64) -> RustdvCtx {
        RustdvCtx { dut, seed, objections: ObjectionRegistry::new(), logger: Logger::new(path) }
    }

    pub fn dut(&self) -> HierarchyHandle {
        self.dut
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// A deterministic RNG seeded from RUSTDV_RANDOM_SEED + test index.
    pub fn rng(&self) -> Rng {
        Rng::new(self.seed)
    }

    /// This component's path — derived by the walk, never stored on the
    /// component itself (D7).
    pub fn path(&self) -> &str {
        self.logger.path()
    }

    // --- Path-aware logging (D7's first real appearance) ------------------
    //
    // `log::info(...)` reaches a global sink with no idea who called it, and
    // a hand-typed `Logger::new("env.loga")` silently lies the moment a
    // component moves. These do not, because the path came from the walk.

    pub fn debug(&self, msg: &str) {
        self.logger.debug(msg);
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
    pub fn critical(&self, msg: &str) {
        self.logger.critical(msg);
    }

    /// The logger itself, for code that wants to hold one.
    pub fn logger(&self) -> &Logger {
        &self.logger
    }

    // --- Objections -------------------------------------------------------

    /// Port of raise_objection, returning a guard whose Drop is
    /// drop_objection (pyuvm: uvm_component.objection()).
    pub fn raise_objection(&self, description: &str) -> ObjectionGuard {
        self.objections.raise(description)
    }

    pub fn objections(&self) -> &ObjectionRegistry {
        &self.objections
    }

    /// Wait until every raised objection has been dropped. Logs the pyuvm
    /// "you never objected" warning if nothing was ever raised.
    pub async fn all_objections_dropped(&self) {
        self.objections.wait_all_dropped().await;
    }
}

// ===========================================================================
// The lifecycle
// ===========================================================================

/// The runtime lifecycle (design-doc §5.3). Default empty bodies replicate
/// pyuvm's no-op base methods — override only what you use.
pub trait Component {
    /// The test body: UVM's `run_phase`. Returning `Err` fails the test.
    ///
    /// `async fn` in a trait costs dyn-compatibility, which is why the sync
    /// phases below are mirrored onto [`DynPhases`] for traversal (D48).
    #[allow(async_fn_in_trait)]
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _ = ctx;
        Ok(())
    }

    /// Spawn free-running behavior (drivers, monitors); bottom-up order,
    /// like pyuvm's run_phase spawning.
    ///
    /// **Transitional.** This is the pre-step-4 spawn hook, kept so the
    /// component chapters and `tinyalu_tb` keep building while ch23 lands.
    /// ch24 folds it into `run`.
    fn start(&mut self, ctx: &mut RustdvCtx) {
        let _ = ctx;
    }
    /// Top-down, post-run.
    fn extract(&mut self) {}
    /// Top-down.
    fn check(&mut self, errors: &mut CheckSink) {
        let _ = errors;
    }
    /// Top-down.
    fn report(&self) {}
    fn final_phase(&self) {}
}

/// Dyn-safe mirror of [`Component`]'s synchronous phases (D48).
///
/// `Component` stopped being dyn-compatible the moment `run` became an
/// `async fn`, and `ComponentNode` needs a dyn-safe supertrait to walk a
/// tree of `&mut dyn` children. The blanket impl means users never write
/// this: they override the phases on `Component` as before, and the
/// distinct method names keep `component.extract()` unambiguous.
pub trait DynPhases {
    fn dyn_start(&mut self, ctx: &mut RustdvCtx);
    fn dyn_extract(&mut self);
    fn dyn_check(&mut self, errors: &mut CheckSink);
    fn dyn_report(&self);
    fn dyn_final(&self);
}

impl<T: Component> DynPhases for T {
    fn dyn_start(&mut self, ctx: &mut RustdvCtx) {
        Component::start(self, ctx)
    }
    fn dyn_extract(&mut self) {
        Component::extract(self)
    }
    fn dyn_check(&mut self, errors: &mut CheckSink) {
        Component::check(self, errors)
    }
    fn dyn_report(&self) {
        Component::report(self)
    }
    fn dyn_final(&self) {
        Component::final_phase(self)
    }
}

/// Structural traversal over the ownership tree (design-doc D5.2).
/// Generated by `#[derive(Component)]` for structs whose children are
/// fields marked `#[component(child)]`; hand-implementable by design
/// (OQ-15: the derive is convenience, not requirement).
pub trait ComponentNode: DynPhases {
    /// The component's type-level name (hierarchical path is synthesized
    /// from field names during traversal).
    fn node_name(&self) -> &'static str;

    /// Visit direct children as (field_name, node) pairs.
    fn visit_children(&mut self, f: &mut dyn FnMut(&str, &mut dyn ComponentNode));
}

// ---------------------------------------------------------------------------
// Traversals (pyuvm _s09 orders: run is bottom-up, the rest top-down)
// ---------------------------------------------------------------------------

/// Bottom-up: children start before parents (pyuvm run_phase order).
pub fn start_all(node: &mut dyn ComponentNode, ctx: &mut RustdvCtx) {
    node.visit_children(&mut |_name, child| start_all(child, ctx));
    node.dyn_start(ctx);
}

/// Top-down.
pub fn extract_all(node: &mut dyn ComponentNode) {
    node.dyn_extract();
    node.visit_children(&mut |_name, child| extract_all(child));
}

/// Top-down.
pub fn check_all(node: &mut dyn ComponentNode, sink: &mut CheckSink) {
    node.dyn_check(sink);
    node.visit_children(&mut |_name, child| check_all(child, sink));
}

/// Top-down.
pub fn report_all(node: &mut dyn ComponentNode) {
    node.dyn_report();
    node.visit_children(&mut |_name, child| report_all(child));
}

pub fn final_all(node: &mut dyn ComponentNode) {
    node.dyn_final();
    node.visit_children(&mut |_name, child| final_all(child));
}

/// The standard post-run tail: extract → check → report → final, returning
/// `Err` if any check failed (an `Err` fails the test, design-doc §0.6).
pub fn run_extract_check_report(node: &mut dyn ComponentNode) -> Result<(), String> {
    extract_all(node);
    let mut sink = CheckSink::new();
    check_all(node, &mut sink);
    report_all(node);
    final_all(node);
    sink.into_result()
}

/// Debug printer: the `visit_children` walker serving pyuvm's hierarchy
/// print (design-doc §5.2).
pub fn print_hierarchy(node: &mut dyn ComponentNode) {
    fn rec(node: &mut dyn ComponentNode, path: &str) {
        rustdv_sim::log::info(&format!("{path} ({})", node.node_name()));
        let parent = path.to_string();
        node.visit_children(&mut |name, child| {
            rec(child, &format!("{parent}.{name}"));
        });
    }
    rec(node, "top");
}
