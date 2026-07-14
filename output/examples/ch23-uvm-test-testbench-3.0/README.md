# Chapter 23: uvm_test Testbench: 3.0 — figure map

Run with:

```
sim-common/run_sim.sh ch23_uvm_test_testbench_3_0 tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

The 2.0 classes (Tester, RandomTester, MaxTester, Scoreboard) now live in
`../tinyalu-utils/src/tb2.rs`.

| Figure | Title | Where |
|---|---|---|
| 1 | The basic rustdv-UVM use model in hello_world | `src/lib.rs` (`hello_world_test`) |
| 2 | Hello, world! | transcript |
| 3 | The uvm_test tower, and its rustdv equivalent | text diagram |
| 4 | base_test — the shared run phase of every test | `src/lib.rs` |
| 5 | The tests build a tester and share base_test | `src/lib.rs` |
| 6 | RandomTest passes | transcript |
| 7 | max_test maxes all the operands | transcript |

All 3 tests end `REGRESSION: PASS` (RUSTDV_RANDOM_SEED=1).
