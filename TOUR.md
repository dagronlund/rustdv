# A Tour of This Repository

*New here — human or AI? This is the walk-around. Ten minutes, and you'll
know what this project is, what's been proven, and where everything lives.
Last verified 2026-07-23; the proven claims below are checked by the
regression suite, not aspirational.*

> **Active work: the UVM restoration.** A prior pass wrongly stripped the
> UVM's dynamic build/connect process and its TLM FIFOs. Branch
> `ch23_onwards` is restoring them, one TinyALU testbench version at a time;
> ch23–29 are done and green (phases, env, logging, ConfigDb, factory), with
> ch30 (TB 5.0) next. The framework and the Part II+ prose are under active
> revision — later chapters are being rewritten from working code, not
> settled. The authoritative decision log is `output/.design-decisions.md`
> (its §0 is the mission and method); read it and CLAUDE.local.md before
> proposing anything architectural.

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
- `output/regression/regress.py` — book-sync 108, examples 96, and the
  un-quarantined sim chapters (custom 12), green on **both Linux and
  macOS/arm64**. Later sim chapters (ch18–21, ch30–39) are quarantined in
  `regress.json` during the restoration and rejoin as each is converted.
- `cargo test --workspace` in `/rustdv` — pure-Rust unit tests for the
  testbench logic, no simulator required.

## Where everything lives

| You want | Look at |
|---|---|
| The framework | `/rustdv` (workspace: `rustdv-gpi-sys` → `rustdv-gpi` → `rustdv-sim` → `rustdv-uvm` → `rustdv`, plus `tinyalu_tb`) |
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
  and each call has a ~45 s budget — chunk accordingly.
- `CLAUDE.md` has the standing rules; persistent memory notes point here.
  Deeper history: `STATUS.md` bottom-to-top.
