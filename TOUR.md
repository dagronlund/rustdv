# A Tour of This Repository

*New here — human or AI? This is the walk-around. Ten minutes, and you'll
know what this project is, what's been proven, and where everything lives.
Last verified 2026-07-13; everything below is checked by the regression
suite, not aspirational.*

## What this project is

**rustdv** is a hardware verification framework in Rust — the cocotb +
pyuvm story retold with a compiler: a simulator-driven async executor,
triggers, a UVM-style component methodology (ownership tree, typed
configs, maker-closure factories, channels/analysis ports, the full
sequencer handshake), running testbenches as native shared libraries
loaded by Icarus Verilog over VPI. Zero external dependencies. The crate
name `rustdv` is registered on crates.io (0.0.1 placeholder).

**"Rust for RTL Verification"** is its book — the sequel to Ray Salemi's
[*Python for RTL Verification*](https://a.co/d/0hTKAJvh): 41 chapters, an 
interlude, and two appendices, complete in `book-pdf/src/` (mdBook). 
The premise: the reader knows the Python book and learns Rust chapter by
chapter while rebuilding the TinyALU testbench, versions 1.0 through 8.0.
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
- `output/regression/regress.py` — 107 book-figure sync checks, 95
  example checks, and 20 sim-chapter runs, green on **both Linux and
  macOS/arm64**.
- `cargo test --workspace` in `/rustdv` — pure-Rust unit tests for the
  testbench logic, no simulator required.

## Where everything lives

| You want | Look at |
|---|---|
| The framework | `/rustdv` (workspace: `rustdv-gpi-sys` → `rustdv-gpi` → `rustdv-sim` → `rustdv-uvm` → `rustdv`, plus `tinyalu_tb`) |
| The book manuscript | `/book-pdf/src` (TOC in `SUMMARY.md`); render with `mdbook build book-pdf` |
| Why it's designed this way | `/output/.design-doc.md` — section-numbered, every decision cited to cocotb/pyuvm/the book |
| Runnable book figures | `/output/examples` (`README.md` has per-chapter run commands) |
| The regression suite | `/output/regression/regress.py` (`--help` works; wired into pre-push) |
| Implementation history & honest deviations | `STATUS.md` (chronological, bottom-up) |
| AI verification skills | `/skills` (spec+RTL → testbench → verified coverage report) |
| Upstream sources | `/reference` — **read-only** (cocotb, pyuvm, the Python book) |

## Highlights worth your first half hour

- **The Interlude** (`book-pdf/src/interlude-tinyalu-testbench.md`) — the
  complete testbench, presented before the climb. The best single answer
  to "what does rustdv code look like?"
- **The compile-error figures** (`output/examples/*/compile-fail/`) — the
  book's running theme is that the compiler finds testbench bugs before
  the simulator runs: a TLM mis-connection is `E0308`, a config conflict
  is `E0062`, a typo'd config field comes back with the fix suggested.
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
- `/reference` is read-only. `/output` holds generated deliverables.
- Book voice, if you edit chapters: "In Python we..." openers,
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
