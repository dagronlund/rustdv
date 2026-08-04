# A Tour of This Repository

*New here — human or AI? This is the walk-around. Ten minutes, and you'll
know what this project is, what's been proven, and where everything lives.
Last verified 2026-08-04; the proven claims below are checked by the
regression suite, not aspirational.*

> **Both products are complete (2026-08-04).** The framework is done —
> ch23–ch39 green on Icarus, the three-tier test suite built on top, D112/D108
> closing the last of it. **The book is done too:** every transcript is real
> simulator output, every ch15–40 listing is checked against its crate, and the
> end-to-end read has run. The UVM restoration that dominated this repo's
> history is finished; `#[component]` and the phase/ConfigDb/factory/TLM layer
> are the settled design, not work in progress.
>
> **"Where the work stands", at the bottom of this file, is the live status** —
> including the one thing still outstanding. `output/.design-decisions.md` is
> the decision log; read its §0 and CLAUDE.local.md before proposing anything
> architectural.

## What this project is

**rustdv** is a hardware verification framework in Rust — the cocotb +
pyuvm story retold with a compiler: a simulator-driven async executor,
triggers, a UVM-style component methodology (ownership tree, typed
configs, maker-closure factories, channels/analysis ports, the full
sequencer handshake), running testbenches as native shared libraries
loaded by Icarus Verilog over VPI. Zero external dependencies. The crate
name `rustdv` is registered on crates.io (0.0.1 placeholder).

