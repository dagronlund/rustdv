# Chapter 37: Fibonacci Testbench: 7.1 — figure map

Run with:

```
sim-common/run_sim.sh ch37_fibonacci_testbench_7_1 tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

| Figure | Title | Where |
|---|---|---|
| 1 | The Fibonacci sequence — each command needs the previous result | `src/lib.rs` (`FibonacciSeq`) |
| 2 | The miracle, made explicit | inside `FibonacciSeq::body` |
| 3 | The 7.1 driver returns responses through item_done | `src/lib.rs` (`RspDriver`) |
| 4 | The 7.1 environment | `src/lib.rs` (`FibEnv`) |
| 5 | The Fibonacci test | `src/lib.rs` (`fibonacci_test`) |
| 6 | The TinyALU computes Fibonacci numbers | transcript |

Ends `REGRESSION: PASS`; the logged list is [0, 1, 1, 2, 3, 5, 8, 13, 21].
