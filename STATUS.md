# STATUS — rustdv implementation sprint

**Date:** 2026-07-11 — **COMPLETE: REGRESSION PASSES ON ICARUS**
**Scope:** complete rustdv code base per `output/.design-doc.md`, demonstrated
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

With these fixes the **full pre-push suite passes on macOS/arm64**
(Ray's machine, 2026-07-13) as well as Linux — rustdv and all 20 book
sim chapters now run on two platforms.

---

## 2026-07-21 — the UVM restoration, step 4: `RustdvCtx` and the test macro

Branch `ch23_onwards`. Decisions and reasoning are in
`output/.design-decisions.md` (D46–D49, which strike D8); this is the
implementation record.

**A test is now a component.** `Component` gained
`async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError>`,
and `#[rustdv::test]` accepts a struct or type alias as well as a free
`async fn`. Chapter 23's aspirational example — written before the
framework could compile it — now compiles and runs unmodified except for
one `&`.

- **`TestCtx` (runner) + `RunCtx` (uvm) → `RustdvCtx` (uvm).** One
  universal context: `dut()`, `seed()`, `rng()`, `path()`, the objection
  registry, and path-aware `info()`/`warning()`/… . It is `Clone` with the
  objection registry `Rc`-shared.
- **`TestError` moved from `rustdv-runner` to `rustdv-uvm`**
  (`rustdv-uvm/src/error.rs`), since `Component::run` returns it and the
  runner sits above the UVM crate. Re-exported from the runner and the
  facade, so `::rustdv::TestError` is unchanged.
- **Deviation — `async fn` in a trait is not dyn-compatible.** Adding `run`
  to `Component` broke `&mut dyn ComponentNode`, whose supertrait it was.
  The synchronous phases are therefore mirrored onto a new dyn-safe
  `DynPhases` trait (blanket impl over every `Component`, distinct method
  names so `component.extract()` stays unambiguous), and `ComponentNode`
  requires that instead. Users write `Component` exactly as before.
- **`Component::start` survives, transitionally.** ch24+ and `tinyalu_tb`
  still use the old spawn hook; ch24 folds it into `run`.
- **The runner** builds `RustdvCtx` with the test's registered name as the
  root path (so logs read `[HelloWorldTest]`, not UVM's `uvm_test_top`),
  runs the test, then awaits objection consensus — guarded on
  `ObjectionRegistry::ever_raised()`, so a cocotb-shaped test that never
  objects does not trigger pyuvm's "you never objected" warning.
- **`#[derive(Component)]` learned unit structs** (`struct HelloWorldTest;`).
- **`tinyalu_utils` is infrastructure only** (D45): `tb2`, `tb4`, `tb6`,
  `tb7`, `env7`, `bfm7`, `alu_item` are no longer compiled. The files stay
  in `src/` so each chapter can lift its copy into the chapter file as it
  converts; ch23 has done so.
- **Mechanical rename** of `TestCtx`/`RunCtx` → `RustdvCtx` across 49
  files: examples, `tinyalu_tb`, `getting-started-with-rustdv.md`, and the
  book chapters. No `file:line` moved, so no transcript regeneration (D31).

**Verified.**

- `output/examples/sim-common/run_sim.sh ch23_uvm_test_testbench_3_0 tinyalu …`
  → 3/3 tests, `REGRESSION: PASS` (seed 1). Transcript in the chapter README.
- `regress.py --suite book-sync` → 108/108; `--suite examples` → 96/96;
  `--suite custom` → 6 passed, 0 failed (sim-ch15/16/17 and the new
  sim-ch23 among them; ch18+ still quarantined).
- `cargo build` and `cargo test` clean in the `rustdv` workspace,
  `cargo build -p tinyalu_tb` clean.
- `tinyalu_utils` and `ch23_uvm_test_testbench_3_0` removed from the
  `regress.json` quarantine block.

**Open, recorded rather than guessed:** struct tests register under their
*type* name (`RandomTest`), not the snake_case `random_test` D27 wrote —
see Q15 in the decisions log. Deciding it before ch24 is free; after ch24
it is a transcript regeneration.

## 2026-07-28 — TLM lands: ports, exports, analysis, and ch31/32/34 un-quarantined

The TLM layer is built and three chapters came out of quarantine. Full
regression: **223 passed, 0 failed** (book-sync + examples + custom, with
ch36–ch39 still quarantined); `cargo test --workspace` clean.

**Connection is a trait method, not a registry (D83b).** The path-keyed port
registry designed on 2026-07-24 was built, and then struck the same week: it
could address a child but not the connecting component itself, because a
component does not know its own path (D7). Ray's correction — *"I guess you now
see why `uvm_test` is a `uvm_component`"* — is that the UVM's uniformity is
load-bearing. `ComponentNode::port_slot(name) -> Option<Rc<dyn Any>>`, generated
by the derive, is reachable through `dyn` and so answers for an erased child and
a concrete `self` alike. The registry, the thread-local store, the per-test
clear and the walk-time path stamping all went away with it; `RustdvPath`
(D83a) stays, since the logger uses it.

**What is new in the framework**

- `rustdv-methodology/src/port.rs` — `PutIf`/`GetIf`/`PeekIf`/`PublishIf`,
  `Port<I>` with the `PutPort`/`GetPort`/`PeekPort`/`PublishPort`/
  `SubscribePort` aliases, `PortName<I>` (carries the *interface*, so a `get`
  export aimed at a `put` port is a compile error), `PortOwner`, `PortField`.
- `rustdv-methodology/src/fifo.rs` — `TlmFifo` rebuilt on a shared inner:
  `put_export()`, `get_export()`, `peek_export()`, and the D23 analysis taps
  `put_ap()`/`get_ap()`. `TlmFifo::new(usize)` replaced `new(Option<usize>)`;
  `unbounded()` is the other constructor.
- `rustdv-methodology/src/analysis.rs` — `AnalysisFifo` is now the broadcast
  hub (renamed `AnalysisBus` on 2026-07-29, D103 — read it as that
  below): `pub_export()` and `sub_export()`. (This bullet first read
  "…, `get_export()`", written before D90 struck the hub's queue later the same
  day; the deviations list below is the correct record. Corrected 2026-07-28.)
  The pre-hub `AnalysisPort`/`Subscriber` stay for the Part IV testbenches not
  yet rebuilt.
- `rustdv-methodology/src/shared.rs` — `RustdvShared<T>`, the state a
  subscriber shares with its port so `write` can be synchronous (D87/D88).
- `rustdv-sim/src/queue.rs` — `peek`/`try_peek` (need `T: Clone`),
  `has_space`, and `wait_for_space` (the FIFO taps need the wait and the
  handover separated, since `try_put` takes ownership).
- `#[derive(Component)]` — `#[port(put|get|peek|publish|subscribe)]` fields
  generate the port lookup, the elaboration report, a typed `PORT_NAME`
  constant, and a `PortOwner` impl. `#[component(fifo)]` is recognised as a
  child.
- Elaboration sweeps the tree and reports **every** unconnected required port
  at once, classified `tlm_unconnected_port` so a test can assert on it.

**A silent bug this work exposed (D82c).** `run_component_test` raced the whole
`run_all` future against the objection-drained event. Losing a race means being
dropped — so at consensus the entire tree's future was dropped, taking with it
the children D82b moves *out* of their slots, and extract/check/report then
walked a tree whose components had been destroyed. ch34 "passed" with its
scoreboard never running. Each component now races the drained event
individually (`run_one`), so every level returns normally and restores its
children.

**Chapters un-quarantined** (all `REGRESSION: PASS` on Icarus, seed 1;
transcripts in each README):

- **ch31** — 5 tests, including the y = 2x² pipeline (a parent's `run`
  concurrent with its children's) and the FIFO taps.
- **ch32** — 3 tests. The broadcast transcript is at `0.00ns`, which is the
  point; the slow-subscriber test then shows writes at `0.00ns` and checks at
  5/10/15ns, which is the other point.
- **ch34 / TB 6.0** — the TinyALU testbench wired with `TlmFifo` and two
  `AnalysisFifo` buses; scoreboard checks all four ops, coverage sees 4 of 4.

**Deviations recorded rather than hidden.**

- `PutPort::try_put` returns `Result<(), T>` and `try_get` returns `Option<T>`,
  not the UVM's bit — `put` takes ownership, so a refusal that kept the item
  would lose it (D89). Found because ch31 Figure 4 demonstrated it over a `u32`,
  where the tempting `while port.try_put(n).is_err()` compiles *because `u32` is
  `Copy`* and breaks on the reader's first real transaction. Figures 4–6 now
  carry a non-`Copy` `Packet` and thread the item back through `Err`. **Rule for
  every future figure: demonstrate an ownership property with a non-`Copy`
  type**, or the compiler stops being the check the book claims it is.
- ch34's Tester holds its objection for twenty clocks after its last `put`,
  where the Python testbench waits ten — the multiply is last and takes longest
  to come back. Without the wait the scoreboard silently checks fewer results
  than it saw commands.
- `AnalysisFifo` holds **nothing** — no queue, no `get_export()` (D90, Ray).
  It broadcasts and returns; a datum nobody subscribed to is lost. An earlier
  build gave it a queue that switched on when `get_export()` was called, which
  was invisible state and left `try_get()` returning `None` forever. Storage now
  belongs to the subscriber: `AnalysisPort::connect_fifo()` hands back an
  unbounded `TlmFifo`. `tinyalu_tb`'s scoreboard fields changed type
  accordingly. ch32's Figure 6 was cut: the UVM's analysis FIFO in a scoreboard
  is a workaround for one `write` per class (`uvm_analysis_imp_decl`), and
  rustdv's two ports and two `WriteSink`s mean there is nothing to work around
  (Ray). **Book obligation recorded:** the subscriber-owns-the-storage pattern
  is new and must be taught — see D90 and the Fable brief. ch32 Figures 6–7 were
  then written to give that teaching runnable code, motivated the honest way: a
  subscriber whose work takes simulation time, since `write` cannot await. ch32
  is seven figures, three tests.

## 2026-07-29 — the test suite: three tiers under the chapter runs

`output/test-plan.md` is the plan and the reasoning; §8 of it records where
the built suite differs from the plan as written. What landed:

- **110 no-simulator tests** (was 12), in `#[cfg(test)]` modules beside the
  code they test, run first by `regress.py --suite unit` in about two seconds.
  The methodology layer — ConfigDb, factory, ports, FIFO, analysis bus,
  objections, the phase walk, the whole sequencer handshake — had no unit test
  at all before this; a bug there surfaced as "some chapter went red" and the
  bisect was manual. `rustdv_sim::testing::block_on` is what makes it possible
  and is also the enforcement: a future still pending when the run queue
  empties panics with an explanation, so a test that quietly needed a
  simulator fails loudly instead of hanging.
- **38 targeted simulator tests** in `rustdv/framework-tests/`, against a new
  `hdl/probe.sv`. One cdylib, six regression entries, selected by a new
  `RUSTDV_TESTCASE` filter in the runner (cocotb's `TESTCASE`, widened to
  case-insensitive substrings; a filter matching nothing is an error, not a
  green run of zero tests). `test.json` grew an `"env"` key to drive it.
- **`sim-mutation`**: the TinyALU's XOR is corrupted to an OR, two testbenches
  are required to fail, and then required to pass again on the real RTL. The
  second half is the part that matters — without it a testbench that had
  stopped compiling would "catch" every mutation.
- **5 compile-fail cases** for the methodology layer, each asserting its
  `error[E….]` code. One of them was vacuous when first written: `let _ =
  cmd.a` after `finish_item(cmd)` **compiles**, because `let _` does not
  evaluate a place expression. The use-after-move only appears if the value is
  really read. That is the argument for asserting the code rather than the
  failure.

Regression: 236 entries, green — 228 before, plus the six targeted groups,
`sim-mutation` and `compile-fail-methodology`.

### Two runner behaviours the tests had to work around

Recorded rather than fixed — both want a decision (test-plan §8).

- **The simulator phase survives a test.** A test ending inside ReadOnly
  leaves the next one starting there, where a write panics on the cocotb rule,
  so a test can pass or fail depending on what ran before it.
  `framework_tests::fresh_phase()` steps out of it. The per-test reset in
  `run_one` — which already clears the ConfigDb and the logging config — is
  where a real fix would go.
- **A clock's write can land inside a ReadOnly callback.** `read_only().await`
  runs the executor from within the ReadOnly callback; a `Clock` task due in
  the same step then writes, and Icarus prints "attempted to put a value to
  variable 'clk' during a read-only synch callback". A diagnostic, not a
  failure — but the pattern that provokes it is await-an-edge-then-`read_only`,
  which is the monitor pattern the book teaches.

## 2026-07-29 — the TinyALU refactor: the last technical debt is gone

`rustdv/tinyalu_tb` was the one thing left running on the pre-restoration
shape — `AluEnv::new(config)` with the BFM and every TLM endpoint injected
through constructors, `#[component(no_factory)]` on all five components, the
legacy `AnalysisPort`, work started from `start` as spawned tasks, and a
hand-rolled `start_all` + `run_extract_check_report` in place of the phaser. It
is now the same testbench the book's chapters teach (D109):

- **Phases.** `AluEnv::build` creates the children and `AluEnv::connect` wires
  them; the tests are `#[rustdv::test]` **structs**, so the runner's phaser
  drives them and there is no hand-rolled phasing anywhere in the crate.
- **The BFM comes from the ConfigDb** (D101), filed once by `BaseTest::build`.
  `TinyAluBfm` gained a hand-written `Debug` for `ConfigDb::dump`.
- **Every component is factory-built** with no constructor arguments —
  `Driver::create_comp()` and friends — so all five `no_factory` opt-outs came
  off. The children are `RustdvComp` slots.
- **TLM through `port_slot`** (D83b): one `AnalysisBus` per stream, a
  `#[component(sequencer)]` sequencer, and connect lines with the same shape as
  ch34's.
- **The subscriber owns the storage** (D90). The scoreboard holds two
  `RustdvShared` logs behind two `SubscribePort`s and two `WriteSink` impls;
  coverage is a third subscriber on the command bus. No `AnalysisPort`, no
  `connect_fifo`.
- **Work happens in `run`**, concurrently (D82), instead of in tasks spawned
  from `start`. `spawn` is left where it belongs: the BFM's own collector loops,
  started in the env's `start_of_simulation`.
- **Stimulus is a program, swapped by the factory.** Two tests, `RandomTest` and
  `MaxTest`, each `set_seq_override::<BaseSeq, _>()` and then share one
  `BaseTest` body that runs `create_seq::<BaseSeq>()`. The old `random_ops` /
  `max_ops` free functions are gone; a struct test registers under its type name
  (D102), which is why the transcript now says `RandomTest`.

**Behaviour is unchanged, and that was the check.** Same counts, same simulated
times, and the log text of every line is the same: RandomTest 20 compared / 0
mismatches with `Add=5 And=5 Mul=5 Xor=5`, at 635.00ns; MaxTest 4 / 0 with
`Add=1 And=1 Mul=1 Xor=1`, at 195.00ns. The only difference in the transcript is
that each line now carries the component's path — `[RandomTest.inner.env.scoreboard]`
— because the framework supplies it instead of the component hand-typing a label
into the message (D49/D62).

**Two things found while doing it.**

- **The scoreboard still has teeth.** Corrupting the XOR to an OR in
  `sim/hdl/tinyalu.sv` makes it report five mismatches and fail the test. Worth
  recording because the storage moved (hub → subscriber) and a scoreboard that
  quietly stopped receiving would have looked identical to one that passed.
- **`sim/hdl/tinyalu.sv` is the bare DUT and takes `clk` in**, unlike the book's
  copy under `output/examples/sim-common/hdl/`, which self-clocks. So this
  testbench drives the clock and the chapters do not (D42). It is in
  `BaseTest::start_of_simulation` with the reason written next to it.
  **Retired 2026-07-30 — see D112 in the decision log.** The exception closes:
  `tinyalu_tb`'s DUT becomes self-clocking and the `Clock::new` line comes out.
  This is also what narrows D108's bug 2 down to ch17's taught `Clock` idiom
  and `framework-tests/hdl/probe.sv`. Transcript timings moved 5ns earlier
  (RandomTest 635→630ns, MaxTest 195→190ns) because the software `Clock`
  started high and the self-clocking RTL starts low; counts and coverage are
  unchanged and `custom/sim-tinyalu-tb` doesn't assert timing, so nothing else
  needed updating.

**The shipped testbench is now in the regression** as `custom/sim-tinyalu-tb`.
Until today nothing in the suite ran it: `cargo test --workspace` proved it
compiled, and "the TinyALU regression passes" rested on a human running
`sim/run_rustdv.sh`. The entry asserts `REGRESSION: PASS` plus the three counts
above, so a scoreboard that stopped comparing fails instead of passing quietly.
Regression: **237 entries** (unit 1, book-sync 108, examples 96, custom 32),
green, `MUTATION: CAUGHT`.

## 2026-07-30 — D108: both runner bugs, and a test that was passing because of one of them

Both landed together — they turned out to be one mechanism, not two. Full
reasoning and the two rejected approaches are in `output/.design-decisions.md`
§36 (D108); this is the short version.

`rustdv_sim::phase::leave_read_only()` (new) is the first thing `run_one`
does: a no-op unless the simulator is currently in the ReadOnly region,
otherwise one precision step's wait to leave it (fix 1, the phase leak).
`deny_write_in_read_only()` (new, factored out of `schedule()`) is now called
by `set_u64_now`/`set_now` (`rustdv-sim/src/handle.rs`) the same way it was
already called for the scheduled-write path — an immediate write caught in
ReadOnly now panics instead of silently reaching Icarus, which used to print
a diagnostic and drop the write (fix 2). `framework_tests::fresh_phase()` and
its four manual call sites are gone; the runner does it for every test now.

**A real finding along the way:** `framework-tests/src/clocks.rs`'s
`clock_two_are_independent` (a 2ns and a 10ns clock run together) was passing
*because* of the bug — Icarus silently dropping both clocks' opening writes
delayed the test's sync point just enough to dodge a tie between two
harmonic clocks' coincident edges. Fixed the write-drop and the test's own
latent tie-break showed up (9 edges instead of 10, not a flake — reproduced
every run). The test now bounds its measurement window with the slow clock's
falling edges instead of rising ones, which is 1ns clear of every fast edge
on both sides; same assertions (`dt == 20.0`, `fast_edges == 10`), argued for
instead of coincidental.

**Also fixed, found while verifying the above, unrelated to D108:**
`custom/sim-smoke-icarus` was red on arrival. `sim/tb/smoke_tb.sv` still drove
its own `clk` into a DUT that no longer takes one as a port (D112 made
`sim/hdl/tinyalu.sv` self-clocking — the smoke test has its own tiny DUT and
had the same pre-D112 shape), and `sim/run_smoke.sh` never compiled
`timescale.v` first, so the smoke test's own clock ran at Icarus's 1s/1s
default instead of 1ns/1ns and its watchdog fired. `smoke_tb.sv` now takes
`wire clk = dut.clk`; `run_smoke.sh` compiles `timescale.v` first for every
simulator entry.

**Caveat on that verification: it was Linux only.** See the 2026-07-30 entry
below on D113 — the macOS run was failing for an unrelated reason at the time
these claims were made, and the claims were stated more broadly than the
evidence supported.

Verified: every `framework-tests` group (`clock`, `trig`, `sig_`, `conc`,
`elab`, `runner_`) and the unfiltered all-39 run, all green, zero `VPI
error`/`SCHEDULER ERROR` lines. `regress.py --suite unit` (110 tests),
`--filter sim-tinyalu-tb`, and `--filter sim-smoke-icarus` all green.
`output/examples` has no `set_u64_now`/`set_now`/`read_only()` calls, so
`leave_read_only()` is a no-op for every existing book chapter and none of
their transcripts needed regenerating — D108's own scope note expected a wider
blast radius than this turned out to need, because D112 (landed the same day)
had already narrowed `Clock`'s real users down to `framework-tests` and one
book chapter (ch17), neither of which this fix touches behaviorally.

## 2026-07-30 — D113: `sim/build/` was poisoning the host's simulator

`sim/run_rustdv.sh` failed on Ray's Mac with `Killed: 9` and **no output at
all**, on the branch, while passing in the Linux sandbox against the same
commit and passing on `master`. It read as a regression in D108/D112. It was
not.

`sim/run_rustdv.sh` and `sim/run_smoke.sh` were the last two scripts writing
their build products into the repo (`mkdir -p build`, `cp "$LIB"
build/tinyalu_tb.vpi`). `sim/build/` is gitignored, so it survives every branch
switch, and the folder is shared with a Linux sandbox — so the file macOS `vvp`
was told to `dlopen` was an **ELF shared object**, byte-identical (same
BuildID) to the sandbox's `libtinyalu_tb.so`. macOS will not load a foreign
image and on Apple Silicon says so with SIGKILL before anything prints, which
is why there was no output to diagnose from. The branch/`master` split was the
sync race resolving differently either side of a rebuild, not a property of the
code.

Both scripts now build under `/tmp/rustdv-$(id -u)/` with `SIM_BUILD_DIR`
overriding — the convention `rustdv/framework-tests/run.sh` and
`output/examples/sim-common/run_sim.sh` already followed, which is exactly why
the 21 chapter entries and the six targeted groups never showed this. Verified
after the change: `sim/run_rustdv.sh release` → `REGRESSION: PASS`,
`sim/run_smoke.sh icarus` → `SMOKE: PASS`, `regress.py --suite custom` → 32
passed / 0 failed, and `sim/build/` untouched by any of it.

**Stale artifacts must be deleted by hand once**, on any machine that ran the
old scripts: `rm -f sim/build/tinyalu_tb.vpi sim/build/tinyalu_rustdv.vvp
sim/build/smoke.vvp` (individual files, not the directory — deleting a
directory inside the synced folder forks a `dir 2/` duplicate).

**The process lesson, recorded because it cost an afternoon.** The Linux
verification of D108/D112 was real but was reported as if it covered the
project's supported platforms; it did not, and the first macOS failure was
argued away as environmental on evidence that could not support that. A green
sandbox run is evidence about the sandbox. macOS/arm64 is a shipping platform
for this project (STATUS, 2026-07-13) and nothing is verified there until it is
run there.

## 2026-07-30 — the renumbering pass

`book-pdf/renumbering-spec.md` applied to the `.rs` captions in the eight
crates it lists: ch27, ch28, ch31, ch32, ch34, ch36, ch37, ch39. Every edit is
a digit change inside an existing comment line, so each file's insertions equal
its deletions — required, because transcripts embed `file:line` and a caption
edit that moved a line would invalidate every transcript in that crate. All
eight crates rebuilt clean afterwards (Linux sandbox).

Two things the spec did not list, both handled: ch34's module doc comment said
"Chapter 34 owns Figures 1–2" and used `// Chapter 34, Figure 1:` as a syntax
example, both stale once the captions moved to 2–3; and ch37's "Figure 4 is a
paragraph, not code" comment was reworded to drop the number, since the book
renders that passage as prose rather than a numbered figure.

**A consequence the spec did not anticipate.** The captions were renumbered into
the *book's* numbering, but the chapter READMEs mapped the *crate's*. Every
chapter the pass touched therefore had a README whose figure numbers no longer
matched its own source — including ch28, ch31, ch32 and ch34, which the spec
lists as needing no work. All eight were rebuilt in the book's numbering, so a
README row number is now the figure number.

## 2026-07-30 — sims rerun, transcripts and READMEs regenerated

Ran on Linux (sandbox), `RUSTDV_RANDOM_SEED=1`, everything built under
`/tmp/rustdv-$(id -u)/` per D113 — `sim/build/` was untouched and stayed empty.
All green: ch27 (4 tests), ch28 (7), ch36 (3), ch37 (1), ch38 (1), ch39 (3),
and `sim/run_rustdv.sh` (RandomTest + MaxTest, 4 compared / 0 mismatches on
MaxTest, `REGRESSION: PASS`).

Rebuilt the READMEs for ch27, ch28, ch31, ch32, ch34, ch36, ch37, ch38, ch39 —
figure maps in book numbering, transcripts pasted verbatim under a labelled
heading so the prose pass can copy them into the manuscript's 13
`[TRANSCRIPT NEEDED]` markers.

Two of them were badly wrong, not merely stale, and had already misled the prose
pass once (HANDOFF.md records it): **ch37's README was titled "Fibonacci
Testbench: 7.1"** and described `FibonacciSeq`/`RspDriver`/`FibEnv`, none of
which exist in that crate — it is the repair desk. **ch38's was titled
"get_response Testbench: 7.2"** and described a `CherryPickSeq` that does not
exist either. Both now match their sources. ch37's run command was also wrong
(`tinyalu`; the crate runs on `playground`).

**ch15–ch21 done too.** All six sim chapters rerun; 52 `src/lib.rs` references
across ch15–ch20 repointed at the real crate roots, and ch21's two repointed at
`rustdv/rustdv-macros/src/rustdv_macros.rs`. Then every transcript line in those
READMEs was checked against the fresh run, which caught three that were not
merely mis-pathed but **wrong**: ch18, ch19 and ch20 carried simulated times 5ns
early throughout (ch18 35/55/75/125 → 40/60/80/130; ch19 45→50 and so on; ch20
145→150, 290→300). The data is identical — same operands, same results, same
seed — so this is D112 landing: the DUT self-clocks now, and the first edge
arrives 5ns later than it did against the externally-clocked version. All three
replaced with the real output and re-verified line by line.

`ch14-modules-crates-cargo/README.md` still says `src/lib.rs` and is **correct**
— that crate really does have one, because it is the chapter that teaches Cargo's
conventions. It is the one deliberate exception to D29, and Part I is frozen.

**New: `rustdv/tinyalu_tb/README.md`.** The shipped testbench had no README, so
the Interlude's and ch40's transcripts had nowhere to be copied from. It now
carries the full `sim/run_rustdv.sh` run verbatim, plus the file list and the
counts, and it says plainly that the crate carries no figure captions.

**New: `output/regression/verify-transcripts.sh`.** One command that reruns all
13 sims and checks every transcript line in every README against the fresh
output. It exists because these transcripts were generated on Linux and go into
a printed book, while macOS/arm64 is a shipping platform.

**Verified on both platforms, 2026-07-30 — 251 transcript lines, character for
character:**

| | Linux/aarch64 (sandbox) | Darwin/arm64 (Ray's Mac) |
|---|---|---|
| Icarus | 14.0 devel | **13.0 stable** |
| Result | all match | all match |

The two runs used *different Icarus versions* and still agreed on every line, so
these transcripts are a property of the framework, not of one toolchain. That is
a stronger result than the check was designed to get.

**The first version of this script was broken, and the Mac run is what exposed
it.** Two defects, both of the same family — a checker that cannot fail:

1. **It compared against output that was never produced.** Every sim failed on
   macOS (no `timeout`; it is GNU coreutils, and the script wrapped every run in
   it), and the script proceeded to diff 13 READMEs against empty files, printing
   a wall of "MISMATCH" that looked like a framework finding and was noise. It
   now aborts before comparing and prints the tail of the failing log inline.
2. **It passed vacuously.** ch16 and ch17 reported "all transcript lines match"
   *in the same run where they had failed to execute* — their READMEs carry no
   timestamped lines, so the comparison loop iterated zero times and reported
   success. Zero transcript lines is now a failure everywhere else, and for those
   two the script verifies the claim they do make ("All 9 tests end `REGRESSION:
   PASS`" → the run must find exactly 9 and pass).

The fixes were mutation-tested rather than assumed: corrupting one timestamp,
inflating ch16's claimed test count, and deleting a transcript wholesale are each
caught, and the clean run still exits 0. A verification script that has never
been made to fail is not evidence — the same argument the mutation check in the
regression rests on, applied to the tool doing the checking.

## 2026-08-04 — the transcript check extended to every chapter, and what it found

`verify-transcripts.sh` originally covered only the 13 chapters that *owed* new
transcripts. Nine more carried README transcripts nothing checked. It now runs
**22 chapters, 431 transcript lines**, and the extension immediately found four
stale READMEs and two other problems:

- **ch23, ch26** — `file:line` references had drifted (ch23 `:152`→`:149`,
  ch26 `:117`→`:124`, and similarly through both files). The code moved; the
  transcripts did not.
- **ch24** — the README documented **one** test; the crate now has **two**
  (`PhaseTest (1/1)` → `(1/2)`). A test was added and the README never caught up.
- **ch29, ch31, ch32** — the READMEs had elided the `[file:line]` suffix from
  their `running …` lines, so they were paraphrases rather than real output,
  unlike every other chapter. Restored.
- **ch25 — a false positive worth keeping.** Its "Verification" block is a
  *mutation demo*: what the run prints with `alu_prediction`'s XOR sabotaged to
  OR. That output cannot appear in a clean run, by design. Rather than
  special-case the chapter, the script now honours an explicit
  `<!-- verify-transcripts: skip -->` marker before a fenced block, and ch25
  carries one. Counterfactual output stays in the README and stays out of the
  comparison.
- **ch26 — a block that could not be checked is now checked.** Its `FileTest`
  deliberately writes to `rustdv_ch26_log.txt` instead of the console, and the
  README quotes that file. The script now folds the log into ch26's captured
  output, so the quote is verified against the real artifact rather than skipped.

All 22 green on Linux/aarch64 after the fixes. **This wants rerunning on the Mac**
— the nine newly-covered chapters have never been checked there.

Also: `TOUR.md`'s "Notes for AI sessions" now carries D113, the
platform-evidence rule, the line-count-neutral caption rule and the
`book-pdf/src` ownership rule, so pointing a new thread at TOUR is sufficient
and prompts no longer restate them.

**The check is now a gate, not a habit.** `custom/readme-transcripts` runs
`verify-transcripts.sh` in the regression, so the pre-push hook fails on
README drift the day it happens. Verified to fail: corrupting one `file:line`
in ch23's README turns the suite red, and restoring it turns it green.

That is the actual lesson from this stretch of work, and it is now a standing
rule in `CLAUDE.md`: **the drift went unnoticed for months while a full green
regression ran over it every single push.** ch23/ch24/ch26 were wrong, and
nothing in 237 passing entries was looking. A rule written in a document is one
someone has to remember; a rule in the regression is enforced. When a class of
error turns up that nothing catches, add the check — and make it fail once on
purpose before trusting it.

## 2026-08-04 — the manuscript's own listings are now checked (ch15–40)

`book-sync` compares ch1–14 listings byte-for-byte against their example files
and has run on every push for months. **It covers Part I only.** Nothing
compared the ch15–40 manuscript — the part being finished right now — against
the crates it quotes. `output/regression/verify-book-listings.py` closes that,
wired in as `custom/book-listings`.

The comparison is code-only: comments and indentation are dropped from both
sides, because the book deliberately strips the crates' teaching comments and
re-wraps. Of 185 listings, **168 verify**, 17 are exempt, 0 are new drift.

**What it found on its first run — a factual error in five places.** The book
still prints `Clock::new(...)` at the top of ch18, ch19, ch20, ch40 and the
Interlude. D112 removed the software clock: the DUT self-clocks, the BFM only
waits on edges, and no crate runs a `Clock` any more. `chapter-notes.md` had
flagged it for ch19 alone. A reader typing any of those five openings gets a
testbench that does not match the one that runs. Recorded for the prose pass in
`chapter-notes.md` and `fable-restart.md`, and held in the script's debt
register so it is counted rather than forgotten.

Mutation-tested: changing one line of ch32's first listing turns the suite red.

### Everything it found is fixed (same day)

The first version of this script excused ten listings as "legitimate
exemptions". Most were not — they were the checker being weak, and the register
was hiding that. Fixed properly:

- **The checker compares one whitespace-normalised token stream**, not
  line-by-line, so the book re-wrapping a long signature is no longer "drift"
  (that alone accounted for ch18/9 and ch35/5). `#[allow(...)]` and
  `#[cfg_attr(...)]` are dropped from the crate side — they silence warnings
  about example code and carry nothing a reader needs. A listing that presents
  two non-adjacent excerpts now passes if **each** excerpt is real.
- **The five `Clock::new(...)` openings are gone from the manuscript** —
  ch18, ch19, ch20, ch40 and the Interlude now show what the crates show. ch40
  and the Interlude also had a whole `start_of_simulation` phase that no longer
  exists in `tinyalu_tb`, and ch40 carried a **paragraph** explaining that the
  shipped testbench drives a clock because it runs against a bare DUT. D112
  retired that exception; the paragraph was false and is rewritten.
- **The D114 `#[component]` sweep is applied** to all 17 manuscript files, so
  the temporary normalisation is deleted and those 50 listings are compared
  strictly.
- **ch21/3** omitted the `#[cfg(target_os = "linux")]` the crate carries;
  added. **ch35/2** silently skipped a `Display` impl; the skip is now marked
  `// ...` and is honest.

**Result: 176 verbatim, 3 spliced-but-real, 3 exempt, 0 drift.** The three
exemptions are permanent and each says why: std's `Future` trait quoted to
explain `async`, ch21's deliberately tidied macro expansion, and ch19's
three-line BFM skeleton with a placeholder comment.

Full regression after all of it: unit 1, book-sync 108, examples 96, custom 34
— **239 entries, 0 failed**, on Linux/aarch64.

## 2026-08-04 — the manuscript's transcripts, checked and then completed

Extending the transcript gate to the book itself found that **the manuscript's
own transcripts were stale in exactly the ways the READMEs had been** — nothing
had ever compared them either. Forty lines across eight chapters: ch15 quoting
the pre-rename `src/lib.rs`, ch19 and ch20 five nanoseconds early (D112), ch23
and ch26 with drifted `file:line`, ch29/ch31/ch32 with the `[file:line]` suffix
elided so the "transcript" was a paraphrase. **ch24 documented one test where
the crate has two** — `PhaseTest (1/1)` against a real `(1/2)`, with a summary
table listing one row. All corrected from verified output.

`verify-transcripts.sh` now checks every timestamped line in `book-pdf/src`
against the live runs, not just the READMEs. Both are gated by
`custom/readme-transcripts`.

**The 13 `[TRANSCRIPT NEEDED]` placeholders are filled.** Ray reserved the prose
pass for writing, not clerical work, so a code thread ran every sim and pasted
the real output: ch27 ×4, ch28, ch36, ch37, ch38, ch39 ×3, and the full
`sim/run_rustdv.sh` run into both ch40 and the Interlude. The manuscript now
carries **549 transcript lines, every one verified against a live run.** No
placeholder of any kind remains in `book-pdf/src`.

## 2026-08-04 — the end-to-end read

Done mechanically where a machine can be exhaustive, by eye where it cannot.

Checked and clean: figure numbering runs 1..N in order in **every** chapter once
`<figcaption>` SVG figures are counted (ch31's fig 9, ch34's fig 1 and ch36's
fig 1 are drawings, which is why the code captions appear to skip); `SUMMARY.md`
matches the files on disk exactly both ways; no reference to the cut Chapter 41;
no chapter reference out of range; every internal link resolves; every repo path
quoted in the book exists; all 12 `error[EXXXX]` codes reconcile against the 20
checked compile-fail cases; every name in Appendix D and the Toolkit page exists
in the framework; no leftover editorial artifact anywhere; all 50 rendered HTML
pages carry real content.

Two things it found, both fixed:

- **Two stale `src/lib.rs` references survived the crate-root renames** — ch17
  quoting a compiler warning's path, ch21 quoting a runner log line. Neither is
  a Rust listing, so `book-listings` could not see them.
- **A hole the `Clock` fix opened.** Removing `Clock::new(...)` from ch18/19/20
  left only a code comment explaining where the clock went; a reader who met
  `Clock` in ch17 and never saw it again got no reason. Chapter 17 now closes
  its `Clock` figure with the reason — the counter is a bare design, the TinyALU
  clocks itself, and a BFM that waits on edges is the same code on an emulator
  while one that drives them is not.

`ch02`'s two figures both captioned "The corrected program" are intentional
(two mistakes, two corrections) and match the frozen manifest.

Final: unit 1, book-sync 108, examples 96, custom 34 — **239 entries, 0 failed**.

*Platform note:* this is a Linux run. macOS/arm64 is a shipping platform and
these transcripts go into the book, so they want confirming on the Mac — a diff,
not a re-read. Fixed seed, single-threaded executor and simulated time should
make them byte-identical; any line that differs is a real finding.

## 2026-07-30 — D114: the child attribute takes no argument

`#[component(child)]`, `#[component(fifo)]` and `#[component(sequencer)]` are
gone; the attribute is bare `#[component]`. Full reasoning in
`output/.design-decisions.md` §42; the short version is that the derive never
read the word. It tested only that one of the three appeared in the attribute
text and set a single flag; what a field becomes is decided by its Rust type
(`RustdvComp` → factory slot, `Option<T>` → declared-but-not-yet-built,
`Vec<T>` → list, otherwise a plain child), and that dispatch reads `f.ty`.

This closes D106's tail, which had asked whether the D84 carve-out should get
one role word or per-type spellings. Neither answer was available: both would
have named something the compiler never saw. `AnalysisBus` declared
`#[component(fifo)]` — the wart D106 recorded — was what prompted reading the
macro, and it was wrong precisely because nothing could catch it.

Changed: all 21 declaration sites under `rustdv/`, every site in the 20 example
crates under `output/examples/`, `skills/rustdv-testbench/SKILL.md`,
`.claude/skills/wire-an-env/SKILL.md`, and the derive itself. The workspace
builds clean (Linux sandbox). The derive still matches on
`starts_with("component")`, so an old spelling would compile; none survives in
code and none should return.

**The manuscript is deliberately not changed** — 17 files in `book-pdf/src`
still print the old form, flagged in `book-pdf/chapter-notes.md` for the prose
pass, which is the only pass that edits the book.

*Noticed alongside, not fixed:* `#[component(no_factory)]` is now the only
argument the derive parses, `struct_attr_contains` exists to serve it, and no
struct in the tree applies it. Live logic, no call site.
