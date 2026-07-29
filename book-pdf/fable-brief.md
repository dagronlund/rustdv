# Brief for the Fable prose pass — start here

*Written 2026-07-24 by the code-side thread, for whoever writes Fable's prompt
and for Fable itself. `notes-for-fable.md` is the running list of directives;
this file is the shape of the job.*

---

## ⛔ THE ABSOLUTE RULE: Fable changes no code. None.

**Fable writes prose. Fable does not touch code — ever, for any reason.**

Off limits, without exception:

- `rustdv/` — the framework, every crate
- `output/examples/` — every example crate, every `.rs` file, every `Cargo.toml`
- `output/regression/` — `regress.py`, `regress.json`, any test
- `sim/`, `skills/`, `toolchain-drop/`, HDL, build scripts

Fable edits **only** `book-pdf/src/*.md` (and `SUMMARY.md`), plus its own notes.

**When the code and the chapter disagree, the code wins.** The examples are the
specification (D2) and they are *verified running*; the manuscript is the thing
known to be stale. If a chapter needs a figure the code doesn't have, or a figure
reads badly, or an example looks wrong — **stop and report it to Ray.** Do not
"just fix" the example to match the prose. That inverts D1 and reintroduces
exactly the failure this whole branch exists to undo: prose driving design.

Why this is absolute, not cautious: every example is regression-verified and
every transcript is real output. A prose-side edit to code silently breaks that
chain — the suite may still pass while the figure no longer means what it
demonstrated. And copying a transcript from a rerun Fable performed after editing
code would put fabricated evidence in the book.

**Transcripts are copied, never composed.** Take them verbatim from the chapter's
`output/examples/*/README.md`. Do not retype, reformat, "clean up," or invent
output — and do not run simulations to produce new ones. If a transcript looks
wrong or is missing, report it.

---

## What the job is

The framework was rebuilt (the UVM restoration). The manuscript was written
against the **old** framework — the one whose design was wrong — so Part II
onward must be rewritten from the working code. This is not an editing pass.
Several chapters argue *for* the very thing that was reversed; those arguments
are wrong at the premise and cannot be patched sentence by sentence.

**Do not start until the code is done.** D77: the manuscript is not touched
until the whole restoration compiles and runs green, and then one dedicated
thread does the entire prose pass. Check `output/.design-decisions.md` §0.5 for
live status before beginning.

---

## Read in this order

1. **`output/.design-decisions.md` §0–0.5** — the mission, the method, and the
   typing thesis (§0.4). This is the argument the book is *about*. A prose pass
   that hasn't internalized §0.4 will re-introduce the overclaims.
2. **This file**, then **`notes-for-fable.md`** — the accumulated directives,
   including two standing Ray requirements (§2.5 the y = 2x² example must be in
   the book; §2.6 the TLM chapters).
3. **`book-pdf/dual-audience-style.md`** — voice rules. The book addresses
   *both* UVM audiences (SystemVerilog and Python); recap blockquotes open
   "**In the UVM…**".
4. **The decision log's per-chapter sections (§11–§24)** — each records what
   landed, what was rejected, and why. Prose that contradicts a decision is a
   bug; prose that ignores the *reasoning* is a missed opportunity.
5. **The example crates** — `output/examples/chNN-*/src/*.rs` and their
   `README.md` figure maps. **These are the source of truth**, not the current
   manuscript.

---

## The three rules that matter most

**1. The code is the spec; the prose follows it (D1/D2).** The original failure
was settling design questions in an essay with no prototype. Every figure in the
book must be code that compiles and runs, and every transcript must be real
output from a real run. If a chapter wants to say something the code doesn't do,
the chapter is wrong.

**2. Sameness is the goal, not superiority (D74).** Restoring the UVM means a
UVM engineer gets the tool they already know, spelled in Rust. When tempted to
write "rustdv's X is better than UVM's X," check: better *for whom*, and on
which of §0.4's axes? The factory arc (D69–D75) is the cautionary tale — four
times the "we're better here" instinct fired and was wrong each time.

