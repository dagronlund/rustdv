# Prompt for a new rustdv thread

**STALE as of 2026-07-30 — do not paste this as-is.** D112 and both of D108's
runner bugs, which this prompt was written around, are now done (see
`output/.design-decisions.md` §36/§40 and STATUS.md's 2026-07-30 entries).
The framework has no known queued work left except what TOUR.md's "Where the
work stands" already calls "Mine, not yours" (the licensing decision, the
`#[component(fifo)]`/`#[component(sequencer)]` naming question) and the
renumbering pass, which waits on the prose. Whoever starts the next thread
should write a fresh version of this prompt around whatever Ray wants worked
on next, using TOUR.md's live section as the source of truth rather than the
body below, which is kept only as the record of how the last one was framed.

*Paste everything below the line into the new thread. Written 2026-07-29, at the
end of the session that finished the TinyALU refactor and prepared the prose
pass; revised 2026-07-30 to add D112 (retiring `tinyalu_tb`'s bare-DUT
exception) ahead of D108 in the work queue. Supersedes the 2026-07-28 version,
whose reading list points at files that no longer exist.*

---

You have two folders: `rustdv` (the repository) and `rustdv-reference`
(read-only upstream sources — cocotb, pyuvm, the SystemVerilog UVM, and my two
earlier books).

**This turn is orientation only. Do not change any file, do not run a build, do
not run the regression, do not propose a plan.** Read, then tell me what you
understand. You may run `git status`, `git log` and `ls` to establish facts
rather than assume them.

## What changed since the last thread, so the shape is not a surprise

The restoration is **finished**. Every chapter crate ch15–ch39 runs on Icarus,
the three-tier test suite is built, and the TinyALU refactor — the last known
technical debt — landed. There is no framework work queued except D112's
self-clocking cleanup and D108's two runner bugs. The manuscript is being
rewritten in a **separate session**, which brings one hard rule for you: see
"Do not touch the book" below.

## Read these, in this order

1. `TOUR.md` — start here, whole file. Its **"Where the work stands"** section
   at the bottom is the live status and is written for this moment.
2. `CLAUDE.md`, then `CLAUDE.local.md` — the standing rules.
3. `output/.design-decisions.md` — the decision log. Read **§0–§0.5** (premise,
   mission, method, the typing thesis, status), then the five newest sections:
   **§40 (D112, retiring `tinyalu_tb`'s bare-DUT exception — this lands
   first)**, **§36 (D108, the two runner bugs — the work after D112)**, **§37
   (D109, the TinyALU refactor)**, **§38 (D110, figure numbering)**, **§39
   (D111, chapter 41 cut)**. Then **§16**, which should now be empty of open
   questions. Skim the rest; the index table at the top maps sections to
   decision numbers.
4. `STATUS.md`, bottom-up. The last two entries — the test suite and the TinyALU
   refactor — are the current state.
5. `output/regression/TESTING.md` — how the suite runs, and the two runner
   behaviours a simulator test has to work around. Those two behaviours **are**
   D108.
6. `output/test-plan.md` §8, "Two findings, for the record" — the same two
   behaviours, written up when they were found rather than decided.
7. The code D108 will touch, and only this much of it:
   - `rustdv/rustdv-runner/src/rustdv_runner.rs` — `run_one`'s per-test reset,
     where the first fix belongs, beside the `ConfigDb::clear()` already there.
   - `rustdv/rustdv-sim/src/phase.rs` — the simulator phase and the ReadOnly
     region, which is where the leak lives.
   - `rustdv/rustdv-sim/src/clock.rs` and `triggers.rs` — the second, deeper
     fix: a write scheduled from inside a ReadOnly callback has to be deferred
     to a region that permits it.
   - `rustdv/framework-tests/src/framework_tests.rs` (`fresh_phase()`, which the
     first fix should make unnecessary), plus `clocks.rs` and `triggers.rs`
     beside it.
8. `rustdv/tinyalu_tb/src/` — all six files. It is the shipped testbench, it was
   just converted, and it is the example every chapter's shape is measured
   against. **Do D112 here first**, before D108: this is the file with the
   `Clock::new` line that D112 deletes, and `sim/hdl/tinyalu.sv` is the RTL
   D112 makes self-clocking. Don't confuse this with D108's own `clock.rs` —
   D112 removes a `Clock` *use*; D108 fixes the executor `Clock` itself is
   built on.

Do **not** follow `output/.design-doc.md`. It is the pre-restoration
specification whose closed-world design caused the problems that were fixed; it
is kept only as the record of what went wrong.

## Do not touch the book

