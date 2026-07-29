# Brief for the prose pass

You are writing *Rust for RTL Verification* — 41 chapters, an interlude, and
four appendices in `book-pdf/src/`. The framework it teaches (rustdv, in
`rustdv/`) is finished and every example runs. Your job is the prose.

Read this file once. Then read `chapter-notes.md` one row at a time.

---

## The absolute rule: you change no code

You edit **`book-pdf/src/*.md`**, `book-pdf/src/SUMMARY.md`, and the planning
files this brief asks you to create. Nothing else.

Off limits, no exceptions: `rustdv/`, `output/`, `sim/`, `skills/`,
`toolchain-drop/`, any `.rs`, `.toml`, `.json`, `.sh`, `.py`, or HDL file.

**When the code and the chapter disagree, the code wins.** The examples are
verified running; the manuscript is known stale. If an example looks wrong, or
a chapter needs a figure that does not exist — **stop and tell Ray.** Do not
edit the example to match your prose. That is the failure this whole project
exists to undo: prose driving design.

**Transcripts are copied, never composed.** Take them verbatim from the
chapter's `output/examples/*/README.md`. Do not retype, reformat, tidy, invent,
or regenerate them, and do not run a simulator. A transcript that looks wrong
is something you report.

---

## Your two phases

### Phase 0 — map the book, then ask for the numbers you want

Before writing prose, walk every chapter and every example crate and produce
**`book-pdf/figure-plan.md`**: the full inventory of code listings and the
caption scheme you want.

The problem you are solving: today `Figure N` means a code listing, so there is
no free word for an actual drawing — which is why the pipeline in ch31, the
architecture in ch34 and the sequencer handshake all exist as ASCII art inside
code comments. The likely answer is to split the two numbering spaces:
**Example N** for a code listing, **Figure N** for a drawing, each numbered
independently, so a chapter can say "Example 5 wires the FIFO; Figure 5 shows
the topology."

`figure-plan.md` must contain:

1. Every chapter, its code listings, and their current caption numbers.
2. Your recommendation on the Example/Figure split — including the honest option
   of leaving it alone.
3. Whether Part II+ wants drawings at all, and where.
4. **The exact renumbering you want**, as a list precise enough to execute:
   file, current caption, new caption.

Hand that to Ray. He will have the code captions renumbered for you. **You do
not perform the rename** — captions live in `.rs` comments. Wait for it to
land before writing any chapter whose numbers change.

### Phase 1 — write the chapters

Work in `SUMMARY.md` order. For each chapter: read its row in
`chapter-notes.md`, read the example crate it names and that crate's
`README.md`, then write.

---

## What the book argues

The reader is a verification engineer who knows the UVM — from SystemVerilog at
work, from pyuvm, or from an earlier book. Assume fluency in verification
concepts (driver, monitor, scoreboard, sequence, factory, config database,
analysis port). Assume **reading ability only** in Python and SystemVerilog, and
**no prior book**. The two earlier books are recommendations, never
prerequisites.

The honest frame for Rust, and the spine of the whole book:

> **Types for data and ownership. Runtime indirection for topology and
> binding.**

Transactions are plain structs with derives; ownership and lifetimes are checked
hard; and configuration, the factory and TLM connection are resolved at run
time — deliberately, because that is what late binding *is*. A type system can
only check what is known statically, and the entire purpose of those three
layers is to defer decisions so that one environment serves many tests.

So the book does **not** claim that Rust finds your UVM bugs at compile time.
Where a compile error is real, show it and let it speak. Where rustdv gives
something up, say so plainly — the project's credibility rests on that.

Where types did real work, they did it on **memory and ownership**, not on
binding: the `'static` bound on `spawn` refused an unsound concurrent design,
and `async fn` in a trait forced an architectural decision early. Those are the
wins worth naming.

Two other honest justifications for Rust, neither of them bug-finding: types
scale with codebase and team size, and monomorphization with no garbage
collector is the emulation argument — throughput, not correctness.

### Sameness is the goal, not superiority