**3. State costs, don't hide them (D4).** Where rustdv gives something up
(chained factory overrides, SV's type-namespaced config DB, compile-time phase
checking), say so plainly. The project's credibility rests on this.

---

## The overclaim to hunt down and kill

The old framework deleted late binding, which made everything statically
decidable, which was then sold as "Rust finds your bugs at compile time." That
claim is an artifact of the deletion, not a fact about Rust (§0.4).

Chapters currently carrying reversed arguments (measured, 2026-07-24):
`chapter-24-components`, `chapter-27-configuration`, `chapter-28-config-debugging`,
`chapter-31-component-communications`, `chapter-34-connections-testbench-6.0`,
`chapter-36-sequence-testbench-7.0`, `chapter-39-virtual-sequence-testbench-8.0`.

Specific claims that must not survive:

- "Build/connect phases are unnecessary" (ch24) — the gap *was* the feature (D5).
- "The ConfigDB problem, solved by types" (ch27) — a path-addressed runtime
  ConfigDb exists (D65–D68). ch28's four compile-fail figures were deleted;
  D68 explains why they proved nothing.
- "Channels replace TLM-1" (ch31) — TLM ports/exports/FIFOs are restored
  (D83–D88). Its `fig08_direction_mismatch` compile-fail figure is already
  deleted; do not resurrect the idea.
- "A TLM mis-connection is a compile error" — connection errors are
  **elaboration** errors, and that is correct (D22, D85).
- Any claim that the objection is decorative — with concurrent runs it is
  load-bearing (D60 discharged by D82).

**The honest frame the book should use instead:** types for data and ownership;
runtime indirection for topology and binding (§0.4). Late binding costs
compile-time checking and buys a debugger's toolkit — that trade is the story.

---

## What is genuinely better, and how to say it

Three places rustdv wins, none of them "compile-time bug finding":

- **Multiple analysis inputs.** Two streams of the *same* type into one
  component: two ports, two `WriteSink`s. SV needs `uvm_analysis_imp_decl`
  macros; pyuvm can't do it with one `write` per class (D88).
- **Elaboration-time cardinality.** Every unconnected port reported at once,
  before any run phase. pyuvm discovers it lazily at first use (D22/D85).
- **Loud config failures.** SV's `get()` collapses four distinct failures into a
  silent `return 0`; rustdv returns a `Result` naming the cause (D14).

- **A failed non-blocking put hands the transaction back** (D89). This one is
  small enough to miss and important enough that ch31 was rewritten for it, so
  the prose must carry it. The pattern the reader must see, from ch31 Figures
  4–6:

  ```rust
  let mut packet = Packet::new(n);
  while let Err(back) = self.put_port.try_put(packet) {
      ctx.info("FIFO full, retrying");
      Timer::ns(1).await;
      packet = back; // the FIFO gave it back; try again with it
  }
  ```

  How to explain it: SV's `try_put` returns a **bit** because it passes a class
  handle and the caller still holds its own. rustdv's `try_put` takes the packet
  **by value** — it must, since a successful put hands the packet to whoever
  gets it next — so a bare "no" would have swallowed a packet that was never
  delivered. `Err(back)` is the packet coming home. Two things to state plainly:

  1. This is *not* Rust catching a bug SV has. SV has no bug here; it has a
     different ownership model. rustdv returns the item because it had to take
     it. Frame it as **what the signature must be**, not as a win.
  2. Written the tempting way — `while port.try_put(packet).is_err()` — it does
     not compile: `packet` was moved on the first attempt. The compiler error is
     real and was run; quote it if it helps, do not invent one.

  Do **not** demonstrate this with a `u32`. A `Copy` item makes the tempting
  loop compile and teaches a pattern that breaks on the reader's first real
  transaction. Ch31 carries a non-`Copy` `Packet` for exactly this reason; leave
  it that way. `try_get`'s `Option<T>` is the same argument in the other
  direction and should be explained alongside it.

