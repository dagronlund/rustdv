<!-- After creating the GitHub repo, replace OWNER below with your GitHub
     username (badges and the Codespaces link need the real path). -->

# Rust for RTL Verification

[![CI](https://github.com/OWNER/rustuvm/actions/workflows/ci.yml/badge.svg)](https://github.com/OWNER/rustuvm/actions/workflows/ci.yml)
[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/OWNER/rustuvm?quickstart=1)

A book teaching Rust to RTL verification engineers who know Python — by the
author of *Python for RTL Verification* — together with runnable code for
every figure and the groundwork for **rustvm**, a Rust testbench framework
driving the TinyALU DUT.

## What's here

| Path | Contents |
|---|---|
| `book-pdf/src/` | The manuscript (mdbook markdown), chapters 1–14 |
| `output/examples/` | Every book figure as runnable code — 105 figures: cargo binaries, intentional compile-errors, intentional panics ([README](output/examples/README.md)) |
| `output/regression/` | The regression system guarding all of it ([TESTING.md](output/regression/TESTING.md)) |
| `sim/` | TinyALU DUT + simulator smoke tests ([README](sim/README.md)) |
| `docker/` | Reproducible environment: Rust + Icarus + Verilator |

## Try it in 30 seconds

- **In the browser, zero install:** every chapter README in
  `output/examples/` has a "Try it" Rust Playground link per figure —
  including the figures that fail to compile on purpose (the error is the
  lesson).
- **Full environment in the browser:** click the Codespaces badge above.
  You get Rust, Icarus Verilog, and Verilator, ready to run everything.
- **Locally:**

```sh
cd output/examples
cargo run --bin ch03_fig03_mutable_binding   # any figure
cd ../.. && output/regression/regress.py     # the whole regression suite
```

## Simulators

| | Status |
|---|---|
| Icarus Verilog | runs in CI (`sim/run_smoke.sh icarus`) |
| Verilator | lints in CI; full simulation arrives with rustvm |
| VCS / Questa / Xcelium | same script (`sim/run_smoke.sh vcs\|questa\|xcelium`); licenses can't live in public CI, so license-holders run the identical regression locally |
| EDA Playground | HDL side only — it has no Rust toolchain; see [sim/README.md](sim/README.md) |

## Development

`output/regression/regress.py` is the single gate: book↔examples sync,
every figure's behavior vs. blessed goldens, simulator smoke tests (which
auto-skip where no simulator is installed), and drop-in custom tests.
`--install-hook` wires it to `git push`. The rule: every new piece of
functionality lands together with the test that would catch its removal.
