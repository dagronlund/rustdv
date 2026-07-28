# Chapter 32: Analysis Ports — figure map

Run with:

```
sim-common/run_sim.sh ch32_analysis_ports playground
```

| Figure | Title | Where |
|---|---|---|
| 1 | A subscriber counts what it sees | `src/ch32_analysis_ports.rs` (`Counter`) |
| 2 | A second subscriber on the same stream | `src/ch32_analysis_ports.rs` (`Collector`) |
| 3 | A source holds an analysis port and writes to it | `src/ch32_analysis_ports.rs` (`NumberGen`) |
| 4 | One publisher, two subscribers, one hub | `src/ch32_analysis_ports.rs` (`BroadcastTest`) |
| 5 | A hub with no subscribers is legal | `src/ch32_analysis_ports.rs` (`NoSubscribersTest`) |
| 6 | The same hub also buffers — pull instead of push | `src/ch32_analysis_ports.rs` (`BufferedTest`) |

Three tests, all ending `REGRESSION: PASS`.

## What this chapter proves

- **Delivery is synchronous and takes no simulation time.** `ap.write(&n)`
  returns after every subscriber's handler has run; the whole transcript below
  is at `0.00ns` (D87).
- **A subscriber shares its state, not itself.** The handler needs `&mut` its
  data while the publisher's `run` holds `&mut publisher`, and siblings cannot
  reach each other — so the data lives in a `RustdvShared<T>` and the port holds
  a second handle (D88).
- **One connection idiom.** `pub_export()`/`sub_export()` read exactly like
  Chapter 31's `put_export()`/`get_export()`; several subscribers on one
  `sub_export()` is what makes it a broadcast.
- **`AnalysisFifo` is not `TlmFifo`.** Broadcast versus queue: every subscriber
  sees every item, nothing is consumed, nobody blocks (D86). The same hub also
  offers `get_export()` for a component that would rather pull.

## Transcript

Real Icarus output (`RUSTDV_RANDOM_SEED=1`):

```
      0.00ns INFO     rustdv: found 3 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running BroadcastTest (1/3)
      0.00ns INFO     [BroadcastTest.source]: wrote 0
      0.00ns INFO     [BroadcastTest.source]: wrote 1
      0.00ns INFO     [BroadcastTest.source]: wrote 2
      0.00ns INFO     [BroadcastTest.counter]: counted 3 items
      0.00ns INFO     [BroadcastTest.collector]: collected [0, 1, 2]
      0.00ns INFO     BroadcastTest PASSED
      0.00ns INFO     running NoSubscribersTest (2/3)
      0.00ns INFO     [NoSubscribersTest.source]: wrote 0
      0.00ns INFO     [NoSubscribersTest.source]: wrote 1
      0.00ns INFO     [NoSubscribersTest.source]: wrote 2
      0.00ns INFO     NoSubscribersTest PASSED
      0.00ns INFO     running BufferedTest (3/3)
      0.00ns INFO     [BufferedTest.source]: wrote 0
      0.00ns INFO     [BufferedTest.source]: wrote 1
      0.00ns INFO     [BufferedTest.source]: wrote 2
      0.00ns INFO     [BufferedTest.drainer]: drained 0
      0.00ns INFO     [BufferedTest.drainer]: drained 1
      0.00ns INFO     [BufferedTest.drainer]: drained 2
      0.00ns INFO     BufferedTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** BroadcastTest                                PASS           0.00      **
** NoSubscribersTest                            PASS           0.00      **
** BufferedTest                                 PASS           0.00      **
******************************************************************************
REGRESSION: PASS
```