`book-pdf/` belongs to a separate prose pass that may be running right now. Its
rules are `book-pdf/FABLE.md`, `book-pdf/chapter-notes.md` and
`book-pdf/fable-prompt.md`. **You do not edit `book-pdf/`, and it does not edit
code.** If you learn something the prose needs, append a line to
`chapter-notes.md` and tell me — that file is the channel between the two
threads. If you find a figure or transcript that looks wrong, report it; do not
fix it from either side.

## Then tell me what you understand

Answer in your own words, not by quoting the documents. I am checking whether you
have the model, so give me the *why*, not just the what.

1. **The mission.** What was destroyed, what belief caused it, and what is the
   one instinct you are to resist? What is D3 and why does it settle arguments?
2. **The method.** In what order do code, prose and decisions get written, and
   why that order?
3. **Where things stand.** What is done, what is the only framework work left,
   and what is waiting on me rather than on you?
4. **D108, in detail** — both behaviours, why each is a bug rather than a quirk,
   why the second is the harder one, and what could break when it is fixed.
   What has to happen to the affected transcripts?
5. **The shipped testbench.** How does `tinyalu_tb` get its BFM, how are its
   components built, how is its stimulus chosen, and where does its clock come
   from — and why is that last answer different from every chapter's?
6. **Verification.** What are the three tiers, what enforces the line between
   the first two, and what does `sim-mutation` prove that a passing regression
   does not?
7. **What is checked and what is not.** Which parts of the book does the
   regression compare against code, and which does it not touch at all?
8. **Anything stale or contradictory.** If two documents disagree, or a document
   claims something the code does not do, say so plainly — including if it
   contradicts something above. Later entries in the log and STATUS.md supersede
   earlier ones; say which you think wins and why.

Finish with the two or three things you are least sure about. An honest gap now
beats a confident wrong answer later.

## The work queued, in order

1. **D112 first: retire `tinyalu_tb`'s bare-DUT exception.** Make
   `sim/hdl/tinyalu.sv` self-clocking like every chapter's DUT (D42), delete
   `tinyalu_tb`'s `Clock::new` line, rerun `custom/sim-tinyalu-tb` and the full
   regression, and check whether the transcript timing shifts (it has before,
   for an unrelated reason — STATUS.md, the `wait_idle` hardening). Small,
   isolated, no scheduler involved.
2. **Then D108's two runner fixes.** Decided, not started. D112 narrows the
   second fix's scope to ch17's `Clock` idiom and
   `framework-tests/hdl/probe.sv` — same executor-level fix, smaller surface to
   verify against. Both land before release.
3. **After the prose pass finishes:** apply `book-pdf/renumbering-spec.md` to the
   `.rs` captions — in place and line-count-neutral, because transcripts embed
   `file:line` — then render and read the book end to end against the render.
4. **Mine, not yours:** the licensing decision (`output/rights-inventory.md`
   has the five places and the current contradictions), and the
   `#[component(fifo)]`/`#[component(sequencer)]` naming question, which D106's
   tail defers to whoever next touches those declarations.

## Working notes for when we do start

- No network. The toolchain comes from `toolchain-drop/install.sh`; the VM wipes
  `/tmp` between sessions, so re-run it and
  `export PATH="/tmp/rust/bin:/tmp/oss-cad-suite/bin:$PATH"`. `mdbook` is in the
  drop too and builds HTML offline; only the PDF backend needs a Chromium the VM
  lacks.
- **Keep all scratch under `/tmp/rustdv-$(id -u)/`.** Sessions share the VM under
  different uids and cannot clean up after each other; a stale shared path blocks
  builds with an unhelpful `Permission denied`. Use
  `CARGO_TARGET_DIR=/tmp/rustdv-$(id -u)/target` for the framework and
  `…/examples-target` for the examples.
- Shell calls have a ~45 s budget and background processes die between them. The
  full regression takes minutes from cold — pre-build the workspaces, then run
  `regress.py` one suite at a time.
- Disk is ~9.6 GB and fills up. `CARGO_PROFILE_DEV_DEBUG=0` cuts a debug build
  from ~1.1 GB to ~240 MB and changes no transcript. A full disk reads as
  `No space left on device` and looks exactly like a real regression.
- Never bulk-delete-and-recreate a directory inside the mounted folder — the sync
  engine races and forks `dir 2/` duplicates. Build in `/tmp` and copy over. The
  same race can leave a stale `.git/index.lock`; check for one if git complains.
- **I make the commits. Never claim branch state without running `git status`.**
  As of 2026-07-29 both `fable-prep` and `master` are at `8af2f96`, pushed, with
  the regression green at 237 entries.
