# Chapter 36: Sequence Testbench: 7.0 — figure map

Run with:

```
sim-common/run_sim.sh ch36_sequence_testbench_7_0 tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

| Figure | Title | Where |
|---|---|---|
| 1 | The sequencer handshake | text diagram |
| 2 | The Sequence trait | fragment (rustdv-uvm/src/sequence.rs) |
| 3 | The driver's side of the handshake | `../tinyalu-utils/src/tb7.rs` (`Driver`) |
| 4 | The sequence-side and driver-side APIs | fragment (rustdv-uvm/src/sequence.rs) |
| 5 | RandomSeq — late generation at grant time | `src/lib.rs` |
| 6 | MaxSeq — same protocol, different data | `src/lib.rs` |
| 7 | The test starts a sequence on the sequencer | `src/lib.rs` (`run_seq_test`) |
| 8 | Testbench 7.0 running | transcript |

Both tests end `REGRESSION: PASS` (RUSTDV_RANDOM_SEED=1).
