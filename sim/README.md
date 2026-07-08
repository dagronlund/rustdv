# Simulator scaffold

The TinyALU DUT and a smoke test proving the simulator toolchain works.
This is groundwork for **rustdv** — the book's Rust testbench framework —
which will drive this same DUT from Rust. Until then, the smoke test keeps
the simulator path exercised in CI so it can't rot.

## Files

- `hdl/tinyalu.sv` — the TinyALU, copied unmodified from the pyuvm project
  (`reference/pyuvm-master/examples/TinyALU/hdl/verilog/tinyalu.sv`,
  Apache-2.0, © Ray Salemi). Single-cycle ADD/AND/XOR, three-cycle MUL,
  start/done handshake.
- `tb/smoke_tb.sv` — minimal self-checking testbench: five operations,
  prints `SMOKE: PASS` on success.
- `run_smoke.sh` — simulator selector.

## Running

| Simulator | Command | Notes |
|---|---|---|
| Icarus Verilog | `sim/run_smoke.sh icarus` | free; runs in CI |
| Verilator | `sim/run_smoke.sh verilator` | free; lint-only for now, runs in CI |
| Synopsys VCS | `sim/run_smoke.sh vcs` | needs license; run locally |
| Siemens Questa | `sim/run_smoke.sh questa` | needs license; run locally |
| Cadence Xcelium | `sim/run_smoke.sh xcelium` | needs license; run locally |

The same tests run through the regression system
(`output/regression/regress.py --suite custom --filter sim`) and skip
automatically on machines without the simulator installed.

Commercial-simulator invocations are the standard ones but **untested here**
— public CI cannot hold EDA licenses. If you have a license and the command
needs adjusting for your site, that's expected; the testbench itself is
plain SystemVerilog and should run anywhere.

## Why no EDA Playground?

EDA Playground runs SystemVerilog and Python/cocotb, but has no Rust
toolchain, so rustdv testbenches can't execute there. The HDL side (this
DUT, this smoke test) pastes into EDA Playground fine. For the book's
pure-Rust figures, use the per-figure Rust Playground links in the chapter
READMEs, or open the repo in GitHub Codespaces for the full environment.
