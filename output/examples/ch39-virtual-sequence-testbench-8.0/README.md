# Chapter 39: Virtual Sequence Testbench: 8.0 — figure map

Run with:

```
sim-common/run_sim.sh ch39_virtual_sequence_testbench_8_0 tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

| Figure | Title | Where |
|---|---|---|
| 1 | A virtual sequence starts other sequences | `src/lib.rs` (`TestAllSeq`) |
| 2 | The test starts the virtual sequence | `src/lib.rs` (`build_and_run`) |
| 3 | Running RandomSeq then MaxSeq | transcript |
| 4 | Running sub-sequences in parallel | `src/lib.rs` (`TestAllParallelSeq`) |
| 5 | The two sequences interleave at the sequencer | transcript |
| 6 | Ten testbenches, one DUT | text table |

Both tests end `REGRESSION: PASS` (RUSTDV_RANDOM_SEED=1), 8 compared each.
