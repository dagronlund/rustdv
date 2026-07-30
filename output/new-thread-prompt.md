# Prompt for a new rustdv thread

*Written 2026-07-30, replacing the 2026-07-29 version (whose work — D108, D112 —
is now done). Kept short on purpose: the previous version's reading list was
large enough to be expensive on its own.*

## Before you open the thread — do this on your Mac

The previous session left Linux build artifacts in the synced folder and a
stale git lock. Clear both, then confirm the tree is actually good:

```sh
rm -f .git/index.lock
rm -f sim/build/tinyalu_tb.vpi sim/build/tinyalu_rustdv.vvp sim/build/smoke.vvp
bash sim/run_rustdv.sh release          # expect REGRESSION: PASS
python3 output/regression/regress.py    # expect 237 entries, 0 failed
```

If both are green, the branch is good and the only thing left is committing it.
If either fails, paste the failure into the new thread as the first message —
that failure is the work, and everything below is context for it.

---

## Paste from here down

You have two folders: `rustdv` (the repository) and `rustdv-reference`
(read-only upstream sources). Work on branch `rewrite-book`.

**Read `TOUR.md` first, whole file** — its "Where the work stands" section at
the bottom is the live status. Then `CLAUDE.md` and `CLAUDE.local.md` for the
standing rules. That is enough to start; pull anything else only when you need
it. `output/.design-decisions.md` is the decision log and is large — read §0–§0.5
for the mission and method, and individual sections (§36 D108, §40 D112,
§41 D113) only if you touch that area. Do **not** follow
`output/.design-doc.md`; it is the pre-restoration spec kept as a record of
what went wrong.

### Where things actually stand

The framework is **done**. ch15–ch39 run on Icarus, the three-tier test suite is
built, and the last three decisions all landed on 2026-07-30:

- **D112** — `tinyalu_tb`'s DUT self-clocks like every chapter's, so no software
  `Clock` runs in the shipped testbench.
- **D108** — both runner bugs, which turned out to be one mechanism: a test
  ending on `read_only().await` started the next test inside the same executor
  drain, before the phase reset. `phase::leave_read_only()` is now the first
  thing `run_one` does; `fresh_phase()` is gone.
- **D113** — `sim/run_rustdv.sh` and `sim/run_smoke.sh` now build under
  `/tmp/rustdv-$(id -u)/` instead of into the repo. **Read this one before you
  run anything** (see the hazard below).

Committed on `rewrite-book`: D108 + D112 + the book prose rewrite. **Uncommitted
in the working tree:** D113's two script changes and the accompanying doc
updates (decision log §41, STATUS.md, TOUR.md, test-plan.md, this file). Ray
makes the commits — never claim branch state without running `git status`.

### The hazard that cost the last session an afternoon

**Never write a loadable binary or compiled design into the repo folder.** It is
synced with your sandbox, and Ray runs macOS. A Linux `.so` copied to
`sim/build/tinyalu_tb.vpi` is what his `vvp` then tries to `dlopen`; macOS
refuses the foreign image with `Killed: 9` and **no output at all**, which is
indistinguishable from a crash in whatever code changed most recently. That is
D113. The sim scripts are fixed now, but check where any script writes its
`.vpi`/`.vvp` before running it, and set `SIM_BUILD_DIR` if in doubt.

Related: **a green sandbox run is evidence about the sandbox.** macOS/arm64 is a
shipping platform for this project. Say "verified on Linux" when that is what
happened, and ask Ray to confirm on the Mac before calling anything done.

### What is left

Nothing queued for you on the framework. In Ray's order:

1. **The prose pass** — runs in a *separate* session under `book-pdf/FABLE.md`
   and `book-pdf/chapter-notes.md`. **You do not edit `book-pdf/`, and it does
   not edit code.** If you learn something the prose needs, append a line to
   `chapter-notes.md` and say so.
2. **README regeneration** — `book-pdf/renumbering-spec.md`'s last section lists
   the stale ones: ch15–ch21 (they cite `src/lib.rs`, and their transcripts
   embed pre-rename paths), plus ch27, ch36, ch37, ch38, ch39. Regenerate
   transcripts **after** any file renames, never before, since log lines carry
   paths. `HANDOFF.md` "Transcripts owed" is the checklist.

**Closed 2026-07-30, so do not re-raise them:** the renumbering pass has run
(eight crates, line-count-neutral); the child attribute is bare `#[component]`
with no argument (D114, §42 — D106's tail is closed, and the answer was that
the derive never read the word); and the licensing question is off the list.

So: unless the verification above failed, **ask what he wants before starting
anything.**

### Working notes

- No network. Toolchain: `bash toolchain-drop/install.sh`, then
  `export PATH="/tmp/rust/bin:/tmp/oss-cad-suite/bin:$PATH"`. It may already be
  installed — check `/tmp/rust/bin/rustc` first.
- All scratch under `/tmp/rustdv-$(id -u)/`. Sessions share the VM under
  different uids and cannot clean up after each other; a stale shared path
  blocks builds with an unhelpful `Permission denied`. Use
  `CARGO_TARGET_DIR=/tmp/rustdv-$(id -u)/target` and
  `CARGO_PROFILE_DEV_DEBUG=0` (cuts a debug build ~1.1 GB → ~240 MB, changes no
  transcript).
- Shell calls have a ~45 s budget; background processes die between them. Build
  incrementally across calls rather than reaching for `nohup`.
- Disk is ~9.6 GB and runs near full. `No space left on device` looks exactly
  like a real regression. `rm -rf /tmp/rustdv-$(id -u)/target/debug/incremental`
  reclaims a few hundred MB.
- Never bulk-delete-and-recreate a directory inside the mounted folder — the
  sync engine forks `dir 2/` duplicates, and the same race leaves a stale
  `.git/index.lock`. Delete individual files; build in `/tmp` and copy over.
- Fast verification loop, cheapest first:
  `regress.py --suite unit` (~2 s) → `RUSTDV_TESTCASE=<clock|trig|sig_|conc|elab|runner_> bash rustdv/framework-tests/run.sh`
  → `regress.py --suite custom` → full `regress.py`.
