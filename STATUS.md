# STATUS — rustdv implementation sprint

**Date:** 2026-07-11 — **COMPLETE: REGRESSION PASSES ON ICARUS**
**Scope:** complete rustdv code base per `output/design-doc.md`, demonstrated
against the TinyALU (`sim/hdl/tinyalu.sv`) on Icarus Verilog.
**Environment:** Path B (offline toolchain drop into `toolchain-drop/`):
rustc/cargo 1.97.0 aarch64-linux + oss-cad-suite 2026-07-11 (Icarus 14.0
devel), both installed to the VM home directory.

## Build & run

```sh
cd sim && ./run_rustdv.sh          # expect "REGRESSION: PASS"
cd rustdv && cargo test            # pure-Rust unit tests, no simulator
```

- [x] `cargo build -p tinyalu_tb` (debug and release) — clean
- [x] `cargo test --workspace` — 8/8 unit tests pass, no simulator needed
- [x] `sim/run_smoke.sh icarus` → SMOKE: PASS
- [x] `sim/run_rustdv.sh` → **REGRESSION: PASS**
      - `random_ops`: 20 ops driven through the full sequencer handshake,
        scoreboard 20 compared / 0 mismatches, coverage Add=5 And=5 Mul=5
        Xor=5, 625 ns sim time
      - `max_ops`: 4 compared / 0 mismatches, all ops covered, 185 ns
      - xUnit XML written to `sim/build/results.xml`
- [x] **Mutation check (verification of the verification):** with the DUT's
      XOR sabotaged to OR, the scoreboard flags every affected transaction,
      both tests report FAILED with per-transaction diagnostics, and the
      run ends `REGRESSION: FAIL`. The checking is not vacuous.

## What was built

Cargo workspace at `rustdv/` per design-doc §2, zero external dependencies:

| Crate | Contents | Design-doc |
|---|---|---|
| `rustdv-gpi-sys` | hand-written VPI FFI subset | §2 D2.1 (deviated, D1 below) |
| `rustdv-gpi` | safe handles/values/callbacks/time; all `unsafe` lives here; RAII `CallbackHandle` | §3.3 |
| `rustdv-sim` | bespoke single-thread executor, 7-state tasks, drop-based cancel, Timer/edges/ReadOnly/ReadWrite/NextTimeStep, buffered writes drained at ReadWrite, Event, FIFO-fair Lock, Queue, first!/join!/with_timeout, Clock, SplitMix64 Rng, sim-time logger | §4 |
| `rustdv-uvm` | Component lifecycle trait + ComponentNode traversal, RAII objections, channel/Sender/Receiver, AnalysisPort/Subscriber/AnalysisFifo, TlmFifo, full sequencer handshake (SeqItem envelope, SeqCtx, SeqItemPort, ResponseQueue) | §5 (R1–R6 revision) |
| `rustdv-macros` | `#[rustdv::test]` (link-section registration), `#[derive(Component)]` (`T`/`Option<T>`/`Vec<T>` children) | §6 |
| `rustdv-runner` | link-time test registry + sentinel, sequential regression, timeouts, expect_fail/skip, seed handling, summary table, xUnit XML, VPI bootstrap | D2.4, §4.5 |
| `rustdv` (facade) | prelude + `vpi_bootstrap!()` | D2.5 |
| `tinyalu_tb` | §7 worked example: transactions (std derives), TinyAluBfm, Driver, CmdMonitor, ResultMonitor, Scoreboard (predict + compare in check), Coverage (Subscriber), AluEnv (ownership tree, Option children), RandomSeq/MaxSeq (late generation at grant), tests `random_ops` + `max_ops` | §7 |

Simulation flow: `sim/run_rustdv.sh` builds the testbench cdylib, copies it
as a `.vpi` module, compiles the DUT with `sim/hdl/timescale.v` (1ns/1ns),
and runs `vvp -M build -m tinyalu_tb`.

## Deviations from the design doc

**D1 — VPI backend instead of cocotb's GPI library (§3.1 D3.1, §3.2 D3.2).**
The sandbox cannot build cocotb's C++ GPI (no cocotb build tooling) and the
zero-dependency constraint rules out bindgen. `rustdv-gpi-sys` therefore
binds the IEEE 1800 VPI C API directly (hand-written subset of
`vpi_user.h`) — the same API cocotb's GPI wraps for Icarus. The safe layer
(`rustdv-gpi`) keeps the GPI-shaped surface from §3.3 (opaque non-null
handles, `Result` acquisition, copied strings, no unwinding across FFI,
one-shot-vs-recurring callback ownership in the type), so swapping in real
`gpi.h` bindings later is contained to the `-sys` crate. Bootstrap is a
plain VPI module (`vlog_startup_routines` via `rustdv::vpi_bootstrap!()`)
instead of the libpygpi entry-symbol scheme. Consequence: v0 runs on
VPI simulators (Icarus); VHPI/FLI arrive with the real GPI reuse (OQ-1/OQ-2
stand).

