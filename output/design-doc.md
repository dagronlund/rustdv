# rustvm: Design Document

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

This section is written for someone fluent in Python — specifically, someone who wrote the Python patterns that cocotb and pyuvm use — and new to Rust. Each concept is introduced by contrast with the Python you already know, and each ends with *why rustvm needs it*.

### 0.1 Ownership: the concept Python never made you think about

In Python, every variable is a reference to an object on the heap, and the garbage collector decides when objects die. When pyuvm builds a component hierarchy, the parent holds a reference to each child in `self._children`, each child holds a reference back to its parent in `self._parent`, and `uvm_component.component_dict` holds a third reference to everything (pyuvm: `_s13_uvm_component.py`, `uvm_component.__init__`). Nobody owns anything; the GC untangles the cycles.

Rust has no garbage collector. Instead, every value has exactly one **owner** — the variable (or struct field) responsible for destroying it. When the owner goes out of scope, the value is dropped, immediately and deterministically. Assignment *moves* ownership rather than copying a reference:

- Python: `b = a` → two names for one object; both alive.
- Rust: `let b = a;` → the value moved into `b`; using `a` afterward is a **compile error**.

This sounds restrictive because it is. The payoff is that an entire category of testbench bug — the monitor that holds a stale handle to a re-built component, the two tasks that mutate one transaction concurrently — becomes a compile error instead of a 2 a.m. debug session.

**Why rustvm cares:** the UVM component tree is a graph with parent↔child cycles, which is the single worst-case data structure for an ownership system. §5.2 spends most of its length on this. The design chooses an arena/ID scheme precisely so that ownership stays simple.

### 0.2 Borrowing: references with rules

You can lend access to a value without giving up ownership, using references: `&T` (shared, read-only) and `&mut T` (exclusive, read-write). The compiler enforces one rule, sometimes called *aliasing XOR mutability*:

> At any moment a value may have **many readers or one writer, never both**.

Python has no such rule — every reference is a `&mut` and races are your problem (the book's NullTrigger discussion shows exactly this class of bug: two tasks racing to observe `transaction_data`; cocotb: `_base_triggers.py`, `NullTrigger` docstring). In Rust, the pattern the book warns against would not compile.

When you genuinely need Python-like shared mutability — several components holding one scoreboard — Rust provides opt-in escape hatches with the checks moved to runtime: `Rc<T>` (shared ownership via reference counting, like CPython's refcounts made explicit) and `RefCell<T>` (borrow checking at runtime — a `panic!` replaces the compile error). `Rc<RefCell<T>>` is, roughly, "a Python object reference." rustvm uses this combination sparingly and deliberately; every use is called out in §5.

### 0.3 Traits: interfaces without inheritance

Python gave pyuvm three tools that Rust doesn't have: class inheritance, duck typing, and metaclasses. Rust replaces all three with **traits** — explicit, named collections of method signatures that a type opts into:

```rust
pub trait Phased {
    fn build_phase(&mut self, ctx: &mut PhaseCtx);
    fn connect_phase(&mut self, ctx: &mut PhaseCtx);
    // ... default (empty) bodies provided, like pyuvm's no-op phase methods
}
```

Key contrasts:

- **No inheritance.** `uvm_driver(uvm_component)` in pyuvm becomes a `Driver` struct that *contains* its component state and *implements* the `Component` and `Phased` traits. Composition plus traits, never subclassing.
- **Default methods** replace the base-class no-op pattern. pyuvm's `uvm_component` defines empty `build_phase()` etc. (pyuvm: `_s13_uvm_component.py` lines 403–419); a Rust trait provides those as default method bodies you override selectively.
- **Two dispatch styles.** Generics (`fn drive<T: Transaction>(t: T)`) are resolved at compile time — zero cost, like C++ templates but type-checked. Trait objects (`Box<dyn Component>`) are resolved at runtime through a vtable — this is what lets rustvm store heterogeneous components in one hierarchy, the way Python lists hold anything.
- **Duck typing becomes bounds.** "This function needs anything with a `write()` method" becomes `T: Subscriber` — checked at compile time, documented in the signature.

### 0.4 `async`/`await`: the same idea you already know, with the engine exposed

This is the concept where your cocotb knowledge transfers most directly — and where the machinery differs most under the hood.

In cocotb, `await RisingEdge(clk)` works because a coroutine is a resumable function: the scheduler calls `coro.send(None)`, the coroutine runs until it yields a `Trigger`, and the scheduler registers a callback so the trigger's firing resumes the task (cocotb: `src/cocotb/task.py`, `Task._resume`; `_base_triggers.py`, `Trigger._register`). The event loop is a `deque` of callbacks drained to exhaustion (cocotb: `_event_loop.py`, `EventLoop.run`).

Rust's `async fn` compiles to a **state machine** implementing the `Future` trait, whose one method is:

```rust
fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
```

Where Python's coroutine *pushes* a Trigger out to the scheduler, a Rust future is *polled* and answers either `Poll::Ready(value)` or `Poll::Pending`. Before returning `Pending`, the future stashes a **`Waker`** — a cheap handle meaning "poll me again" — with whatever will eventually fire (in rustvm: a trigger's callback list). The Waker plays exactly the role of cocotb's `TriggerCallback` (cocotb: `_base_triggers.py`, `TriggerCallback`).

Two consequences matter enormously for rustvm:

1. **Rust ships no event loop.** `async`/`await` is pure language; the executor is a library you choose — or write. cocotb *already* had to write its own executor because asyncio can't block on simulator time. rustvm is in the same position, and a simulator-driven executor is small (cocotb's is 82 lines). We write our own; we do **not** pull in tokio (§4, rationale there).
2. **Cancellation is dropping.** cocotb cancels a task by throwing `CancelledError` into it, and the coroutine's `try/finally` blocks run (cocotb: `task.py`, `Task.cancel`). Rust cancels a task by *dropping the future* — the state machine is destroyed, and cleanup happens in `Drop` implementations (destructors). There is no exception to catch. §4.6 treats this asymmetry as a first-class design topic, because every driver/monitor "kill the task at end of test" pattern from the book crosses it.

### 0.5 Proc macros vs. decorators: compile time vs. run time

`@cocotb.test()` is a function that receives your function and wraps it — at *import time*, with full runtime power: it can inspect signatures, consult environment variables, and register the test in a global list (cocotb: `_decorators.py`, `test()`; `regression.py`, `RegressionManager.discover_tests`). pyuvm's factory goes further: a *metaclass* registers every component class as a side effect of the `class` statement itself (pyuvm: `_utility_classes.py`, `FactoryMeta`).

Rust has no import time and no metaclasses. Its equivalent power tool is the **procedural macro**: a function that runs *inside the compiler*, receives your code as a token stream, and emits replacement code. Three kinds matter here:

- **Attribute macros** — `#[rustvm::test]` sits where `@cocotb.test()` sat and rewrites the annotated `async fn` into a registered test entry.
- **Derive macros** — `#[derive(Transaction)]` sits where pyuvm's reliance on `__dict__` introspection sat: since Rust can't discover struct fields at runtime, the macro generates field-wise `do_copy`/`do_compare`/`convert2string` at compile time.
- **Declarative macros** (`macro_rules!`) — simple pattern-based rewriting, used sparingly.

The catch: because macros run at compile time, *runtime registration must be replaced by link-time collection*. There is no moment when "all classes have been imported" — so rustvm uses distributed static registration (the technique behind the `inventory`/`linkme` crates) to build the test list and factory registry before `main` runs. ⚠ This mechanism has platform-specific subtleties (§8, OQ-4).

### 0.6 `Result`/`Option` vs. exceptions

Rust has no exceptions. Fallible functions return `Result<T, E>` (either `Ok(value)` or `Err(error)`), and absent values are `Option<T>` (either `Some(value)` or `None`). The `?` operator propagates errors up the call stack with one character, giving `try/except`-like ergonomics without invisible control flow:

```rust
fn get_dut_signal(dut: &HierarchyHandle, name: &str) -> Result<LogicHandle, HandleError>;
// caller: let clk = get_dut_signal(&dut, "clk")?;
```

Contrasts that matter for rustvm's API design:

- pyuvm raises `UVMConfigItemNotFound` when a ConfigDB key is missing (pyuvm: `_s13_uvm_component.py`, `ConfigDB._not_found`); rustvm returns `Result<T, ConfigError>` — the *signature* tells you it can fail.
- cocotb marks a test failed by letting any exception propagate out of the test coroutine (cocotb: `regression.py`, `_score_test`). rustvm tests return `Result<(), TestError>`; an `Err` fails the test.
- Rust *does* have `panic!` — an abort-the-task mechanism for "this is a bug" situations, used by `assert!`/`assert_eq!`. Panics in a rustvm test are caught at the task boundary and scored as test failure, mirroring how cocotb catches `BaseException` per task (cocotb: `task.py`, `Task._resume`). Panics are for assertion failures; `Result` is for expected fallibility (missing signals, config lookups). This split is a designed convention, stated in §7.

### 0.7 Odds and ends you'll hit immediately

- **No GIL, but also no threads (here).** GPI is not thread-safe, and cocotb runs everything on the simulator's thread. rustvm does the same; the type system *enforces* it by making key types `!Send` (unable to leave the thread they were created on) — see §3.4. Rust's fearless-concurrency story is real but mostly unused in rustvm's core.
- **`String` vs. `&str`** — owned string vs. borrowed string slice; the practical rule is "store `String`, pass `&str`."
- **Lifetimes** (`'a`) — annotations telling the compiler how long borrows live. rustvm's public API is designed to keep user-facing lifetimes rare; where signatures in this document show them, that is a deliberate, commented choice.
- **`cargo`** — think `pip` + `venv` + `make` + `pytest` in one tool. The build/run story in §7 is cargo-native.

---

## 1. Concept Mapping Table (Python → Rust)

Legend: **[C]** = cocotb source, **[P]** = pyuvm source, **[B]** = *Python for RTL Verification*. ⚠ marks mappings with unresolved alternatives (detailed in §8).

### 1.1 Language & runtime layer

| # | Python (as used today) | Rust (rustvm design) | Rationale | Source |
|---|---|---|---|---|
| 1 | `async def` coroutine, resumed via `coro.send(None)` | `async fn` → `impl Future`, resumed via `poll()` | Same user-facing model; engine differs (§0.4) | [C] `task.py` |
| 2 | `@cocotb.test()` decorator | `#[rustvm::test]` attribute macro | Runtime wrapping → compile-time rewriting (§0.5, §6.1) | [C] `_decorators.py` |
| 3 | Exceptions for test failure | `Result<(), TestError>` return + caught panics for assertions | No exceptions in Rust; signatures document fallibility (§0.6) | [C] `regression.py` `_score_test` |
| 4 | `cocotb.start_soon(coro)` | `rustvm::spawn(future) -> TaskHandle<T>` | Same semantics: queue task, run at next loop turn | [C] `_test_manager.py` `start_soon` |
| 5 | `Task` 7-state machine (UNSTARTED…CANCELLED) | `TaskState` enum, same seven states | Proven model; keep debugging story identical | [C] `task.py` `_TaskState` |
| 6 | `task.kill()` / `task.cancel()` + `CancelledError` | `TaskHandle::cancel()` → future dropped; cleanup via `Drop` | Rust cancellation is drop-based — semantic shift ⚠ (§4.6, OQ-5) | [C] `task.py` `cancel` |
| 7 | `TaskManager` / `async with` task groups | `TaskGroup` with RAII guard (`Drop` cancels children) | Context-manager → RAII is the idiomatic translation | [C] `_task_manager.py` |
| 8 | Metaclass side effects (`FactoryMeta`) | Derive macro + link-time registration | No metaclasses; only compile/link-time hooks exist ⚠ | [P] `_utility_classes.py` |
| 9 | `getattr(comp, "build_phase")()` string dispatch | `Phased` trait, direct method calls | Reflection unavailable; traits are faster and checked | [P] `_s09_phasing.py` `uvm_phase.execute` |
| 10 | Store-anything containers (`dict` of `Any`) | `Box<dyn Any>` + typed `downcast` at retrieval | Type erasure is explicit and checked at the boundary | [P] `ConfigDB` |
| 11 | Python `logging` hierarchy (`cocotb.task.X`) | `tracing` crate with span/target hierarchy ⚠ | Structured, hierarchical, filterable; maps to GPI log handler | [C] `logging.py`, `gpi.h` logging group |

### 1.2 cocotb core layer

| # | Python (cocotb) | Rust (rustvm design) | Rationale | Source |
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
| 26 | `TestFactory` / `@parametrize` runtime generation | `#[rustvm::parametrize(...)]` compile-time expansion ⚠ | No runtime class creation; macro generates N registered tests (OQ-10) | [C] `_test_factory.py`, `_decorators.py` |

### 1.3 pyuvm/UVM layer

| # | Python (pyuvm) | Rust (rustvm design) | Rationale | Source |
|---|---|---|---|---|
| 27 | `uvm_object` base class | `UvmObject` trait (+ `#[derive(UvmObject)]`) | Name/id/type-name surface without inheritance | [P] `_s05_base_classes.py` |
| 28 | `clone/copy/compare` via `do_copy/do_compare` hooks | `Transaction` derive generating field-wise ops, with overridable trait methods | No runtime `__dict__`; compile-time field enumeration | [P] `_s05_base_classes.py` |
| 29 | `uvm_component(name, parent)` tree of references | Arena-owned tree: `ComponentId` keys, `Box<dyn Component>` storage | Breaks parent↔child ownership cycle (§5.2) | [P] `_s13_uvm_component.py` |
| 30 | `uvm_root()` singleton | Explicit `UvmRoot` owned by the test runner (no global) ⚠ | Global mutable state is hostile in Rust; runner owns the world (OQ-12) | [P] `UVM_ROOT_Singleton` |
| 31 | 9 common phases, topdown/bottomup traversal | Same phase list & traversal orders; `Phased` trait | Faithful port of pyuvm's simplification of IEEE 1800.2 | [P] `_s09_phasing.py` |
| 32 | `raise_objection`/`drop_objection` + handler singleton | `ObjectionGuard` RAII handle from `ctx.raise_objection(desc)` | Drop-based release is strictly safer than manual drop | [P] `ObjectionHandler`; `uvm_component.objection()` |
| 33 | `ConfigDB().set/get` glob paths, `Any` values | `ConfigDb::set::<T>/get::<T>` — glob paths, `Box<dyn Any>`, typed downcast, `Result` errors | Same wildcard semantics incl. build-phase depth precedence | [P] `ConfigDB` |
| 34 | `uvm_factory()` create-by-name/type, overrides | `Factory` registry: name→constructor map, type/inst overrides with chain+loop detection | Port `find_override` algorithm as-is | [P] `_s08_factory_classes.py`, `_utility_classes.py` `FactoryData.find_override` |
| 35 | TLM-1 ports/exports (blocking/nonblocking put/get/peek/transport) | `Port<T>`/`Export<T>` generic types over a channel core; async trait methods ⚠ | Full taxonomy preserved; dyn-compatibility of async traits is OQ-3 | [P] `_s12_uvm_tlm_interfaces.py` |
| 36 | `uvm_analysis_port.write()` fan-out | `AnalysisPort<T>`: broadcast to N subscribers, non-blocking | 1-to-many, fire-and-forget, as in UVM | [P] `_s12` `uvm_analysis_port` |
| 37 | `uvm_tlm_fifo`, analysis FIFO, req/rsp channel | `TlmFifo<T>` etc. on `sim::Queue<T>` | Same size-1 default, `used()`, `flush()` surface | [P] `_s12` `uvm_tlm_fifo_base` |
| 38 | `uvm_sequence.start/start_item/finish_item/get_response` | `Sequence` trait with `async fn body(&mut self, ctx: SeqCtx<REQ, RSP>)` | Handshake protocol preserved event-for-event (§5.6) | [P] `_s14_15_python_sequences.py` |
| 39 | `ResponseQueue` txn-id cherry-picking | `ResponseQueue<RSP>` with `get_response(Option<TxnId>)` | Same select-by-id or FIFO-order behavior | [P] `ResponseQueue` |
| 40 | `uvm_driver` with `seq_item_port` | `Driver<REQ, RSP>` generic struct + `SeqItemPort<REQ, RSP>` | Typed transactions end run-time type errors at the driver boundary | [P] `_s13_predefined_component_classes.py` |
| 41 | `uvm_subscriber.write()` abstract method | `Subscriber<T>` trait: `fn write(&mut self, item: &T)` | Abstract method → required trait method | [P] `uvm_subscriber` |
| 42 | `uvm_agent` active/passive via ConfigDB | Same: `Agent` reads `is_active` from ConfigDB in `build_phase` | Behavior parity with book's agent chapters | [P] `uvm_agent`; [B] testbench 6.0 chapters |