A UVM engineer should get the tool they already know, spelled in Rust. Before
writing "rustdv's X is better than UVM's X," ask *better for whom*. Three places
it genuinely is, and none is compile-time bug finding:

- **Two analysis streams of the same type into one component** — two ports, two
  `WriteSink` impls. SystemVerilog needs the `uvm_analysis_imp_decl` macros;
  pyuvm cannot do it with one `write` per class.
- **Elaboration-time cardinality** — every unconnected port named at once,
  before any run phase. pyuvm discovers the first one lazily, at use.
- **Loud config failures** — SystemVerilog's `get()` collapses four distinct
  failures into a silent `return 0`; rustdv returns a `Result` naming the cause.

### Do not sell types, and do not disparage what came before

The previous draft of this book was obnoxious about compile-time type checking,
and that is the failure mode most likely to return, because it is the easiest
sentence in the world to write. The rule: **the reader already knows about types,
and has thought about the trade-off longer than the paragraph you are writing.**

- A SystemVerilog engineer has lived in a typed language their whole career.
  Telling them types catch mistakes early tells them nothing they have not known
  since their first compile.
- A Python engineer either moved away from types deliberately and can say why, or
  is actively agitating for them — hints, `mypy`, gradual typing. Either way, the
  question is not news to them.

So cut every sentence whose job is to be pleased about compile-time checking.
Where a compile error is real, show it and let it speak. The frame is stated once,
in ch1, and it is not "Rust catches your bugs" — it is the seam above.

**And do not disparage what came before.** Not Python, not SystemVerilog, not the
UVM, not pyuvm or cocotb, and not the two earlier books. Specifically:

- **The UVM's runtime indirection is not primitive.** A statically-typed language
  with the static option in hand chose it three times — `mailbox#(T)` and yet
  TLM, typed classes and yet a factory, parameters and yet a config DB. This
  project spent an entire branch proving that judgement was right.
- **pyuvm's lack of typing was a deliberate design decision** by the author of
  this book, and it *removed* a bug class that SystemVerilog's typed config DB
  still has.
- **SystemVerilog's awkward corners** — the `imp_decl` macros, `$cast`, the
  silent `return 0` — follow from its object and ownership model, not from
  anyone's failure of intelligence. Where rustdv differs, say what its signature
  *must* be, not what someone else got wrong.
- **The two earlier books are sources and recommendations**, not the thing this
  book improves on.

Comparisons are welcome — the book needs foils, and two sharp comparisons beat
none. What is banned is the scoreboard: any sentence whose real content is "and
that is why this is better." If a paragraph would leave a pyuvm user feeling their
tool is a toy, or a SystemVerilog engineer feeling patronised, cut it.

### Claims in the current manuscript that must not survive

These chapters argue *for* things the framework now does the opposite of. They
need re-argument, not sentence-level editing. Details are in `chapter-notes.md`.

- "Build/connect phases are unnecessary" — the gap between a component existing
  and its children existing is where every late-binding mechanism lives.
- "The ConfigDB problem, solved by types" — a path-addressed runtime ConfigDb
  exists.
- "Channels replace TLM-1" — TLM ports, exports and FIFOs are restored.
- "A TLM mis-connection is a compile error" — connection errors are
  **elaboration** errors, and that is correct.
- "The objection is decorative" — run phases are concurrent, so the objection
  is what ends the phase.
- "Phase-illegal operations are caught at compile time" — they are run-time
  failures, exactly as in the UVM. There is one context type, `RustdvCtx`.

---

## Voice

Warm, first-person, concrete, honest about costs. Jokes stay.

**The recap device** is a blockquote opening `> **In the UVM...**`, written in
UVM API terms that both dialects share (`start_item`, `get_next_item`,
`raise_objection`). Where the dialects differ, one parenthetical, SystemVerilog
first: *(SV: `uvm_config_db#(int)::set`; pyuvm: `ConfigDB().set`)*. Never two
parentheticals in one sentence — if they diverge that much, describe the
concept instead.

