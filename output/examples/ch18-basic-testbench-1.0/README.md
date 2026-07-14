# Chapter 18: Basic Testbench: 1.0 — figure map

Testbench version 1.0 against the real TinyALU. Run with:

```
sim-common/run_sim.sh ch18_basic_testbench_1_0 tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

| Figure | Title | Where |
|---|---|---|
| 1 | The TinyALU's interface | `sim-common/hdl/tinyalu.sv` (port list) |
| 2 | The operation enumeration | `src/lib.rs` (`Ops`) |
| 3 | The prediction function for the scoreboard | `src/lib.rs` (`alu_prediction`) |
| 4 | The start of the TinyALU test. Reset the DUT | `src/lib.rs` (`alu_test`) |
| 5 | Creating one transaction for each operation | `src/lib.rs` (`alu_test`) |
| 6 | Creating a TinyALU command | `src/lib.rs` (`alu_test`) |
| 7 | Erroring on a state that must never happen | `src/lib.rs` (`alu_test`) |
| 8 | If we are in an operation, continue | `src/lib.rs` (`alu_test`) |
| 9 | The operation is complete | `src/lib.rs` (`alu_test`) |
| 10 | Checking results against the prediction | `src/lib.rs` (`alu_test`) |
| 11 | Checking functional coverage using a set | `src/lib.rs` (`alu_test`) |
| 12 | The final check relays pass/fail to rustdv | `src/lib.rs` (`alu_test`) |
| 13 | A successful test | transcript below |

Transcript (RUSTDV_RANDOM_SEED=1):

```
      0.00ns INFO     running alu_test (1/1)  [ch18-basic-testbench-1.0/src/lib.rs:46]
     35.00ns INFO     PASSED: c1 Add 67 = 0128
     55.00ns INFO     PASSED: 5e And 0b = 000a
     75.00ns INFO     PASSED: b9 Xor 80 = 0039
    125.00ns INFO     PASSED: a5 Mul 75 = 4b69
    125.00ns INFO     Covered all operations
    125.00ns INFO     alu_test PASSED
REGRESSION: PASS
```
