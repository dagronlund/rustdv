# Notes for Fable — directives for the book pass

> **Read `book-pdf/fable-brief.md` first.** It is the shape of the job — reading
> order, the rules that matter, the overclaims to hunt down, the verification
> mechanics. *This* file is the running list of specific directives that brief
> refers to.
>
> **⛔ Fable changes no code — ever.** Only `book-pdf/src/*.md`. Not `rustdv/`,
> not `output/examples/`, not `output/regression/`. When the code and a chapter
> disagree, **the code wins** — the examples are verified running, the manuscript
> is the stale thing. If a figure looks wrong or is missing, stop and report it
> to Ray; do not edit the example to match the prose. Transcripts are copied
> verbatim from the chapter READMEs, never composed or regenerated.

Running list of changes the manuscript needs. Written during the
build/connect refactor, to be applied when Fable does the prose pass
**after** the framework and examples are working.

> **When (D77, 2026-07-23):** the manuscript is not touched until *all* the
> code is finished — the whole restoration through TB 6.0–8.0 green — and then
> a dedicated Fable thread does the entire prose pass in one go. Nothing here
> is interleaved with the code work; this file is the backlog that thread will
> work from, alongside the "prose still owed" notes in
> `output/.design-decisions.md` and each chapter's example README (the
> working code and its real transcripts are the source of truth). Until then
> the manuscript is expected to be stale — that is the plan.

---

## 1. Undeclared resources: the `prelude::*` problem

### The problem

Every Part II+ example opens with:

```rust
use rustdv::prelude::*;
```

That single line imports **about fifty identifiers**. The reader then meets
`Clock`, `SimDuration`, `RustdvCtx`, `spawn_named`, `log::info` and
the rest with no declaration site on the page and no prior introduction. A
reader cannot tell what is standard Rust, what is rustdv, and what is the
testbench's own code — which is a serious problem for an audience learning
both the language and the framework at once.

Part I is careful about this: every Rust concept is introduced before it is
used. Part II abandons that discipline at exactly the moment the reader is
most dependent on it.

### What to write

**A new chapter (or a substantial opening section) cataloguing what rustdv
provides, placed before the first example that uses it.** rustdv's surface
first appears in Chapter 15 (`Hello world as a test`), so the catalogue
belongs at the opening of Part II — either as its own chapter ahead of 15,
or folded into Chapter 17 with 15's figures deferring to it.

It does not need to document every method. It needs to answer, for each
name the reader will meet: *what is this, which layer does it come from,
and when would I reach for it?*

### The inventory to cover

Grouped as the chapter should probably group it. Taken from
`rustdv/src/lib.rs`.

**Simulator core** (`rustdv-sim`)

- *Time and scheduling* — `SimDuration`, `Timer`, `Clock`, `sim_time_ns`,
  `next_time_step`, `read_only`, `read_write`, `NullTrigger`
- *Tasks* — `spawn`, `spawn_named`, `TaskHandle`, `with_timeout`, `first2`,
  `join2`, `Either`, and the `first!` / `join!` macros
- *DUT access* — `HierarchyHandle`, `LogicHandle`, `AnyHandle`, `Logic`,
  `LogicArray`, `HandleError`
- *Synchronization* — `Event`, `Lock`, `Queue`, `Rng`
- *Logging* — `log`

**Runner** (`rustdv-runner`)

- `#[rustdv::test]`, `vpi_bootstrap!`
- *(`TestCtx` and `TestError` moved to `rustdv-uvm` in step 4 — see below.
  Both are still reached as `rustdv::…`, so the reader never sees the move;
  it matters only if the chapter names the crate a thing comes from.)*

**UVM layer** (`rustdv-uvm`)

- *Components and phases* — `Component`, `ComponentNode`,
  `#[derive(Component)]`, `RustdvCtx`, `TestError`, `ObjectionGuard`,
  `print_hierarchy`, `start_all`, `run_extract_check_report`, `CheckSink`
- *TLM* — `channel`, `Sender`, `Receiver`, `TlmFifo`, `AnalysisPort`,
  `AnalysisFifo`, `Subscriber`
- *Sequences* — `Sequence`, `Sequencer`, `SeqCtx`, `SeqItem`, `SeqItemPort`,
  `SeqError`, `TxnId`, `ResponseQueue`
- *Configuration* — `Active`

This list will grow with the refactor: the port and export types, the
ConfigDb, and the factory all join it.

### The context needs its own section

`ctx` is the least self-explanatory thing in the book and is currently never
explained. It deserves real treatment, because it is load-bearing:

