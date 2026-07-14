# Chapter 19: TinyAluBfm — figure map

The BFM (figures 2–13) lives in the shared crate `../tinyalu-utils/src/lib.rs`;
the test (figures 14–18) is in `src/lib.rs` here. Run with:

```
sim-common/run_sim.sh ch19_tinyalubfm tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

| Figure | Title | Where |
|---|---|---|
| 1 | Every BFM loop lives on the falling edge | skeleton (fragment; the pattern of figs 5–7) |
| 2 | The TinyAluBfm struct — one owner of the pins | `tinyalu-utils/src/lib.rs` |
| 3 | Initializing the TinyAluBfm | `tinyalu-utils/src/lib.rs` |
| 4 | Centralizing the reset function | `tinyalu-utils/src/lib.rs` |
| 5 | Monitoring the result bus | `tinyalu-utils/src/lib.rs` (`result_mon`) |
| 6 | Monitoring the command signals | `tinyalu-utils/src/lib.rs` (`cmd_mon`) |
| 7 | Driving commands on the falling edge of clk | `tinyalu-utils/src/lib.rs` (`cmd_driver`) |
| 8 | Drive a command when the bus is idle | inside `cmd_driver` |
| 9 | If start is 1 check done | inside `cmd_driver` |
| 10 | Start the BFM tasks | `tinyalu-utils/src/lib.rs` (`start_tasks`) |
| 11 | The get_cmd() coroutine returns the next command | `tinyalu-utils/src/lib.rs` |
| 12 | The get_result() coroutine returns the next result | `tinyalu-utils/src/lib.rs` |
| 13 | send_op puts the command into the command Queue | `tinyalu-utils/src/lib.rs` |
| 14 | Starting a test by resetting the DUT and starting the BFM tasks | `src/lib.rs` (`test_alu`) |
| 15 | Creating a command and sending it | `src/lib.rs` |
| 16 | Wait to get the command from the DUT and store it in the coverage set | `src/lib.rs` |
| 17 | Wait for the result, then create a prediction | `src/lib.rs` |
| 18 | Check the result against the predicted result | `src/lib.rs` |
| 19 | Another successful test | transcript below |

Transcript (RUSTDV_RANDOM_SEED=1) — same operands as Chapter 18, on purpose:

```
     45.00ns INFO     PASSED: c1 Add 67 = 0128
     65.00ns INFO     PASSED: 5e And 0b = 000a
     85.00ns INFO     PASSED: b9 Xor 80 = 0039
    135.00ns INFO     PASSED: a5 Mul 75 = 4b69
    135.00ns INFO     Covered all operations
    135.00ns INFO     test_alu PASSED
REGRESSION: PASS
```
