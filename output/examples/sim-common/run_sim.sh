#!/usr/bin/env bash
# Build a chapter's sim-figure crate as a VPI module and run it on Icarus.
# Usage: sim-common/run_sim.sh <crate-name> <top-module> [hdl files...]
#
# Part II+ figures run inside a simulator; each sim chapter is a cdylib
# crate whose #[rustdv::test] functions are the chapter's figures.
set -euo pipefail
cd "$(dirname "$0")/.."
CRATE="$1"; TOP="$2"; shift 2
HDL=("$@"); [ ${#HDL[@]} -eq 0 ] && HDL=(sim-common/hdl/timescale.v "sim-common/hdl/${TOP}.sv")
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/rustdv-examples-target}"
cargo build --release -p "$CRATE" --quiet
BUILD="${SIM_BUILD_DIR:-/tmp/rustdv-examples-sim}"; mkdir -p "$BUILD"
# Linux builds lib<crate>.so; macOS builds lib<crate>.dylib
LIB="$CARGO_TARGET_DIR/release/lib${CRATE}.so"
[ -f "$LIB" ] || LIB="$CARGO_TARGET_DIR/release/lib${CRATE}.dylib"
cp "$LIB" "$BUILD/${CRATE}.vpi"
iverilog -g2012 -o "$BUILD/${CRATE}.vvp" -s "$TOP" "${HDL[@]}"
export RUSTDV_RANDOM_SEED="${RUSTDV_RANDOM_SEED:-1}"
vvp -M "$BUILD" -m "$CRATE" "$BUILD/${CRATE}.vvp"