- rustdv has no globals — no `uvm_root`, no singletons, no parent pointers.
  Everything UVM reaches for ambiently (`uvm_config_db::get`, the factory,
  the report server) must instead be **handed to the component**. `ctx` is
  that delivery mechanism: *the framework, passed as a parameter*.
- The closest UVM analogy is the `uvm_phase phase` argument every phase
  method already receives. rustdv's context is that same parameter carrying
  more, because there is nothing global to fall back on.
- Because components have no parent pointer, `ctx` is also how a component
  learns its own hierarchical path.

**There is one context type, `RustdvCtx` — this reverses what this section
said before 2026-07-21, and the reversal must not be quietly undone.**

The earlier plan was one type per phase (`BuildCtx`, `ConnectCtx`,
`RunCtx`), so that `build_child` would not exist on the run context and
adding a component during run would *fail to compile* — a genuine advantage
over UVM, which catches the analogue at run time ("Attempt to connect … at
or after end_of_elaboration phase. Ignoring."). That is decision D8 in
`output/.design-decisions.md`, and D47 strikes it.

The reason is the reader, not the compiler. Part II teaches a complete
testbench with **no components and therefore no phases**; handing that
reader a type named after a phase names a concept the book has not
introduced, and the alternative naming (`BuildCtx`/`ConnectCtx`/`RustdvCtx`)
is an incoherent family.

So the book must **state the cost, not hide it**: phase-illegal operations
in rustdv are run-time failures, exactly as in UVM. Chapter 24 was going to
claim the compile-time check as a Rust win. It cannot. Do not write that
claim, and do not let the surrounding argument imply it.

### One more Rust cost to state plainly (D48)

`Component::run` is an `async fn` in a trait, which in Rust means the trait
is not object-safe. The framework works around it with a dyn-safe mirror
trait (`DynPhases`) that users never write. It is invisible in every figure,
so it does not need a chapter — but if a chapter claims traits and `dyn`
compose freely, that claim is wrong, and the honest version is a good
sidebar: this is a real edge Rust has not finished yet.

### Standing rule from here on

**No identifier appears in a figure before it has been introduced.** If a
figure needs `Clock`, either `Clock` was explained earlier or the surrounding
prose explains it there.

### And an appendix — the other half of the job

The chapter teaches these names *in order*, once. A reader who meets
`with_timeout` in Chapter 34 and cannot remember what it is needs somewhere
to look it up, not a chapter to re-read.

**Add "Appendix D: What rustdv Provides"**, in the format Appendices B and C
already use — a reference table with a **Chapter** column pointing back to
where the name was taught:

| Name | What it is | From | Chapter |
|---|---|---|---|
| `Clock` | drives a signal at a fixed period; `.start()` spawns it | sim | 17 |
| `SimDuration` | simulation time span — `SimDuration::ns(10)` | sim | 15 |
| `RustdvCtx` | the one context: path, DUT, RNG, objections, logging, and later `build_child`, config, factory | uvm | 15 |
| `with_timeout` | run a future, fail it after a deadline | sim | 16 |

Group it the same way as the chapter, so the two mirror each other, and add
the entry to `SUMMARY.md` after Appendix C.

Include the macros, which are the easiest things to forget and currently have
no home anywhere: `#[rustdv::test]`, `#[derive(Component)]`, `first!`,
`join!`, `vpi_bootstrap!`.

**Keep it honest mechanically.** The prelude lives in `rustdv/src/lib.rs` and
is about to grow a lot during this refactor. Add a `book-sync` check that
every name exported from `rustdv::prelude` appears in Appendix D, and fails
the regression when one doesn't. Otherwise the appendix rots the first time
someone adds an export — which is exactly the drift this project's
book-sync suite exists to prevent.

### Import policy — recommendation, for Ray to confirm

Early Part II figures should use **explicit imports** so the reader sees
provenance:

```rust
use rustdv::{Clock, RustdvCtx, SimDuration, TestError};
```

Then, once the surface has been taught, switch to `use rustdv::prelude::*`
and say plainly that this is the `from pyuvm import *` analog and what it
brings in. Teaching the glob before the names is what created the problem.

---

## 2. Chapter crate files are named, not `lib.rs`

With ~25 chapter crates, a tree of identical `lib.rs` files is unsearchable
by `ls` or `grep` and unreadable in an editor's tab bar. Rust hit this same
wall and moved off `mod.rs` in the 2018 edition for exactly this reason.

Each Part II+ chapter crate now names its root after the crate:

```toml
[lib]
crate-type = ["cdylib"]
path = "src/ch23_uvm_test_testbench_3_0.rs"
```

Applied to ch23; the rest follow as each chapter is converted. This also
matches Part I, whose per-figure files are already descriptively named
(`ch12_fig09_tinyalu_operand_pair_stream.rs`).

**Do not split Part II chapters by figure.** Part I figures are standalone
programs, so one file each is right. Part II figures compose a single
testbench per chapter, so the crate is one coherent program and splitting it
would obscure that.

**Ordering constraint:** rustdv log lines embed `file:line`, so every book
transcript showing a log message contains the source filename. All renames
must land *before* transcripts are regenerated, or every transcript gets
touched twice.

Chapter 23 is done on both counts: the crate root is
`src/ch23_uvm_test_testbench_3_0.rs`, and the working transcript in the
chapter's `README.md` is real Icarus output at those paths. Take the
transcript from the README, not from the current manuscript.

---

## 2.5 REQUIRED NEW EXAMPLE — the y = 2x² pipeline (Ray, 2026-07-24)

**Ray's directive: this must become an example in the book.** It is in the
ch31 aspirational file (`output/examples/ch31-component-communications/`,
Figures 9–12) as `SquareIt`, `TimesTwo`, and `MathTest`. Do not drop it when
writing the chapter; it earns its place three times over.

```
MathTest(run) --put--> [x_fifo] --get--> SquareIt(run) --put--> [sq_fifo]
                                                                    |
MathTest(run) <--get-- [y_fifo] <--put-- TimesTwo(run) <--get-------+
```

The test picks x = 1..4, sends it into the pipeline, waits for y to come back,
and compares against 2x² (2, 8, 18, 32). Two worker components do the
arithmetic; each is connected only to FIFOs and neither knows the other exists.

What it teaches, and why no other figure covers it:

- **The parent is a stage, not just a builder.** Every other component figure
  has a test that only builds and connects. Here the test has a `run` phase of
  its own, and that run must be concurrent with its children's — the case that
  drove the whole concurrency design (D82/D82a). A phaser that runs children to
  completion first cannot execute this at all.
- **A request/response round trip**, not a one-way stream: put x, await y. It
  is the first figure with a closed loop through the component tree.
- **A parent holding its own ports** — `connect((self, MathTest::X_OUT))`.
- **Responders that never return.** The workers `loop` forever; the phase ends
  when the test's objection drops, which is what makes objections load-bearing
  (D60 discharged). Worth stating in the prose.
- **It is self-checking** — a broken pipeline fails rather than passing quietly.

It is also the small, DUT-free rehearsal for TB 7.0, where the test's `run`
starts a sequence while the driver waits for items forever. If the chapter
prose explains this figure well, ch36 gets much easier.

## 2.6 The TLM chapters (ch31, ch32, ch34) — write from these decisions

Settled with Ray 2026-07-24. **The aspirational example files are the spec**;
the decisions below say *why*, which is what the prose has to carry. Read
D83–D88 in `output/.design-decisions.md` before writing a word of these
chapters, and D82/D82b for the concurrency they depend on.

The inherited ch31/ch32/ch34 chapters argue the **destroyed** design
("channels replace TLM-1"). They are rewritten from scratch, not edited. The
old `compile-fail/fig08_direction_mismatch` figure has already been deleted: it
staged a `Sender`/`Receiver` type clash and called it catching a connection
error, which is the §0.4 overclaim in TLM form. **Do not reintroduce any claim
that TLM connection errors are compile errors** — they are elaboration errors,
by design (D22, D85).

**The one connection idiom, taught once and reused everywhere.** A concrete
FIFO sits between two components; the FIFO drives the connect; the endpoint is
named by a generated constant:

```rust
self.cmd_fifo.put_export().connect(&self.tester, Tester::CMD_PORT);
```

Explain *why* it is not `tester.cmd_port.connect(...)`: the child is an erased
`RustdvComp`, Rust has no `$cast`-to-base, and reaching into a child is the
anti-pattern UVM's own guidelines warn against. Ports register themselves by
path; connection is a lookup (D83). The FIFO being a concrete child is a
deliberate carve-out from "every child is `RustdvComp`" (D84).

**Two different mechanisms — do not blur them** (D86). `TlmFifo` queues data for
a consumer to pull (blocking, consumer-paced, one taker per item). `AnalysisFifo`
stores nothing: `write()` calls every subscriber immediately and returns
(publisher-paced, zero time, everyone sees everything). Ray corrected a proposed
merge of the two; the distinction is the chapter's spine, not a footnote.

**The zero-time requirement is the reason for the whole subscriber design**
(D87). A monitor must write and forget: the handlers run and control returns
without simulation time advancing. That is why a subscriber shares its *state*
(`RustdvShared<T>`) rather than being reached as a component — the publisher
holds `&mut` to itself and can never be handed `&mut` to a sibling. Teach the
sequence: state struct → `impl WriteSink` → `RustdvShared` handle → `on_write`.
A "queue it and deliver later" design is **wrong** and worth one sentence saying
so, because it is the obvious first idea.

**Say plainly what is better and what is different.** Better: two streams of the
*same* type into one component need two ports and two sinks — no
`uvm_analysis_imp_decl` macros, which is a real SV pain point, and something
pyuvm cannot do at all with one `write` per class. Different: rustdv puts a hub
in the analysis path where UVM has none, and the subscriber implements a trait
method rather than subclassing `uvm_subscriber`. Do not sell the differences as
wins beyond what they are (D74's rule).

**Chapter 31 must include the y = 2x² pipeline** — see §2.5 above, which is a
standing Ray directive. It is where the parent's own `run` runs concurrently
with its children's, and it rehearses TB 7.0.

**Naming to keep consistent:** `RustdvComp`, `RustdvCtx`, `RustdvShared` — the
`Rustdv` prefix marks the types a user holds in their own struct, so a reader
who goes looking for them knows they are ours, not std (D88). No `uvm` in any
symbol (D79). Never `r#in` or other raw identifiers in a figure — `input`,
`source`, and so on read fine.

## 3. Other pending directives (stubs — expand as the refactor lands)

Recorded so they survive between sessions. Each needs its own section before
the prose pass begins.

- **Build and connect phases are being restored.** Chapters 23, 24, 25, 27,
  29, 31, 32 and 34 currently *argue for* their removal. Those arguments are
  wrong and need re-writing, not editing.
- **Chapter 27** is titled "Configuration: The ConfigDB Problem, Solved by
  Types." A path-addressed, runtime ConfigDB is being added. The chapter
  needs re-argument from scratch.
- **Chapter 27** claims "Nobody ever used `wait_modified` in anger, and
  neither will we." Both UVM 1800.2-2020 and pyuvm implement it. Cut the line.
- **Chapter 29's ledger** lists `create() + type override` as ported to a
  maker closure. Type override is *not* ported. Split into two rows.
- **Chapter 24** says "one-pass construction has no gap for them to fill."
  The gap was the feature. Rewrite.
- **Phase directions**: rustdv follows pyuvm's traversal order, which differs
  from SV UVM for `end_of_elaboration`, `start_of_simulation`, `extract`,
  `check` and `report`. Whichever we adopt, state the divergence explicitly.

---

## 4. Directives from step 4 (2026-07-21) — ch23 now works

The framework increment behind Chapter 23 has landed and the chapter's
example compiles and passes on Icarus. These follow from it.

- **Two front doors, both first-class (D46).** `#[rustdv::test]` annotates
  *either* a free `async fn` *or* a struct. This is not a convenience: it is
  the two source books made concrete — `@cocotb.test()` decorates a
  coroutine function, `@pyuvm.test()` decorates a class. Part II's
  function-shaped testbench is a legitimate destination, not a stepping
  stone toward components (D38). Chapter 23 introduces the struct form
  *without* implying the function form was training wheels.

- **Chapter 23's real shape (D43).** At 3.0 the test is the only component.
  The BFM and scoreboard are ordinary locals inside `run`; the tester is a
  plain value, not a component (D44). Do not let the chapter introduce a
  component tree three chapters early — that is ch24's job.

- **Classes are re-shown, not imported (D45).** Chapter 23's file repeats
  `Tester`, `RandomTester`, `MaxTester` and `Scoreboard` under "Copied from
  testbench 2.0", as the Python book does. The prose should say why: these
  classes *evolve* — `Tester` is a plain trait at 3.0 and a component at
  4.0 — and re-showing them is how the reader sees the change. Factoring
  them into a shared crate hid the very thing the book is teaching.

- **Component paths are named after your test (D49).** Logs read
  `[HelloWorldTest]`, where UVM always says `uvm_test_top`. Worth one
  sentence as a divergence, and it is the first place `ctx.info()` earns
  its keep over bare `log::info` — the path came from the framework, so it
  cannot silently lie the way a hand-typed `Logger::new("env.loga")` does.

- **There is no software clock (D42).** The RTL self-clocks and the BFM only
  waits on edges. `Clock` is still taught once, as a cocotb feature, but
  chapters must stop opening with `Clock::new(...)`. The reason is
  emulation: a BFM that waits on edges ports to a transactor unchanged;
  one that drives them does not.

- **Open, do not guess: test naming (Q15).** Struct tests currently register
  under the *type* name (`RandomTest`), so that is what the transcripts say.
  pyuvm agrees; the SV Primer's snake_case `random_test` does not. Ray has
  not decided. If it changes, it changes before transcripts are regenerated.
