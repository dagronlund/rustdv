---
name: debug-a-regression
description: Use when a rustdv regression fails, hangs, times out, or passes suspiciously — reading runner output, reproducing with seeds, diagnosing hangs and scoreboard errors, distinguishing testbench bugs from DUT bugs, and mutation-testing the checkers. Also covers the known link/simulator quirks.
---

# Debug a rustdv regression

## Reproduce first
The seed is printed on the first line (`RUSTDV_RANDOM_SEED=...`). Re-run
with `RUSTDV_RANDOM_SEED=<n> sim/run_rustdv.sh` — stimulus is fully
deterministic per seed. Each test's source location is printed
(`[file:line]`), and `RUSTDV_RESULTS_XML` holds the machine-readable table.

## Failure taxonomy — where to look
- **`Err`/scoreboard ERROR lines** → a *check* failed → suspect the DUT or
  the predictor. The mismatch line shows the command, got, and expected.
- **`panicked` messages** → a *testbench bug* → suspect your code, not the
  DUT (double `get_next_item`, illegal ReadOnly-phase write, index bugs).
  A panic in ANY spawned task fails the current test, even if the test
  body returned Ok.
- **`timeout after ...`** → something never completed; see Hangs below.

## Hangs / timeouts — the usual suspects, in order
1. **A panic earlier in the log.** A dead driver task leaves the sequence
   blocked in `finish_item` forever. Always scroll up before theorizing.
2. **No clock.** Awaiting an edge of a signal nothing toggles. Did the
   test call `Clock::new(...).start()`?
3. **Handshake protocol violation** — `get_next_item` twice without
   `item_done` (panics), or a driver that never calls `item_done`.
4. **Objection held forever** — an `ObjectionGuard` bound outside the
   block that should scope it (RAII: it drops at end of scope).
5. **Missing drain** — test ends, monitors still owe transactions →
   "orphaned command/result" errors instead of a hang; add
   `bfm.wait_idle().await` before checking.

## Scoreboard says PASS — should you believe it?
Run the mutation test: sabotage one op in the RTL and confirm FAIL.

```sh
sed 's/A} ^ {8'"'"'d0,B}/A} | {8'"'"'d0,B}/' hdl/dut.sv > /tmp/mutated.sv
iverilog -g2012 -o /tmp/mut.vvp -s <top> timescale.v /tmp/mutated.sv
vvp -M build -m <tb> /tmp/mut.vvp   # expect: mismatches + REGRESSION: FAIL
```
If the mutant passes, the checker is vacuous — check that the scoreboard
actually compares (`"scoreboard: N compared"` with N > 0; N == 0 is itself
flagged) and that monitors publish on the port the scoreboard reads.

## Known quirks (already solved once — don't re-debug)
- `undefined reference to vpi_*` linking **tests**: add the
  `rustdv-vpi-stubs` dev-dependency + `#[cfg(test)] use ... as _;`.
  Do not use `--unresolved-symbols` linker flags (corrupts aarch64 PLT).
- `unexpected PLT reloc type 0x00` at test startup: you used those flags.
- Times off by ~1e9: missing `` `timescale `` (Icarus defaults to 1 s);
  compile a timescale.v first.
- vvp silent exit: `-m` name ≠ `.vpi` filename, or `vpi_bootstrap!()`
  missing.
- Writes to top-level input ports on Icarus are fine (NoDelay put_value);
  reads returning all-x mid-sim usually mean the DUT was never reset.