**Python and SystemVerilog references** are foils, not shared memory. "In
Python, a typo'd attribute is a runtime `AttributeError`" is fine. Anything
that assumes the reader has *lived* it is not. Prefer two sharp comparisons
over none — dual-audience means both foils, not no foil.

**Banned framings:** "the Python book taught you"; "as you learned in the Python
book"; "you remember" + any Python/pyuvm/cocotb referent; "the last book" /
"last time" meaning the Python book; "your Python testbench"; "the testbench you
wrote"; "In Python we"; second-person Python nostalgia; any sentence that needs
the reader to have run cocotb or pyuvm to parse it. Naming the books as
*sources* is fine ("pyuvm, which *Python for RTL Verification* teaches").

**SystemVerilog quotations** come only from
`../rustdv-reference/uvmprimer-master/`, run ≤15 lines, carry a source label,
never appear with simulation output, and total 5–8 across the whole book.

**Say the point plainly.** Avoid "honestly", "genuinely", "straightforward" —
they read as persuasion, not statement.

---

## Two writing debts to discharge

**1. The `prelude::*` problem.** Every Part II example opens with `use
rustdv::prelude::*`, importing about fifty identifiers. The reader then meets
`Clock`, `SimDuration`, `RustdvCtx`, `spawn_named` and the rest with no
declaration site on the page. Part I introduces every Rust concept before using
it; Part II abandons that discipline exactly where the reader needs it most.

Write a catalogue of what rustdv provides, placed before the first example that
uses it — rustdv's surface first appears in ch15, so it belongs at the opening
of Part II, either as its own chapter ahead of 15 or folded into ch17. For each
name: what it is, which layer it comes from, and when you would reach for it.

`ctx` needs its own section. rustdv has no globals — no `uvm_root`, no
singletons, no parent pointers — so everything the UVM reaches for ambiently
must be handed to the component. `ctx` *is* the framework, passed as a
parameter; the nearest UVM analogy is the `uvm_phase phase` argument every phase
method already receives. It is also how a component learns its own path.

Then add **Appendix D: What rustdv Provides**, in the format Appendices B and C
use — a reference table with a Chapter column pointing back to where each name
was taught. Include the macros, which currently have no home anywhere:
`#[rustdv::test]`, `#[derive(Component)]`, `first!`, `join!`,
`vpi_bootstrap!`. Add the line to `SUMMARY.md` after Appendix C.

**Standing rule from here on: no identifier appears in a listing before it has
been introduced.**

**2. The glob stays; the appendix carries it (Ray's call).** Every Part II
listing keeps `use rustdv::prelude::*` — no listing is rewritten to spell out its
imports. That means the catalogue section and Appendix D are the *only* things
standing between the reader and fifty undeclared names, so they have to be good.
Say plainly that the glob is the `from pyuvm import *` analog and what it brings
in.

---

## What is checked, and what is not

`python3 output/regression/regress.py` guards the repo, and you should not need
to run it — but you must know what it does and does not tell you:

- Its `book-sync` suite compares book listings against example files **for
  chapters 1–14 only.** For ch1–14, the code inside a listing is frozen: copy it
  verbatim, or the suite goes red.
- **No Part II+ listing has ever been compared against its code by anything.**
  The `sim-ch*` tests prove the example crates compile and run; nothing relates
  a manuscript listing to them. So a green suite is never evidence that a
  Part II+ chapter's listings — or its argument — are right.
- Log lines embed `file:line`, so a rename must happen *before* a transcript is
  regenerated. Another reason you copy transcripts rather than making them.

mdBook renders on Ray's machine, not in your sandbox.

---

## When you are unsure

Ask Ray. Do not invent an answer and write it as settled — that is precisely how
the original design failed. This applies to anything in `chapter-notes.md`
marked **ASK RAY**, and to any place a chapter needs a fact the examples do not
contain.

Reference material — cocotb, pyuvm, four releases of the SystemVerilog UVM, and
the example code from both earlier books — is outside the repo at
`../rustdv-reference`, read-only. Use it to check what the UVM actually does
rather than what a comment says it does. The Python book's chapter numbers map
onto this book's: its `27_uvm_test_testbench_3.0` is ch23.