**"Rust for RTL Verification"** is its book — the third in Ray Salemi's
series after [*The UVM Primer*](https://www.uvmprimer.com) (SystemVerilog)
and [*Python for RTL Verification*](https://a.co/d/0hTKAJvh): 40 chapters,
an interlude, and appendices in `book-pdf/src/` (mdBook), complete as of
2026-08-04.
The premise: the reader is a UVM verification engineer — from SystemVerilog
or Python; neither earlier book is a prerequisite — who learns Rust chapter
by chapter while rebuilding the TinyALU testbench, versions 1.0 through 8.0.
Every figure runs; every simulation transcript in the book is genuine 
Icarus output.

## What's been proven

All of this is reproducible from this tree and enforced by the git
pre-push hook:

- `sim/run_rustdv.sh` — the shipped TinyALU testbench ends `REGRESSION: PASS`
  (RandomTest: 20 compared, 0 mismatches, every op covered; MaxTest: 4/0). It
  runs in the suite as `custom/sim-tinyalu-tb`, which asserts those counts.
- **Mutation-checked**: with the DUT's XOR deliberately corrupted to OR,
  the scoreboard flags every affected transaction and the regression
  fails; restored, it passes. The checking has teeth.
- `output/regression/regress.py` — **239 entries, 0 failed**, green on **both
  Linux and macOS/arm64**. One package is quarantined in `regress.json`:
  `ch21_macros`, a macro demonstration with no simulator test, which never
  comes off the list. Every chapter crate ch15–ch39 runs.
- **Three tiers of framework test under the 21 chapter runs** — 110
  no-simulator tests (`--suite unit`, ~2 s), 38 targeted simulator tests in
  `rustdv/framework-tests/` plus `sim-mutation`, and 5 compile-fail cases each
  asserting its `error[E….]`. `output/regression/TESTING.md` is the operating
  manual.
- `cargo test --workspace` in `/rustdv` — the whole methodology layer
  (ConfigDb, factory, ports, FIFO, analysis bus, objections, the phase walk,
  the sequencer handshake), no simulator required.

## Where everything lives

| You want | Look at |
|---|---|
| The framework | `/rustdv` (workspace: `rustdv-gpi-sys` → `rustdv-gpi` → `rustdv-sim` → `rustdv-methodology` → `rustdv`, plus `tinyalu_tb`) |
| The book manuscript | `/book-pdf/src` (TOC in `SUMMARY.md`); render with `mdbook build book-pdf` |
| Why it's designed this way | `output/.design-decisions.md` — the restoration's authoritative decision log (§0 = mission + method). **Do not** follow `output/.design-doc.md`: it is the pre-restoration specification whose closed-world design *caused* the problems now being fixed, kept only as the record of what went wrong. |
| Runnable book figures | `/output/examples` (`README.md` has per-chapter run commands) |
| The regression suite | `/output/regression/regress.py` (`--help` works; wired into pre-push) |
| Implementation history & honest deviations | `STATUS.md` (chronological, bottom-up) |
| The book's prose pass | `book-pdf/FABLE.md` (the rules) and `book-pdf/chapter-notes.md` (one row per chapter). These supersede the older `fable-brief.md`, `notes-for-fable.md` and `dual-audience-style.md`. |
| AI verification skills | `/skills` (spec+RTL → testbench → verified coverage report) |
| Upstream sources | `../rustdv-reference` — **read-only, outside the repo** (cocotb, pyuvm, SystemVerilog UVM, both earlier books) |

## Highlights worth your first half hour

- **The Interlude** (`book-pdf/src/interlude-tinyalu-testbench.md`) — the
  complete testbench, presented before the climb. The best single answer
  to "what does rustdv code look like?"
- **The compile-error figures** (`output/examples/*/compile-fail/`) — Part I
  shows the compiler catching mistakes before the simulator runs (a mis-typed
  handle is `E0308`, and so on). *Note under the restoration:* this covers
  data and ownership, **not** the late-binding layer — config, factory and
  TLM resolve at run time by design, so the old "a config conflict is a
  compile error" figures were removed (D68, and the reasoning in §0.4).
- **Fibonacci on the TinyALU** (chapter 38 — TB 7.2) — stimulus that needs the
  DUT's answers: `Fibonacci Sequence: [0, 1, 1, 2, 3, 5, 8, 13, 21]`.
- **The honest-gaps culture** — `STATUS.md`'s deviations and the decision log's
  open questions. What this project can't do yet is written down next to what it
  can. (The book's own "future of Rust in verification" chapter was cut — D111.)

## Rules of the road

- Keep the pre-push hook green: if you touch code or book figures, run
  `python3 output/regression/regress.py` before pushing.
- Transcripts in the book and READMEs are real output and must stay in
  sync with reruns — verify claims by running things.
- `../rustdv-reference` (outside the repo) is read-only. `/output` holds
  generated deliverables.
- **Do not edit `book-pdf/src` from a code thread.** The manuscript belongs to
  a dedicated prose pass whose rules are `book-pdf/FABLE.md`; that pass changes
  no code. A code thread records what the prose will need — in
  `chapter-notes.md` — and moves on. Two checks now enforce the other
  direction: `custom/book-listings` and `custom/readme-transcripts` fail the
  push if the book and the code disagree.
- **Figures are one numbering space** (D110): a chapter's code listings,
  drawings, tables and transcripts all draw from the same sequence, in order of
  appearance, and every one of them is a "Figure". If Figure 1 is a drawing, the
  first listing is Figure 2. Captions read `// Chapter N, Figure M:` in the
  example crates, output follows `--`, and each chapter README in
  `output/examples` maps its figures to runnable code.

---

## Notes for AI sessions

Context that matters to an AI working in this folder (via Claude Cowork
or similar) and to nobody else:

- **Never write a loadable binary or compiled design into the repo folder
  (D113).** This is the expensive one. The folder syncs to Ray's Mac, so a Linux
  `.so` copied to `sim/build/tinyalu_tb.vpi` is what his `vvp` then tries to
  `dlopen` — macOS refuses the foreign image with `Killed: 9` and **no output at
  all**, which is indistinguishable from a crash in whatever code changed most
  recently. It cost an afternoon once. The sim scripts build under
  `/tmp/rustdv-$(id -u)/` now and `SIM_BUILD_DIR` overrides, but check where any
  script writes its `.vpi`/`.vvp` before running it. Source and documents into
  the repo, yes; `.vpi`, `.vvp`, `.so`, `.dylib`, object files and simulator
  output directories, no.
- **A green sandbox run is evidence about the sandbox.** macOS/arm64 is a
  shipping platform for this project. Say "verified on Linux" when that is what
  happened, and ask Ray to confirm on the Mac before calling anything done. The
  same applies to a checker: `output/regression/verify-transcripts.sh` had two
  bugs that only surfaced when it was run somewhere the sims genuinely failed.
- **Never bulk-delete-and-recreate directories from the sandbox VM** —
  the desktop sync engine races and forks `dir 2/` duplicates. Build
  trees in `/tmp` and `cp` over; file deletion needs the permission tool.
- The Cowork VM has no network: toolchain comes from `toolchain-drop/`
  (extract from a `/tmp` copy — extracting off the mount is ~15× slower).
  `mdbook` **is** in the drop and installs offline to `/tmp/rust/bin/mdbook` —
  it builds the book's HTML in the VM, which is enough to catch a broken
  `SUMMARY.md` or an orphaned chapter. The **PDF** backend is what needs a
  Chromium the VM lacks, so PDF rendering happens on Ray's Mac.
- Long shell commands: the VM kills background processes between calls
  and each call has a ~45 s budget — chunk accordingly. `regress.py` takes
  a few minutes from cold, so pre-build the example workspace first
  (`cargo build --workspace --exclude` each quarantined crate) and then run it.
- **Keep all your scratch in one place you own: `/tmp/rustdv-$(id -u)/`.**
  More than one session can share the VM, each under a different uid, and each
  sees the other's files as owned by `nobody`. A bare `/tmp/rustdv-target` is
  therefore a landmine: whoever creates it first owns it, and the next session
  cannot write to it *or* delete it (`/tmp` is sticky), so a stale directory
  from a thread that has since gone away can block builds indefinitely. The
  failure is unhelpful — `Permission denied` deep in a cargo or `cp` line, or
  every sim test failing at once. `run_sim.sh` and `sim/run_rustdv.sh` default
  into this root already; put your own logs and extracted files under
  `scratch/` there too, and clean up with one `rm -rf /tmp/rustdv-$(id -u)`.
  For the same reason, never write to a fixed shared path from a test.
- **Disk fills up.** The VM has ~9.6 GB and a debug build of the framework plus
  the examples reaches ~1.1 GB; a full disk shows up as `regress.py` failures
  reading `No space left on device`, which looks like a real regression and is
  not. Two ways out: `CARGO_PROFILE_DEV_DEBUG=0` cuts those builds to ~240 MB
  (debuginfo is nearly all of it, and transcripts are unaffected — `file!()`
  and `line!()` are compile-time macros), and
  `rm -rf /tmp/rustdv-$(id -u)/*-target/debug/incremental` reclaims a few
  hundred MB more.
- **Editing captions must be line-count-neutral.** Transcripts embed `file:line`
  (`…/ch25_uvm_env_testbench_4_0.rs:256`), so adding or removing a line in an
  example crate invalidates every transcript in it. Change digits inside
  existing comment lines.
- **`book-pdf/src` belongs to the prose pass, not to code threads.** A code
  thread that learns something the manuscript needs appends to
  `book-pdf/chapter-notes.md` and moves on.
- `CLAUDE.md` has the standing rules; persistent memory notes point here.
  Deeper history: `STATUS.md` bottom-to-top.

**The fast checks, cheapest first:**

```
python3 output/regression/regress.py --suite unit    # 110 tests, ~2 s
bash output/regression/verify-transcripts.sh          # 22 chapters, every README transcript
python3 output/regression/regress.py                  # full, a few minutes
```

### Where the work stands (2026-08-04) — read this before proposing anything

Branch `rewrite-book`. **The restoration's code is done.** ch23–ch39 are
converted, run on Icarus, and are out of quarantine — phases, the ConfigDb, the
factory, the whole TLM layer, transactions, and all four sequence testbenches
(TB 7.0, 7.1, 7.2, 8.0). The only quarantined package left is `ch21_macros`,
which is a macro demonstration with no simulator test and is marked
`no_sim_test` in `regress.json`.

**The test suite is built** (`output/test-plan.md` is the plan and the
reasoning; its §8 records where the built suite differs). Three tiers under the
21 chapter runs:

| Tier | Where | What it is |
|---|---|---|
| no-simulator | `#[cfg(test)]` modules in `rustdv/` | 110 tests, `regress.py --suite unit`, ~2 s |
| targeted simulator | `rustdv/framework-tests/` | 38 tests in six named groups, plus `sim-mutation` |
| compile-fail | `rustdv/framework-tests/compile-fail/` | 5 cases, each asserting its `error[E….]` |

Regression: **239 entries, green**, and the pre-push hook runs all of it.
`output/regression/TESTING.md` is the operating manual, including the two
runner behaviours a simulator test has to know about (the phase survives a
test; vvp exits on an empty event queue).

The three things a new thread most needs to know, all in
`output/.design-decisions.md`:

- **D83b — connection is a trait method, not a registry.** A parent reaches an
  erased child's port through `ComponentNode::port_slot`, which works through
  `dyn` and therefore answers for a child slot and for `self` alike. A path-keyed
  registry was built first and struck: it could address a child but not the
  connecting component itself. If a mechanism works for a child but needs a
  second spelling for `self`, it has broken the UVM's uniformity — that is the
  tell.
- **D82b/D82c — concurrency and cancellation.** `RustdvComp` children are moved
  *out* of the parent for the run phase so a parent's `run` is concurrent with
  theirs, and each component races the objection-drained event *individually*.
  Racing the whole tree drops it mid-phase and destroys the components before
  extract/check/report can walk them — which showed up as a test passing with
  its scoreboard never running.
- **D90 — the analysis hub holds nothing.** `AnalysisBus` is a subscriber list;
  `write` calls each subscriber and returns, and a datum broadcast to nobody is
  gone. Storage belongs to the subscriber.

**Done 2026-07-29: the TinyALU refactor (D109).** `tinyalu_tb` was the last
thing running on the pre-restoration shape and now runs on phases, the ConfigDb,
the factory, `AnalysisBus` and a sequencer like every chapter, with two struct
tests that swap stimulus through the sequence factory. Behaviour is unchanged —
same counts, same simulated times, same log text, only the component path added.
It is now in the suite as `custom/sim-tinyalu-tb`, which it was not before.

**Done 2026-07-30: D112 and D108.** `tinyalu_tb`'s bare-DUT exception is
retired (`sim/hdl/tinyalu.sv` self-clocks, byte-for-byte the same file as
`output/examples/sim-common/hdl/tinyalu.sv` now) and both runner bugs are
fixed — they turned out to be one mechanism (a test ending on `read_only()`
starting the next test inside the same executor drain, before the phase
resets), not two. `rustdv_sim::phase::leave_read_only()` is the first thing
`run_one` does now; `fresh_phase()` is gone. Along the way, a framework test
(`clock_two_are_independent`) was found to be passing *because* of the bug
(Icarus was silently dropping a write that the fix now applies for real,
which exposed the test's own latent tie-break between two harmonic clocks —
fixed by changing what the test's measurement window is bounded by, not by
loosening its assertions). Full account: `output/.design-decisions.md` §36
(D108), STATUS.md's 2026-07-30 entries.

**Done 2026-07-30: the renumbering pass and the component attribute.** D110
settled the figure numbering (one sequence per chapter) and
`book-pdf/renumbering-spec.md` was applied to eight crates' `.rs` captions,
line-count-neutral. D114 (§42) removed the child attribute's argument:
`#[component(child)]`/`(fifo)`/`(sequencer)` are all bare `#[component]` now,
because the derive never read the word — a field's role is decided by its Rust
type.

## Done 2026-08-04 — the book is closed out, and three CI bugs are fixed

**The manuscript is finished and machine-checked.** Every `[TRANSCRIPT NEEDED]`
marker is filled from real simulator output; the `#[component]` sweep is applied
across all 17 manuscript files; five `Clock::new(...)` openings that D112 had
deleted are gone from ch18/19/20/40 and the Interlude, along with a
`start_of_simulation` phase and a ch40 paragraph that described a retired
bare-DUT exception. The end-to-end read is done. **Nothing is owed on the book.**

**Two new checks, both in the pre-push regression, both mutation-tested:**

| check | gates |
|---|---|
| `custom/readme-transcripts` | every transcript line in an example README *and in `book-pdf/src`* is what the simulator prints — 22 chapters, 549 lines in the book |
| `custom/book-listings` | every Rust listing in ch15–40 is real code from that chapter's crate (`book-sync` covers ch1–14 only, which is how Part II+ drifted unseen) |

Writing them found genuine errors: stale `file:line` in ch23/ch26, a ch24
transcript documenting one test where the crate has two, ch29/31/32 transcripts
that were paraphrases rather than output, and the five `Clock` openings.

**Three CI failures, three different root causes, all fixed:**

1. **The toolchain pin was ignored.** `rust-toolchain.toml` pins 1.97.0;
   `dtolnay/rust-toolchain@stable` exports `RUSTUP_TOOLCHAIN`, which overrides
   it. CI now reads the channel *from* the file and a guard step fails if
   `rustc --version` disagrees.
2. **ANSI colour defeated a text match.** GitHub sets `CARGO_TERM_COLOR=always`,
   so a compile-fail case's `error[E0277]` line began with an escape sequence
   and `grep -E '^error'` never matched — CI reported "compiled — no longer
   rejected" while printing the error underneath. The check now uses cargo's
   **exit status**; `regress.py` forces `CARGO_TERM_COLOR=never` and strips
   escapes.
3. **Verilator lint needed `--timing`.** D112's self-clocking `always #5 clk`
   makes a delay, and Verilator ≥5.020 refuses to lint a design with delays
   unless told how to handle them. The sandbox's 5.051 is lenient; Debian's
   5.020 is not.

**Regression: 239 entries, 0 failed**, and the transcripts are verified on both
Linux/aarch64 and macOS/arm64 under two different Icarus versions.


**The manuscript's rules still hold (D77).** `book-pdf/src` belongs to the prose
pass, not to code threads; its instructions are `book-pdf/FABLE.md` and
`book-pdf/chapter-notes.md`. A code thread that learns something the prose needs
appends to `chapter-notes.md`.
