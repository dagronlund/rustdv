# Verilator support for rustdv — what we need, and an open invitation

**From:** Ray Salemi ([rdsalemi@gmail.com](mailto:rdsalemi@gmail.com))
**Project:** [rustdv](https://github.com/rustdv/rustdv) — a Rust hardware
verification framework (a cocotb/pyuvm analog), companion to the book
*Rust for RTL Verification*.

## What rustdv is

rustdv lets you write a testbench entirely in Rust and drive real RTL with
it — no separate SystemVerilog testbench at all. A testbench compiles to a
`cdylib`, the simulator loads it, and Rust code drives the DUT's signals,
waits on clock edges, and checks results, the same shape as cocotb but
statically typed and dependency-free.

It works today on **Icarus Verilog**. A complete example — BFM, driver,
monitors, scoreboard, functional coverage — runs a TinyALU regression and
passes. Verilator currently only **lints** the DUT in our CI (a single
`verilator --lint-only -sv --top-module tinyalu`); it can't run a rustdv
testbench yet. That's the gap this note is about.

**Our environment.** CI installs Verilator from Ubuntu's package repo
(`apt-get install verilator`, currently the 5.x line) — no pinned version,
so we can build against whatever release makes your answer easiest to give.
We have not yet attempted a running Verilator simulation of a rustdv
testbench; the lint step is as far as we've taken it, precisely because the
loading model below is unresolved.

## How rustdv talks to Icarus today

rustdv's simulator interface (`rustdv-gpi-sys`) is a hand-written subset of
the IEEE 1800 VPI C API — no bindgen, no C code of our own, pure Rust
`extern "C"` declarations resolved at load time against whatever the host
process exports (`vpi_handle_by_name`, `vpi_get_value`/`vpi_put_value`,
`vpi_register_cb` with reasons like `cbValueChange`/`cbReadWriteSynch`,
etc.).

The embedding hook is the standard VPI bootstrap table:

```rust
#[no_mangle]
pub static vlog_startup_routines: [Option<extern "C" fn()>; 2] =
    [Some(__rustdv_vpi_entry), None];
```

The flow is: `cargo build` produces a `cdylib` → we rename it `.vpi` →
`iverilog` compiles the RTL → `vvp -M builddir -m testbench_name` dlopens
the `.vpi` at runtime and walks `vlog_startup_routines`, calling our entry
point, which registers a start-of-simulation callback and takes over from
there. No C++ compilation step anywhere in this path.

## Why Verilator is different, and where we're stuck

Verilator doesn't work this way. It translates RTL to C++ at "verilate"
time and produces a generated model (`Vtop.h`) that gets compiled into a
standalone executable — there's no long-running simulator process to
`dlopen` a `.vpi` module into at runtime the way `vvp` provides.

We looked at how cocotb solves this
(`share/lib/verilator/verilator.cpp` in the cocotb source): it hand-writes
a `main()` that statically includes the generated `Vtop.h`, links
Verilator's own VPI emulation layer (`verilated_vpi.h`,
`VerilatedVpi::callValueCbs()`), and manually calls
`vlog_startup_routines_bootstrap()` to invoke the loaded module's startup
table — since nothing calls it automatically the way `vvp` does.

cocotb's harness is a single generic file, not literally rewritten per
project — but it still has to be *compiled per project* at verilate time,
against that project's generated `Vtop.h` and top-module name. So it means
a **C++ compile step in every build**, a fundamentally different flow than
our current "build once, dlopen at runtime" model, and one that would pull
a C++ toolchain into what's otherwise a zero-external-dependency, pure-Rust
path. (That cocotb's file is already generic is encouraging for question 2
below.)

## What would help us

We don't need a finished integration — just answers, or pointers to
existing work, on:

1. **Is dynamic loading possible at all?** Is there a way to get a
   Verilated executable to `dlopen` a prebuilt VPI-compliant shared
   library at runtime and call its `vlog_startup_routines` table, the way
   `vvp` does — instead of requiring the VPI module to be statically
   linked into a custom `main()` at compile time? If Verilator could do
   this, our existing Icarus-shaped flow would likely work with little
   to no change.

2. **If not, what's the minimal generic bridge?** Is there (or could
   there be) a reusable, project-agnostic `main()`/harness — something we
   could ship once in rustdv rather than hand-write per testbench — that
   wraps *any* generated `Vtop.h` and hands control to a dynamically
   loaded VPI module? cocotb's `verilator.cpp` is written against a known
   top module name; we'd want something top-module-agnostic if possible.

3. **VPI coverage.** Which parts of the API surface we rely on does
   `verilated_vpi.h` actually implement today (and in which Verilator
   version — we can pin to whatever you reference), and are there known
   gaps or behavioral differences from Icarus worth knowing before we
   build against it? Specifically: `vpi_handle_by_name`, `vpi_iterate`/
   `vpi_scan`, `vpi_get`/`vpi_get_str`, `vpi_get_value`/`vpi_put_value`,
   `vpi_get_time`, and callback reasons `cbValueChange`,
   `cbReadWriteSynch`, `cbReadOnlySynch`, `cbAfterDelay`,
   `cbStartOfSimulation`, `cbEndOfSimulation`.

4. **Build flow shape.** Our Icarus flow is four steps and no hand-written
   glue code: `cargo build` → rename `.so` → `iverilog` → `vvp -M -m`. Is
   there a realistic path to something comparably simple for Verilator —
   ideally still driven from a small Rust-side runner rather than a
   hand-authored C++ file per project — or is a per-project generated
   `main()` unavoidable given how Verilator works?

## Reference material in the repo

- [`rustdv-gpi-sys/src/lib.rs`](https://github.com/rustdv/rustdv/blob/master/rustdv/rustdv-gpi-sys/src/lib.rs)
  — the VPI FFI surface we currently use
- [`rustdv-gpi/src/lib.rs`](https://github.com/rustdv/rustdv/blob/master/rustdv/rustdv-gpi/src/lib.rs)
  — the safe layer on top of it
- [`tinyalu_tb/`](https://github.com/rustdv/rustdv/tree/master/rustdv/tinyalu_tb)
  — the full working example, running on Icarus today
- [`output/design-doc.md`](https://github.com/rustdv/rustdv/blob/master/output/design-doc.md),
  Open Question OQ-2 — where we've recorded this as the highest-risk
  unresolved item in rustdv's design

We'd genuinely welcome help — whether that's someone pointing us at prior
art, a maintainer weighing in on feasibility, or an actual contribution
once rustdv 0.1 ships. Feel free to open a discussion on the repo or email
me directly.
