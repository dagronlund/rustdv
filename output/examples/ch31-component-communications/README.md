# Chapter 31: Component Communications — figure map

Run with:

```
sim-common/run_sim.sh ch31_component_communications playground
```

| Figure | Title | Where |
|---|---|---|
| 1 | Thirty classes, six methods | text table |
| 2 | A producer holds the Sender, a consumer the Receiver | `src/lib.rs` |
| 3 | Blocking put/get is send/recv on a channel of size 1 | `src/lib.rs` (`blocking_test`) |
| 4 | Alternating, as a size-1 channel must | transcript |
| 5 | Nonblocking put/get: the failure is a value | `src/lib.rs` (`nonblocking_test`) |
| 6 | peek reads without removing (needs T: Clone) | `src/lib.rs` (`peek_test`) |
| 7 | TlmFifo — a FIFO that lives in the hierarchy | `src/lib.rs` (`fifo_test`) |

All 4 tests end `REGRESSION: PASS`.
