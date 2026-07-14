# Chapter 38: get_response Testbench: 7.2 — figure map

Run with:

```
sim-common/run_sim.sh ch38_get_response_testbench_7_2 tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

| Figure | Title | Where |
|---|---|---|
| 1 | finish_item returns the transaction's id | `src/lib.rs` (`CherryPickSeq`) |
| 2 | Cherry-picking responses by id, in reverse order | inside `CherryPickSeq::body` |
| 3 | Responses retrieved in the opposite of arrival order | transcript |

Ends `REGRESSION: PASS` (RUSTDV_RANDOM_SEED=1). The driver/env are ch37's shapes.
