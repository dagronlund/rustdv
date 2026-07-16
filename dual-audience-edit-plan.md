# Dual-Audience Edit Plan: *Rust for RTL Verification*

*Plan only — no book changes made. Prepared 2026-07-16 after reviewing all three
books: The UVM Primer (24 chapters, docx), Python for RTL Verification (pdf), and
the current 41-chapter Rust manuscript, plus the uvmprimer examples repo.*

## 1. The problem, measured

The manuscript is 85,700 words. Its Python-reader assumption is not a flavor —
it is load-bearing:

- **~880 mentions** of Python/pyuvm/cocotb across the 41 chapters.
- **"the Python book" appears 204 times.** Chapter 1 tells non-readers to
  leave: "If neither is true, read that book first."
- **37 "> In Python we..." recap blockquotes** structure Parts II–V. They are
  the book's memory device — and they address only one of the two audiences.
- **Part I's framing device is the "Python twin"**: Ch. 5 opens "Every chapter
  so far has had a Python twin"; Ch. 10 opens "Every chapter so far has taken
  something Python did at runtime..."
- **SystemVerilog appears ~30 times total**, mostly in passing. The 80% of the
  market that lives in SV-UVM is invisible.
- **The typing sermon** concentrates in Ch. 1, 2, 10, 11, 13, 21 and is pitched
  as *escape from an untyped language* — exactly the pitch that makes a
  SystemVerilog engineer say "no kidding, just teach me the language."

## 2. The strategy: re-anchor on what both readers share

Both readers arrive from one of your two books. What they share is not a
language — it is **the UVM and the TinyALU**. Both know what a driver, monitor,
scoreboard, sequence, factory, and config database are for. Both have built
this exact testbench before. The book should address *the UVM engineer*, not
*the Python engineer*.

Three moves, in order of importance:

### Move 1 — Re-pitch the typing theme; don't delete it

The compile-time theme is the book's spine and its best material (the
compile-fail figures are unique in the literature). The fix is the *pitch*,
not the theme:

- **To the Python reader** the pitch stays: your bugs move from runtime to
  compile time.
- **To the SV reader** the honest pitch is sharper: *you already live the typed
  life, and your typed language still defers its most painful checks to
  runtime.* `uvm_config_db#(T)::get` type mismatches, `$cast` failures, null
  virtual interfaces, factory overrides that fail by string lookup, TLM
  connections that error at `connect_phase` — every one is a runtime event in
  SystemVerilog and a compile error in rustdv. That is not "typing is good"
  (they know); it is "your compiler stops too early."

