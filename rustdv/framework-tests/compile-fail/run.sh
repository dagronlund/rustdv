#!/usr/bin/env bash
# Require each case in this directory to fail to compile, with the right error.
#
# A category pyuvm cannot have at all: these are claims the book makes about
# what the compiler will not let a reader do, and the compiler is the only
# thing that can check them.
#
# The expected code is asserted, not just "it failed". Without that, a case
# that started failing for an unrelated reason — a renamed type, a missing
# import, a typo in the test itself — would keep passing and stop testing
# anything. One of these cases was vacuous when first written: `let _ = cmd.a`
# does not evaluate a place expression, so the use-after-move compiled and
# "it failed to compile" would have been the wrong conclusion.
#
# Success criterion: prints "COMPILE-FAIL: PASS".
set -uo pipefail
cd "$(dirname "$0")"

RUSTDV_TMP="/tmp/rustdv-$(id -u)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$RUSTDV_TMP/cf-target}"

# case                        expected code   what it pins down
CASES=(
    "export_interface_mismatch|E0308|a get export wired to a put port"
    "misspelled_port_name|E0599|a PortName constant that does not exist"
    "use_after_finish_item|E0382|reading a transaction the driver now owns"
    "eq_on_a_float|E0277|deriving Eq on a struct carrying an f64"
    "port_attr_on_non_port|E0277|#[port(..)] on a field that is not a port"
)

status=0
for entry in "${CASES[@]}"; do
    IFS='|' read -r dir want desc <<< "$entry"
    out=$(cd "$dir" && cargo build 2>&1)
    if [ -z "$(grep -E '^error' <<< "$out")" ]; then
        echo "  FAIL $dir compiled — $desc is no longer rejected" >&2
        status=1
        continue
    fi
    got=$(grep -oE 'error\[E[0-9]+\]' <<< "$out" | head -1 | tr -d 'error[]')
    if [ "$got" != "$want" ]; then
        echo "  FAIL $dir failed with ${got:-an unclassified error}, expected $want" >&2
        grep -E '^error' <<< "$out" | head -3 >&2
        status=1
        continue
    fi
    echo "  ok   $dir rejected with $want ($desc)"
done

if [ "$status" -eq 0 ]; then
    echo "COMPILE-FAIL: PASS"
else
    echo "COMPILE-FAIL: FAIL"
fi
exit "$status"
