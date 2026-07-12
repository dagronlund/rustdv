#!/usr/bin/env bash
# Template: build the testbench cdylib and run it on Icarus Verilog.
# Success criterion: prints "REGRESSION: PASS".
set -euo pipefail
cd "$(dirname "$0")"

TB_CRATE=<dut>_tb          # cargo package name
TOP=<top_module>           # DUT top-level module name
RTL="hdl/<dut>.sv"         # RTL sources

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/${TB_CRATE}-target}"
mkdir -p build

cargo build --release -p "$TB_CRATE"
cp "$CARGO_TARGET_DIR/release/lib${TB_CRATE}.so" "build/${TB_CRATE}.vpi"

# timescale.v first: it sets 1ns/1ns for everything after it.
iverilog -g2012 -o build/design.vvp -s "$TOP" timescale.v $RTL

export RUSTDV_RANDOM_SEED="${RUSTDV_RANDOM_SEED:-1}"
export RUSTDV_RESULTS_XML="${RUSTDV_RESULTS_XML:-$(pwd)/build/results.xml}"

vvp -M build -m "$TB_CRATE" build/design.vvp
