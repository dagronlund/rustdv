# A Tour of This Repository

*New here — human or AI? This is the walk-around. Ten minutes, and you'll
know what this project is, what's been proven, and where everything lives.
Last verified 2026-07-28; the proven claims below are checked by the
regression suite, not aspirational.*

> **Active work: the UVM restoration.** A prior pass wrongly stripped the
> UVM's dynamic build/connect process and its TLM FIFOs. Branch
> `ch23_onwards` is restoring them, one TinyALU testbench version at a time;
> **ch23–ch34 are done and green** (phases, env, logging, ConfigDb, factory,
> and the whole TLM layer), with ch35 and the sequence chapters ch36–ch39 /
> TB 7.0–8.0 next. The framework and the Part II+ prose are under active
> revision — later chapters are being rewritten from working code, not
> settled. The authoritative decision log is `output/.design-decisions.md`
> (its §0 is the mission and method); read it and CLAUDE.local.md before
> proposing anything architectural. **"Where the work stands", at the bottom
> of this file, is the live status; this callout is the one-line version.**

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

- `sim/run_rustdv.sh` — the TinyALU regression ends `REGRESSION: PASS`
  (random_ops: 20 compared, 0 mismatches, full op coverage; max_ops: 4/0).
- **Mutation-checked**: with the DUT's XOR deliberately corrupted to OR,
  the scoreboard flags every affected transaction and the regression
  fails; restored, it passes. The checking has teeth.
- `output/regression/regress.py` — **223 passed, 0 failed** (book-sync 108,
  examples 96, custom 19), green on **both Linux and macOS/arm64**. The
  remaining sim chapters (ch35–ch39, the transaction and sequence chapters,
  plus ch21 which has no sim test) are quarantined in `regress.json` during
  the restoration and rejoin as each is converted.
- **The TLM layer, as of 2026-07-28** — ch31 (put/get/peek, the y = 2x²
  pipeline, FIFO analysis taps), ch32 (broadcast, and a slow subscriber that
  buffers for itself), and ch34 / TB 6.0 (the TinyALU testbench wired with
  `TlmFifo` and two `AnalysisFifo` buses) all run on Icarus.
- `cargo test --workspace` in `/rustdv` — pure-Rust unit tests for the
  testbench logic, no simulator required.

## Where everything lives

| You want | Look at |
|---|---|
| The framework | `/rustdv` (workspace: `rustdv-gpi-sys` → `rustdv-gpi` → `rustdv-sim` → `rustdv-methodology` → `rustdv`, plus `tinyalu_tb`) |
| The book manuscript | `/book-pdf/src` (TOC in `SUMMARY.md`); render with `mdbook build book-pdf` |
| Why it's designed this way | `output/.design-decisions.md` — the restoration's authoritative decision log (§0 = mission + method). **Do not** follow `output/.design-doc.md`: it is the pre-restoration specification whose closed-world design *caused* the problems now being fixed, kept only as the record of what went wrong. |
| Runnable book figures | `/output/examples` (`README.md` has per-chapter run commands) |
| The regression suite | `/output/regression/regress.py` (`--help` works; wired into pre-push) |
| Implementation history & honest deviations | `STATUS.md` (chronological, bottom-up) |
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
- **Fibonacci on the TinyALU** (chapter 37) — stimulus that needs the
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
- Book voice, if you edit chapters: the book addresses *both* UVM
  audiences (SystemVerilog and Python) — recap blockquotes open
  "**In the UVM...**"; see `book-pdf/dual-audience-style.md` for the rules.
  `// Figure N:` captions, output after `--`, and the chapter READMEs in
  `output/examples` map every figure to its runnable code.

---

## Notes for AI sessions

Context that matters to an AI working in this folder (via Claude Cowork
or similar) and to nobody else:

- **Never bulk-delete-and-recreate directories from the sandbox VM** —
  the desktop sync engine races and forks `dir 2/` duplicates. Build
  trees in `/tmp` and `cp` over; file deletion needs the permission tool.
- The Cowork VM has no network: toolchain comes from `toolchain-drop/`
  (extract from a `/tmp` copy — extracting off the mount is ~15× slower).
  mdBook is **not** installed in the VM; rendering happens on Ray's Mac.
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

### Where the work stands (2026-07-28) — read this before proposing anything

Branch `ch23_onwards`. The restoration has reached the end of the TLM work:
**ch23–ch34 are converted, run on Icarus, and are out of quarantine.**

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
- **D90 — the analysis hub holds nothing.** `AnalysisFifo` is a subscriber list;
  `write` calls each subscriber and returns, and a datum broadcast to nobody is
  gone. Storage belongs to the subscriber.

**Next up:** ch35 (transactions) and the sequence chapters ch36–ch39 / TB
7.0–8.0, which need D80's sequence factory (a second registry for non-component
objects). `tinyalu_tb` still runs on the pre-restoration `AnalysisPort` and is
the D75 retrofit. Two naming questions are parked for Ray and should not be
settled silently: **Q18** (caption code listings "Example N" rather than
"Figure N") and **Q19** (`AnalysisFifo` names storage on a thing that has none).

**The manuscript waits (D77).** Part II+ prose is written from working code by a
separate Fable pass; its instructions are `book-pdf/fable-brief.md`, and Fable
is forbidden from changing any code. Figure numbers therefore live in code
comments and are final on the code side.
