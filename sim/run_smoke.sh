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
mkdir -p build

HDL="hdl/tinyalu.sv"
TB="tb/smoke_tb.sv"

case "$SIM" in
  icarus)
    iverilog -g2012 -o build/smoke.vvp "$HDL" "$TB"
    vvp build/smoke.vvp
    ;;
  verilator)
    verilator --lint-only -sv --top-module tinyalu "$HDL" -Mdir build/obj_dir
    echo "LINT: PASS"
    ;;
  vcs)        # Synopsys — requires a license; untested in this repo's CI
    vcs -full64 -sverilog -o build/simv "$HDL" "$TB"
    build/simv
    ;;
  questa)     # Siemens — requires a license; untested in this repo's CI
    qrun -sv "$HDL" "$TB" -top smoke_tb -outdir build/qrun.out
    ;;
  xcelium)    # Cadence — requires a license; untested in this repo's CI
    xrun -sv "$HDL" "$TB" -top smoke_tb
    ;;
  *)
    echo "unknown simulator: $SIM (use icarus|verilator|vcs|questa|xcelium)" >&2
    exit 2
    ;;
esac
