# Chapter 24: Components: The Hierarchy Problem, Solved by Ownership

`uvm_component` is the backbone of the UVM: everything in a pyuvm testbench extends it, inherits its phase methods, and hangs in a runtime tree beneath `uvm_test_top`. This chapter ports the backbone — and it is the chapter where rustdv's single deepest design decision lives, so let's say it plainly at the top: **the component tree is the ownership tree.** No parent pointers, no global component registry, no name strings passed to constructors. Children are struct fields. Everything else in the chapter unfolds from that sentence.

> **In Python we...** extended `uvm_component`, whose nine phase methods the UVM called in a fixed order — `build`, `connect`, `end_of_elaboration`, `start_of_simulation`, `run` (the only coroutine, objection-gated), `extract`, `check`, `report`, `final` — and we built the hierarchy in `build_phase()` by instantiating children with a *name* and a *parent*: `self.mc = MiddleComp("mc", self)`. pyuvm wove those references into a tree and derived paths like `uvm_test_top.mc.bc`.

## Nine phases, inventoried

Port the phase list first, because deciding what each phase *was for* decides what happens to it in Rust.

```text
# Figure 1: pyuvm's nine phases, and where each went

pyuvm phase                    rustdv
-----------                    ------
build_phase                    the constructor ("build convention")
connect_phase                  constructor arguments ("connect convention")
end_of_elaboration_phase       code after construction, before start
start_of_simulation_phase      (same — the gap between new() and start)
run_phase                      fn start(&mut self, ctx) spawns tasks; objections end the run
extract_phase                  fn extract(&mut self)
check_phase                    fn check(&mut self, errors: &mut CheckSink)
report_phase                   fn report(&self)
final_phase                    fn final_phase(&self)
```

The bottom five — the *runtime* lifecycle — survive as methods on the `Component` trait, with pyuvm's exact traversal orders (start is bottom-up like `run_phase` forking; extract/check/report are top-down) and pyuvm's no-op-by-default convention, via default method bodies: override only what you use.

The top four dissolve, and the reasoning deserves a paragraph rather than a bullet, because it is the memo that shaped rustdv. `build_phase` and `connect_phase` exist in the UVM because factory-driven construction is *two-stage*: the factory instantiates your component bare, and only afterward can it create children (`build`) and wire them (`connect`). Rust constructors are not two-stage. A `new()` function constructs children bottom-up in one pass, and connections are arguments passed down — there is no moment when a component exists but its children don't, so there is no phase to put in that moment. Building is what constructors *do*; connecting is what constructor arguments *are*. rustdv keeps the words as comment conventions (`// build:`, `// connect:` in the worked examples) because the book's chapter structure survives even where the mechanism dissolved. The two elaboration phases were empty in every example the Python book wrote; their job — code that runs after the tree exists and before the run starts — is simply the lines between `new()` and `start_all()` in your test.

## The lifecycle, demonstrated

pyuvm proved its phase order with a `PhaseTest` that printed from all nine methods. Here is the rustdv equivalent, printing from everything that remains:

```rust
// Figure 2: A component demonstrating the lifecycle methods

struct PhaseComp;

impl PhaseComp {
    fn new() -> PhaseComp {
        log::info("1 new() — the build convention");
        PhaseComp
    }
}

impl Component for PhaseComp {
    fn start(&mut self, _ctx: &mut RunCtx) {
        log::info("2 start");
    }
    fn extract(&mut self) {
        log::info("3 extract");
    }
    fn check(&mut self, _errors: &mut CheckSink) {
        log::info("4 check");
    }
    fn report(&self) {
        log::info("5 report");
    }
    fn final_phase(&self) {
        log::info("6 final_phase");
    }
}
```

```rust
// Figure 3: The test drives the lifecycle in order

#[rustdv::test]
async fn phase_test(_ctx: TestCtx) -> Result<(), TestError> {
    let mut comp = PhaseComp::new(); // build (and connect, had it children)

    let mut run_ctx = RunCtx::new();
    start_all(&mut comp, &mut run_ctx); // spawn free-running behavior
    run_ctx.all_objections_dropped().await; // the run "phase" is objection-gated

    run_extract_check_report(&mut comp).map_err(TestError::from)
}
```

```text
# Figure 4: The lifecycle runs in order
--
      0.00ns INFO     1 new() — the build convention
      0.00ns INFO     2 start
      0.00ns WARNING  all_objections_dropped awaited but no objection was ever raised
      0.00ns INFO     3 extract
      0.00ns INFO     4 check
      0.00ns INFO     5 report
      0.00ns INFO     6 final_phase
      0.00ns INFO     phase_test PASSED
```

Two things in this transcript repay attention. First, *the test is the phase engine*: where pyuvm's machinery invisibly called your methods in order, the rustdv test calls `start_all` and `run_extract_check_report` itself — three visible lines, no dispatcher, and the order is in your file rather than in a framework's. Second, that WARNING is pyuvm's own diagnostic, ported: a run phase in which nobody ever objected usually means somebody forgot their guard, and the framework says so. (`PhaseComp::start` spawns nothing, so nothing objected. The hierarchy test below does it properly.)

`check` deserves its one note now, since every scoreboard forever will use it: it receives a `&mut CheckSink`, and errors reported there accumulate and fail the test through `run_extract_check_report`'s `Result` — checks report through the sink; panics stay reserved for testbench bugs. The taxonomy holds.

## Building the hierarchy: fields, not registrations

Now the main event: pyuvm's `TestTop` → `MiddleComp` → `BottomComp` tower, rebuilt. In pyuvm, each `build_phase` instantiated its child with a name string and `self` as parent, and pyuvm knitted the references into a tree with a global `component_dict` watching over it all. In rustdv:

```rust
// Figure 5: A three-level hierarchy: children are fields

struct BottomComp;

impl Component for BottomComp {
    fn start(&mut self, ctx: &mut RunCtx) {
        let obj = ctx.raise_objection("bc run");
        spawn_named(
            async move {
                log::info("bc run phase");
                drop(obj);
            },
            "bc.run",
        );
    }
}

#[derive(rustdv::Component)]
struct MiddleComp {
    #[component(child)]
    bc: BottomComp,
}

impl Component for MiddleComp {}

#[derive(rustdv::Component)]
struct TestTop {
    #[component(child)]
    mc: MiddleComp,
}

impl Component for TestTop {
    fn final_phase(&self) {
        log::info("final phase");
    }
}
```

There is the whole hierarchy: `TestTop` owns `mc`, `MiddleComp` owns `bc`, and the tree is the struct nesting — the compiler enforces its shape, its construction order, and its destruction order, because that is what ownership *is*. The `#[derive(Component)]` you met in Chapter 21 writes the traversal (`visit_children` over fields marked `#[component(child)]`); `BottomComp`, childless, implements `ComponentNode` trivially — or would by the same derive; the chapter's code spells one out by hand to show there is no magic in it.

Look at `BottomComp::start` closely, because it is every driver and monitor you will ever write in miniature: raise an objection guard, `move` it into the spawned task, and let the task's completion drop it. The guard moving into the task is the ownership system doing end-of-test bookkeeping: the objection lives exactly as long as the work does.

```rust
// Figure 6: Constructors are the build phase

#[rustdv::test]
async fn hierarchy_test(_ctx: TestCtx) -> Result<(), TestError> {
    // build: bottom-up, in one expression
    let mut top = TestTop { mc: MiddleComp { bc: BottomComp } };

    print_hierarchy(&mut top);

    let mut run_ctx = RunCtx::new();
    start_all(&mut top, &mut run_ctx);
    run_ctx.all_objections_dropped().await;

    run_extract_check_report(&mut top).map_err(TestError::from)
}
```

```text
# Figure 7: The hierarchy, with names synthesized from field names
--
      0.00ns INFO     top (TestTop)
      0.00ns INFO     top.mc (MiddleComp)
      0.00ns INFO     top.mc.bc (BottomComp)
      0.00ns INFO     bc run phase
      0.00ns INFO     final phase
      0.00ns INFO     hierarchy_test PASSED
```

`top.mc.bc` — the path pyuvm spelled `uvm_test_top.mc.bc` — synthesized entirely from *field names*, at compile time, by the derive. Nobody passed `"mc"` to a constructor; the field is named `mc`, so the component is. The convention the Python book taught ("we give components the same name as their variable") stopped being a convention and became the only possibility. And child access is field access: `top.mc.bc` in a path, `self.mc.bc` in code — no `lookup("uvm_test_top.mc.bc")`, no string to typo, no runtime miss.

What did we give up against pyuvm's runtime tree? Three things it could do that fields cannot: address a component by *string path* from anywhere (its consumers were the ConfigDB and factory overrides — Chapters 27 and 29 explain why neither needs it here); enumerate *all* components globally (`visit_children` traversal covers the debug-print and hierarchy-walk uses, as `print_hierarchy` just showed); and hold *cyclic* references, child pointing back to parent — which is not a capability, it is the bug factory the ownership tree exists to close. A monitor that needs the scoreboard does not reach up and over via parent pointers; it gets a channel endpoint at construction, which is Chapter 31's whole subject.

## The predefined components

pyuvm shipped a taxonomy of `uvm_component` extensions; the roles all survive, wearing different amounts of type:

```text
# Figure 8: The predefined component taxonomy in rustdv

pyuvm class          rustdv form
-----------          -----------
uvm_test             the #[rustdv::test] function (Ch. 23)
uvm_env              a plain struct of children (Ch. 25)
uvm_agent            struct with Option<Driver>/Option<Sequencer> children (Ch. 34)
uvm_driver           Driver<REQ, RSP> with a typed SeqItemPort (Ch. 33, 36)
uvm_monitor          a convention, not a marker trait
uvm_scoreboard       a convention, not a marker trait
uvm_subscriber       trait Subscriber<T> { fn write(&mut self, item: &T); } (Ch. 32)
```

Monitor and scoreboard lose their marker classes because a methodless base class is ceremony in a language without inheritance — the book teaches the roles; the code doesn't need the tag. `uvm_subscriber` stays a real trait because `AnalysisPort` dispatches through its `write` method — and where pyuvm enforced the abstract `write` by raising `UVMFatalError` if you forgot to override it, Rust makes a `Subscriber` without `write` a compile error. The agent's `Option` children are the chapter-34 payoff planted now: a passive agent doesn't carry a disabled driver; it carries `None`.

## Summary

`uvm_component` ported as two small traits and one large idea. The idea: the ownership tree is the component tree — children are struct fields, hierarchy paths come from field names via `#[derive(Component)]`, child access is field access, and the parent-child reference cycles that pyuvm's GC untangled never exist. The traits: `Component` carries the surviving runtime lifecycle (`start`, spawning tasks under objection guards, then `extract`/`check`/`report`/`final_phase`, with pyuvm's traversal orders preserved by `start_all` and `run_extract_check_report`), and `ComponentNode` carries traversal, usually derived. Build and connect phases dissolved into constructors and their arguments — one-pass construction has no gap for them to fill — while the "you never objected" warning and the objection-gated run survive to the log line.

Version 4.0 puts the machinery to work: a real `AluEnv` whose fields are the testbench, built by constructors, run by the lifecycle — and the log output showing the traversal orders doing their jobs.
