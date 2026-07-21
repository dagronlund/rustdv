# Notes for Fable — directives for the book pass

Running list of changes the manuscript needs. Written during the
build/connect refactor, to be applied when Fable does the prose pass
**after** the framework and examples are working.

---

## 1. Undeclared resources: the `prelude::*` problem

### The problem

Every Part II+ example opens with:

```rust
use rustdv::prelude::*;
```

That single line imports **about fifty identifiers**. The reader then meets
`Clock`, `SimDuration`, `RunCtx`, `TestCtx`, `spawn_named`, `log::info` and
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

- `TestCtx`, `TestError`, `#[rustdv::test]`, `vpi_bootstrap!`

**UVM layer** (`rustdv-uvm`)

- *Components and phases* — `Component`, `ComponentNode`,
  `#[derive(Component)]`, `RunCtx`, `ObjectionGuard`, `print_hierarchy`,
  `start_all`, `run_extract_check_report`, `CheckSink`
- *TLM* — `channel`, `Sender`, `Receiver`, `TlmFifo`, `AnalysisPort`,
  `AnalysisFifo`, `Subscriber`
- *Sequences* — `Sequence`, `Sequencer`, `SeqCtx`, `SeqItem`, `SeqItemPort`,
  `SeqError`, `TxnId`, `ResponseQueue`
- *Configuration* — `Active`

This list will grow with the refactor: `BuildCtx`, `ConnectCtx`, the port
and export types, the ConfigDB, and the factory all join it.

### The contexts need their own section

`ctx` is the least self-explanatory thing in the book and is currently never
explained. It deserves real treatment, because it is load-bearing:

- rustdv has no globals — no `uvm_root`, no singletons, no parent pointers.
  Everything UVM reaches for ambiently (`uvm_config_db::get`, the factory,
  the report server) must instead be **handed to the component**. `ctx` is
  that delivery mechanism: *the framework, passed as a parameter*.
- The closest UVM analogy is the `uvm_phase phase` argument every phase
  method already receives. rustdv's context is that same parameter carrying
  more, because there is nothing global to fall back on.
- There is one context type per phase — `BuildCtx`, `ConnectCtx`, `RunCtx` —
  because each phase permits different operations. `build_child` does not
  exist on `RunCtx`, so adding a component during the run phase *fails to
  compile*. Compare UVM, which catches the analogous mistake at run time:
  "Attempt to connect … at or after end_of_elaboration phase. Ignoring."
- Because components have no parent pointer, `ctx` is also how a component
  learns its own hierarchical path.

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
| `BuildCtx` | build-phase context: path, DUT, RNG, `build_child`, config, factory | uvm | 24 |
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
use rustdv::{Clock, RunCtx, SimDuration, TestError};
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
transcript showing a log message contains the source filename — e.g.
Chapter 23 Figure 2 currently reads `[…/src/lib.rs:18]`. All renames must
land *before* transcripts are regenerated, or every transcript gets touched
twice.

---

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
