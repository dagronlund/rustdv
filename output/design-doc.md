# rustdv: Design Document

**A Rust port of cocotb plus a UVM-equivalent verification library**

| | |
|---|---|
| Status | Draft for review — design phase only, no implementation |
| Author | Prepared for Ray Salemi |
| Date | 2026-07-07 |
| Sources | cocotb @ `cf833ee`, pyuvm @ `dfcd1ff`, *Python for RTL Verification* (Salemi, 2022, final version) — see `/reference/SOURCES.md` |

**Reading conventions.** Every design decision cites the source that informed it, in the form *(cocotb: `src/cocotb/task.py`, `Task._resume`)*, *(pyuvm: `_s09_phasing.py`)*, or *(book: "Coroutines" chapter)*. Anywhere I am uncertain or guessing, the paragraph is marked ⚠ and the issue is restated in §8 Open Questions. Code in this document is limited to trait and function *signatures* — no implementation bodies, per the project scope.

---

## Table of Contents

0. [Rust Concepts You'll Need](#0-rust-concepts-youll-need)
1. [Concept Mapping Table](#1-concept-mapping-table-python--rust)
2. [Crate/Module Structure](#2-cratemodule-structure)
3. [Simulator Interface Layer](#3-simulator-interface-layer)
4. [Concurrency/Scheduling Model](#4-concurrencyscheduling-model)
5. [Testbench/UVM-Analog API](#5-testbenchuvm-analog-api)
6. [Macro Strategy](#6-macro-strategy)
7. [Testing Conventions (Worked Example)](#7-testing-conventions-worked-example)
8. [Open Questions / Known Gaps](#8-open-questions--known-gaps)

---

## 0. Rust Concepts You'll Need

This section is written for someone fluent in Python — specifically, someone who wrote the Python patterns that cocotb and pyuvm use — and new to Rust. Each concept is introduced by contrast with the Python you already know, and each ends with *why rustdv needs it*.

### 0.1 Ownership: the concept Python never made you think about

In Python, every variable is a reference to an object on the heap, and the garbage collector decides when objects die. When pyuvm builds a component hierarchy, the parent holds a reference to each child in `self._children`, each child holds a reference back to its parent in `self._parent`, and `uvm_component.component_dict` holds a third reference to everything (pyuvm: `_s13_uvm_component.py`, `uvm_component.__init__`). Nobody owns anything; the GC untangles the cycles.

Rust has no garbage collector. Instead, every value has exactly one **owner** — the variable (or struct field) responsible for destroying it. When the owner goes out of scope, the value is dropped, immediately and deterministically. Assignment *moves* ownership rather than copying a reference:

- Python: `b = a` → two names for one object; both alive.
- Rust: `let b = a;` → the value moved into `b`; using `a` afterward is a **compile error**.

This sounds restrictive because it is. The payoff is that an entire category of testbench bug — the monitor that holds a stale handle to a re-built component, the two tasks that mutate one transaction concurrently — becomes a compile error instead of a 2 a.m. debug session.

**Why rustdv cares:** the UVM component tree, as pyuvm builds it, is a graph with parent↔child cycles — the single worst-case data structure for an ownership system. §5.2 spends most of its length on this. The design's answer is to make the ownership tree itself the component tree: children are struct fields, and the cycles never exist.

### 0.2 Borrowing: references with rules

You can lend access to a value without giving up ownership, using references: `&T` (shared, read-only) and `&mut T` (exclusive, read-write). The compiler enforces one rule, sometimes called *aliasing XOR mutability*:

> At any moment a value may have **many readers or one writer, never both**.

Python has no such rule — every reference is a `&mut` and races are your problem (the book's NullTrigger discussion shows exactly this class of bug: two tasks racing to observe `transaction_data`; cocotb: `_base_triggers.py`, `NullTrigger` docstring). In Rust, the pattern the book warns against would not compile.

When you genuinely need Python-like shared mutability — several components holding one scoreboard — Rust provides opt-in escape hatches with the checks moved to runtime: `Rc<T>` (shared ownership via reference counting, like CPython's refcounts made explicit) and `RefCell<T>` (borrow checking at runtime — a `panic!` replaces the compile error). `Rc<RefCell<T>>` is, roughly, "a Python object reference." rustdv uses this combination sparingly and deliberately; every use is called out in §5.

### 0.3 Traits: interfaces without inheritance

Python gave pyuvm three tools that Rust doesn't have: class inheritance, duck typing, and metaclasses. Rust replaces all three with **traits** — explicit, named collections of method signatures that a type opts into:

```rust
pub trait Component {
    fn start(&mut self, ctx: &mut RunCtx);
    fn check(&mut self, errors: &mut CheckSink);
    // ... default (empty) bodies provided, like pyuvm's no-op phase methods
}
```

Key contrasts:

- **No inheritance.** `uvm_driver(uvm_component)` in pyuvm becomes a `Driver` struct that *contains* its state and *implements* the `Component` lifecycle trait. Composition plus traits, never subclassing.
- **Default methods** replace the base-class no-op pattern. pyuvm's `uvm_component` defines empty `build_phase()` etc. (pyuvm: `_s13_uvm_component.py` lines 403–419); a Rust trait provides those as default method bodies you override selectively.
- **Two dispatch styles.** Generics (`fn drive<T: Transaction>(t: T)`) are resolved at compile time — zero cost, like C++ templates but type-checked. Trait objects (`Box<dyn Component>`) are resolved at runtime through a vtable — this is what lets rustdv store heterogeneous components in one hierarchy, the way Python lists hold anything.
- **Duck typing becomes bounds.** "This function needs anything with a `write()` method" becomes `T: Subscriber` — checked at compile time, documented in the signature.

### 0.4 `async`/`await`: the same idea you already know, with the engine exposed

This is the concept where your cocotb knowledge transfers most directly — and where the machinery differs most under the hood.

In cocotb, `await RisingEdge(clk)` works because a coroutine is a resumable function: the scheduler calls `coro.send(None)`, the coroutine runs until it yields a `Trigger`, and the scheduler registers a callback so the trigger's firing resumes the task (cocotb: `src/cocotb/task.py`, `Task._resume`; `_base_triggers.py`, `Trigger._register`). The event loop is a `deque` of callbacks drained to exhaustion (cocotb: `_event_loop.py`, `EventLoop.run`).

Rust's `async fn` compiles to a **state machine** implementing the `Future` trait, whose one method is:

```rust
fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
```

Where Python's coroutine *pushes* a Trigger out to the scheduler, a Rust future is *polled* and answers either `Poll::Ready(value)` or `Poll::Pending`. Before returning `Pending`, the future stashes a **`Waker`** — a cheap handle meaning "poll me again" — with whatever will eventually fire (in rustdv: a trigger's callback list). The Waker plays exactly the role of cocotb's `TriggerCallback` (cocotb: `_base_triggers.py`, `TriggerCallback`).

Two consequences matter enormously for rustdv:

1. **Rust ships no event loop.** `async`/`await` is pure language; the executor is a library you choose — or write. cocotb *already* had to write its own executor because asyncio can't block on simulator time. rustdv is in the same position, and a simulator-driven executor is small (cocotb's is 82 lines). We write our own; we do **not** pull in tokio (§4, rationale there).
2. **Cancellation is dropping.** cocotb cancels a task by throwing `CancelledError` into it, and the coroutine's `try/finally` blocks run (cocotb: `task.py`, `Task.cancel`). Rust cancels a task by *dropping the future* — the state machine is destroyed, and cleanup happens in `Drop` implementations (destructors). There is no exception to catch. §4.6 treats this asymmetry as a first-class design topic, because every driver/monitor "kill the task at end of test" pattern from the book crosses it.

### 0.5 Proc macros vs. decorators: compile time vs. run time

`@cocotb.test()` is a function that receives your function and wraps it — at *import time*, with full runtime power: it can inspect signatures, consult environment variables, and register the test in a global list (cocotb: `_decorators.py`, `test()`; `regression.py`, `RegressionManager.discover_tests`). pyuvm's factory goes further: a *metaclass* registers every component class as a side effect of the `class` statement itself (pyuvm: `_utility_classes.py`, `FactoryMeta`).

Rust has no import time and no metaclasses. Its equivalent power tool is the **procedural macro**: a function that runs *inside the compiler*, receives your code as a token stream, and emits replacement code. Three kinds matter here:

- **Attribute macros** — `#[rustdv::test]` sits where `@cocotb.test()` sat and rewrites the annotated `async fn` into a registered test entry.
- **Derive macros** — these sit where pyuvm's reliance on `__dict__` introspection sat: since Rust can't discover struct fields at runtime, macros generate field-wise code at compile time. For transactions, the *standard* derives (`Clone`/`PartialEq`/`Debug`) already do the job (§5.1); rustdv adds one derive of its own, `#[derive(Component)]`, which generates hierarchy traversal from a struct's fields (§5.2, §6.3).
- **Declarative macros** (`macro_rules!`) — simple pattern-based rewriting, used sparingly.

The catch: because macros run at compile time, *runtime registration must be replaced by link-time collection*. There is no moment when "all classes have been imported" — so rustdv uses distributed static registration (the technique behind the `inventory`/`linkme` crates) to build the test list before the runner starts. (Component creation needs no registry at all — constructor injection, §5.5.) ⚠ This mechanism has platform-specific subtleties (§8, OQ-4).

### 0.6 `Result`/`Option` vs. exceptions

Rust has no exceptions. Fallible functions return `Result<T, E>` (either `Ok(value)` or `Err(error)`), and absent values are `Option<T>` (either `Some(value)` or `None`). The `?` operator propagates errors up the call stack with one character, giving `try/except`-like ergonomics without invisible control flow:

```rust
fn get_dut_signal(dut: &HierarchyHandle, name: &str) -> Result<LogicHandle, HandleError>;
// caller: let clk = get_dut_signal(&dut, "clk")?;
```

Contrasts that matter for rustdv's API design:

- cocotb raises `AttributeError` when a DUT signal name doesn't resolve (cocotb: `handle.py`, `HierarchyObject.__getattr__`); rustdv's `dut.child("name")` returns a `Result` — the *signature* tells you it can fail. (pyuvm's `UVMConfigItemNotFound` disappears by a different route: configuration is compile-time-checked structs, §5.4.)
- cocotb marks a test failed by letting any exception propagate out of the test coroutine (cocotb: `regression.py`, `_score_test`). rustdv tests return `Result<(), TestError>`; an `Err` fails the test.
- Rust *does* have `panic!` — an abort-the-task mechanism for "this is a bug" situations, used by `assert!`/`assert_eq!`. Panics in a rustdv test are caught at the task boundary and scored as test failure, mirroring how cocotb catches `BaseException` per task (cocotb: `task.py`, `Task._resume`). Panics are for assertion failures; `Result` is for expected fallibility (missing signals, config lookups). This split is a designed convention, stated in §7.

### 0.7 Odds and ends you'll hit immediately

- **No GIL, but also no threads (here).** GPI is not thread-safe, and cocotb runs everything on the simulator's thread. rustdv does the same; the type system *enforces* it by making key types `!Send` (unable to leave the thread they were created on) — see §3.4. Rust's fearless-concurrency story is real but mostly unused in rustdv's core.
- **`String` vs. `&str`** — owned string vs. borrowed string slice; the practical rule is "store `String`, pass `&str`."
- **Lifetimes** (`'a`) — annotations telling the compiler how long borrows live. rustdv's public API is designed to keep user-facing lifetimes rare; where signatures in this document show them, that is a deliberate, commented choice.
- **`cargo`** — think `pip` + `venv` + `make` + `pytest` in one tool. The build/run story in §7 is cargo-native.

---

## 1. Concept Mapping Table (Python → Rust)

Legend: **[C]** = cocotb source, **[P]** = pyuvm source, **[B]** = *Python for RTL Verification*. ⚠ marks mappings with unresolved alternatives (detailed in §8).

### 1.1 Language & runtime layer

| # | Python (as used today) | Rust (rustdv design) | Rationale | Source |
|---|---|---|---|---|
| 1 | `async def` coroutine, resumed via `coro.send(None)` | `async fn` → `impl Future`, resumed via `poll()` | Same user-facing model; engine differs (§0.4) | [C] `task.py` |
| 2 | `@cocotb.test()` decorator | `#[rustdv::test]` attribute macro | Runtime wrapping → compile-time rewriting (§0.5, §6.1) | [C] `_decorators.py` |
| 3 | Exceptions for test failure | `Result<(), TestError>` return + caught panics for assertions | No exceptions in Rust; signatures document fallibility (§0.6) | [C] `regression.py` `_score_test` |
| 4 | `cocotb.start_soon(coro)` | `rustdv::spawn(future) -> TaskHandle<T>` | Same semantics: queue task, run at next loop turn | [C] `_test_manager.py` `start_soon` |
| 5 | `Task` 7-state machine (UNSTARTED…CANCELLED) | `TaskState` enum, same seven states | Proven model; keep debugging story identical | [C] `task.py` `_TaskState` |
| 6 | `task.kill()` / `task.cancel()` + `CancelledError` | `TaskHandle::cancel()` → future dropped; cleanup via `Drop` | Rust cancellation is drop-based — semantic shift ⚠ (§4.6, OQ-5) | [C] `task.py` `cancel` |
| 7 | `TaskManager` / `async with` task groups | `TaskGroup` with RAII guard (`Drop` cancels children) | Context-manager → RAII is the idiomatic translation | [C] `_task_manager.py` |
| 8 | Metaclass side effects (`FactoryMeta`) | Link-time registration for test discovery only ⚠; component creation is constructor injection (review-memo R5) | No metaclasses; only compile/link-time hooks exist | [P] `_utility_classes.py` |
| 9 | `getattr(comp, "build_phase")()` string dispatch | `Component` lifecycle trait, direct method calls; build/connect are constructors (review-memo R3) | Reflection unavailable; traits are faster and checked | [P] `_s09_phasing.py` `uvm_phase.execute` |
| 10 | Store-anything containers (`dict` of `Any`) | Typed config structs (§5.4); type erasure not ported (review-memo R4) | The store-anything pattern was mechanism, not need | [P] `ConfigDB` |
| 11 | Python `logging` hierarchy (`cocotb.task.X`) | `tracing` crate with span/target hierarchy ⚠ | Structured, hierarchical, filterable; maps to GPI log handler | [C] `logging.py`, `gpi.h` logging group |

### 1.2 cocotb core layer

| # | Python (cocotb) | Rust (rustdv design) | Rationale | Source |
|---|---|---|---|---|
| 12 | `Trigger` with `_prime/_unprime/_react/_register` | `Trigger` trait: `subscribe/unsubscribe/prime/unprime` | Same lazy-prime lifecycle: prime on first subscriber, unprime on last | [C] `_base_triggers.py` |
| 13 | `Timer(2, unit="ns")` | `Timer::ns(2).await` (constructors per unit; `SimDuration` type) | Unit-safe construction; rejects zero/negative at compile time where possible | [C] `_gpi_triggers.py` `Timer` |
| 14 | `RisingEdge(sig)` / `FallingEdge` / `ValueChange` | `sig.rising_edge().await` etc., methods on typed handles | cocotb 2.x already steers users to `signal.rising_edge` | [C] `_gpi_triggers.py` notes |
| 15 | `ReadOnly()` / `ReadWrite()` / `NextTimeStep()` singletons | Singleton trigger handles (`read_only()`, `read_write()`, `next_time_step()`) | Same phase-callback semantics, incl. illegal-transition checks | [C] `_gpi_triggers.py` |
| 16 | `First(...)` / `Combine(...)` | `first!(...)` / `join!(...)` combinator futures | Standard Rust select/join, with drop-based cancellation of losers | [C] `_extended_awaitables.py` |
| 17 | `Event` / `Lock` (sim-aware, fair) | `sim::Event` / `sim::Lock` (custom, executor-aware, FIFO-fair) | std/tokio primitives don't know sim time; Lock fairness is documented cocotb behavior | [C] `_base_triggers.py` `Lock` |
| 18 | `cocotb.queue.Queue/PriorityQueue/LifoQueue` | `sim::Queue<T>` family (bounded, executor-aware) | Foundation for TLM FIFOs, as in pyuvm | [C] `queue.py`; [P] `UVMQueue` |
| 19 | `dut.signal_name` via `__getattr__` discovery | `dut.child("signal_name")? -> AnyHandle`, plus optional codegen'd typed DUT struct ⚠ | No runtime attribute invention in Rust; dynamic lookup returns `Result` (OQ-6) | [C] `handle.py` `HierarchyObject` |
| 20 | `sig.value` get/set property | `sig.get() -> Logic` / `sig.set(v)`; explicit methods | cocotb 2.x itself moved to `get()`/`set()` | [C] `handle.py` `ValueObjectBase` |
| 21 | `Deposit/Force/Freeze/Release/Immediate` actions | `SetAction` enum parameter on `set_with(action, v)` | Direct GPI mapping (`gpi_set_action`) | [C] `handle.py`; `gpi.h` |
| 22 | Scheduled writes applied at ReadWrite phase | Identical write-buffer, drained on ReadWrite trigger | Simulator-quirk workaround worth porting verbatim | [C] `handle.py` `_apply_scheduled_writes`, `_gpi_triggers.py` `ReadWrite._do_callbacks` |
| 23 | `LogicArray`, `Logic`, `Range`, `Array` types | `Logic` (4-state enum), `LogicArray`, `Range` structs with `From`/`TryFrom` conversions | Typed value layer; conversion failures are `Result`s not exceptions | [C] `types/` |
| 24 | `Clock(dut.clk, 10, unit="ns").start()` | `Clock::new(&clk, Duration::ns(10)).start() -> TaskHandle` | Keep optional fast-path in compiled code (cocotb has C++ `cpp_clock`) | [C] `clock.py`, `simulator.pyi` `cpp_clock` |
| 25 | `bridge`/`resume` (blocking ↔ async thread hop) | `sim::block_on_external` / channel bridge ⚠ | Needed for file/network I/O in testbenches; design deferred (OQ-7) | [C] `_bridge.py` |
| 26 | `TestFactory` / `@parametrize` runtime generation | `#[rustdv::parametrize(...)]` compile-time expansion ⚠ | No runtime class creation; macro generates N registered tests (OQ-10) | [C] `_test_factory.py`, `_decorators.py` |

### 1.3 pyuvm/UVM layer

| # | Python (pyuvm) | Rust (rustdv design) | Rationale | Source |
|---|---|---|---|---|
| 27 | `uvm_object` base class | Plain structs; `Debug` + `std::any::type_name` cover the surface — no base trait (review-memo R1) | Rust derives what Python had to hand-roll | [P] `_s05_base_classes.py` |
| 28 | `clone/copy/compare` via `do_copy/do_compare` hooks | std derives (`Clone`/`PartialEq`/`Debug`); comparison policy lives in the scoreboard (review-memo R1) | Field enumeration at compile time is the language's job | [P] `_s05_base_classes.py` |
| 29 | `uvm_component(name, parent)` tree of references | Ownership tree: children are struct fields; `#[derive(Component)]` traversal (review-memo R2) | The ownership tree *is* the component tree (§5.2) | [P] `_s13_uvm_component.py` |
| 30 | `uvm_root()` singleton | Deleted — the `#[rustdv::test]` fn constructs and owns the env (review-memo R2) | No global state; test-by-name lives in the runner registry | [P] `UVM_ROOT_Singleton` |
| 31 | 9 common phases, topdown/bottomup traversal | build/connect become constructor conventions; `start`/`extract`/`check`/`report`/`final` on the `Component` trait, same traversal orders (review-memo R3) | Two-stage construction was a factory artifact; the rest is methodology | [P] `_s09_phasing.py` |
| 32 | `raise_objection`/`drop_objection` + handler singleton | `ObjectionGuard` RAII handle from `ctx.raise_objection(desc)` | Drop-based release is strictly safer than manual drop | [P] `ObjectionHandler`; `uvm_component.objection()` |
| 33 | `ConfigDB().set/get` glob paths, `Any` values | Typed, nested config structs passed to constructors (review-memo R4) | Compile-time contract replaces runtime store; its failure modes become compile errors | [P] `ConfigDB` |
| 34 | `uvm_factory()` create-by-name/type, overrides | Constructor injection: config-carried maker closures at designed variation points (review-memo R5); string registry survives for test discovery only | Rust passes constructors as values; no chains to chase, no loops to detect | [P] `_s08_factory_classes.py`, `_utility_classes.py` `FactoryData.find_override` |
| 35 | TLM-1 ports/exports (blocking/nonblocking put/get/peek/transport) | `channel<T>()` → `Sender`/`Receiver`; `AnalysisPort<T>` and `TlmFifo<T>` kept as semantically distinct (review-memo R6) | Twelve port classes become six methods on two types | [P] `_s12_uvm_tlm_interfaces.py` |
| 36 | `uvm_analysis_port.write()` fan-out | `AnalysisPort<T>`: broadcast to N subscribers, non-blocking | 1-to-many, fire-and-forget, as in UVM | [P] `_s12` `uvm_analysis_port` |
| 37 | `uvm_tlm_fifo`, analysis FIFO, req/rsp channel | `TlmFifo<T>` etc. on `sim::Queue<T>` | Same size-1 default, `used()`, `flush()` surface | [P] `_s12` `uvm_tlm_fifo_base` |
| 38 | `uvm_sequence.start/start_item/finish_item/get_response` | `Sequence` trait with `async fn body(&mut self, ctx: SeqCtx<REQ, RSP>)` | Handshake protocol preserved event-for-event (§5.6) | [P] `_s14_15_python_sequences.py` |
| 39 | `ResponseQueue` txn-id cherry-picking | `ResponseQueue<RSP>` with `get_response(Option<TxnId>)` | Same select-by-id or FIFO-order behavior | [P] `ResponseQueue` |
| 40 | `uvm_driver` with `seq_item_port` | `Driver<REQ, RSP>` generic struct + `SeqItemPort<REQ, RSP>` | Typed transactions end run-time type errors at the driver boundary | [P] `_s13_predefined_component_classes.py` |
| 41 | `uvm_subscriber.write()` abstract method | `Subscriber<T>` trait: `fn write(&mut self, item: &T)` | Abstract method → required trait method | [P] `uvm_subscriber` |
| 42 | `uvm_agent` active/passive via ConfigDB | Config enum + `Option` children: a passive agent doesn't construct a driver (review-memo R2/R4) | Illegal states become unrepresentable vs. pyuvm's runtime warning path | [P] `uvm_agent`; [B] testbench 6.0 chapters |

---

## 2. Crate/Module Structure

cocotb splits into a Python package (`src/cocotb`), a C++ simulator-interface library (`src/cocotb/share/lib/gpi`), an embedding shim (`share/lib/pygpi`), and a tools package (`src/cocotb_tools`) *(cocotb: source layout)*. rustdv mirrors that separation as a cargo **workspace** — the boundaries earned their keep in cocotb and the FFI boundary *must* be its own crate in Rust anyway (`-sys` convention).

```
rustdv/                          # cargo workspace root
├── rustdv-gpi-sys/              # raw FFI bindings to gpi.h (bindgen), no logic
├── rustdv-gpi/                  # safe wrapper: handles, callbacks, values
├── rustdv-sim/                  # executor, tasks, triggers, time, clock, queues
│   └── (modules) executor, task, trigger, handle, types, clock, simtime, queue
├── rustdv-uvm/                  # the UVM analog
│   └── (modules) component, lifecycle, objection, config,
│       channel, analysis, fifo, sequence, predefined
├── rustdv-macros/               # proc macros: #[test], #[parametrize], derives
├── rustdv-runner/               # regression manager, test registry, entry point,
│                                #   cdylib bootstrap, result reporting (xUnit)
└── rustdv/                      # facade crate: re-exports the public API
```

Design decisions and their sources:

**D2.1 — `rustdv-gpi-sys` is bindings-only.** Machine-generated from `gpi.h` (cocotb: `share/include/gpi.h`), no hand-written logic, everything `unsafe extern "C"`. This is the standard Rust `-sys` crate discipline: one crate owns "what the C API is," another owns "how to use it safely." Keeping it generated means tracking upstream cocotb GPI changes is a re-run of bindgen, not a port.

**D2.2 — `rustdv-sim` does not depend on `rustdv-uvm`.** pyuvm imports cocotb, never the reverse (pyuvm: `_utility_classes.py` imports `cocotb.queue`, `cocotb.triggers`). Same direction here: you can write book-style "testbench 1.0/2.0" (pre-UVM) programs against `rustdv-sim` alone, which the book's pedagogy requires — chapters 23–27 of the book use cocotb without pyuvm (book: "Basic testbench: 1.0" through "Class-based testbench: 2.0").

**D2.3 — `rustdv-macros` is a separate crate by necessity.** Rust requires proc macros to live in their own crate type. It depends on nothing at runtime; `rustdv-uvm` and `rustdv-runner` provide the symbols the generated code calls.

**D2.4 — `rustdv-runner` owns `main`-equivalent duties.** cocotb's `RegressionManager` discovers tests, runs them in order, times them, scores exceptions vs. expectations, and writes xUnit XML (cocotb: `regression.py`). The runner crate ports this: test registry (populated at link time by `#[rustdv::test]`), sequential test execution, `RANDOM_SEED` handling (cocotb: `_init.py`, `_setup_random_seed`), result table, and the simulator entry point (§3.2).

**D2.5 — the `rustdv` facade re-exports a curated prelude.** The book teaches `import cocotb` / `from pyuvm import *` (book: every example). The Rust equivalent of that ergonomics is `use rustdv::prelude::*;` — one line for users, while the workspace stays modular behind it.

**Feature flags.** ⚠ Simulator selection (Icarus/Verilator/Questa/…) is a *runtime* concern in cocotb (GPI impl `.so` chosen by the makefiles; cocotb: `share/def/*.def`, `cocotb_tools/makefiles`). rustdv keeps that runtime model rather than cargo features where possible, but the build flow for linking testbench-as-cdylib against each simulator is OQ-2.

---

## 3. Simulator Interface Layer

### 3.1 Decision: reuse cocotb's GPI, don't rewrite it

**D3.1 — rustdv links against cocotb's existing GPI C++ library and binds to `gpi.h`.**

The GPI layer is cocotb's crown jewel: one C API (`gpi.h`, 589 lines) abstracting VPI, VHPI, and FLI, with a decade of accumulated simulator-quirk fixes (cocotb: `share/lib/gpi/GpiCommon.cpp`, per-simulator `def` files, and the FLI sensitivity-list workaround documented in `gpi.h` lines 29–32). The header is deliberately C-compatible — opaque handle pointers, plain enums, function pointers — i.e., it is *already* an FFI boundary designed for exactly this kind of consumption.

Rewriting VPI/VHPI/FLI handling in Rust would be years of re-learning quirks the GPI already encodes, for zero user-visible benefit. The port boundary is `gpi.h`, full stop. A pure-Rust GPI remains a possible *future* phase and is recorded as OQ-1.

What this buys, concretely — the full `gpi.h` surface rustdv binds:

| `gpi.h` group | Functions (abridged) | rustdv safe wrapper |
|---|---|---|
| Sim control/query | `gpi_get_sim_time`, `gpi_get_sim_precision`, `gpi_get_simulator_product/version`, `gpi_finish` | `SimContext` methods; time as `SimTime` (u64 steps + precision) |
| Object query | `gpi_get_root_handle`, `gpi_get_handle_by_name`, `gpi_get_handle_by_index` | `HierarchyHandle::child(name/index) -> Result<AnyHandle>` |
| Object properties | `gpi_get_object_type`, `gpi_get_num_elems`, `gpi_get_range_*`, `gpi_is_constant/indexable/signed` | typed-handle downcasting (§3.3) |
| Signal values | `gpi_get_signal_value_binstr/str/real/long`, `gpi_set_signal_value_*` + `gpi_set_action` | `LogicHandle::get/set`, `SetAction` enum |
| Iteration | `gpi_iterate`, `gpi_next` | `HierarchyHandle::children() -> impl Iterator` |
| Callbacks | `gpi_register_{timed, value_change, readonly, nexttime, readwrite}_callback`, `gpi_remove_cb` | trigger primitives (§4) |
| Logging | `gpi_set_log_handler`, log levels | bridge to Rust logging (⚠ OQ-8) |

### 3.2 Embedding: how rustdv code gets into the simulator process

cocotb's chain today: simulator loads a GPI implementation library (VPI/VHPI/FLI `.so`) → GPI loads `libpygpi` (`embed.cpp`) → which starts an embedded CPython → which imports the user's test module (cocotb: `share/lib/pygpi/embed.cpp`, `src/pygpi/entry.py`, `_init.py`, `init_package_from_simulation`).

**D3.2 — the rustdv testbench compiles to a `cdylib`** that exports the same entry-point symbols `libpygpi` exports today, so the existing GPI loader machinery (`dynload.cpp`, environment-variable-driven library discovery) can load a Rust testbench in place of the Python interpreter. The user's test crate links `rustdv`, and `rustdv-runner` provides the exported entry functions; from GPI's point of view nothing changed.

⚠ I am confident about the *shape* of this (the loader is explicitly designed around an env-var-named library with known entry points), but the exact symbol set, initialization ordering, and per-simulator loading quirks need a prototype before this section can be called settled. OQ-2.

### 3.3 The safe/unsafe split

All `unsafe` lives in `rustdv-gpi`, upholding these invariants so that everything above it is safe Rust:

1. **Handles are opaque and non-null.** `gpi_sim_hdl` etc. are incomplete-type pointers (cocotb: `gpi.h` lines 45–83). Wrapper: `struct RawObjHandle(NonNull<c_void>)`. Fallible acquisition (`gpi_get_handle_by_name` returns NULL for not-found) becomes `Result`/`Option` at the boundary — never a nullable handle in user code.
2. **Handle lifetime = simulation lifetime.** GPI object handles are valid until sim end (cocotb treats them so: `handle.py` caches them for the process lifetime). Wrappers are therefore freely cloneable ID types; no `Drop` frees a sim object. Callback handles differ: they invalidate on `gpi_remove_cb` or after firing — modeled as consuming methods (`fn deregister(self)`) so a stale callback handle is unrepresentable.
3. **Strings are copied at the boundary.** `const char*` returns point into simulator-owned memory of unspecified lifetime; the wrapper copies to `String` immediately, on every call.
4. **No unwinding across FFI.** Every Rust function passed to C as a callback wraps its body in `catch_unwind`; a panic is converted to test-failure state, never propagated into the simulator. (cocotb has the same concern with C++ exceptions; embed layer catches everything.)
5. **Callback user-data ownership.** `gpi_register_*_callback(fn, void* data, ...)` takes a C function pointer plus context. The wrapper boxes a Rust closure, passes it as the `void*`, and reclaims the `Box` when the callback is deregistered or fires for the last time. One-shot callbacks (timers — GPI callbacks are single-fire; cocotb re-registers each time, see `_gpi_triggers.py` `Timer._prime`) reclaim on fire; the wrapper encodes one-shot vs. recurring in the type.

### 3.4 Thread affinity

GPI has no thread-safety guarantees; cocotb only ever calls it from the simulator callback thread, and shunts real threads through the bridge (cocotb: `_bridge.py`). rustdv encodes this in the type system: `SimContext`, all handles' *methods*, and the executor are `!Send` — the compiler rejects any attempt to move them to another thread. External threads interact only through the bridge channel (⚠ OQ-7 for its design). This turns cocotb's documentation-level rule into a compile-time rule — one of the clearest wins of the port. ⚠ Whether *some* GPI calls are in fact safe off-thread on some simulators: unknown, assumed no. OQ-13.

---

## 4. Concurrency/Scheduling Model

### 4.1 What cocotb actually does (the spec for our port)

Distilled from source — this sequence is the contract rustdv must reproduce:

1. A GPI callback fires (timer, edge, phase). The trigger's `_react()` runs all callbacks registered on that trigger — each callback typically marks one task SCHEDULED and pushes its resume onto the event loop — and then **drains the event loop to exhaustion** before returning to the simulator (cocotb: `_gpi_triggers.py`, `GPITrigger._react`; `_event_loop.py`, `EventLoop.run`).
2. Resuming a task means `coro.send(None)`; the task runs until it finishes, raises, or yields the next `Trigger` it awaits (cocotb: `task.py`, `Task._resume`).
3. A trigger primes its underlying GPI mechanism lazily — on first registered callback — and unprimes when its last callback deregisters (cocotb: `_base_triggers.py`, `Trigger._register`/`_deregister`).
4. Signal writes via `set()` are buffered and applied at the start of the next ReadWrite phase (inertial-write workaround; cocotb: `handle.py` write scheduler, `ReadWrite._do_callbacks`).
5. Everything happens on one thread. "Parallelism" is cooperative interleaving at await points — the model the book teaches with the producer/consumer and BFM examples (book: "Coroutines" and "cocotb Queue" chapters).

### 4.2 Decision: a bespoke single-threaded executor (not tokio)

**D4.1** — rustdv implements its own executor in `rustdv-sim`. Rationale:

- General-purpose runtimes (tokio, async-std) own their event loop and expect to block on OS I/O. Our event source is the *simulator*; control flow must return to the simulator after every drain, exactly as cocotb's `EventLoop.run()` returns after exhausting its deque. This inversion (executor as guest, not host) is disqualifying for tokio's architecture and is precisely why cocotb never used asyncio's loop either.
- The executor cocotb needs is tiny — its Python one is 82 lines. A run queue (`VecDeque<TaskId>`), a task arena, and waker plumbing.
- Zero heavyweight dependencies keeps the teaching story clean (book audience installs one crate, sees no runtime magic).

### 4.3 Core trait signatures

```rust
/// A future event a task can await. Port of cocotb's Trigger
/// (cocotb: _base_triggers.py), adapted to waker-based polling.
pub trait Trigger {
    /// Register interest. First subscriber primes the underlying mechanism.
    /// (cocotb: Trigger._register)
    fn subscribe(&self, waker: TriggerWaker) -> Result<SubscriptionId, TriggerError>;

    /// Remove interest. Last unsubscribe unprimes. (cocotb: Trigger._deregister)
    fn unsubscribe(&self, id: SubscriptionId);
}

/// GPI-backed triggers additionally manage a simulator callback handle.
/// (cocotb: _gpi_triggers.py, GPITrigger)
pub trait GpiTrigger: Trigger {
    fn prime(&self) -> Result<(), TriggerError>;   // register GPI callback
    fn unprime(&self);                              // gpi_remove_cb
}

/// The executor. Owned by the SimContext; never leaves the sim thread (!Send).
pub struct Executor { /* run queue, task arena — fields elided */ }

impl Executor {
    /// Port of cocotb.start_soon (cocotb: _test_manager.py).
    pub fn spawn<F>(&self, fut: F, name: Option<&str>) -> TaskHandle<F::Output>
    where F: Future + 'static;

    /// Drain the run queue to exhaustion, then return to the simulator.
    /// Port of EventLoop.run (cocotb: _event_loop.py).
    pub fn run_until_idle(&self);
}

/// Public task control surface. Port of cocotb Task (cocotb: task.py).
pub struct TaskHandle<T> { /* task id, shared state — elided */ }

impl<T> TaskHandle<T> {
    pub fn cancel(&self);                       // drop-based; see §4.6
    pub fn done(&self) -> bool;
    pub fn state(&self) -> TaskState;           // same 7 states as cocotb
    pub fn result(&self) -> Result<T, TaskError>;  // InvalidState if not done
}
// TaskHandle<T> is also a Future: `handle.await` == awaiting task completion,
// mirroring `await task` in cocotb 2.x.
```

`TriggerWaker` wraps a std `Waker` plus the task id, playing the role of cocotb's `TriggerCallback` (cocotb: `_base_triggers.py`, `TriggerCallback`). When a GPI callback trampoline fires, it calls the trigger's react path: wake all subscribers (pushing their tasks onto the run queue), then call `executor.run_until_idle()` — the exact shape of `GPITrigger._react` (cocotb: `_gpi_triggers.py` lines 40–49).

### 4.4 Trigger inventory

Ports of the full cocotb set, same semantics, same names where possible:

- **`Timer`** — one-shot timed callback; rejects non-positive durations (cocotb: `Timer.__init__` raises `ValueError`; rustdv makes the constructor return `Result` or use unit-typed constructors that can't express zero ⚠ minor).
- **`RisingEdge` / `FallingEdge` / `ValueChange`** — via `gpi_register_value_change_callback` with `gpi_edge` selector; exposed as methods on typed handles (mapping row 14).
- **`ReadOnly` / `ReadWrite` / `NextTimeStep`** — phase triggers, singletons per sim context; `ReadWrite` drains the write buffer before waking subscribers, and awaiting `ReadOnly`-from-`ReadOnly` or `ReadWrite`-from-`ReadOnly` is a runtime error, both exactly as cocotb (cocotb: `_gpi_triggers.py`, `ReadOnly.__await__`/`ReadWrite.__await__`).
- **`sim::Event`** — manual-reset event; `wait()` completes immediately if already set (cocotb: `_base_triggers.py`, `_Event._prime`).
- **`sim::Lock`** — FIFO-fair mutex; acquisition order = request order, documented behavior in cocotb (cocotb: `_base_triggers.py`, `Lock` docstring "Guarantees fair scheduling").
- **`NullTrigger`** — yield-once; kept for parity but documented as a smell, matching the book's and cocotb's guidance to prefer `Event` (cocotb: `NullTrigger` docstring; book: same example).
- **`first!` / `join!`** — ports of `First`/`Combine` (cocotb: `_extended_awaitables.py`). In Rust these are future combinators; `first!` drops the losing futures, which unsubscribes their triggers via `Drop` — the cleanup cocotb does manually with kill-on-completion tasks falls out of RAII for free.

### 4.5 Test execution layer

`#[rustdv::test]` registers a `TestCase` (name, module, source location, options: `timeout`, `expect_fail`, `expect_error`, `skip`, `stage` — the full option set of cocotb's `Test` class (cocotb: `_decorators.py`, `Test.__init__`)). The runner executes tests sequentially, one `TestManager`-equivalent per test that: spawns the test future, tracks all tasks spawned during the test, cancels survivors at test end, converts stray panics/errors in *any* task into test failure — porting the "child task exception fails the test" behavior (cocotb: `_test_manager.py`, `TestManager._task_done_callback`).

### 4.6 Cancellation: the one big semantic divergence

cocotb cancels by throwing `CancelledError` *into* the coroutine, which can catch it, run `finally` blocks, even (buggily) suppress it — cocotb devotes real machinery to detecting suppression (cocotb: `task.py`, `_must_cancel` checks). Rust cancellation is: the executor drops the future. Cleanup runs in `Drop` impls of whatever the future held. No task-side code executes after the drop point; there is nothing to suppress and no way to observe "being cancelled" from inside.

Consequences the design embraces:

- Driver/monitor cleanup must live in `Drop` (RAII guards), not in `finally`-style code after the await. §5 APIs are shaped so components' resources are naturally `Drop`-owned. This is *simpler* and less error-prone than cocotb's model — the class of "forgot to re-raise CancelledError" bugs (which cocotb explicitly detects and errors on) cannot exist.
- ⚠ One cocotb capability is lost: a task doing final work *at cancellation time that requires awaiting* (e.g., drive bus idle over several cycles on kill). Rust drops synchronously. Teams needing this pattern must restructure (explicit shutdown message + join instead of kill). Recorded as OQ-5 with the recommended idiom.

---

## 5. Testbench/UVM-Analog API

> **Revision note.** §5 was rewritten after adopting recommendations R1–R6 of `review-memo.md` (in this directory), which classified each pyuvm concept as *methodology* (the problem it solves — kept) or *mechanism* (the Python/OO machinery it solves it with — not ported). The memo is the rationale record for every deletion below; this section states the resulting design.

This section addresses pyuvm's architecture, subsystem by subsystem, in pyuvm's own file order. The guiding principle, sharpened by the memo: **pyuvm simplified SystemVerilog UVM by using Python's dynamism; rustdv solves the same methodology problems with Rust's type system — and where a pyuvm mechanism existed only to serve another mechanism, both are deleted rather than ported.** Where pyuvm checks types at runtime (`assert issubclass(type(item), uvm_sequence_item)` — pyuvm: `_s14_15_python_sequences.py`, `uvm_seq_item_port.put_response`), rustdv makes the check a generic bound; where pyuvm maintained runtime registries and string paths, rustdv uses ownership and constructors.

### 5.1 Transactions: plain data plus the `SeqItem` envelope

*(Adopts review-memo R1. Replaces the earlier `UvmObject`/`ObjectOps`/`Transaction` trait-and-derive design.)*

pyuvm's `uvm_object` ecosystem (pyuvm: `_s05_base_classes.py`) exists to give Python objects capabilities the language couldn't derive: field-wise copy (`do_copy`), field-wise comparison (`do_compare`), printable form (`convert2string`), and identity. Rust derives all of these from the struct definition. A rustdv transaction is therefore a plain struct:

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct AluCommand { pub a: u8, pub b: u8, pub op: Ops }
```

No rustdv base trait. `Clone` is `do_copy`; `PartialEq` is `do_compare`; `Debug` is `convert2string`; `std::any::type_name` is `get_type_name`. What pyuvm hand-rolled by walking `__dict__` at runtime (pyuvm: `_s05`), the std derives generate at compile time — the same field-enumeration job SV-UVM's field macros did, done by the language itself.

The one genuine methodology item in the sequence-item lineage is identity for request/response correlation. That identity belongs to the *infrastructure*, not the user's data type — pyuvm itself signals the mechanism leak by storing scheduler events on the transaction (pyuvm: `_s14_15_python_sequences.py`, `uvm_sequence_item.__init__` creates `start_condition`/`finish_condition`/`item_ready` on the item). rustdv moves both the events *and* the identity into an envelope owned by the sequencer channel:

```rust
/// What the driver receives from get_next_item(). The infrastructure owns
/// the id; the payload is the user's plain struct.
pub struct SeqItem<REQ> { /* txn id + payload — elided */ }
impl<REQ> SeqItem<REQ> {
    pub fn txn_id(&self) -> TxnId;
    pub fn payload(&self) -> &REQ;
    pub fn payload_mut(&mut self) -> &mut REQ;
}
```

`item_done(Some(rsp))` tags the response with the envelope's id internally; pyuvm's `set_context` (pyuvm: `_s05` `uvm_transaction.set_id_info`, `_s14_15` `uvm_sequence_item.set_context`) has no user-visible equivalent because the user can no longer forget to call it.

**Comparison policy moves to the scoreboard.** pyuvm bakes one notion of equality into the data type (`do_compare` overrides, field exclusions). rustdv scoreboards take a comparator — `PartialEq` by default, a closure or projection where the check differs from structural equality. Comparison is checker policy, not data-type property; the earlier `#[uvm(skip)]` field-attribute design is deleted along with the derive that carried it.

What is **deliberately dropped**, unchanged from the previous revision: pack/unpack, recording hooks, policies — pyuvm itself raises `UVMNotImplemented` or provides stubs for most of these (pyuvm: `_s05_base_classes.py`, `uvm_policy.__new__` raises). Listed in §8 as known gaps, not open questions.

### 5.2 Components and hierarchy: the ownership tree

*(Adopts review-memo R2. Replaces the earlier arena/`ComponentId` design.)*

pyuvm's runtime component graph — child holds `_parent`, parent holds `_children[name]`, and a global `component_dict` maps full names to instances (pyuvm: `_s13_uvm_component.py`, `uvm_component.__init__`) — exists because SV/Python build topology at runtime through the factory and address components by string path. Neither driver survives into this design: §5.5 removes the runtime factory, and string addressing served the ConfigDB, removed in §5.4. What remains is the methodology — structured composition — and Rust already has a structured-composition mechanism with compiler enforcement: **ownership**.

**D5.2 (revised) — the component tree is the ownership tree.** A component is a plain struct; its children are its fields:

```rust
pub struct AluEnv {
    agent: AluAgent,        // children are fields — the hierarchy is the struct tree
    scoreboard: Scoreboard,
    coverage: Coverage,
}
```

`#[derive(Component)]` (§6.3, revised) generates the structural plumbing as an implementation of the internal `ComponentNode` traversal trait: visiting fields marked `#[component(child)]` — including `Option<T>` (conditional children, e.g. passive agents) and `Vec<T>` (configured counts) — synthesizing hierarchical names from field names at compile time, and emitting a `visit_children(&mut dyn FnMut(&dyn ComponentInfo))` walker for debug printing. There is no arena, no `ComponentId`, no `Hierarchy` object, and no `uvm_root`: the `#[rustdv::test]` function constructs the env, owns it, and drives its lifecycle. (Test-by-name selection, `uvm_root.run_test`'s remaining job, already belongs to the runner registry — §4.5.)

Consequences:

- **Child access is field access** — `self.agent.monitor` — strictly better ergonomics than both the arena's context-parameter plumbing and pyuvm's string `lookup`. This retires OQ-14 as stated; the *risk* did not vanish but moved into the derive macro (now OQ-15, §8).
- **Cross-component references** (monitor→scoreboard) go through channels (§5.6) — which is the UVM's own prescription (TLM). The arena existed largely to serve lookups the retyped design no longer performs.
- **`lookup`/`find_all` glob search** (pyuvm: `uvm_root.find_all`; `_utility_classes.py`, `uvm_is_match`) is not ported: its consumers were string-keyed config and factory paths (both deleted) and debug printing (served by `visit_children`).

**Predefined components** (pyuvm: `_s13_predefined_component_classes.py`), reclassified per the memo:

| pyuvm class | rustdv form | Notes |
|---|---|---|
| `uvm_test` | the `#[rustdv::test]` function itself | constructs and owns the env; no test component class |
| `uvm_env` | plain struct of children | structural container; the role is conventional |
| `uvm_agent` | struct with `Option<Driver>`/`Option<Sequencer>` fields | active/passive is a config enum (§5.4); a passive agent simply doesn't construct them — pyuvm's illegal-value warning path (pyuvm: `uvm_agent.build_phase`) becomes an unrepresentable state |
| `uvm_driver` | `Driver<REQ, RSP = REQ>` with `seq_item_port: SeqItemPort<REQ, RSP>` | unchanged; REQ/RSP are plain data types (§5.1) |
| `uvm_monitor`, `uvm_scoreboard` | conventions, not marker traits | a methodless marker trait is ceremony in Rust; the book teaches the roles, the code doesn't need the tag |
| `uvm_subscriber` | `trait Subscriber<T> { fn write(&mut self, item: &T); }` | kept as a real trait — `AnalysisPort` dispatches through it; pyuvm enforces the abstract `write` by raising `UVMFatalError` when the un-overridden method is called (pyuvm: `uvm_subscriber.write`); Rust enforces it at compile time |

### 5.3 Lifecycle and objections

*(Adopts review-memo R3: build/connect collapse into constructors; five runtime phases remain; objections unchanged. Per the memo's §4 recommendation — accepted — `build`/`connect` survive as **documented conventions**, not trait methods.)*

pyuvm runs nine common phases by tree traversal, dispatching by method-name string (pyuvm: `_s09_phasing.py`, `uvm_phase.execute`; `run_phase` spawned per component via `uvm_threaded_execute_phase`). Reclassified: `build_phase` and `connect_phase` exist because factory-driven construction is two-stage — components are instantiated before their children or connections can exist. Rust constructors compose bottom-up in one pass, and channel endpoints are created by parents and passed down. Building and connecting are what constructors *do*:

- **build convention** — a component's `new(config, ...)` constructs its children: the body of what would have been `build_phase`.
- **connect convention** — channel endpoints are constructor arguments; wiring happens where construction happens. A missing connection is a missing argument — a compile error — where pyuvm delivers a runtime `UVMTLMConnectionError` or an unconnected-export failure (pyuvm: `_s12` `uvm_port_base.connect` checks; `_s14_15` `get_next_item` assert on `export is not None`).

The book keeps its build/connect chapter structure; rustdv example code marks the corresponding constructor regions with `// build:` and `// connect:` comment conventions. This is an explicit teachability concession (review-memo §4), recorded as such.

What remains at runtime is the lifecycle trait:

```rust
pub trait Component {
    /// Spawn free-running behavior (drivers, monitors, sequencer service);
    /// the runner manages returned tasks. Port of run_phase spawning
    /// (pyuvm: _s09, uvm_threaded_execute_phase; bottomup order preserved).
    fn start(&mut self, ctx: &mut RunCtx);
    fn extract(&mut self) {}                          // topdown, post-run (pyuvm: _s09)
    fn check(&mut self, errors: &mut CheckSink) {}    // topdown
    fn report(&self) {}                               // topdown
    fn final_phase(&self) {}
}
```

Default empty bodies replicate pyuvm's no-op base methods — components override only what they use, the book's teaching pattern (book: uvm_component chapter). Dispatch is static through the derive-generated traversal (§5.2), so the previous revision's `BoxFuture` compromise for `run_phase` is no longer needed here (OQ-3 residual moves to `Sequence::body`, §5.6). `end_of_elaboration`/`start_of_simulation` fold into the test body between construction and `start` — they were empty in every book example. Custom phases: the runner's phase list remains a `Vec<PhaseDescriptor>` for the rare team that needs one; no schedules, no domains — same scope cut as pyuvm, same rationale (pyuvm: `_s09` header comment).

**Objections: unchanged.** The RAII design survives the reclassification untouched — distributed end-of-test consensus is methodology, and the guard was already the idiomatic mechanism. pyuvm's `ObjectionHandler` counts raised/dropped objections with an `Event` signaled at zero, plus a warning if `run_phase` completes with nothing ever raised (pyuvm: `_utility_classes.py`, `ObjectionHandler.run_phase_complete`); its diagnostics (raiser, description, source line — the `Objection` dataclass) are ported in full.

```rust
impl RunCtx {
    /// Port of raise_objection, returning a guard whose Drop is drop_objection.
    /// (pyuvm: _s13 uvm_component.raise_objection/drop_objection/objection())
    pub fn raise_objection(&self, description: &str) -> ObjectionGuard;
}
```

pyuvm already gestures at this with its `objection()` context manager (pyuvm: `_s13_uvm_component.py`, `uvm_component.objection`). Rust's version is strictly better: forgetting to drop is impossible, and the raiser/source-line diagnostics are captured in the guard constructor. The "you never objected" warning and the objection-report-on-timeout are ported as-is.

### 5.4 Configuration: typed config trees

*(Adopts review-memo R4. Replaces the earlier `ConfigDb` runtime-store design.)*

The methodology: tests parameterize components buried N levels deep, including sharing resources like a BFM. pyuvm's mechanism — a two-level dict of glob-capable path keys → field name → {precedence → value}, with build-phase depth precedence, path-specificity retrieval ordering, and tracing (pyuvm: `_s13_uvm_component.py`, `ConfigDB.set/get`, lines 663–805) — is Python compensating for having no typed contract between test and component. Rust has one: the config struct, nested to mirror the ownership tree:

```rust
pub struct AluEnvConfig {
    pub agent: AluAgentConfig,       // nesting mirrors the hierarchy
    pub enable_coverage: bool,
}
pub struct AluAgentConfig {
    pub is_active: Active,           // enum — not a string-keyed int
    pub bfm: Rc<TinyAluBfm>,         // shared resource: an Rc field
}
```

The test builds the tree top-down and passes it to `AluEnv::new(config)`. pyuvm's failure modes map to compile errors: wrong type (was: explosion at the point of use — a bug class the `Any`-valued store couldn't even detect at `get`); missing key (was: `UVMConfigItemNotFound` — pyuvm: `ConfigDB._not_found`); shadowed precedence (was: a whole debugging chapter — now there is exactly one value, constructed in test code you can read). Glob patterns ("configure every driver") become a loop or a shared `Rc` in the test, visible where they act. `wait_modified` (pyuvm: `_s13`, `ConfigDB.wait_modified`) has no direct port — a `sim::Event` field in a config struct covers the pattern where it arises. ⚠ I found no `wait_modified` use in the book's chapters; confidence that nothing of pedagogical value is lost is high but not total.

There is no runtime store, no path strings, no `Box<dyn Any>`, and no precedence algorithm. OQ-9 is retired accordingly (§8).

### 5.5 Variation points: constructor injection replaces the factory

*(Adopts review-memo R5. Replaces the earlier `Factory` registry design.)*

The factory's methodology — a test changes what the testbench does without editing the env (book: "The UVM factory" chapter) — is kept in full. Its mechanism — metaclass self-registration (pyuvm: `_utility_classes.py`, `FactoryMeta.__init__`), override tables with glob paths and recursive chain resolution with loop detection (pyuvm: `FactoryData.find_override`, lines 114–181), `create_component_by_name/by_type` (pyuvm: `_s08_factory_classes.py`) — compensated for SV/Python's inability to pass constructors as values. Rust passes constructors as values natively. In increasing power:

1. **Sequence selection** — the dominant per-test variation in the book — needs no machinery at all: the test starts a different sequence.
2. **Component substitution at a designed variation point** — the config struct (§5.4) carries a maker:

```rust
pub struct AluAgentConfig {
    /// The variation point, explicit in the type. A default is supplied;
    /// a test overrides the driver by assigning a different closure.
    pub make_driver: Box<dyn FnOnce(DriverCtx) -> Box<dyn DriverLike>>,
    // ...
}
```

Three visible lines in test code — no strings, no registry, checked end-to-end. Override *chaining* and loop detection have no equivalent because there is nothing to chase: assignment to a struct field is visible and final, where pyuvm's `xyz → foo → bar` chains required a recursive resolver with a loop-error path (pyuvm: `FactoryData.find_override`, `check_override`).

3. **Instance-path-pattern overrides** ("every driver under `*.agent2`") are the honest loss: with no ambient registry there is nothing to pattern-match against. In exchange, an env's possible behaviors are exactly what its config type declares — no action at a distance. The consequence for source-unavailable env reuse is recorded as a new [gap] in §8.

The string-keyed registry survives in exactly one place: **test discovery**. `#[rustdv::test]` link-time registration (§6.1) remains, because command-line test selection is genuinely stringly and cocotb's model is correct there (cocotb: `regression.py`, `discover_tests`). Everything else — `create_component_by_name`, factory debug printing, override setters — is deleted along with the registry. Factory aliases were unimplemented in pyuvm anyway — both raise `UVMNotImplemented`, noting the SystemVerilog UVM doesn't implement them either (pyuvm: `_s08_factory_classes.py`, lines 322–350).

### 5.6 Communication: channels, analysis broadcast, and the sequencer handshake

*(Adopts review-memo R6. Channels become the primary transport; the sequence machinery is unchanged apart from §5.1's envelope.)*

**Channels replace the TLM-1 taxonomy.** pyuvm implements the ~30-class port/export matrix — blocking/nonblocking × put/get/peek/transport, master/slave composites, runtime `connect()` compatibility checking — as a facade over cocotb queues (pyuvm: `_s12_uvm_tlm_interfaces.py`, `uvm_port_base` lines 60–160; `uvm_tlm_fifo_base` wrapping `UVMQueue`). rustdv ports the queue and deletes the facade:

```rust
pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>);

impl<T> Sender<T> {           // the put family (pyuvm: _s12, 12.2.5)
    pub async fn send(&self, item: T) -> Result<(), TlmError>;
    pub fn try_send(&self, item: T) -> Result<(), TlmFull<T>>;   // returns item on full
    pub fn can_send(&self) -> bool;
}
impl<T> Receiver<T> {         // the get/peek families (pyuvm: _s12, 12.2.5)
    pub async fn recv(&self) -> Result<T, TlmError>;
    pub fn try_recv(&self) -> Result<T, TlmEmpty>;
    pub async fn peek(&self) -> Result<T, TlmError> where T: Clone;
    pub fn try_peek(&self) -> Result<T, TlmEmpty> where T: Clone;
}
```

Blocking/nonblocking × put/get/peek — twelve pyuvm port classes — become six methods on two types. Port/export duality was directionality bureaucracy; `Sender`/`Receiver` state the direction in the type name. `connect_phase` wiring dissolves because endpoints are constructor arguments (§5.3), and a direction mismatch is not a runtime `UVMTLMConnectionError` but a type error. The transport/master/slave composites are dropped: the book never teaches them, and pyuvm's own sources describe the SV machinery they mimic as complexity to escape (pyuvm: `_s14_15`, header comment).

Two abstractions remain as named types because they are semantically distinct, not renamed channels:

- **`AnalysisPort<T>`** — 1-to-many, never blocks, zero-or-more subscribers: a broadcast, not a queue (pyuvm: `_s12`, `uvm_analysis_port.write`, 12.2.8). Subscribers implement `Subscriber<T>` (§5.2) or attach an `AnalysisFifo<T>`; `write(&T)` clones only for subscribers needing ownership.
- **`TlmFifo<T>`** — a *component* wrapping a channel, for when the FIFO should be visible in the hierarchy with `size/used/is_empty/is_full/flush` (pyuvm: `_s12`, lines 849–908). `AnalysisFifo<T>` is its unbounded analysis variant; `ReqRspChannel<REQ, RSP>` ports the composite channel (pyuvm: `_s12`, lines 933–1040).

**The sequencer handshake is unchanged** — it survived the memo's reclassification as methodology: late stimulus generation at the moment of grant is protocol semantics with observable ordering the book teaches (review-memo §4; book: "Sequence testbench: 7.0"). The protocol, preserved event-for-event (pyuvm: `_s14_15_python_sequences.py`, header comment block — the file's own narrative):

1. Sequence: `start_item(req)` → enqueue on the sequencer's request path; block until this item's turn arrives (pyuvm: the item's *start condition*).
2. Driver: `get_next_item()` → dequeue; grant; block until the item is ready.
3. Sequence: fills request fields; `finish_item(req)` → hand off; block until done.
4. Driver: processes the transaction against the DUT; `item_done(Some(rsp))` → release the sequence; response (if any) into the response path, tagged with the envelope's txn id.
5. Sequence (optionally): `get_response(txn_id)` → FIFO-or-by-id retrieval (pyuvm: `ResponseQueue.get_response`).

The signatures differ from the previous revision only in R1's envelope and the removal of the `Transaction` bound — REQ/RSP are plain data types:

```rust
/// User-implemented sequence. Port of uvm_sequence (pyuvm: _s14_15).
pub trait Sequence<REQ, RSP = REQ> {
    fn body(&mut self, ctx: SeqCtx<REQ, RSP>) -> BoxFuture<'_, Result<(), SeqError>>;
    // pre_body/post_body default hooks, gated by start(call_pre_post) as in pyuvm
}

/// Handed to a running sequence; knows its sequencer. Port of the
/// sequence-side API (pyuvm: _s14_15 uvm_sequence.start_item/finish_item).
pub struct SeqCtx<REQ, RSP> { /* sequencer handle, running item id — elided */ }
impl<REQ, RSP> SeqCtx<REQ, RSP> {
    pub async fn start_item(&mut self, item: &mut REQ);
    pub async fn finish_item(&mut self, item: REQ);
    pub async fn get_response(&mut self, txn_id: Option<TxnId>) -> RSP;
}

/// Driver-side port. Port of uvm_seq_item_port (pyuvm: _s14_15).
pub struct SeqItemPort<REQ, RSP> { /* channel endpoints — elided */ }
impl<REQ, RSP> SeqItemPort<REQ, RSP> {
    /// Errors if called twice without item_done — same rule as pyuvm
    /// (pyuvm: uvm_seq_item_export.get_next_item raises UVMSequenceError).
    pub async fn get_next_item(&mut self) -> SeqItem<REQ>;
    /// Tags rsp with the current envelope's txn id (replaces set_context).
    pub fn item_done(&mut self, rsp: Option<RSP>);
    pub async fn get_response(&mut self, txn_id: Option<TxnId>) -> RSP;
}

/// The sequencer component. Port of uvm_sequencer (pyuvm: _s14_15):
/// a queue of sequences feeding one item channel.
pub struct Sequencer<REQ, RSP = REQ> { /* seq queue, channel — elided */ }
```

Handshake state lives in the sequencer's channel internals; items are plain data (contrast pyuvm, where any code holding the item can fire its conditions — pyuvm: `uvm_sequence_item.__init__`). Virtual sequences port as sequences whose `SeqCtx` has no item channel — calling `start_item` on a virtual context is a compile-time impossibility rather than pyuvm's runtime `UVMSequenceError` (pyuvm: `uvm_sequence.start_item` raise). The Fibonacci and `get_response` patterns from the book (book: testbenches 7.1, 7.2) were checked against these signatures on paper; both express directly. ⚠ `Sequence::body` returns `BoxFuture` because sequencers store sequences heterogeneously — this is now the *only* residual of OQ-3 (§8). Sequence arbitration beyond FIFO (grab/lock/priority) is absent in pyuvm and stays absent here — known gap, not open question.

---

## 6. Macro Strategy

Governing principle: **a macro is justified only where pyuvm/cocotb used runtime dynamism that Rust lacks.** Everywhere else, plain traits and generics. The dynamism inventory, from source:

| Python dynamism | Where used | rustdv macro answer |
|---|---|---|
| Decorator wrapping + global registration | `@cocotb.test()` (cocotb: `_decorators.py`) | `#[rustdv::test]` attribute macro |
| Runtime test generation | `TestFactory`, `@parametrize` (cocotb: `_test_factory.py`, `_decorators.py`) | `#[rustdv::parametrize]` compile-time expansion ⚠ OQ-10 |
| Metaclass class registration | `FactoryMeta` (pyuvm: `_utility_classes.py`) | link-time inventory for `#[rustdv::test]` only; component creation is constructor injection (review-memo R5) |
| `__dict__` field walking | `do_copy`/`do_compare` defaults (pyuvm: `_s05`) | std derives (`Clone`/`PartialEq`/`Debug`) — no rustdv macro needed (review-memo R1) |
| `getattr` phase dispatch | `uvm_phase.execute` (pyuvm: `_s09`) | **no macro** — the `Component` lifecycle trait suffices |
| `__getattr__` DUT discovery | `HierarchyObject` (cocotb: `handle.py`) | **no macro required** — dynamic `child()` API; optional codegen is OQ-6 |

### 6.1 `#[rustdv::test]`

Accepts the cocotb `Test` option set as attribute arguments — `timeout_time`/`timeout_unit`, `expect_fail`, `expect_error`, `skip`, `stage`, `name` (cocotb: `_decorators.py`, `test()` signature) — and expands the annotated `async fn` into: the original function, plus a static `TestRegistration` (name, module path, `file!()`/`line!()` for the report table, options, and a monomorphized spawn shim). The runner's regression table then reproduces cocotb's per-test and summary output shape (cocotb: `regression.py`, `_log_test_summary`).

### 6.2 `#[rustdv::parametrize]`

cocotb's `@parametrize` produces one test per option combination with suffixed names (cocotb: `_decorators.py`, `parametrize`, `TestGenerator.generate_tests`). The macro form takes literal value lists and expands N registrations at compile time. ⚠ What is lost vs. Python: parameter sets computed at runtime (from env vars or files). Mitigation: runtime *filtering* stays (cocotb's `COCOTB_TEST_FILTER` equivalent; cocotb: `regression.py`, `add_filters`), and data-driven cases inside one test body remain ordinary Rust. OQ-10.

### 6.3 Derives

*(Revised per review-memo R1/R2: the `UvmObject` and `Transaction` derives are deleted along with the traits they served; `#[derive(Component)]` is redefined.)*

- **Transactions need no rustdv derive.** `#[derive(Clone, Debug, PartialEq)]` covers copy/compare/print (§5.1). Comparison policy lives in the scoreboard, not on the data type, so the `#[uvm(skip)]` field-attribute machinery is gone with it.
- **`#[derive(Component)]`** — generates the structural plumbing of §5.2 as a `ComponentNode` impl: traversal of `#[component(child)]` fields (`T`, `Option<T>`, `Vec<T>`), hierarchical-name synthesis from field names, logging-span wiring, and the `visit_children` debug walker. It emits **no** factory registration — component creation is constructor injection (§5.5). This is now the design's single load-bearing macro; its complexity is tracked as OQ-15, and `ComponentNode` stays hand-implementable so the derive is convenience, not requirement.

### 6.4 What deliberately stays macro-free

The lifecycle, channel wiring, configuration, sequences — all plain code. Two reasons: macro-heavy APIs are miserable to debug for exactly the audience this project serves (error messages point into generated code), and every one of these has a clean trait/generic expression as §5 showed. The bar for adding a macro later: it must delete user-visible boilerplate *and* be explainable in one book paragraph.

---

## 7. Testing Conventions (Worked Example)

The worked example is the book's TinyALU (book: "Basic testbench: 1.0" — A/B 8-bit operands, `op` bus, `start`/`done` single-cycle handshake, ADD/AND/XOR one cycle, MUL three cycles). This section shows the *shape* of the rustdv testbench in signatures and prose; the implementation phase fills in bodies.

### 7.1 Project layout

```
tinyalu_tb/
├── Cargo.toml            # [lib] crate-type = ["cdylib"]; depends on rustdv
├── hdl/tinyalu.sv
├── src/
│   ├── lib.rs            # test entry, #[rustdv::test] functions
│   ├── alu_item.rs       # AluCommand / AluResult transactions
│   ├── alu_bfm.rs        # BFM: owns DUT handles, exposes async API
│   ├── components.rs     # Driver, Monitors, Scoreboard, Coverage
│   ├── env.rs            # AluEnv
│   └── sequences.rs      # AluSeq, per-op sequences
└── rustdv.toml           # simulator selection, sources, top module ⚠ OQ-2
```

### 7.2 The pieces, by signature

**Transactions** — std derives replace the `uvm_sequence_item` subclass with manual `__eq__`/`__str__` (book: sequence chapters); no rustdv-specific derive (review-memo R1):

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct AluCommand { pub a: u8, pub b: u8, pub op: Ops }

#[derive(Clone, Debug, PartialEq)]
pub struct AluResult { pub result: u16 }

#[derive(Clone, Copy, Debug, PartialEq)]  // port of Ops(IntEnum) from
pub enum Ops { Add = 1, And = 2, Xor = 3, Mul = 4 }  // tinyalu_utils (book)
```

**BFM** — port of the book's `TinyAluBfm` (book: "TinyAluBfm" chapter): owns typed handles, runs driver/monitor loops as spawned tasks, exposes queue-fed async methods:

```rust
pub struct TinyAluBfm { /* typed handles, queues — elided */ }
impl TinyAluBfm {
    pub fn new(dut: &HierarchyHandle) -> Result<Self, HandleError>;
    pub async fn reset(&self);
    pub async fn send_op(&self, a: u8, b: u8, op: Ops);
    pub async fn get_cmd(&self) -> AluCommand;     // monitor stream
    pub async fn get_result(&self) -> AluResult;   // result stream
    pub fn start_tasks(&self, ex: &Executor);
}
```

**Components** — driver pulls from its typed `SeqItemPort`; monitors publish on analysis ports; scoreboard subscribes and checks in `check_phase`, all per the book's 6.0 architecture (book: "Components in testbench 6.0", "Connections in testbench 6.0").

**Test** — the top level reads like the book's tests:

```rust
#[rustdv::test(timeout_time = 100, timeout_unit = "us")]
async fn random_ops(ctx: TestCtx) -> Result<(), TestError>;

#[rustdv::test]
async fn max_ops(ctx: TestCtx) -> Result<(), TestError>;
// body shape: build config (sequence + maker choices) → construct env →
// start lifecycle → await sequence. max_ops differs from random_ops only
// in the config it builds — the variation-point pattern replacing the
// factory override (review-memo R5; book: "UVM factory testbench: 5.0"
// teaches the problem this solves).
```

### 7.3 Conventions (the "testing style guide" the book will teach)

1. **Failure taxonomy.** `Result::Err` for *checks* ("result mismatch"); `panic!`/`assert!` for *testbench bugs* ("driver called item_done twice"). Both fail the test; the report distinguishes them, following cocotb's exception-vs-SimFailure separation (cocotb: `regression.py`, `_score_test` error paths).
2. **Every `start`-spawned task that must hold the test open takes an objection guard** in its first statement — the book's rule, RAII-enforced (book: uvm_test chapters; pyuvm: `ObjectionHandler` warning path).
3. **Scoreboards check in `check`, report in `report`** — matching pyuvm traversal order guarantees (pyuvm: `_s09`, both are topdown, post-run).
4. **No sleeps for synchronization.** `Event`/queues, never `Timer`-and-hope — porting the book's NullTrigger lesson (book: Coroutines chapter; cocotb: `NullTrigger` docstring).
5. **Unit tests without a simulator.** Pure-Rust components (scoreboard predictors, transaction ops) get ordinary `#[test]` cargo tests — a genuinely new capability vs. the Python stack, worth a book chapter.

### 7.4 Runner flow

`cargo build` produces the cdylib → `rustdv run` (a small cargo subcommand in `rustdv-runner`) invokes the simulator with the GPI libs and env pointing at the testbench library, mirroring `cocotb_tools.runner`'s role (cocotb: `cocotb_tools/runner.py`). Test selection/filter env vars port from cocotb's (`COCOTB_TEST_FILTER` etc.; cocotb: `regression.py`). Results: console table + xUnit XML (cocotb: `_xunit_reporter.py`). ⚠ Full build-flow ergonomics (one command from zero to waveform) is OQ-2's second half.

---

## 8. Open Questions / Known Gaps

Every ⚠ from the body, consolidated. Items marked **[gap]** are consciously descoped, not unresolved. Rows marked **Retired** record questions resolved by adopting review-memo R1–R6; they are kept for the decision history.

| # | Question | Where raised | Current lean / needed to resolve |
|---|---|---|---|
| OQ-1 | Reuse cocotb's C++ GPI vs. eventual pure-Rust VPI/VHPI layer | §3.1 | Reuse now (D3.1). Revisit only after rustdv is functional on 2+ simulators. |
| OQ-2 | Embedding/bootstrap: exact entry symbols, init ordering, per-simulator loading; and the `rustdv run` build flow | §3.2, §2, §7.4 | Shape is clear; needs a spike against Icarus + Verilator + one commercial sim before freezing. Highest-risk item in the design. |
| OQ-3 | Async-fn-in-trait dyn-compatibility | §5.6 | **Downgraded (R2/R6):** static hierarchy means static lifecycle dispatch, and channels are concrete generic types. Sole residual: `Sequence::body`'s `BoxFuture` (sequencers store sequences heterogeneously). Measure allocation cost under a busy sequencer. |
| OQ-4 | Link-time registration (inventory/linkme technique) reliability across platforms/linkers/LTO | §0.5, §6.1 | **Downgraded (R5):** registration now backs test discovery only; the component factory it was load-bearing for no longer exists. Explicit-registration fallback API still ships. |
| OQ-5 | Drop-based cancellation loses "awaiting cleanup on kill" | §4.6, mapping row 6 | Lean: document the shutdown-message-then-join idiom as the convention; verify against the book's kill-using examples (e.g., Fibonacci 7.1 stopping patterns). |
| OQ-6 | DUT access: purely dynamic `dut.child("x")?` vs. build-time codegen of a typed DUT struct from HDL introspection | mapping row 19, §6 | Lean: dynamic first (teachable, no build magic); codegen as later ergonomics layer. Needs decision before Chapter-level API examples freeze. If a compile-time typed DUT layer is added later, it must be additive to the dynamic `dut.child()` API, not a breaking replacement — existing testbenches written against the dynamic API must continue to work unchanged. |
| OQ-7 | Blocking-world bridge (`bridge`/`resume`/`run_in_executor` equivalents) for file/network/co-simulation I/O | §3.4, mapping row 25 | Undesigned beyond "channel + dedicated OS thread." cocotb's `_bridge.py` state machine is the reference. Defer until a concrete use case (book doesn't teach it). |
| OQ-8 | Logging backend: `tracing` vs. `log`, and mapping onto `gpi_set_log_handler` + sim-time-stamped formatting | §3.1, mapping row 11 | Lean `tracing` (hierarchical targets match `cocotb.task.X` logger naming and pyuvm's per-component loggers). Must reproduce the book's `2.00ns INFO ...` output format. |
| OQ-9 | ConfigDB: `Box<dyn Any>` + `Rc` convention vs. typed keys | §5.4 | **Retired — resolved by redesign (R4):** typed config trees; there is no runtime store, so the question no longer exists. |
| OQ-10 | Parametrized tests: compile-time expansion can't consume runtime parameter sets | §6.2, mapping row 26 | Accept the limit; document data-driven-loop idiom for runtime cases. |
| OQ-11 | Write-scheduling parity: cocotb's inertial-write buffering has per-simulator behavioral nuance | §4.1(4), mapping row 22 | Port the mechanism verbatim; needs cross-simulator regression tests to claim parity. |
| OQ-12 | `UvmRoot`/singleton elimination: runner-owned world vs. thread-local access for deep helper code | mapping row 30, §5.2 | **Retired — resolved by redesign (R2):** the test function owns the env by ordinary ownership; no root object exists to eliminate. |
| OQ-13 | Are any GPI calls actually thread-safe on specific simulators? | §3.4 | Assume none. Only relevant if OQ-7 wants shortcuts; don't take them. |
| OQ-14 | Ergonomics of arena/ID hierarchy: context-parameter plumbing vs. Python's `self.parent.thing` | §5.2 | **Retired (R2):** the arena is gone; child access is field access. The usability risk did not vanish — it transferred into the traversal derive macro. See OQ-15. |
| OQ-15 | `#[derive(Component)]` traversal macro: child discovery over `T`/`Option<T>`/`Vec<T>` fields, hierarchical-name synthesis, logging-span wiring — the design's new load-bearing magic, with macro-grade error messages when it misbehaves | §5.2, §6.3 (review-memo §5, item 1) | Risk is concentrated (one macro, testable by us) rather than distributed (every user's code) — an improvement, but real macro engineering. Prototype as early as OQ-2; ship the `ComponentNode` trait as hand-implementable so the derive is convenience, not requirement. |
| **[gap]** | pack/unpack, recording, policies, run-time phases/domains, sequence arbitration (grab/lock/priority), `uvm_resource_db` | §5.1, §5.3, §5.6 | Same cuts pyuvm made (pyuvm: `_s05` stubs, `_s09` header comment, `_s13` ConfigDB comment). Not planned. |
| **[gap]** | Vertical reuse without source access: overriding components inside an env you cannot edit (UVM instance-path overrides). An env without designed variation points can only be forked | §5.5 (review-memo §5, item 2) | Accepted consequence of R5. Mitigation is a design convention, stated in the book: envs intended for reuse expose maker fields in their config structs. rustdv is honestly weaker than SV-UVM here. |
| **[gap]** | Register abstraction layer (pyuvm `_reg/`) | — | Out of scope for this design doc entirely; pyuvm's RAL port would be its own document. |

---

*End of design document. Companion deliverable: `book-outline.md`.*



