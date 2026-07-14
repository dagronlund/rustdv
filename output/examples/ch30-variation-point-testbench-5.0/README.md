# Chapter 30: Variation-Point Testbench: 5.0 — figure map

Run with:

```
sim-common/run_sim.sh ch30_variation_point_testbench_5_0 tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

| Figure | Title | Where |
|---|---|---|
| 1 | The slot's contract, and the maker that fills it | `src/lib.rs` |
| 2 | One environment with a designed variation point | `src/lib.rs` (`AluEnvConfig`/`AluEnv`) |
| 3 | The shared test body takes a config | `src/lib.rs` (`run_test`) |
| 4 | random_test picks its tester with three visible lines | `src/lib.rs` |
| 5 | max_test differs only in the maker it sends | `src/lib.rs` |
| 6 | One env, two behaviors | transcript |

Both tests end `REGRESSION: PASS` (RUSTDV_RANDOM_SEED=1); results match
testbench 4.0's bit for bit.
