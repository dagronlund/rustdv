---
name: new-rustdv-testbench
description: Use when creating a new rustdv testbench crate for an RTL design (DUT) — setting up the cdylib/VPI module, Cargo.toml, test entry points, run script, and cargo-test linking. Covers every non-obvious setup step; skipping any of them produces confusing link or simulator errors.
---

# Create a new rustdv testbench crate

A rustdv testbench compiles to a shared library the simulator loads as a
VPI module. There is **no Verilog testbench file** — Rust drives the DUT's
top-level ports directly. The working reference is `rustdv/tinyalu_tb` in
this repository; the templates in `templates/` are minimal versions of it.

## Checklist (every step is load-bearing)

1. **Crate setup.** Copy `templates/Cargo.toml`. The two crate types are
   both required:
   - `"cdylib"` — the simulator-loaded module;
   - `"rlib"` — lets `cargo test` run the pure-Rust unit tests.

2. **Bootstrap.** Call `rustdv::vpi_bootstrap!();` **exactly once**, at the
   crate root (`lib.rs`). It exports `vlog_startup_routines`, which the
   simulator resolves by symbol name. Zero calls → the module loads and
   does nothing. Two calls → duplicate-symbol link error.

3. **cargo-test linking.** Test *executables* cannot carry the undefined
   `vpi_*` symbols the cdylib is allowed to have. Add the stub crate:
   ```toml
   [dev-dependencies]
   rustdv-vpi-stubs = { path = "<path-to>/rustdv/rustdv/rustdv-vpi-stubs" }
   ```
   (not working from a clone: `cargo add --dev rustdv-vpi-stubs`)

   and in `lib.rs`:
   ```rust
   #[cfg(test)]
   use rustdv_vpi_stubs as _;
   ```
   Do NOT try linker flags instead (`--unresolved-symbols` corrupts
   aarch64 PLT relocations — see STATUS.md fix log #2).

4. **Tests.** Signature is fixed; always set a timeout (a hung handshake
   otherwise hangs the whole regression):
   ```rust
   #[rustdv::test(timeout_time = 500, timeout_unit = "us")]
   async fn my_test(ctx: TestCtx) -> Result<(), TestError> { ... }
   ```
   `Err` = a check failed; `panic!` = a testbench bug. Both fail the test.

5. **Build & run flow** (see `templates/run.sh`):
   - `cargo build --release -p <tb-crate>` → `lib<tb>.so`
   - copy it as `<name>.vpi` (vvp requires the `.vpi` extension)
   - `iverilog -g2012 -o design.vvp -s <top_module> timescale.v <rtl files>`
   - `vvp -M <dir-with-vpi> -m <name> design.vvp`

6. **Timescale.** Without any `` `timescale `` directive Icarus defaults to
   **1 s** precision and every `Timer::ns()` silently misbehaves. Compile
   `templates/timescale.v` (`` `timescale 1ns/1ns ``) **first** on the
   iverilog command line — the directive carries into later files.

7. **Environment knobs.** `RUSTDV_RANDOM_SEED` (printed at start of every
   run; set it to reproduce), `RUSTDV_RESULTS_XML` (xUnit output path).

8. **Verify before trusting.** Run once expecting PASS, then sabotage the
   DUT (or the predictor) and confirm the regression FAILS. A checker that
   has never failed is unverified. See the `debug-a-regression` skill.

## Common failures at this stage

| Symptom | Cause |
|---|---|
| vvp runs, prints nothing, exits | `vpi_bootstrap!()` missing, or `-m` name doesn't match the `.vpi` file |
| `undefined reference to vpi_get...` building tests | step 3 skipped |
| Timer/clock times absurdly long or zero | timescale missing (step 6) |
| test hangs at 0 ns | no clock started — `Clock::new(&clk, SimDuration::ns(10)).start();` |