- **The subscriber owns the storage** (D90). This is a **new pattern the book
  must teach**, not a footnote — it is where a UVM engineer's habit will
  mislead them.

  An `AnalysisBus` is not a FIFO. It holds nothing: `write` calls every
  subscribed object and returns, and a datum broadcast to nobody is gone. So
  the question "where does the traffic go?" has a different answer than in the
  UVM: **wherever the subscriber decides to put it.** A tally (ch32 Figure 1), a
  `Vec` (ch32 Figure 2), a comparison against a prediction (ch34's scoreboard) —
  the shape is the subscriber's choice, and if it wants a queue it declares a
  `TlmFifo` of its own.

  Say explicitly what this replaces. A UVM scoreboard routes each stream into a
  `uvm_tlm_analysis_fifo` because a class gets **one** `write` method: a second
  stream needs the `uvm_analysis_imp_decl` macros to mint a differently-named
  one, and a FIFO per stream is the way around that. A rustdv subscriber
  declares two `SubscribePort`s and two `WriteSink` impls, so the workaround has
  nothing to work around, and the FIFO that used to sit in the scoreboard is
  simply absent. A reader who does not see this said plainly will go looking for
  the analysis FIFO and conclude something is missing.

  The one reason a rustdv subscriber *would* own a queue is different from the
  UVM's, and **ch32 Figures 6–7 exist to teach it**: `write` is synchronous and
  cannot await, so a subscriber whose work *takes simulation time* splits the
  job. `write` does the one instant thing — `try_put` into an unbounded
  `TlmFifo` it owns — and the component's `run` gets from that FIFO and takes as
  long as it likes. `SlowChecker` is the component; `SlowSubscriberTest` is the
  proof.

  Three things to draw out of those figures:

  1. **The FIFO is connected to no port.** It is an ordinary handoff *inside*
     one component, between a synchronous method and an asynchronous one — not
     part of the testbench topology. A reader who has just learned `connect`
     will expect otherwise.
  2. **The inbox must be unbounded.** `write` cannot wait for space and analysis
     has no back-pressure, so a bounded inbox could only drop items. Say why,
     not just what.
  3. **The transcript is the argument.** All three writes land at `0.00ns`; the
     checks come out at 5, 10 and 15ns. The publisher is never held up by what a
     subscriber does with an item. Use the real transcript from the ch32 README.

Two justifications for Rust that survive §0.4 and should not be confused with
bug-finding: types scale with codebase and team size, and monomorphization with
no GC is the *emulation* argument (D36) — throughput, not correctness.

---

## Mechanics

- **Figure conventions:** `// Figure N:` captions; output after `--`; each
  chapter's `README.md` in `output/examples/` maps every figure to runnable code.
- **Transcripts are real.** Take them from the chapter README, not the current
  manuscript. Log lines embed `file:line`, so **renames must precede transcript
  regeneration** (D31).
- **Verification:** `python3 output/regression/regress.py` — the `book-sync`
  suite checks that book figures match the example files. It is wired into the
  git pre-push hook. **Caution: book-sync green does not mean the prose is
  right** — Part II+ chapters are checked against example *files* only, so a
  chapter can keep arguing a superseded design with the suite none the wiser.
- **Two writing debts already known:** the `prelude::*` problem (notes §1 — ~50
  identifiers arrive undeclared; needs a catalogue chapter plus "Appendix D:
  What rustdv Provides", with a book-sync check that every prelude export
  appears in it), and per-chapter directives in notes §3–§4.

---

## Open questions Fable must not silently settle

- **Q15** — do struct tests register as `RandomTest` or `random_test`? Currently
  the type name; transcripts show that. If it changes it must change *before*
  regeneration (D31).
- Anything in §16 of the decision log. If the prose needs an answer, ask Ray;
  do not invent one and write it as settled — that is exactly how the original
  design failed (OQ-9, D4).

---

## Where the sources live

Ground truth for the methodology is **outside** the repo at
`../rustdv-reference`: `cocotb-master`, `pyuvm-master`, `UVM/` (four releases),
`uvmprimer-master`, `Python4RTLVerification-master`, and `salemi_books/` (the
two earlier books as PDFs). Read-only; never copy into the repo.

The Python book's chapter numbers map onto rustdv's — its
`27_uvm_test_testbench_3.0` is rustdv ch23. Use it to check that a chapter
teaches the same lesson the source book taught, in the same order.
