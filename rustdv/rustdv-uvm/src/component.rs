//! Component lifecycle and hierarchy traversal (design-doc §5.2/§5.3).
//!
//! > **Superseded, being replaced — do not build on it.** What this file
//! > still implements is review-memo R3: build/connect as *constructor
//! > conventions* rather than phases, a component's `new(config, ...)`
//! > constructing its children (`// build:`) and taking channel endpoints
//! > as arguments (`// connect:`), with only the runtime lifecycle left as
//! > a trait.
//! >
//! > **D5 and D6 reverse that, and the reversal has begun.** `build`
//! > (top-down) and `connect` (bottom-up) are real phase methods again
//! > (D51), because the gap between "a component exists" and "its children
//! > exist" is where all late binding lives — path-addressed configuration,
//! > factory overrides, TLM connection. The [`Component`] trait below now
//! > carries all nine phases and [`run_component_test`] drives them.
//! > `new(config, ...)`-style construction survives only in not-yet-
//! > converted testbenches (`tinyalu_tb`), not as the design.
//!
//! **What has landed.** Step 4 (D46–D49): a test is a component with an
//! `async fn run`, receiving the one universal [`RustdvCtx`]. Ch24 first
//! half (D51/D52): the nine phases, the phaser, path-aware phase logging.
//! Ch24 second half: two-stage construction (a parent creates children as
//! `Option<T>`/`Vec<T>` in its own `build`) and the bottom-up [`run_all`]
//! traversal firing every component's run.

use std::future::Future;
use std::pin::Pin;

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

    /// A child context: same services, path extended by `name` (D9). The
    /// phase traversals hand each child its own context so a component's
    /// `ctx.info()` logs the path the walk derived, never a stored string.
    pub fn child(&self, name: &str) -> RustdvCtx {
        RustdvCtx {
            dut: self.dut,
            seed: self.seed,
            objections: self.objections.clone(),
            logger: Logger::new(&format!("{}.{}", self.logger.path(), name)),
        }
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

/// The UVM phase lifecycle (design-doc §5.3, D51), restored in full. Nine
/// phases, each a method with a default no-op body — override only what you
/// use, exactly as pyuvm's `uvm_component` does. **`build` and `connect`
/// are real phases again**, not the "constructor conventions" R3 collapsed
/// them into; restoring them is the point of this chapter.
///
/// Every phase receives the context so it can log with the component's
/// derived path (D52/D7). The runner drives the whole sequence over the
/// tree (see [`run_component_test`]), so a component author never calls a
/// phase by hand.
pub trait Component {
    /// 1. `build` — top-down. Where a component constructs its children
    /// (D6); the gap between "a component exists" and "its children exist"
    /// that all late binding lives in.
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let _ = ctx;
    }
    /// 2. `connect` — bottom-up. Wire children together once they exist.
    fn connect(&mut self, ctx: &mut RustdvCtx) {
        let _ = ctx;
    }
    /// 3. `end_of_elaboration` — top-down. The hierarchy is final.
    fn end_of_elaboration(&mut self, ctx: &mut RustdvCtx) {
        let _ = ctx;
    }
    /// 4. `start_of_simulation` — top-down. Last chance before time moves.
    fn start_of_simulation(&mut self, ctx: &mut RustdvCtx) {
        let _ = ctx;
    }
    /// 5. `run` — bottom-up, async, objection-gated. The test body; `Err`
    /// fails the test.
    ///
    /// `async fn` in a trait costs dyn-compatibility, which is why the sync
    /// phases are mirrored onto [`DynPhases`] for traversal (D48).
    #[allow(async_fn_in_trait)]
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _ = ctx;
        Ok(())
    }
    /// 6. `extract` — top-down, post-run.
    fn extract(&mut self, ctx: &mut RustdvCtx) {
        let _ = ctx;
    }
    /// 7. `check` — top-down. Report failures into the sink.
    fn check(&mut self, ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        let _ = (ctx, errors);
    }
    /// 8. `report` — top-down.
    fn report(&mut self, ctx: &mut RustdvCtx) {
        let _ = ctx;
    }
    /// 9. `final_phase` — top-down. (`final` is a Rust keyword.)
    fn final_phase(&mut self, ctx: &mut RustdvCtx) {
        let _ = ctx;
    }

    /// **Transitional spawn hook**, pre-dating the restored `run`
    /// traversal. `tinyalu_tb` and the not-yet-converted chapters still
    /// spawn free-running behavior here; it folds into `run` as each
    /// converts. Not part of the nine-phase lifecycle.
    fn start(&mut self, ctx: &mut RustdvCtx) {
        let _ = ctx;
    }
}

