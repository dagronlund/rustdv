Work on the **Analysis Port explanation** in rustdv. Branch `rewrite-book`.

Read `TOUR.md` first (whole file; "Where the work stands" at the bottom is live
status), then `CLAUDE.md` and `CLAUDE.local.md` for the standing rules.

## What I want changed

I will explain that once you are in context. **Read yourself in first, then stop
and wait.** Do not edit anything, and do not diagnose what you think is wrong
with the current explanation — I have something specific in mind and a guess
will send us both the wrong way.

When you are ready, tell me in a few lines what you understand the current
explanation to claim, so I can see what you are working from.

## Where the explanation lives — four layers, different owners

| Layer | File | Who edits it |
|---|---|---|
| The framework's own docs | `rustdv/rustdv-methodology/src/analysis.rs` (~15 KB, doc comments carry real argument) | you |
| The chapter's runnable code | `output/examples/ch32-analysis-ports/src/ch32_analysis_ports.rs` | you |
| The chapter's figure map + transcripts | `output/examples/ch32-analysis-ports/README.md` | you |
| **The book chapter** | `book-pdf/src/chapter-32-analysis-ports.md` | **NOT you — see below** |

**`book-pdf/src` is off limits to a code thread.** A separate prose pass owns the
manuscript (`book-pdf/FABLE.md`), and as of 2026-07-30 a Fable thread is being
restarted to finish it — including work that touches ch32. If you edit that file
you will collide with it. When you learn something the prose must say, append to
`book-pdf/chapter-notes.md` (the ch32 row, and the long-form section "ch32 — the
subscriber owns the storage" at line ~229) and say so out loud.

## What the explanation currently claims

The governing decisions, in `output/.design-decisions.md` — read these before
proposing a change:

- **§26, D90 — the analysis hub holds nothing.** `AnalysisBus` is a subscriber
  list. `write` calls each subscriber and returns; a datum broadcast to nobody is
  gone. Storage belongs to the subscriber. This is the chapter's thesis.
- **§24, D86–D88** — publish, subscribe, and `RustdvShared`.
- **§32, D103** — `AnalysisFifo` was renamed `AnalysisBus`, because the old name
  came from `uvm_tlm_analysis_fifo` (a *subscriber-side buffer*, the one thing
  this design decided rustdv does not need) while the type it named is the
  broadcast hub, which the UVM has no counterpart for.

Two arguments the current explanation leans on, worth checking as you go:

1. **The UVM contrast.** A UVM scoreboard routes each stream into a
   `uvm_tlm_analysis_fifo` because a class gets *one* `write` method; a second
   stream needs `uvm_analysis_imp_decl`. A rustdv subscriber declares two
   `SubscribePort`s and two `WriteSink` impls, so the workaround has nothing to
   work around. Ground truth for UVM intent is `../rustdv-reference/UVM/`, not
   pyuvm.
2. **Why a subscriber would still own a queue.** `write` is synchronous and
   cannot await, so a subscriber whose work takes simulation time splits it:
   `write` does `try_put` into an unbounded `TlmFifo` it owns, and `run` gets
   from that FIFO. The inbox must be unbounded — `write` cannot wait for space
   and analysis has no back-pressure. `SlowChecker`/`SlowSubscriberTest` are the
   proof; the transcript shows writes at `0.00ns` and checks at 5/10/15ns.

## Verifying

Toolchain: check `/tmp/rust/bin/rustc` first, else `bash toolchain-drop/install.sh`,
then `export PATH="/tmp/rust/bin:/tmp/oss-cad-suite/bin:$PATH"`.

```
cd output/examples && sim-common/run_sim.sh ch32_analysis_ports playground
python3 output/regression/regress.py --suite unit      # ~2 s
python3 output/regression/regress.py                   # full, a few minutes
```

**Never write a `.vpi`/`.vvp`/`.so` into the repo folder** — it is synced to a
Mac, and a Linux image there kills `vvp` with no output (D113, §41). Keep scratch
under `/tmp/rustdv-$(id -u)/`. A green sandbox run is evidence about the sandbox;
say "verified on Linux" when that is what happened.

**Known gap, relevant here:** `output/regression/verify-transcripts.sh` checks
README transcripts against fresh runs, but covers only 13 chapters — **ch32 is
not among them**, so its 21 README transcript lines are unverified. If you touch
ch32's transcripts, add it to that script's run list rather than trusting them.
