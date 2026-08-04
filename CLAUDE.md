# Project: rustdv & "Rust for RTL Verification"

## Context
rustdv is a Rust hardware verification framework (a cocotb + pyuvm analog) by
Ray Salemi, companion to the book *Rust for RTL Verification*. The repo holds
two products (see TOUR.md for the tour):

1. **rustdv** — the framework (crates in /rustdv), with a TinyALU regression
   passing on Icarus Verilog.
2. **"Rust for RTL Verification"** — a 41-chapter book in /book-pdf/src
   (mdBook), every figure verified running.

**Active work: the UVM restoration.** A prior pass wrongly stripped the UVM's
dynamic build/connect process and its TLM FIFOs; branch `ch23_onwards` is
restoring them (build/connect phases, the ConfigDb, the factory, TLM), one
TinyALU testbench version at a time. This is reworking the framework and, from
the working code, the Part II+ prose — so treat the later chapters as under
revision, not settled. `output/.design-decisions.md` is the authoritative
decision log (its §0 is the mission and method); read it and CLAUDE.local.md
before proposing anything architectural. Do **not** follow the older
`output/.design-doc.md` — it is the pre-restoration spec whose closed-world
design caused the problems now being fixed.

**Where it stands (2026-07-29):** the restoration's code is done. ch23–ch39
are converted and out of quarantine — phases, ConfigDb, factory, the whole TLM
layer, transactions, and all four sequence testbenches (TB 7.0–8.0). The test
suite is built on top: 110 no-simulator tests, 38 targeted simulator tests in
`rustdv/framework-tests/`, 5 compile-fail cases, and `sim-mutation`; the
regression is 237 and the pre-push hook runs all of it
(`output/regression/TESTING.md`). **The TinyALU refactor is done too (D109),
so there is no known technical debt left** — `tinyalu_tb` runs on phases, the
ConfigDb, the factory and `AnalysisBus` like every chapter, and it is in the
suite as `custom/sim-tinyalu-tb`. What remains is D108's two runner fixes and
the book's prose pass. TOUR.md's "Where the work stands" section is the
orientation for a new thread; the decisions that most shape new code are D83b
(connection is a trait method, not a registry), D82b/D82c (children move out for
the run phase; each component races the objection event, never the whole tree),
and D90 (the analysis hub stores nothing — the subscriber owns its storage).

Ground truth for the methodology — the cocotb, pyuvm and SystemVerilog UVM
sources, plus the example code from the earlier books — lives outside this
repository at ../rustdv-reference, so the repo carries only its own product.

## Folder map
- /rustdv — the framework workspace (crates rustdv-gpi-sys … rustdv, plus
  tinyalu_tb). Runs via sim/run_rustdv.sh.
- /book-pdf/src — the manuscript. Rebuild rendered book: `mdbook build book-pdf`.
- /output/examples — every book figure, runnable (Part I per-figure files,
  Parts II–V as sim chapter crates).
- /output/regression — regress.py, wired into the git pre-push hook.
- /skills — AI verification skills (rtl-spec-analysis, rustdv-testbench,
  rustdv-verify-cover).
- STATUS.md — authoritative implementation history and deviations log.
- TOUR.md — orientation for new readers and new threads (start there).

## Standing rules
- The pre-push hook runs the full regression; keep it green. Verify claims
  by running things — transcripts in the book/README files are real output
  and must stay in sync with reruns.

### Leave no documentation debt — the rule that costs the most when ignored

**When you change code, update the artifacts that quote it, in the same
session.** This project's expensive failures have all been the same shape:
something changed, the prose describing it did not, and nothing noticed for
weeks. ch23/ch24/ch26 carried wrong `file:line` references and an undocumented
test; ch18–20's transcripts were 5ns stale after the DUT began self-clocking;
ch37's README described a chapter that does not exist. Each was cheap to fix at
the time and expensive to find later — one session spent ~20% of its context
paying that debt down.

So, if you change:

| this | then regenerate |
|---|---|
| an example crate's code | its `README.md` figure map **and** its transcript |
| anything a transcript quotes (timing, paths, test count) | every README quoting it — `bash output/regression/verify-transcripts.sh` finds them |
| a `.rs` figure caption | the README figure map — they must agree, and caption edits stay line-count-neutral |
| framework syntax (an attribute, a name) | every call site, `skills/`, `.claude/skills/`, and a note in `book-pdf/chapter-notes.md` |

**Prefer a check to a note.** A rule written down is a rule someone must
remember; a rule in the regression is enforced. Two checks exist because the
drift they catch went unnoticed for months while a full green regression ran
over it every push:

| check | what it gates |
|---|---|
| `custom/readme-transcripts` | every transcript in an example README is what the simulator prints (22 chapters, 431 lines) |
| `custom/book-listings` | every Rust listing in ch15–40 is real code from that chapter's crate |
| `book-sync` (pre-existing) | ch1–14 listings, byte-for-byte |

`book-listings` separates two things that look alike and are not.
`QUOTED` holds listings that were never ours — the `Future` trait from the
standard library, a deliberately tidied macro expansion. Those are quotations,
carry no debt, and will not shrink. `KNOWN_DRIFT` is the debt register: a book
and a codebase disagreeing. **It is currently empty, and adding to it to make a
build pass is how drift comes back.** Keep the two apart; listing permanent
quotations as outstanding work makes a clean register look dirty and trains
everyone to ignore it.

If you find a class of error nothing catches, add the check rather than only
documenting the instance — and **make the check fail once on purpose before
trusting it.** Both of the above were mutation-tested that way; the first
version of `verify-transcripts.sh` reported success on chapters whose sims had
not run at all.

**Ask the process, not the prose.** A check that decides pass/fail by
pattern-matching human-readable output can be defeated by formatting. The
compile-fail suite tested "did it compile?" with `grep -E '^error'`; GitHub's
cargo emits ANSI colour, so the line began with an escape sequence, `^error`
never matched, and CI reported "compiled — no longer rejected" while printing
`error[E0277]` underneath. Exit codes, structured output and
`CARGO_TERM_COLOR=never` are not decoration — they are the difference between a
check and a guess.

**Do not create per-thread prompt files.** Orientation lives in TOUR.md
("Notes for AI sessions" has the sandbox hazards) and in this file. A new thread
is pointed at those, not handed a restatement of them that will itself go stale.
- Flag uncertainty openly (Open Questions / STATUS deviations) rather than
  presenting guesses as settled.