**D2 — Trigger lifecycle simplified (§4.3/§4.4).** Instead of shared
trigger objects with subscribe/unsubscribe lists and lazy prime/unprime,
each awaited trigger future registers its own VPI callback on first poll
and removes it via RAII on drop. User-facing API is per design
(`sig.rising_edge().await`, `Timer::ns(2).await`); the shared-subscriber
optimization (one VPI callback per signal) is future work. Phase triggers
(ReadOnly/ReadWrite/NextTimeStep) *are* hub-managed singletons per design,
including the write-buffer drain ordering and illegal-transition checks.

**D3 — Proc macros without syn/quote (§6).** Zero-dependency constraint
(Path B: no crates.io). `#[rustdv::test]` and `#[derive(Component)]` are
hand-written token parsers with narrow supported grammar (plain async fns;
non-generic structs with named fields). Link-time registration uses the
ELF `__start_/__stop_` section technique directly (what linkme does),
Linux-only for now — OQ-4's platform caveats apply; the explicit-
registration fallback is the sentinel pattern in `rustdv-runner`.

**D4 — Logging is a minimal built-in (`OQ-8`).** The `tracing` crate
mapping is deferred (no external deps). A small sim-time-stamped logger
reproduces the book's `  2.00ns INFO ...` format with level filtering.

**D5 — `#[rustdv::parametrize]` not implemented (§6.2, OQ-10).** Neither
TinyALU test needs it; the data-driven-loop idiom covers the demo. Planned
follow-up.

**D6 — TinyALU env has no agent layer (§5.4 example).** The testbench
follows the book's 6.0 architecture (§7.2: driver/monitors/scoreboard/
coverage directly in the env). `Active`/`Option<Driver>` and
`enable_coverage`/`Option<Coverage>` still demonstrate the
conditional-children pattern the agent chapter needs.

**D7 — Timeout diagnostics.** On test timeout the runner reports the
timeout and kills surviving tasks, but does not yet print the objection
table (the per-test `RunCtx` lives inside the test body, invisible to the
runner). pyuvm's richer report is future work.

**D8 — `bridge`/blocking-world integration (OQ-7), `wait_modified`,
transport/master/slave composites, grab/lock/priority arbitration,
pack/unpack/recording/policies** — not ported, matching the design doc's
own [gap] list.

**D9 — `rustdv-vpi-stubs` dev-dependency crate (build-flow addition).**
Unit-test *executables* link the whole crate graph, and unlike the cdylib
they cannot carry undefined `vpi_*` symbols. Crates with unit tests take
`rustdv-vpi-stubs` as a dev-dependency (`#[cfg(test)] use ... as _;`): it
defines panicking stubs so test binaries link. It is never linked into the
`.vpi` module, where the real symbols come from the simulator process.
(A first attempt via linker flags — `-z lazy` +
`--unresolved-symbols=ignore-all` — corrupted aarch64 PLT relocations and
was abandoned; `rustdv/.cargo/config.toml` is intentionally empty.)

## Fix log (build/sim loop)

1. `Join2::poll` needed `unsafe get_unchecked_mut` (output types may be
   `!Unpin`; the inner futures are boxed, so this is sound).