---

## 2. Crate/Module Structure

cocotb splits into a Python package (`src/cocotb`), a C++ simulator-interface library (`src/cocotb/share/lib/gpi`), an embedding shim (`share/lib/pygpi`), and a tools package (`src/cocotb_tools`) *(cocotb: source layout)*. rustvm mirrors that separation as a cargo **workspace** — the boundaries earned their keep in cocotb and the FFI boundary *must* be its own crate in Rust anyway (`-sys` convention).

```
rustvm/                          # cargo workspace root
├── rustvm-gpi-sys/              # raw FFI bindings to gpi.h (bindgen), no logic
├── rustvm-gpi/                  # safe wrapper: handles, callbacks, values
├── rustvm-sim/                  # executor, tasks, triggers, time, clock, queues
│   └── (modules) executor, task, trigger, handle, types, clock, simtime, queue
├── rustvm-uvm/                  # the UVM analog
│   └── (modules) object, component, hierarchy, phase, objection,
│       config_db, factory, tlm, sequence, predefined
├── rustvm-macros/               # proc macros: #[test], #[parametrize], derives
├── rustvm-runner/               # regression manager, test registry, entry point,
│                                #   cdylib bootstrap, result reporting (xUnit)
└── rustvm/                      # facade crate: re-exports the public API
```

Design decisions and their sources:

**D2.1 — `rustvm-gpi-sys` is bindings-only.** Machine-generated from `gpi.h` (cocotb: `share/include/gpi.h`), no hand-written logic, everything `unsafe extern "C"`. This is the standard Rust `-sys` crate discipline: one crate owns "what the C API is," another owns "how to use it safely." Keeping it generated means tracking upstream cocotb GPI changes is a re-run of bindgen, not a port.

**D2.2 — `rustvm-sim` does not depend on `rustvm-uvm`.** pyuvm imports cocotb, never the reverse (pyuvm: `_utility_classes.py` imports `cocotb.queue`, `cocotb.triggers`). Same direction here: you can write book-style "testbench 1.0/2.0" (pre-UVM) programs against `rustvm-sim` alone, which the book's pedagogy requires — chapters 23–27 of the book use cocotb without pyuvm (book: "Basic testbench: 1.0" through "Class-based testbench: 2.0").

**D2.3 — `rustvm-macros` is a separate crate by necessity.** Rust requires proc macros to live in their own crate type. It depends on nothing at runtime; `rustvm-uvm` and `rustvm-runner` provide the symbols the generated code calls.

**D2.4 — `rustvm-runner` owns `main`-equivalent duties.** cocotb's `RegressionManager` discovers tests, runs them in order, times them, scores exceptions vs. expectations, and writes xUnit XML (cocotb: `regression.py`). The runner crate ports this: test registry (populated at link time by `#[rustvm::test]`), sequential test execution, `RANDOM_SEED` handling (cocotb: `_init.py`, `_setup_random_seed`), result table, and the simulator entry point (§3.2).

**D2.5 — the `rustvm` facade re-exports a curated prelude.** The book teaches `import cocotb` / `from pyuvm import *` (book: every example). The Rust equivalent of that ergonomics is `use rustvm::prelude::*;` — one line for users, while the workspace stays modular behind it.

**Feature flags.** ⚠ Simulator selection (Icarus/Verilator/Questa/…) is a *runtime* concern in cocotb (GPI impl `.so` chosen by the makefiles; cocotb: `share/def/*.def`, `cocotb_tools/makefiles`). rustvm keeps that runtime model rather than cargo features where possible, but the build flow for linking testbench-as-cdylib against each simulator is OQ-2.

---

## 3. Simulator Interface Layer

### 3.1 Decision: reuse cocotb's GPI, don't rewrite it

**D3.1 — rustvm links against cocotb's existing GPI C++ library and binds to `gpi.h`.**

The GPI layer is cocotb's crown jewel: one C API (`gpi.h`, 589 lines) abstracting VPI, VHPI, and FLI, with a decade of accumulated simulator-quirk fixes (cocotb: `share/lib/gpi/GpiCommon.cpp`, per-simulator `def` files, and the FLI sensitivity-list workaround documented in `gpi.h` lines 29–32). The header is deliberately C-compatible — opaque handle pointers, plain enums, function pointers — i.e., it is *already* an FFI boundary designed for exactly this kind of consumption.

Rewriting VPI/VHPI/FLI handling in Rust would be years of re-learning quirks the GPI already encodes, for zero user-visible benefit. The port boundary is `gpi.h`, full stop. A pure-Rust GPI remains a possible *future* phase and is recorded as OQ-1.

What this buys, concretely — the full `gpi.h` surface rustvm binds:

| `gpi.h` group | Functions (abridged) | rustvm safe wrapper |
|---|---|---|
| Sim control/query | `gpi_get_sim_time`, `gpi_get_sim_precision`, `gpi_get_simulator_product/version`, `gpi_finish` | `SimContext` methods; time as `SimTime` (u64 steps + precision) |
| Object query | `gpi_get_root_handle`, `gpi_get_handle_by_name`, `gpi_get_handle_by_index` | `HierarchyHandle::child(name/index) -> Result<AnyHandle>` |
| Object properties | `gpi_get_object_type`, `gpi_get_num_elems`, `gpi_get_range_*`, `gpi_is_constant/indexable/signed` | typed-handle downcasting (§3.3) |
| Signal values | `gpi_get_signal_value_binstr/str/real/long`, `gpi_set_signal_value_*` + `gpi_set_action` | `LogicHandle::get/set`, `SetAction` enum |
| Iteration | `gpi_iterate`, `gpi_next` | `HierarchyHandle::children() -> impl Iterator` |
| Callbacks | `gpi_register_{timed, value_change, readonly, nexttime, readwrite}_callback`, `gpi_remove_cb` | trigger primitives (§4) |
| Logging | `gpi_set_log_handler`, log levels | bridge to Rust logging (⚠ OQ-8) |

### 3.2 Embedding: how rustvm code gets into the simulator process

cocotb's chain today: simulator loads a GPI implementation library (VPI/VHPI/FLI `.so`) → GPI loads `libpygpi` (`embed.cpp`) → which starts an embedded CPython → which imports the user's test module (cocotb: `share/lib/pygpi/embed.cpp`, `src/pygpi/entry.py`, `_init.py`, `init_package_from_simulation`).

**D3.2 — the rustvm testbench compiles to a `cdylib`** that exports the same entry-point symbols `libpygpi` exports today, so the existing GPI loader machinery (`dynload.cpp`, environment-variable-driven library discovery) can load a Rust testbench in place of the Python interpreter. The user's test crate links `rustvm`, and `rustvm-runner` provides the exported entry functions; from GPI's point of view nothing changed.

⚠ I am confident about the *shape* of this (the loader is explicitly designed around an env-var-named library with known entry points), but the exact symbol set, initialization ordering, and per-simulator loading quirks need a prototype before this section can be called settled. OQ-2.

### 3.3 The safe/unsafe split

All `unsafe` lives in `rustvm-gpi`, upholding these invariants so that everything above it is safe Rust:

1. **Handles are opaque and non-null.** `gpi_sim_hdl` etc. are incomplete-type pointers (cocotb: `gpi.h` lines 45–83). Wrapper: `struct RawObjHandle(NonNull<c_void>)`. Fallible acquisition (`gpi_get_handle_by_name` returns NULL for not-found) becomes `Result`/`Option` at the boundary — never a nullable handle in user code.
2. **Handle lifetime = simulation lifetime.** GPI object handles are valid until sim end (cocotb treats them so: `handle.py` caches them for the process lifetime). Wrappers are therefore freely cloneable ID types; no `Drop` frees a sim object. Callback handles differ: they invalidate on `gpi_remove_cb` or after firing — modeled as consuming methods (`fn deregister(self)`) so a stale callback handle is unrepresentable.
3. **Strings are copied at the boundary.** `const char*` returns point into simulator-owned memory of unspecified lifetime; the wrapper copies to `String` immediately, on every call.
4. **No unwinding across FFI.** Every Rust function passed to C as a callback wraps its body in `catch_unwind`; a panic is converted to test-failure state, never propagated into the simulator. (cocotb has the same concern with C++ exceptions; embed layer catches everything.)
5. **Callback user-data ownership.** `gpi_register_*_callback(fn, void* data, ...)` takes a C function pointer plus context. The wrapper boxes a Rust closure, passes it as the `void*`, and reclaims the `Box` when the callback is deregistered or fires for the last time. One-shot callbacks (timers — GPI callbacks are single-fire; cocotb re-registers each time, see `_gpi_triggers.py` `Timer._prime`) reclaim on fire; the wrapper encodes one-shot vs. recurring in the type.

### 3.4 Thread affinity

GPI has no thread-safety guarantees; cocotb only ever calls it from the simulator callback thread, and shunts real threads through the bridge (cocotb: `_bridge.py`). rustvm encodes this in the type system: `SimContext`, all handles' *methods*, and the executor are `!Send` — the compiler rejects any attempt to move them to another thread. External threads interact only through the bridge channel (⚠ OQ-7 for its design). This turns cocotb's documentation-level rule into a compile-time rule — one of the clearest wins of the port. ⚠ Whether *some* GPI calls are in fact safe off-thread on some simulators: unknown, assumed no. OQ-13.

---

## 4. Concurrency/Scheduling Model

### 4.1 What cocotb actually does (the spec for our port)

Distilled from source — this sequence is the contract rustvm must reproduce:

1. A GPI callback fires (timer, edge, phase). The trigger's `_react()` runs all callbacks registered on that trigger — each callback typically marks one task SCHEDULED and pushes its resume onto the event loop — and then **drains the event loop to exhaustion** before returning to the simulator (cocotb: `_gpi_triggers.py`, `GPITrigger._react`; `_event_loop.py`, `EventLoop.run`).
2. Resuming a task means `coro.send(None)`; the task runs until it finishes, raises, or yields the next `Trigger` it awaits (cocotb: `task.py`, `Task._resume`).
3. A trigger primes its underlying GPI mechanism lazily — on first registered callback — and unprimes when its last callback deregisters (cocotb: `_base_triggers.py`, `Trigger._register`/`_deregister`).
4. Signal writes via `set()` are buffered and applied at the start of the next ReadWrite phase (inertial-write workaround; cocotb: `handle.py` write scheduler, `ReadWrite._do_callbacks`).
5. Everything happens on one thread. "Parallelism" is cooperative interleaving at await points — the model the book teaches with the producer/consumer and BFM examples (book: "Coroutines" and "cocotb Queue" chapters).

### 4.2 Decision: a bespoke single-threaded executor (not tokio)

**D4.1** — rustvm implements its own executor in `rustvm-sim`. Rationale:

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

