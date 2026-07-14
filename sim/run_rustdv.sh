#!/usr/bin/env bash
# Build the rustdv TinyALU testbench and run it on Icarus Verilog.
#
# Usage:  sim/run_rustdv.sh [debug|release]     (default: release)
#
# Flow (design-doc §7.4, VPI-module variant):
#   cargo build (cdylib) -> copy as build/tinyalu_tb.vpi
#   iverilog the DUT (no Verilog testbench: rustdv drives the top module)
#   vvp -M build -m tinyalu_tb  -> the VPI module bootstraps the regression
#
# Success criterion: prints "REGRESSION: PASS".
set -euo pipefail
cd "$(dirname "$0")"

PROFILE="${1:-release}"
REPO_ROOT="$(cd .. && pwd)"

# Build outside the (possibly mounted) repo for speed; see STATUS.md.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/rustdv-target}"

mkdir -p build

case "$PROFILE" in
  release) (cd "$REPO_ROOT/rustdv" && cargo build --release -p tinyalu_tb) ;;
  debug)   (cd "$REPO_ROOT/rustdv" && cargo build -p tinyalu_tb) ;;
  *) echo "unknown profile: $PROFILE (use debug|release)" >&2; exit 2 ;;
esac

PROFDIR="$( [ "$PROFILE" = release ] && echo release || echo debug )"
LIB="$CARGO_TARGET_DIR/$PROFDIR/libtinyalu_tb.so"          # Linux
[ -f "$LIB" ] || LIB="$CARGO_TARGET_DIR/$PROFDIR/libtinyalu_tb.dylib"  # macOS
cp "$LIB" build/tinyalu_tb.vpi

# timescale.v first: it sets 1ns/1ns for everything after it.
iverilog -g2012 -o build/tinyalu_rustdv.vvp -s tinyalu hdl/timescale.v hdl/tinyalu.sv

export RUSTDV_RANDOM_SEED="${RUSTDV_RANDOM_SEED:-1}"
export RUSTDV_RESULTS_XML="${RUSTDV_RESULTS_XML:-$(pwd)/build/results.xml}"

vvp -M build -m tinyalu_tb build/tinyalu_rustdv.vvp