2. Test-binary linking → D9 above.
3. Icarus rejects `vpiSuppressVal` on value-change callbacks ("value
   format 10 not supported"); callback registration now passes a NULL
   value pointer (closures read signals themselves).
4. `Executor::cancel_after` off-by-one: watermark comparison must be `>=`
   so a timed-out test's own task is killed (caught in self-review).

That was the entire loop — four fixes from first compile to passing
regression.

## Open items

- Shared-subscriber trigger optimization (D2) and the runner-side
  objection dump on timeout (D7) are the two nearest-term improvements.
- Portability beyond Linux/ELF for the link-section test registry (OQ-4)
  and beyond VPI/Icarus for the backend (D1/OQ-1/OQ-2) are the two
  structural follow-ups before the book can claim multi-simulator support.

## Re-verification — 2026-07-13 (fresh VM)

Independent end-to-end rerun in a new session/VM, from the Path B
toolchain drop. No source changes were needed; results reproduce the
2026-07-11 run exactly.

- Toolchain reinstalled from `toolchain-drop/`: rustc/cargo 1.97.0
  aarch64-linux, Icarus 14.0 (devel, s20260301). Install note: extract
  the tarballs from a VM-local copy (`cp` to `/tmp` first) — extracting
  directly from the mount is ~15× slower and exceeds shell timeouts.
- [x] `sim/run_smoke.sh icarus` → SMOKE: PASS
- [x] `cargo build --workspace` — clean; `cargo test --workspace` — 8/8
- [x] `sim/run_rustdv.sh` → **REGRESSION: PASS** (`random_ops` 20/0
  mismatches, coverage Add=5 And=5 Mul=5 Xor=5, 625 ns; `max_ops` 4/0,
  all ops, 185 ns); fresh `sim/build/results.xml` committed
- [x] Mutation check repeated (XOR→OR at `tinyalu.sv:48`): scoreboard
  flags all 5 + 1 affected transactions, both tests FAIL, run ends
  `REGRESSION: FAIL`; DUT restored and clean run re-confirmed PASS

## Book completion — 2026-07-13

The manuscript (`book-pdf/src/`) is now **complete**: chapters 15–41 plus
Appendices A (chapter map) and B (idiom translations) written, SUMMARY.md
updated. Every figure in the new chapters runs: 20 new sim-chapter example
crates under `output/examples/` (each a cdylib whose `#[rustdv::test]` fns
are the chapter's figures, ending `REGRESSION: PASS` on Icarus), plus
pure-Rust bins and compile-fail figures in the Part I style. Shared
testbench code (the Rust `tinyalu_utils`, testbench versions 2.0–8.0)
lives in `output/examples/tinyalu-utils/`.

Regression: `output/regression/regress.py` — book-sync now scoped to
Part I (Part II+ figures live inside sim crates), `custom/sim-ch15..39`
tests run every sim chapter, examples suite unchanged. Full run on this
VM: 107 book-sync + 95 examples + 20 sim chapters + smoke, all green.

Library changes made for the book (all regression-verified):
- **rustdv-sim**: `Debug` for `LogicHandle`/`HierarchyHandle`;
  `#[must_use]` on Timer/Edge/NullTrigger (ch17's "forgot the await"
  warning); `log::critical`; hierarchical per-target log levels
  (`Logger`, `set_level_for`) and a file handler (`log_to_file`) — ch26.
- **rustdv-gpi(-sys)**: SV variable vpiTypes (610–620) classify as logic
  signals (counter.sv's `byte unsigned` port).
- **rustdv-uvm**: `print_hierarchy` exported in facade/prelude; `Debug`
  on `TlmFull`.
- **rustdv-macros**: `#[derive(Component)]` now supports generic structs
  and fields with generic types containing commas.
- **tinyalu_tb / examples BFMs**: `wait_idle` hardened to require two
  consecutive idle edges (fixed a race that could end a test with the
  last command undriven; interlude transcript timings updated 625→635,
  810→830 ns accordingly).

Not done in the VM: regenerating `book-pdf/book/` (HTML/PDF) — mdBook
isn't available offline here. `mdbook build book-pdf` on the host (or the
docker flow) will rebuild the rendered book from the updated sources.

### macOS portability fixes — 2026-07-13

Ray's pre-push regression on macOS exposed two Linux-isms in the new
example code; both fixed, Linux sweep re-verified green:

- **VPI cdylibs wouldn't link on Mach-O** (undefined `vpi_*` symbols are
  a load-time feature on Linux, an error on macOS). Added
  `-undefined dynamic_lookup` for the two Apple targets in
  `output/examples/.cargo/config.toml` and `rustdv/.cargo/config.toml`
  (target-scoped: Linux builds untouched, D9's caveat still respected).
  `sim-common/run_sim.sh` and `sim/run_rustdv.sh` now fall back from
  `lib*.so` to `lib*.dylib`.
- **ch21 fig04 (ELF `__start_/__stop_` section demo)** is inherently
  Linux-only; the registration/collect machinery is now
  `#[cfg(target_os = "linux")]` with a non-Linux `main` that says so.
  Behavior on Linux unchanged.
- **The test registry itself (OQ-4) now has Mach-O spellings.** rustc
  rejects free-form `link_section` names on Apple targets, so the
  `#[rustdv::test]` macro and the runner's sentinel emit
  `__DATA,rustdv_tests` under `cfg(target_vendor = "apple")`, and
  `collect_tests` reaches the section bounds through
  `section$start$/section$end$` link_names (the linkme technique).
  ELF path is byte-for-byte what it was; Linux workspace tests +
  regression re-verified green. OQ-4's remaining platform is Windows.