/// Dyn-safe mirror of [`Component`]'s non-async phases (D48).
///
/// `Component` stopped being dyn-compatible the moment `run` became an
/// `async fn`, and `ComponentNode` needs a dyn-safe supertrait to walk a
/// tree of `&mut dyn` children. The blanket impl means users never write
/// this: they override the phases on `Component`, and the distinct method
/// names keep `component.extract(..)` unambiguous.
pub trait DynPhases {
    fn dyn_build(&mut self, ctx: &mut RustdvCtx);
    fn dyn_connect(&mut self, ctx: &mut RustdvCtx);
    fn dyn_end_of_elaboration(&mut self, ctx: &mut RustdvCtx);
    fn dyn_start_of_simulation(&mut self, ctx: &mut RustdvCtx);
    /// The async `run`, boxed so it can be awaited behind `dyn` (D48). This
    /// is the object-safe shim `run_all` needs to fire each component's run.
    fn dyn_run<'a>(
        &'a mut self,
        ctx: &'a mut RustdvCtx,
    ) -> Pin<Box<dyn Future<Output = Result<(), TestError>> + 'a>>;
    fn dyn_extract(&mut self, ctx: &mut RustdvCtx);
    fn dyn_check(&mut self, ctx: &mut RustdvCtx, errors: &mut CheckSink);
    fn dyn_report(&mut self, ctx: &mut RustdvCtx);
    fn dyn_final(&mut self, ctx: &mut RustdvCtx);
    fn dyn_start(&mut self, ctx: &mut RustdvCtx);
}

impl<T: Component> DynPhases for T {
    fn dyn_build(&mut self, ctx: &mut RustdvCtx) {
        Component::build(self, ctx)
    }
    fn dyn_run<'a>(
        &'a mut self,
        ctx: &'a mut RustdvCtx,
    ) -> Pin<Box<dyn Future<Output = Result<(), TestError>> + 'a>> {
        Box::pin(Component::run(self, ctx))
    }
    fn dyn_connect(&mut self, ctx: &mut RustdvCtx) {
        Component::connect(self, ctx)
    }
    fn dyn_end_of_elaboration(&mut self, ctx: &mut RustdvCtx) {
        Component::end_of_elaboration(self, ctx)
    }
    fn dyn_start_of_simulation(&mut self, ctx: &mut RustdvCtx) {
        Component::start_of_simulation(self, ctx)
    }
    fn dyn_extract(&mut self, ctx: &mut RustdvCtx) {
        Component::extract(self, ctx)
    }
    fn dyn_check(&mut self, ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        Component::check(self, ctx, errors)
    }
    fn dyn_report(&mut self, ctx: &mut RustdvCtx) {
        Component::report(self, ctx)
    }
    fn dyn_final(&mut self, ctx: &mut RustdvCtx) {
        Component::final_phase(self, ctx)
    }
    fn dyn_start(&mut self, ctx: &mut RustdvCtx) {
        Component::start(self, ctx)
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

    /// Direct children as (field-derived name, node) pairs, in declaration
    /// order. An **owned Vec**, not a `visit_children`-style sync callback:
    /// [`run_all`] awaits inside the walk, and a higher-ranked closure
    /// cannot yield a child borrow that outlives the call, so the borrows
    /// have to come back in a value the caller holds. An `Option<T>` child
    /// created during `build` (D6) appears here only once it is `Some`.
    fn children_mut(&mut self) -> Vec<(String, &mut dyn ComponentNode)>;
}

// ---------------------------------------------------------------------------
// Phase traversals (pyuvm order, D34): build top-down, connect bottom-up,
// run bottom-up, the elaboration and post-run phases top-down.
//
// Each child is walked with its own context (path extended, D9), so a
// component always logs under the path the walk gave it. Top-down = the
// node acts, then its children; bottom-up = children first, then the node.
// ---------------------------------------------------------------------------

/// Top-down: `build` a node, then build the children it just created (D6).
/// Reading `children_mut` *after* `build` is what lets a parent construct
/// them in its own build phase and have the walk descend into them.
pub fn build_all(node: &mut dyn ComponentNode, ctx: &mut RustdvCtx) {
    node.dyn_build(ctx);
    for (name, child) in node.children_mut() {
        let mut cctx = ctx.child(&name);
        build_all(child, &mut cctx);
    }
}

/// Bottom-up: children `connect` before parents.
pub fn connect_all(node: &mut dyn ComponentNode, ctx: &mut RustdvCtx) {
    for (name, child) in node.children_mut() {
        let mut cctx = ctx.child(&name);
        connect_all(child, &mut cctx);
    }
    node.dyn_connect(ctx);
}

/// Top-down.
pub fn end_of_elaboration_all(node: &mut dyn ComponentNode, ctx: &mut RustdvCtx) {
    node.dyn_end_of_elaboration(ctx);
    for (name, child) in node.children_mut() {
        let mut cctx = ctx.child(&name);
        end_of_elaboration_all(child, &mut cctx);
    }
}