- **`Timer`** — one-shot timed callback; rejects non-positive durations (cocotb: `Timer.__init__` raises `ValueError`; rustvm makes the constructor return `Result` or use unit-typed constructors that can't express zero ⚠ minor).
- **`RisingEdge` / `FallingEdge` / `ValueChange`** — via `gpi_register_value_change_callback` with `gpi_edge` selector; exposed as methods on typed handles (mapping row 14).
- **`ReadOnly` / `ReadWrite` / `NextTimeStep`** — phase triggers, singletons per sim context; `ReadWrite` drains the write buffer before waking subscribers, and awaiting `ReadOnly`-from-`ReadOnly` or `ReadWrite`-from-`ReadOnly` is a runtime error, both exactly as cocotb (cocotb: `_gpi_triggers.py`, `ReadOnly.__await__`/`ReadWrite.__await__`).
- **`sim::Event`** — manual-reset event; `wait()` completes immediately if already set (cocotb: `_base_triggers.py`, `_Event._prime`).
- **`sim::Lock`** — FIFO-fair mutex; acquisition order = request order, documented behavior in cocotb (cocotb: `_base_triggers.py`, `Lock` docstring "Guarantees fair scheduling").
- **`NullTrigger`** — yield-once; kept for parity but documented as a smell, matching the book's and cocotb's guidance to prefer `Event` (cocotb: `NullTrigger` docstring; book: same example).
- **`first!` / `join!`** — ports of `First`/`Combine` (cocotb: `_extended_awaitables.py`). In Rust these are future combinators; `first!` drops the losing futures, which unsubscribes their triggers via `Drop` — the cleanup cocotb does manually with kill-on-completion tasks falls out of RAII for free.

### 4.5 Test execution layer

`#[rustvm::test]` registers a `TestCase` (name, module, source location, options: `timeout`, `expect_fail`, `expect_error`, `skip`, `stage` — the full option set of cocotb's `Test` class (cocotb: `_decorators.py`, `Test.__init__`)). The runner executes tests sequentially, one `TestManager`-equivalent per test that: spawns the test future, tracks all tasks spawned during the test, cancels survivors at test end, converts stray panics/errors in *any* task into test failure — porting the "child task exception fails the test" behavior (cocotb: `_test_manager.py`, `TestManager._task_done_callback`).

### 4.6 Cancellation: the one big semantic divergence

cocotb cancels by throwing `CancelledError` *into* the coroutine, which can catch it, run `finally` blocks, even (buggily) suppress it — cocotb devotes real machinery to detecting suppression (cocotb: `task.py`, `_must_cancel` checks). Rust cancellation is: the executor drops the future. Cleanup runs in `Drop` impls of whatever the future held. No task-side code executes after the drop point; there is nothing to suppress and no way to observe "being cancelled" from inside.

Consequences the design embraces:

- Driver/monitor cleanup must live in `Drop` (RAII guards), not in `finally`-style code after the await. §5 APIs are shaped so components' resources are naturally `Drop`-owned. This is *simpler* and less error-prone than cocotb's model — the class of "forgot to re-raise CancelledError" bugs (which cocotb explicitly detects and errors on) cannot exist.
- ⚠ One cocotb capability is lost: a task doing final work *at cancellation time that requires awaiting* (e.g., drive bus idle over several cycles on kill). Rust drops synchronously. Teams needing this pattern must restructure (explicit shutdown message + join instead of kill). Recorded as OQ-5 with the recommended idiom.

---

## 5. Testbench/UVM-Analog API

This section ports pyuvm's architecture, subsystem by subsystem, in pyuvm's own file order. The guiding principle throughout: **pyuvm simplified SystemVerilog UVM by using Python's dynamism; rustvm re-simplifies pyuvm by using Rust's type system.** Where pyuvm checks types at runtime (`assert issubclass(type(item), uvm_sequence_item)` — pyuvm: `_s14_15_python_sequences.py`, `uvm_seq_item_port.put_response`), rustvm makes the check a generic bound and deletes the runtime code.

### 5.1 `UvmObject` and transactions

pyuvm's `uvm_object` (pyuvm: `_s05_base_classes.py`) provides: name management, instance id, type name, `create`/`clone`, `copy`/`do_copy`, `compare`/`do_compare`, `convert2string`, and (unimplemented) pack/record/policy stubs. `uvm_transaction` adds transaction ids and (mostly unimplemented) timing/recording hooks.

**D5.1 — split into a small required trait plus a derive.**

```rust
/// The required surface. Port of uvm_object naming/id (pyuvm: _s05, 5.3.3–5.3.4).
pub trait UvmObject {
    fn name(&self) -> &str;
    fn set_name(&mut self, name: &str);
    fn inst_id(&self) -> InstId;
    fn type_name(&self) -> &'static str;
}

/// Field-wise operations. In pyuvm these walk __dict__ at runtime;
/// in Rust the derive macro generates them at compile time (§6.3).
pub trait ObjectOps: UvmObject {
    fn do_copy(&mut self, rhs: &dyn Any) -> Result<(), CopyError>;
    fn do_compare(&self, rhs: &dyn Any) -> bool;
    fn convert_to_string(&self) -> String;
}

/// Transactions add identity for req/rsp correlation.
/// (pyuvm: _s05 uvm_transaction + _s14_15 uvm_sequence_item id plumbing)
pub trait Transaction: ObjectOps {
    fn transaction_id(&self) -> TxnId;
    fn set_context(&mut self, req: &dyn Transaction);   // rsp.set_context(req)
}
```

Users write `#[derive(Transaction)]` on a plain struct of fields and get all three traits with field-wise compare/copy/to-string — replacing the inheritance chain `uvm_object → uvm_transaction → uvm_sequence_item` with one line. Overriding `do_compare` for a custom notion of equality remains possible by implementing the method manually (the derive skips what you define). *(Decision informed by the book's uvm_object chapter, which teaches exactly `do_copy`/`do_compare`/`convert2string` as the customization points; book: "uvm_object in Python".)*

What is **deliberately dropped**: pack/unpack, recording hooks, policies — pyuvm itself raises `UVMNotImplemented` or provides stubs for most of these (pyuvm: `_s05_base_classes.py`, `uvm_policy.__new__` raises). Porting stubs would be cargo-culting. Listed in §8 as known gaps, not open questions.

### 5.2 Components and hierarchy

The heart of the problem. pyuvm's hierarchy is mutually-referential: child holds `_parent`, parent holds `_children[name]`, and a class-level `component_dict` maps full names to components globally (pyuvm: `_s13_uvm_component.py`, `uvm_component.__init__`). In Rust, `Rc<RefCell<...>>` cycles leak (Rc cannot collect cycles) and make every access a runtime borrow gamble.

**D5.2 — the hierarchy is an arena owned by `UvmRoot`.** Components live in a slab/arena keyed by `ComponentId` (a copyable integer newtype). Parent/child links are `ComponentId`s, not references. All traversal goes through the arena, which is owned by the root, which is owned by the runner.

```rust
pub struct ComponentId(/* opaque */);

/// The component tree. Port of uvm_root + component_dict
/// (pyuvm: _s13_uvm_component.py) as one owned structure, no globals.
pub struct Hierarchy { /* arena, name index — elided */ }

impl Hierarchy {
    pub fn parent(&self, id: ComponentId) -> Option<ComponentId>;
    pub fn children(&self, id: ComponentId) -> impl Iterator<Item = ComponentId> + '_;
    pub fn full_name(&self, id: ComponentId) -> String;      // 13.1.3.2
    pub fn get_child(&self, id: ComponentId, name: &str) -> Option<ComponentId>;
    pub fn lookup(&self, from: ComponentId, path: &str) -> Option<ComponentId>; // 13.1.3.7
    /// Glob search, porting uvm_is_match + find_all
    /// (pyuvm: _utility_classes.py uvm_is_match; _s13 uvm_root.find_all)
    pub fn find_all(&self, pattern: &str) -> Vec<ComponentId>;
    pub fn find(&self, pattern: &str) -> Option<ComponentId>;
}
```

Rationale: (a) no reference cycles, so no leaks and no `RefCell` panics; (b) `ComponentId` is `Copy`, so "hold a handle to the scoreboard" is trivial for any component; (c) mirrors how pyuvm *actually* resolves cross-hierarchy access anyway — through the global `component_dict` by full name (pyuvm: `uvm_component.lookup`). The arena is that dict, made explicit and owned. ⚠ The ergonomic cost — user methods receive `&mut Hierarchy` access through a context parameter rather than `self.parent.thing` — is real, and the exact context-passing API needs prototyping pressure-testing. OQ-14.

**The `Component` trait and construction.**

```rust
/// What every component implements, usually via #[derive(Component)] + impl Phased.
pub trait Component: UvmObject + Phased + Any {
    /// Called by the factory/hierarchy with identity already assigned.
    /// Port of uvm_component.__init__(name, parent) (pyuvm: _s13, 13.1.2.1),
    /// split so construction can't forget hierarchy registration.
    fn init(ctx: ComponentCtx) -> Self where Self: Sized;
}
```

In pyuvm, `Comp("name", parent)` registers itself via `__init__` side effects. In Rust, components are *always* created through the hierarchy (`hierarchy.create::<MyDriver>("drv", parent_id)` or the factory), which allocates the id, registers name/parent, then calls `init`. Direct struct construction bypassing registration doesn't compile against the API. This closes a pyuvm footgun (constructing a component with a wrong/absent parent silently reparents to root; pyuvm: `uvm_component.__init__`).

**Predefined components** (pyuvm: `_s13_predefined_component_classes.py`) port as follows:

| pyuvm class | rustvm form | Notes |
|---|---|---|
| `uvm_test` | `Test` marker trait | run_test entry constraint |
| `uvm_env` | `Env` marker trait | structural container |
| `uvm_agent` | `Agent` base struct (generic over its driver/monitor/sequencer types) | reads `is_active` from ConfigDB in `build_phase`, defaults to active, warns and resets to active on illegal values — exact pyuvm behavior (pyuvm: `uvm_agent.build_phase`) |
| `uvm_driver` | `Driver<REQ: Transaction, RSP: Transaction = REQ>` with `seq_item_port: SeqItemPort<REQ, RSP>` | typed at last |
| `uvm_monitor` | `Monitor` marker trait | convention carrier |
| `uvm_scoreboard` | `Scoreboard` marker trait | checks in `check_phase` per book convention (book: scoreboard chapters) |
| `uvm_subscriber` | `trait Subscriber<T>: Component { fn write(&mut self, item: &T); }` | pyuvm enforces the abstract `write` by raising `UVMFatalError` when the un-overridden method is called (pyuvm: `uvm_subscriber.write`); Rust enforces it by the trait not compiling without it |

### 5.3 Phasing and objections

pyuvm deliberately ignores IEEE 1800.2 generalized phasing and runs the nine common phases by simple tree traversal — topdown or bottomup per phase — dispatching by *method name string* (pyuvm: `_s09_phasing.py`, header comment and `uvm_phase.execute`; `run_phase` is spawned as a cocotb task per component via `uvm_threaded_execute_phase`). `uvm_root.run_test` drives the sequence and awaits objection drain after `run_phase` (pyuvm: `_s13_uvm_component.py`, `uvm_root.run_test`).

**D5.3 — same nine phases, same traversal orders, trait dispatch instead of `getattr`.**

```rust
pub trait Phased {
    // topdown (pyuvm: _s09, 9.8.1)
    fn build_phase(&mut self, ctx: &mut PhaseCtx) {}
    fn end_of_elaboration_phase(&mut self, ctx: &mut PhaseCtx) {}
    fn start_of_simulation_phase(&mut self, ctx: &mut PhaseCtx) {}
    fn extract_phase(&mut self, ctx: &mut PhaseCtx) {}
    fn check_phase(&mut self, ctx: &mut PhaseCtx) {}
    fn report_phase(&mut self, ctx: &mut PhaseCtx) {}
    fn final_phase(&mut self, ctx: &mut PhaseCtx) {}
    // bottomup (pyuvm: _s09, uvm_connect_phase)
    fn connect_phase(&mut self, ctx: &mut PhaseCtx) {}
    // spawned as a task per component, bottomup order
    // (pyuvm: _s09, uvm_run_phase(uvm_threaded_execute_phase, uvm_bottomup_phase))
    fn run_phase(&mut self, ctx: PhaseCtx) -> Option<BoxFuture<'static, ()>> { None }
}
```

Default empty bodies replicate pyuvm's no-op base methods, so components override only what they use — the book's teaching pattern (book: uvm_component chapter). ⚠ `run_phase` returning an optional boxed future (rather than being an `async fn`) sidesteps async-fn-in-trait dyn-compatibility; whether this is the least-bad signature is OQ-3, shared with TLM.

Custom phases: pyuvm documents "add a method and insert into the phase list" as the extension path (pyuvm: `_s09` comment block). rustvm ports the same austerity: the phase list is a `Vec<PhaseDescriptor>` the runner iterates; inserting a custom phase means registering a descriptor with a traversal order and a dispatch closure. No schedules, no domains — same scope cut as pyuvm, same rationale.

**Objections.** pyuvm's `ObjectionHandler` is a singleton counting raised/dropped objections per component, with an `Event` signaled when the count hits zero, plus a warning if `run_phase` completes with nothing ever raised (pyuvm: `_utility_classes.py`, `ObjectionHandler.run_phase_complete`). Its diagnostics (raiser name, description, source line — the `Objection` dataclass) are worth keeping.

**D5.4 — objections become RAII guards.**

```rust
impl PhaseCtx {
    /// Port of raise_objection, returning a guard whose Drop is drop_objection.
    /// (pyuvm: _s13 uvm_component.raise_objection/drop_objection/objection())
    pub fn raise_objection(&self, description: &str) -> ObjectionGuard;
}
```

pyuvm already gestures at this with its `objection()` context manager (pyuvm: `_s13_uvm_component.py`, `uvm_component.objection`). Rust's version is strictly better: forgetting to drop is impossible (guard drops when it leaves scope), and the raiser/source-line diagnostics are captured in the guard constructor. The "you never objected" warning and the objection-report-on-timeout are ported as-is.

### 5.4 ConfigDB

pyuvm's `ConfigDB` is a two-level dict: glob-capable instance-path keys → field name → {precedence → value}, with precedence favoring shallower components during `build_phase` (depth-based), glob matching on *stored* paths only (wildcards illegal in `get` paths), most-specific-path-wins retrieval ordering, `wait_modified`, and tracing (pyuvm: `_s13_uvm_component.py`, `ConfigDB.set/get`, lines 663–805).

**D5.5 — port the semantics exactly; change only the typing and error story.**

```rust
pub struct ConfigDb { /* path dict, events, trace flag — elided */ }

impl ConfigDb {
    /// Port of ConfigDB.set incl. build-phase depth precedence
    /// (pyuvm: _s13, ConfigDB.set). Key rules identical: globs allowed
    /// in inst_path, not in field names.
    pub fn set<T: Any>(&mut self, ctx: Option<ComponentId>, inst_path: &str,
                       field: &str, value: T) -> Result<(), ConfigError>;

    /// Port of ConfigDB.get: same path-specificity ordering; returns
    /// typed value or a structured error instead of UVMConfigItemNotFound.
    pub fn get<T: Any + Clone>(&self, ctx: Option<ComponentId>, inst_path: &str,
                               field: &str) -> Result<T, ConfigError>;

    pub fn exists(&self, ctx: Option<ComponentId>, inst_path: &str, field: &str) -> bool;

    /// Port of ConfigDB.wait_modified (pyuvm: _s13, async wait on set()).
    pub async fn wait_modified(&self, ctx: Option<ComponentId>,
                               inst_path: &str, field: &str);
}
```

The `ConfigError` enum distinguishes *not found* from *found-but-wrong-type* — the latter is a bug class pyuvm can't even express (whatever you stored comes back; a type mismatch explodes later at the point of use). `cdb_set`/`cdb_get` convenience methods on `PhaseCtx` port the component-scoped sugar (pyuvm: `_s13`, `uvm_component.cdb_set/cdb_get`).

⚠ Values are stored as `Box<dyn Any>`; `get<T>` clones out. For non-`Clone` resources (a BFM), the stored type is `Rc<T>` and you get a cheap handle clone — matching how the book actually uses the ConfigDB (storing one shared BFM; book: ConfigDB chapters). Whether this convention is ergonomic enough, or typed keys (`ConfigKey<T>`) should replace stringly-typed fields entirely, is OQ-9. The stringly-typed design wins for now because the book's pedagogy depends on path/field strings and their debugging story ("Debugging the ConfigDB()" is a whole chapter).

### 5.5 Factory

What the factory does in pyuvm: every `uvm_void` subclass self-registers by name at class-creation time via metaclass (pyuvm: `_utility_classes.py`, `FactoryMeta.__init__`); overrides are stored per-original-type as a type override plus ordered instance overrides with glob paths (`Override` class); `find_override` chases override chains recursively with loop detection; `create_component_by_name/by_type` instantiate through the override resolution (pyuvm: `_s08_factory_classes.py`). The book teaches this as the mechanism enabling test-by-test behavior swapping (book: "The UVM factory" chapter).

**D5.6 — registration moves to the `#[derive(Component)]` macro; resolution algorithm ports unchanged.**

```rust
pub struct Factory { /* name → CreatorFn registry, overrides — elided */ }

/// What the derive macro registers, at link time.
pub struct ComponentRegistration {
    pub type_name: &'static str,
    pub type_id: TypeId,
    pub create: fn(ComponentCtx) -> Box<dyn Component>,
}

impl Factory {
    // Port of _s08 8.3.1.3 override setters, same glob path semantics
    pub fn set_type_override<Orig: Component, Over: Component>(&mut self, replace: bool);
    pub fn set_inst_override<Orig: Component, Over: Component>(&mut self, inst_path: &str);
    pub fn set_type_override_by_name(&mut self, orig: &str, over: &str, replace: bool)
        -> Result<(), FactoryError>;
    pub fn set_inst_override_by_name(&mut self, orig: &str, over: &str, inst_path: &str)
        -> Result<(), FactoryError>;

    /// Port of FactoryData.find_override: recursive chain-following with
    /// loop detection, instance overrides before type overrides
    /// (pyuvm: _utility_classes.py lines 114–181).
    pub fn resolve(&self, requested: TypeId, inst_path: Option<&str>) -> TypeId;

    pub fn create_component_by_name(&self, type_name: &str, parent_inst_path: &str,
        name: &str, hierarchy: &mut Hierarchy, parent: Option<ComponentId>)
        -> Result<ComponentId, FactoryError>;

    /// Debug printing at levels 0/1/2, port of uvm_factory.print
    /// (pyuvm: _s08, debug_level semantics).
    pub fn print(&self, debug_level: u8) -> String;
}
```

The mechanism replacing the metaclass: `#[derive(Component)]` emits a static `ComponentRegistration` into a linker-collected slice (the `inventory`/`linkme` technique previewed in §0.5). Before the runner starts, it folds all registrations into the `Factory`'s name map — the moral equivalent of pyuvm's `FactoryData().classes`, built at link time instead of import time. ⚠ Link-section collection is well-trodden but has known platform edges (LTO, some linkers, wasm); a fallback explicit-registration API (`factory.register::<MyComp>()`) is part of the design regardless. OQ-4.

Loop detection in `resolve` logs and returns the loop-forming type, exactly as pyuvm's error path does (pyuvm: `FactoryData.find_override`, `check_override`). Aliases (`set_type_alias`/`set_inst_alias`) are unimplemented in pyuvm — both raise `UVMNotImplemented`, noting the SystemVerilog UVM doesn't implement them either (pyuvm: `_s08_factory_classes.py`, lines 322–350) — so rustvm omits them knowingly (listed under [gap] in §8).

### 5.6 TLM and sequences

**TLM-1 ports/exports.** pyuvm ports the full IEEE taxonomy: blocking/nonblocking × put/get/peek/transport, master/slave composites, analysis ports, all as classes checking `connect()` compatibility at runtime by verifying the export implements the port's method set (pyuvm: `_s12_uvm_tlm_interfaces.py`, `uvm_port_base.check_export` mechanism, lines 60–160).

**D5.7 — ports are generic structs; compatibility is checked by the type system at `connect`.**

```rust
/// Blocking put. Port of uvm_blocking_put_port (pyuvm: _s12, 12.2.5).
pub struct BlockingPutPort<T>(/* connection slot — elided */);
impl<T> BlockingPutPort<T> {
    pub fn connect(&mut self, export: BlockingPutExport<T>);
    pub async fn put(&self, item: T) -> Result<(), TlmError>;
}

/// Nonblocking put. try_put/can_put (pyuvm: _s12, 12.2.5.2).
pub struct NonBlockingPutPort<T>(/* elided */);
impl<T> NonBlockingPutPort<T> {
    pub fn try_put(&self, item: T) -> Result<(), TlmFull<T>>;  // returns item on full
    pub fn can_put(&self) -> bool;
}

// get/peek/transport families follow the same pattern; composite ports
// (put_port = blocking + nonblocking) are structs exposing both method sets.

/// Analysis port: 1-to-many, never blocks, port of uvm_analysis_port.write
/// (pyuvm: _s12, 12.2.8). Subscribers receive &T; fan-out clones only
/// when a subscriber needs ownership.
pub struct AnalysisPort<T>(/* subscriber list — elided */);
impl<T> AnalysisPort<T> {
    pub fn connect_subscriber(&mut self, sub: ComponentId /* impl Subscriber<T> */);
    pub fn connect_fifo(&mut self, fifo: &AnalysisFifo<T>);
    pub fn write(&self, item: &T);
}
```

A `connect` type mismatch (put port → get export) is now a compile error, deleting pyuvm's runtime `check_export` and its `UVMTLMConnectionError`. `TlmFifo<T>` ports `uvm_tlm_fifo` (size-1 default, `size/used/is_empty/is_full/flush` — pyuvm: `_s12`, lines 849–908); `AnalysisFifo<T>` ports the unbounded analysis variant; `ReqRspChannel<REQ, RSP>` and the transport channel port the composite channels (pyuvm: `_s12`, lines 933–1040).

⚠ Blocking TLM methods are `async fn` on generic structs — fine as designed. If a *heterogeneous* collection of ports is ever needed (`Vec<Box<dyn AnyPort>>`), async-fn-in-trait dyn-compatibility bites. The design avoids needing it (ports are fields, not collection members), but this constraint must be validated against real testbench topologies. OQ-3.

**Sequences.** The pyuvm handshake, which the port preserves *event-for-event* because the book teaches its observable ordering (book: "Sequence testbench: 7.0"; pyuvm: `_s14_15_python_sequences.py`, header comment block — the file's own narrative of the protocol):

1. Sequence: `start_item(req)` → enqueue on sequencer's request path; block until this item's *start condition* fires (its turn arrives).
2. Driver: `get_next_item()` → dequeue; fire start condition; block on *item ready*.
3. Sequence: fills request fields; `finish_item(req)` → fire item-ready; block on *finish condition*.
4. Driver: processes the transaction against the DUT; `item_done(Some(rsp))` → fire finish condition; response (if any) into response queue.
5. Sequence (optionally): `get_response(txn_id)` → FIFO-or-by-id retrieval from `ResponseQueue` (pyuvm: `ResponseQueue.get_response`).

```rust
/// User-implemented sequence. Port of uvm_sequence (pyuvm: _s14_15).
pub trait Sequence<REQ: Transaction, RSP: Transaction = REQ> {
    fn body(&mut self, ctx: SeqCtx<REQ, RSP>) -> BoxFuture<'_, Result<(), SeqError>>;
    // pre_body/post_body default hooks, gated by start(call_pre_post) as in pyuvm
}

/// Handed to a running sequence; knows its sequencer. Port of the
/// sequence-side API (pyuvm: _s14_15 uvm_sequence.start_item/finish_item).
pub struct SeqCtx<REQ, RSP> { /* sequencer handle, running item id — elided */ }
impl<REQ: Transaction, RSP: Transaction> SeqCtx<REQ, RSP> {
    pub async fn start_item(&mut self, item: &mut REQ);
    pub async fn finish_item(&mut self, item: REQ);
    pub async fn get_response(&mut self, txn_id: Option<TxnId>) -> RSP;
}

/// Driver-side port. Port of uvm_seq_item_port (pyuvm: _s14_15).
pub struct SeqItemPort<REQ, RSP> { /* export link — elided */ }
impl<REQ: Transaction, RSP: Transaction> SeqItemPort<REQ, RSP> {
    /// Errors if called twice without item_done — same rule as pyuvm
    /// (pyuvm: uvm_seq_item_export.get_next_item raises UVMSequenceError).
    pub async fn get_next_item(&mut self) -> SeqItem<REQ>;
    pub fn item_done(&mut self, rsp: Option<RSP>);
    pub async fn get_response(&mut self, txn_id: Option<TxnId>) -> RSP;
}

/// The sequencer component. Port of uvm_sequencer (pyuvm: _s14_15):
/// a queue of sequences feeding one seq_item_export.
pub struct Sequencer<REQ, RSP = REQ> { /* seq queue, export — elided */ }
```

Notable typing win: pyuvm's start/finish conditions live *on the sequence item* as cocotb `Event`s (pyuvm: `uvm_sequence_item.__init__`), meaning any code holding the item can fire them. rustvm moves the handshake state into the sequencer's channel internals; items are plain data. Virtual sequences (no sequencer; coordinate sub-sequences) port as sequences whose `SeqCtx` has no item channel — calling `start_item` on a virtual context is a compile-time impossibility rather than pyuvm's runtime `UVMSequenceError` (pyuvm: `uvm_sequence.start_item` raise). The Fibonacci and `get_response` patterns from the book (book: testbenches 7.1, 7.2) were checked against these signatures on paper; both express directly. ⚠ Sequence arbitration beyond FIFO (grab/lock/priority) is absent in pyuvm and stays absent here — known gap, not open question.

---

## 6. Macro Strategy

Governing principle: **a macro is justified only where pyuvm/cocotb used runtime dynamism that Rust lacks.** Everywhere else, plain traits and generics. The dynamism inventory, from source:

| Python dynamism | Where used | rustvm macro answer |
|---|---|---|
| Decorator wrapping + global registration | `@cocotb.test()` (cocotb: `_decorators.py`) | `#[rustvm::test]` attribute macro |
| Runtime test generation | `TestFactory`, `@parametrize` (cocotb: `_test_factory.py`, `_decorators.py`) | `#[rustvm::parametrize]` compile-time expansion ⚠ OQ-10 |
| Metaclass class registration | `FactoryMeta` (pyuvm: `_utility_classes.py`) | `#[derive(Component)]` + link-time inventory |
| `__dict__` field walking | `do_copy`/`do_compare` defaults (pyuvm: `_s05`) | `#[derive(Transaction)]` field-wise codegen |
| `getattr` phase dispatch | `uvm_phase.execute` (pyuvm: `_s09`) | **no macro** — the `Phased` trait suffices |
| `__getattr__` DUT discovery | `HierarchyObject` (cocotb: `handle.py`) | **no macro required** — dynamic `child()` API; optional codegen is OQ-6 |

### 6.1 `#[rustvm::test]`

Accepts the cocotb `Test` option set as attribute arguments — `timeout_time`/`timeout_unit`, `expect_fail`, `expect_error`, `skip`, `stage`, `name` (cocotb: `_decorators.py`, `test()` signature) — and expands the annotated `async fn` into: the original function, plus a static `TestRegistration` (name, module path, `file!()`/`line!()` for the report table, options, and a monomorphized spawn shim). The runner's regression table then reproduces cocotb's per-test and summary output shape (cocotb: `regression.py`, `_log_test_summary`).

### 6.2 `#[rustvm::parametrize]`

cocotb's `@parametrize` produces one test per option combination with suffixed names (cocotb: `_decorators.py`, `parametrize`, `TestGenerator.generate_tests`). The macro form takes literal value lists and expands N registrations at compile time. ⚠ What is lost vs. Python: parameter sets computed at runtime (from env vars or files). Mitigation: runtime *filtering* stays (cocotb's `COCOTB_TEST_FILTER` equivalent; cocotb: `regression.py`, `add_filters`), and data-driven cases inside one test body remain ordinary Rust. OQ-10.

### 6.3 Derives

- **`#[derive(UvmObject)]`** — name/id storage + trait impl for a struct with a `uvm: UvmObjectFields` field (or macro-injected equivalent ⚠ minor design point: field injection vs. required field; leaning required-field for transparency).
- **`#[derive(Transaction)]`** — implies `UvmObject`; generates field-wise `do_copy`, `do_compare`, `convert_to_string` honoring `#[uvm(skip)]` / `#[uvm(compare = false)]` field attributes (covering the book's "exclude the timestamp from compare" pattern; book: uvm_object chapter exercises).
- **`#[derive(Component)]`** — implies `UvmObject`; emits the factory `ComponentRegistration` (§5.5) and the hierarchy plumbing glue for `init`.

### 6.4 What deliberately stays macro-free

Phasing, TLM connection, ConfigDB access, sequences — all plain code. Two reasons: macro-heavy APIs are miserable to debug for exactly the audience this project serves (error messages point into generated code), and every one of these has a clean trait/generic expression as §5 showed. The bar for adding a macro later: it must delete user-visible boilerplate *and* be explainable in one book paragraph.

---

## 7. Testing Conventions (Worked Example)

The worked example is the book's TinyALU (book: "Basic testbench: 1.0" — A/B 8-bit operands, `op` bus, `start`/`done` single-cycle handshake, ADD/AND/XOR one cycle, MUL three cycles). This section shows the *shape* of the rustvm testbench in signatures and prose; the implementation phase fills in bodies.

### 7.1 Project layout

```
tinyalu_tb/
├── Cargo.toml            # [lib] crate-type = ["cdylib"]; depends on rustvm
├── hdl/tinyalu.sv
├── src/
│   ├── lib.rs            # test entry, #[rustvm::test] functions
│   ├── alu_item.rs       # AluCommand / AluResult transactions
│   ├── alu_bfm.rs        # BFM: owns DUT handles, exposes async API
│   ├── components.rs     # Driver, Monitors, Scoreboard, Coverage
│   ├── env.rs            # AluEnv
│   └── sequences.rs      # AluSeq, per-op sequences
└── rustvm.toml           # simulator selection, sources, top module ⚠ OQ-2
```

### 7.2 The pieces, by signature

**Transactions** — one derive line replaces the `uvm_sequence_item` subclass with manual `__eq__`/`__str__` (book: sequence chapters):

```rust
#[derive(Transaction, Clone, Debug)]
pub struct AluCommand { pub a: u8, pub b: u8, pub op: Ops }

#[derive(Transaction, Clone, Debug)]
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
#[rustvm::test(timeout_time = 100, timeout_unit = "us")]
async fn random_ops(ctx: TestCtx) -> Result<(), TestError>;

#[rustvm::test]
async fn max_ops(ctx: TestCtx) -> Result<(), TestError>;
// body shape: build env via factory → run phases → sequence on sequencer,
// with a factory override turning random_ops into max_ops, per the book's
// factory-override teaching (book: "UVM factory testbench: 5.0").
```

### 7.3 Conventions (the "testing style guide" the book will teach)

1. **Failure taxonomy.** `Result::Err` for *checks* ("result mismatch"); `panic!`/`assert!` for *testbench bugs* ("driver called item_done twice"). Both fail the test; the report distinguishes them, following cocotb's exception-vs-SimFailure separation (cocotb: `regression.py`, `_score_test` error paths).
2. **Every `run_phase` that must hold the test open takes an objection guard** in its first statement — the book's rule, RAII-enforced (book: uvm_test chapters; pyuvm: `ObjectionHandler` warning path).
3. **Scoreboards check in `check_phase`, report in `report_phase`** — matching pyuvm traversal order guarantees (pyuvm: `_s09`, both are topdown, post-run).
4. **No sleeps for synchronization.** `Event`/queues, never `Timer`-and-hope — porting the book's NullTrigger lesson (book: Coroutines chapter; cocotb: `NullTrigger` docstring).
5. **Unit tests without a simulator.** Pure-Rust components (scoreboard predictors, transaction ops) get ordinary `#[test]` cargo tests — a genuinely new capability vs. the Python stack, worth a book chapter.

### 7.4 Runner flow

`cargo build` produces the cdylib → `rustvm run` (a small cargo subcommand in `rustvm-runner`) invokes the simulator with the GPI libs and env pointing at the testbench library, mirroring `cocotb_tools.runner`'s role (cocotb: `cocotb_tools/runner.py`). Test selection/filter env vars port from cocotb's (`COCOTB_TEST_FILTER` etc.; cocotb: `regression.py`). Results: console table + xUnit XML (cocotb: `_xunit_reporter.py`). ⚠ Full build-flow ergonomics (one command from zero to waveform) is OQ-2's second half.

---

## 8. Open Questions / Known Gaps

Every ⚠ from the body, consolidated. Items marked **[gap]** are consciously descoped, not unresolved.

| # | Question | Where raised | Current lean / needed to resolve |
|---|---|---|---|
| OQ-1 | Reuse cocotb's C++ GPI vs. eventual pure-Rust VPI/VHPI layer | §3.1 | Reuse now (D3.1). Revisit only after rustvm is functional on 2+ simulators. |
| OQ-2 | Embedding/bootstrap: exact entry symbols, init ordering, per-simulator loading; and the `rustvm run` build flow | §3.2, §2, §7.4 | Shape is clear; needs a spike against Icarus + Verilator + one commercial sim before freezing. Highest-risk item in the design. |
| OQ-3 | Async-fn-in-trait dyn-compatibility for `run_phase` and any heterogeneous port collections | §5.3, §5.6 | Lean: `BoxFuture` returns at the two trait boundaries; measure allocation cost under a busy sequencer before accepting. |
| OQ-4 | Link-time registration (inventory/linkme technique) reliability across platforms/linkers/LTO | §0.5, §5.5, §6.1 | Lean: use it, but ship the explicit `factory.register::<T>()` fallback API from day one. |
| OQ-5 | Drop-based cancellation loses "awaiting cleanup on kill" | §4.6, mapping row 6 | Lean: document the shutdown-message-then-join idiom as the convention; verify against the book's kill-using examples (e.g., Fibonacci 7.1 stopping patterns). |
| OQ-6 | DUT access: purely dynamic `dut.child("x")?` vs. build-time codegen of a typed DUT struct from HDL introspection | mapping row 19, §6 | Lean: dynamic first (teachable, no build magic); codegen as later ergonomics layer. Needs decision before Chapter-level API examples freeze. If a compile-time typed DUT layer is added later, it must be additive to the dynamic `dut.child()` API, not a breaking replacement — existing testbenches written against the dynamic API must continue to work unchanged. |
| OQ-7 | Blocking-world bridge (`bridge`/`resume`/`run_in_executor` equivalents) for file/network/co-simulation I/O | §3.4, mapping row 25 | Undesigned beyond "channel + dedicated OS thread." cocotb's `_bridge.py` state machine is the reference. Defer until a concrete use case (book doesn't teach it). |
| OQ-8 | Logging backend: `tracing` vs. `log`, and mapping onto `gpi_set_log_handler` + sim-time-stamped formatting | §3.1, mapping row 11 | Lean `tracing` (hierarchical targets match `cocotb.task.X` logger naming and pyuvm's per-component loggers). Must reproduce the book's `2.00ns INFO ...` output format. |
| OQ-9 | ConfigDB: `Box<dyn Any>` + `Rc` convention vs. typed keys | §5.4 | Lean stringly-typed for book compatibility; revisit if type-mismatch errors dominate early user feedback. |
| OQ-10 | Parametrized tests: compile-time expansion can't consume runtime parameter sets | §6.2, mapping row 26 | Accept the limit; document data-driven-loop idiom for runtime cases. |
| OQ-11 | Write-scheduling parity: cocotb's inertial-write buffering has per-simulator behavioral nuance | §4.1(4), mapping row 22 | Port the mechanism verbatim; needs cross-simulator regression tests to claim parity. |
| OQ-12 | `UvmRoot`/singleton elimination: runner-owned world vs. thread-local access for deep helper code | mapping row 30, §5.2 | Lean runner-owned + context parameters; prototype must confirm this doesn't make user code miserable (see OQ-14). |
| OQ-13 | Are any GPI calls actually thread-safe on specific simulators? | §3.4 | Assume none. Only relevant if OQ-7 wants shortcuts; don't take them. |
| OQ-14 | Ergonomics of arena/ID hierarchy: context-parameter plumbing vs. Python's `self.parent.thing` | §5.2 | The design's biggest *usability* risk. Resolve by writing the full TinyALU example against the draft API (paper-prototyped here; needs real code in implementation phase). |
| **[gap]** | pack/unpack, recording, policies, run-time phases/domains, sequence arbitration (grab/lock/priority), `uvm_resource_db` | §5.1, §5.3, §5.6 | Same cuts pyuvm made (pyuvm: `_s05` stubs, `_s09` header comment, `_s13` ConfigDB comment). Not planned. |
| **[gap]** | Register abstraction layer (pyuvm `_reg/`) | — | Out of scope for this design doc entirely; pyuvm's RAL port would be its own document. |

---

*End of design document. Companion deliverable: `book-outline.md`.*



