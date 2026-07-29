#!/usr/bin/env bash
# Run the framework's targeted simulator tests (test-plan §3) on Icarus.
#
# Usage:  framework-tests/run.sh [testcase-substring[,...]]
#
# With no argument every test in the crate runs. With one, RUSTDV_TESTCASE
# selects a group by name prefix — `trig_`, `clock_`, `sig_`, `conc_`,
# `elab_`, `runner_` — which is how the regression gets one entry per
# mechanism off a single build.
#
# Success criterion: prints "REGRESSION: PASS".
set -euo pipefail
cd "$(dirname "$0")"

# Scratch under one per-user root; see output/examples/sim-common/run_sim.sh
# for why the uid is in the path.
RUSTDV_TMP="/tmp/rustdv-$(id -u)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$RUSTDV_TMP/target}"

(cd .. && cargo build --release -p framework_tests --quiet)

BUILD="${SIM_BUILD_DIR:-$RUSTDV_TMP/framework-tests}"; mkdir -p "$BUILD"
LIB="$CARGO_TARGET_DIR/release/libframework_tests.so"            # Linux
[ -f "$LIB" ] || LIB="$CARGO_TARGET_DIR/release/libframework_tests.dylib"  # macOS
cp "$LIB" "$BUILD/framework_tests.vpi"

iverilog -g2012 -o "$BUILD/probe.vvp" -s probe hdl/probe.sv

export RUSTDV_RANDOM_SEED="${RUSTDV_RANDOM_SEED:-1}"
export RUSTDV_RESULTS_XML="${RUSTDV_RESULTS_XML:-$BUILD/results.xml}"
if [ $# -gt 0 ]; then
    export RUSTDV_TESTCASE="$1"
fi

vvp -M "$BUILD" -m framework_tests "$BUILD/probe.vvp"