/// Top-down.
pub fn start_of_simulation_all(node: &mut dyn ComponentNode, ctx: &mut RustdvCtx) {
    node.dyn_start_of_simulation(ctx);
    for (name, child) in node.children_mut() {
        let mut cctx = ctx.child(&name);
        start_of_simulation_all(child, &mut cctx);
    }
}

/// Bottom-up: every component's `run` fires, children first (D48). The walk
/// is boxed-recursive because `dyn_run` yields a boxed future we await, and
/// the children's borrows are held across those awaits.
///
/// **Sequential, not concurrent (recorded limitation).** Each `run` is
/// awaited to completion before the next — correct while run bodies are
/// self-contained (raise objection, act, drop). True concurrent run phases
/// need spawned `'static` tasks (the `start` hook's model); the executor's
/// `spawn` is `'static`-bound, so a borrowed-tree concurrent run is a later
/// increment. See the design-decisions log.
pub fn run_all<'a>(
    node: &'a mut dyn ComponentNode,
    ctx: &'a mut RustdvCtx,
) -> Pin<Box<dyn Future<Output = Result<(), TestError>> + 'a>> {
    Box::pin(async move {
        for (name, child) in node.children_mut() {
            let mut cctx = ctx.child(&name);
            run_all(child, &mut cctx).await?;
        }
        node.dyn_run(ctx).await
    })
}

/// Bottom-up: children start before parents (transitional spawn hook).
pub fn start_all(node: &mut dyn ComponentNode, ctx: &mut RustdvCtx) {
    for (name, child) in node.children_mut() {
        let mut cctx = ctx.child(&name);
        start_all(child, &mut cctx);
    }
    node.dyn_start(ctx);
}

/// Top-down.
pub fn extract_all(node: &mut dyn ComponentNode, ctx: &mut RustdvCtx) {
    node.dyn_extract(ctx);
    for (name, child) in node.children_mut() {
        let mut cctx = ctx.child(&name);
        extract_all(child, &mut cctx);
    }
}

/// Top-down.
pub fn check_all(node: &mut dyn ComponentNode, ctx: &mut RustdvCtx, sink: &mut CheckSink) {
    node.dyn_check(ctx, sink);
    for (name, child) in node.children_mut() {
        let mut cctx = ctx.child(&name);
        check_all(child, &mut cctx, sink);
    }
}

/// Top-down.
pub fn report_all(node: &mut dyn ComponentNode, ctx: &mut RustdvCtx) {
    node.dyn_report(ctx);
    for (name, child) in node.children_mut() {
        let mut cctx = ctx.child(&name);
        report_all(child, &mut cctx);
    }
}

/// Top-down.
pub fn final_all(node: &mut dyn ComponentNode, ctx: &mut RustdvCtx) {
    node.dyn_final(ctx);
    for (name, child) in node.children_mut() {
        let mut cctx = ctx.child(&name);
        final_all(child, &mut cctx);
    }
}

/// The standard post-run tail: extract → check → report → final, returning
/// `Err` if any check failed (an `Err` fails the test, design-doc §0.6).
pub fn run_extract_check_report(
    node: &mut dyn ComponentNode,
    ctx: &mut RustdvCtx,
) -> Result<(), String> {
    extract_all(node, ctx);
    let mut sink = CheckSink::new();
    check_all(node, ctx, &mut sink);
    report_all(node, ctx);
    final_all(node, ctx);
    sink.into_result()
}

/// The full phaser: drive every UVM phase over a component tree, in order
/// (D51) — the analog of pyuvm handing the test class to its phaser. The
/// runner calls this for a `#[rustdv::test]` struct, so the test body is
/// just the phase methods; no hand-rolled `start_all`.
pub async fn run_component_test<T: Component + ComponentNode>(
    test: &mut T,
    ctx: &mut RustdvCtx,
) -> Result<(), TestError> {
    build_all(test, ctx);
    connect_all(test, ctx);
    end_of_elaboration_all(test, ctx);
    start_of_simulation_all(test, ctx);

    let run_result = run_all(test, ctx).await;

    // The run phase completes when its objections drain (pyuvm), so the
    // post-run phases wait for consensus first. A test that never objected
    // is not made to wait (D46).
    if ctx.objections().ever_raised() {
        ctx.all_objections_dropped().await;
    }

    let post = run_extract_check_report(test, ctx).map_err(TestError::from);
    run_result.and(post)
}

/// Debug printer: the child walker serving pyuvm's hierarchy print
/// (design-doc §5.2).
pub fn print_hierarchy(node: &mut dyn ComponentNode) {
    fn rec(node: &mut dyn ComponentNode, path: &str) {
        rustdv_sim::log::info(&format!("{path} ({})", node.node_name()));
        let parent = path.to_string();
        for (name, child) in node.children_mut() {
            rec(child, &format!("{parent}.{name}"));
        }
    }
    rec(node, "top");
}
