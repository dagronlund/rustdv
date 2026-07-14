# Chapter 25: uvm_env Testbench: 4.0 — figure map

Run with:

```
sim-common/run_sim.sh ch25_uvm_env_testbench_4_0 tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

| Figure | Title | Where |
|---|---|---|
| 1 | The 4.0 structure | text diagram |
| 2 | The tester as a component — start() is its run phase | `src/lib.rs` (`TesterComp`) |
| 3 | The 2.0 testers don't need to change | `src/lib.rs` (the `use tinyalu_utils::tb2::...` line) |
| 4 | The Scoreboard as a component | `src/lib.rs` |
| 5 | start() launches the monitoring tasks | `src/lib.rs` |
| 6 | Checking results in the check phase | `src/lib.rs` |
| 7 | pyuvm's environment tower, and its Rust replacement | text diagram |
| 8 | The environment: a struct whose fields are the testbench | `src/lib.rs` (`AluEnv`) |
| 9 | The shared test body: build the env, run the lifecycle | `src/lib.rs` (`run_env_test`) |
| 10 | The tests build the right environment | `src/lib.rs` |
| 11 | The tests running using environments | transcript |

Both tests end `REGRESSION: PASS` (RUSTDV_RANDOM_SEED=1).
