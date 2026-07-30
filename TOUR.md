# A Tour of This Repository

*New here — human or AI? This is the walk-around. Ten minutes, and you'll
know what this project is, what's been proven, and where everything lives.
Last verified 2026-07-29; the proven claims below are checked by the
regression suite, not aspirational.*

> **Active work: the UVM restoration.** A prior pass wrongly stripped the
> UVM's dynamic build/connect process and its TLM FIFOs. Branch
> `ch23_onwards` restored them, one TinyALU testbench version at a time, and
> **the code is done: ch23–ch39 are green, the test suite is built on top, and
> the TinyALU refactor has landed — no known technical debt is left.** What
> remains is D108's two runner fixes and the prose. The Part II+ manuscript is
> stale by design and is rewritten from the working code by a separate pass
> (`book-pdf/FABLE.md`).
>
> **"Where the work stands", at the bottom of this file, is the live status.**
> `output/.design-decisions.md` is the decision log — read its §0 and
> CLAUDE.local.md before proposing anything architectural.

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
and [*Python for RTL Verification*](https://a.co/d/0hTKAJvh): 41 chapters,
an interlude, and three appendices in `book-pdf/src/` (mdBook), with Part II+
under revision as the restoration lands (see the callout above).
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
- `output/regression/regress.py` — **237 entries, 0 failed**, green on **both
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
- **The honest-gaps culture** — `STATUS.md` deviations, the design doc's
  Open Questions, chapter 41's missing-pieces inventory. What this
  project can't do yet is written down next to what it can.

## Rules of the road

- Keep the pre-push hook green: if you touch code or book figures, run
  `python3 output/regression/regress.py` before pushing.
- Transcripts in the book and READMEs are real output and must stay in
  sync with reruns — verify claims by running things.
- `../rustdv-reference` (outside the repo) is read-only. `/output` holds
  generated deliverables.
- **Do not edit `book-pdf/src` from a code thread.** The manuscript is stale by
  design and is rewritten in one dedicated prose pass; its rules are
  `book-pdf/FABLE.md`, and that pass changes no code. A code thread records
  what the prose will need — in `chapter-notes.md` — and moves on.
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
- `CLAUDE.md` has the standing rules; persistent memory notes point here.
  Deeper history: `STATUS.md` bottom-to-top.

### Where the work stands (2026-07-29) — read this before proposing anything

Branch `ch23_onwards`. **The restoration's code is done.** ch23–ch39 are
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

Regression: **237 entries, green**, and the pre-push hook runs all of it.
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

**Next up, in the order Ray set:**

1. **The two runner behaviours — both now decided as bugs to fix (D108).** The
   simulator phase must stop surviving a test (the fix goes in `run_one`'s
   per-test reset, and `fresh_phase()` then goes away), and a `Clock` must stop
   writing inside a ReadOnly callback — the pattern that provokes it is the
   monitor pattern the book teaches. The second is the deeper one: writes
   scheduled from inside a ReadOnly callback have to be deferred to a region
   that permits them. Both land before release.
2. **A numeric renumbering pass, after the prose.** Q18 is settled (D110): one
   figure sequence per chapter, everything in it called a "Figure", nothing
   renamed. A drawing or table the prose pass inserts ahead of a listing shifts
   the captions after it, so it owes a `renumbering-spec.md` and a mechanical
   pass applies it — in place and line-count-neutral, because transcripts embed
   `file:line`.
3. **`#[component(fifo)]` names a type, not a role.** A child exempt from factory
   override gets its own attribute per type — `fifo`, then `sequencer` — and
   `AnalysisBus` is declared `#[component(fifo)]` while being no such thing. One
   role word for all of them, or per-type spellings recorded as the design.
   Ray's call, deferred to whoever next touches those declarations (D106's tail).

**The manuscript waits (D77).** Part II+ prose is written from working code by a
separate prose pass that **changes no code**; its instructions are
`book-pdf/FABLE.md` plus `book-pdf/chapter-notes.md`. Figure numbers live in
code comments and are final on the code side, which is why Q18 runs as a
request-then-rename rather than an edit.
