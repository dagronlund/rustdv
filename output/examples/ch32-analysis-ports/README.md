# Chapter 32: Analysis Ports — figure map

Run with:

```
sim-common/run_sim.sh ch32_analysis_ports playground
```

| Figure | Title | Where |
|---|---|---|
| 1 | A Subscriber counts what it sees | `src/lib.rs` (`OpCounter`) |
| 2 | One write, every subscriber hears it | `src/lib.rs` (`fan_out_test`) |
| 3 | A second subscriber, three lines long | `src/lib.rs` (`OpLogger`) |
| 4 | The broadcast reaches both | transcript |
| 5 | An AnalysisFifo turns broadcast into a stream | `src/lib.rs` (`analysis_fifo_test`) |
| 6 | Zero subscribers is not an error | `src/lib.rs` (`no_subscribers_test`) |

All 3 tests end `REGRESSION: PASS`.
