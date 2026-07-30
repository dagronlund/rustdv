#!/usr/bin/env bash
# Run the TinyALU smoke test on a chosen simulator.
#
# Usage:  sim/run_smoke.sh [icarus|verilator|vcs|questa|xcelium]
#         (or set SIM=...; the argument wins)
#
# Success criterion: prints "SMOKE: PASS" (Icarus & commercial sims run the
# testbench; Verilator currently lints the DUT and prints "LINT: PASS" —
# full Verilator simulation arrives with rustdv).
set -euo pipefail
cd "$(dirname "$0")"
SIM="${1:-${SIM:-icarus}}"

# Scratch under the per-user root, never in the repo (D113) — `sim/build/` is
# gitignored, so anything left there outlives every branch switch, and a
# sandbox on another OS can leave a foreign binary where this script's
# simulator would load it.
BUILD="${SIM_BUILD_DIR:-/tmp/rustdv-$(id -u)/smoke}"
mkdir -p "$BUILD"

# Compiled first, as in output/examples/sim-common/run_sim.sh: tinyalu.sv
# declares no timescale, and since D112 it is the DUT that generates the clock
# — so without this its `always #5` runs at Icarus's 1s/1s default and the
# handshake never completes inside the watchdog.
TS="hdl/timescale.v"
HDL="hdl/tinyalu.sv"
TB="tb/smoke_tb.sv"

case "$SIM" in
  icarus)
    iverilog -g2012 -o "$BUILD/smoke.vvp" "$TS" "$HDL" "$TB"
    vvp "$BUILD/smoke.vvp"
    ;;
  verilator)
    verilator --lint-only -sv --top-module tinyalu "$HDL" -Mdir "$BUILD/obj_dir"
    echo "LINT: PASS"
    ;;
  vcs)        # Synopsys — requires a license; untested in this repo's CI
    vcs -full64 -sverilog -o "$BUILD/simv" "$TS" "$HDL" "$TB"
    "$BUILD/simv"
    ;;
  questa)     # Siemens — requires a license; untested in this repo's CI
    qrun -sv "$TS" "$HDL" "$TB" -top smoke_tb -outdir "$BUILD/qrun.out"
    ;;
  xcelium)    # Cadence — requires a license; untested in this repo's CI
    xrun -sv "$TS" "$HDL" "$TB" -top smoke_tb
    ;;
  *)
    echo "unknown simulator: $SIM (use icarus|verilator|vcs|questa|xcelium)" >&2
    exit 2
    ;;
esac
