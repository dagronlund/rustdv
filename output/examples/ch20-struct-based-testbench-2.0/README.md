# Chapter 20: Struct-Based Testbench: 2.0 — figure map

Run with:

```
sim-common/run_sim.sh ch20_struct_based_testbench_2_0 tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

| Figure | Title | Where |
|---|---|---|
| 1 | Tester structure | text diagram |
| 2 | Common behavior across all testers | `src/lib.rs` (`Tester`) |
| 3 | RandomTester overrides get_operands() | `src/lib.rs` |
| 4 | MaxTester overrides get_operands() | `src/lib.rs` |
| 5 | Initializing the Scoreboard | `src/lib.rs` |
| 6 | The Scoreboard's data-gathering tasks | `src/lib.rs` (`start_tasks`) |
| 7 | The check_results() phase | `src/lib.rs` |
| 8 | The Scoreboard checks functional coverage | inside `check_results` |
| 9 | The execute_test coroutine starts the tasks | `src/lib.rs` |
| 10 | Execute the tester | inside `execute_test` |
| 11 | The tests launch execute_test with a tester | `src/lib.rs` (`random_test`) |
| 12 | The max test differs only in its tester | `src/lib.rs` (`max_test`) |
| 13 | Two tests, one testbench | transcript below |

Transcript (RUSTDV_RANDOM_SEED=1):

```
    145.00ns INFO     PASSED: c1 Add 67 = 0128
    145.00ns INFO     PASSED: 5e And 0b = 000a
    145.00ns INFO     PASSED: b9 Xor 80 = 0039
    145.00ns INFO     PASSED: a5 Mul 75 = 4b69
    145.00ns INFO     Covered all operations
    145.00ns INFO     random_test PASSED
    290.00ns INFO     PASSED: ff Add ff = 01fe
    290.00ns INFO     PASSED: ff And ff = 00ff
    290.00ns INFO     PASSED: ff Xor ff = 0000
    290.00ns INFO     PASSED: ff Mul ff = fe01
    290.00ns INFO     Covered all operations
    290.00ns INFO     max_test PASSED
REGRESSION: PASS
```