Execution: state the dual pitch **once, strongly, in Chapter 1**, then let the
compile-fail figures carry it. Cut the recurring sermons (the "this is the
book's deeper reason" reprises) down to the evidence. SV engineers don't mind
being shown; they mind being told.

### Move 2 — Make the recap device serve both readers

The "> **In Python we...**" blockquote becomes "> **In the UVM...**". This
works because pyuvm deliberately mirrors SV-UVM's API:
`start_item`/`finish_item`, `get_next_item`/`item_done`, analysis ports,
`raise_objection` — one recap written in UVM API terms serves both dialects.
Where the dialects genuinely differ, a parenthetical: *(pyuvm: `ConfigDB()`;
SV: `uvm_config_db#(T)`)*.

Occasionally a short SV quote is a *better* foil than prose — e.g.,
parameterized classes (Primer Ch. 8) against generics in Ch. 11, or
`type_id::create` + string overrides against maker closures in Ch. 29. Policy
for these in §5.

### Move 3 — Reframe Part I from "Python twin" to "the languages you know"

Part I comparisons often land *harder* against SystemVerilog than Python:

| Rust chapter | Python foil (current) | SV foil (add) |
|---|---|---|
| 4 match | if chains | `case` without `unique`/`priority` traps |
| 5–6 ownership | garbage collector | SV's automatic class lifetimes — *also* no twin |
| 7 enums | — | SV enums are ints in a trench coat; Rust enums carry data |
| 9 Result/Option | exceptions | null handles; the missing error story |
| 10 traits | inheritance/super() | virtual classes, pure virtual methods |
| 11 generics | duck typing | parameterized classes — a direct, familiar analog |
| 14 cargo | pip/venv | the .f-file + vendor-flags swamp; no package manager at all |

The rewrite keeps Python comparisons where they illuminate but writes them so
a Primer reader is never lost — "in Python" as shared context (they can read
it), never "as the Python book taught you" (they didn't read it).

## 3. Chapter-by-chapter scope

**Tier 1 — full rewrite (4 files).**
- **Ch. 1 Why Rust?** New opening: this book stands alone. It assumes the
  reader knows verification and the UVM — from SV-UVM, from pyuvm, or from
  either of the two prior books, which are referenced as places to *learn* the
  UVM, not as prerequisites. The dual pitch from Move 1. Restate the TinyALU
  spec on its own terms. New subtitle: **"A complete course in Rust and
  rustdv."**
- **Ch. 2 Rust Concepts.** Add a "Where you're coming from" section: two short
  subsections — what the SV engineer should carry over and unlearn; what the
  Python engineer should. Rest of chapter reframed per Move 3.
- **Appendix A** becomes a tri-book map: Primer ch. ↔ Python-book ch. ↔ Rust ch.
  (The Primer maps cleanly: its Ch. 23 Sequences ↔ Rust Ch. 36, etc.)
- **New Appendix C: SystemVerilog-UVM → rustdv translations**, sibling to the
  existing Appendix B (which stays, retitled "For readers of *Python for RTL
  Verification*"). Rows: `uvm_component_utils` → `#[derive(Component)]`,
  `uvm_config_db` → typed configs, `type_id::create`/overrides → maker
  closures, TLM port classes → channels, `uvm_do` macros → explicit
  `start_item`/`finish_item`, phases → constructors + async phases, etc.

**Tier 2 — heavy prose revision, figures untouched (Ch. 3–14, 12 files).**
Replace the "Python twin" spine with dual framing per Move 3. Every "In
Python..." sentence gets one of three treatments: keep (works as shared
context), pair (add the SV analog), or neutralize (say it in Rust's own terms).
The Rust code, figure numbers, and transcripts do not move.

**Tier 3 — moderate revision (Ch. 15–31, 33, 35–39, ~23 files).**
Convert the 37 recap blockquotes per Move 2. Sweep "the Python book"
references: keep a handful that are true attributions, recast the rest.
Ch. 22 (Why UVM?) loses "the story has not changed since the Python book told
it" — the Primer reader *lived* that story; acknowledge both routes.

**Tier 4 — light touch (Ch. 32, 34, 40, 41, Interlude, ~5 files).**
These already teach rustdv on its own terms (Ch. 40 has one Python mention).
Sweep and verify only.

**Outside the manuscript:** TOUR.md's book-voice rule ("In Python we..."
openers) changes to the new convention; output/examples chapter READMEs
swept for the same device; back-cover/marketing copy if any.

## 4. What does not change

- The structure: 41 chapters, interlude, testbench 1.0 → 8.0. The Primer and
  the Python book share this arc, so it already serves both audiences.
- **Every figure, every transcript, every compile-fail example.** This edit is
  prose-only. The 107 figure-sync checks in regress.py enforce that promise.
- The voice and quality bar. The failure mode to avoid is a book that reads
  like it was patched.
- Teaching order: Rust first (Part I), then rustdv (Parts II–V).

## 5. SystemVerilog code policy

Where an SV foil earns its place (est. 5–8 spots), quote *real* code from the
uvmprimer repo (`claude` branch) — never write fresh, untested SV. Constraints:

- Short excerpts, clearly labeled as from The UVM Primer's examples.
- **Never presented with simulation output** — we have no commercial-simulator
  license to rerun them (run.do targets Questa). They are quotations, not
  claimed-run figures.
- **Sync-check without a cross-repo dependency:** the uvmprimer repo is not
  part of rustdv, and regress.py must stay self-contained. So: vendor the
  handful of quoted .svh files into `/reference/uvmprimer/` (a one-time curated
  copy, recording the source commit hash), following the existing pattern —
  `/reference` already holds cocotb, pyuvm, and the Python book for exactly
  this purpose. regress.py sync-checks book quotes against the vendored copies.
  The uvmprimer `claude` branch is where I select and verify the excerpts; an
  optional extra check can compare vendored copies against the live repo when a
  session happens to have both mounted, but nothing requires it.

## 6. Process and verification

1. **Style sheet first** (`book-pdf/dual-audience-style.md`, internal): the
   recap-device wording, the three treatments for Python references, the
   dialect-parenthetical format, banned framings ("as the Python book taught
   you"), the SV-quote policy. You approve this before any chapter changes.
2. **Three exemplar chapters** — 1 (Tier 1), 10 (Tier 2), 36 (Tier 3) — for
   your review before the bulk edit. Cheap to redo if the tone is off.
3. **Execute by part**, chapter-by-chapter commits on a branch in rustdv.
4. **Verification per part:** regress.py green (figure sync + example checks +
   sim runs); a new grep-lint in the regression for banned framings; mdbook
   render on your Mac for visual check.
5. **Two persona read-throughs at the end:** once as a Primer reader who never
   touched Python ("am I ever lost or lectured?"), once as a Python-book reader
   ("did I lose anything I loved?"). Fix what either finds.

Estimated effort: Tier 1 + style sheet + exemplars ≈ 3–4 working sessions;
Tier 2 ≈ 4–5; Tiers 3–4 + appendices + persona passes ≈ 4–5. Roughly a dozen
sessions end to end.

## 7. Risks

- **Symmetric irritation.** Python-book readers liked "In Python we...". The
  neutral recaps still speak fluent UVM (which they know), and Appendix B keeps
  the full Python→Rust translation table. Net loss to them should be near zero;
  the exemplar review is the checkpoint.
- **Blandness.** Neutralizing can flatten. The style sheet's rule: prefer *two
  sharp comparisons* over zero — dual-audience means both foils, not no foil.
- **Scope creep into figures.** Guarded by regress.py; any edit that would
  touch a figure gets flagged rather than made.
- **SV accuracy.** I will quote, not compose, SV (per §5), and hedge nothing I
  can't run.

## 8. Fable 5 or Opus?

Recommendation: **Fable 5**, with one honest caveat — you are asking Fable 5,
so discount accordingly. The reasoning:

- This is editorial-judgment work, not mechanical transformation. The hard part
  is hearing when a sentence condescends to a SystemVerilog veteran or strands
  a Python reader — persona-sensitivity across 85,000 words is exactly where
  model capability differences show up most.
- Voice consistency wants one hand. The prose you called excellent was written
  at this capability level; patches must be indistinguishable from it.
- The mechanical fraction is small. The 37 blockquotes and 204 references each
  need individual judgment (keep / pair / neutralize), so there is no large
  batch job to delegate cheaply.

A defensible hybrid if you want to conserve Fable usage: Fable 5 writes the
style sheet, Tier 1, the exemplars, and the final persona passes; Opus 4.8
executes Tier 3 conversions against the approved exemplars; Fable reviews the
diff. Workable, but the review pass eats much of the savings, and the seams
would be mine to catch. For a book you intend to sell, I'd keep one hand on
the pen.

## 9. Decisions (resolved 2026-07-16)

1. Recap label: "**In the UVM...**" — "you know" added nothing.
2. The book is standalone. The Primer and the Python book are referenced as
   places to learn the UVM, not prerequisites.
3. New subtitle: "A complete course in Rust and rustdv."
4. Videos are out of scope.
5. SV sync-checks use vendored copies in `/reference/uvmprimer/` so regress.py
   never depends on a second repo being mounted (§5).
