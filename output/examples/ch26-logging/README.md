# Chapter 26: Logging — figure map

Run with:

```
sim-common/run_sim.sh ch26_logging playground
```

| Figure | Title | Where |
|---|---|---|
| 1 | Logging messages of all levels | `src/lib.rs` (`LogComp`) |
| 2 | The default level is Info | transcript (`log_test`) |
| 3 | Logging levels | text table |
| 4 | Setting the logging level for a hierarchy | `src/lib.rs` (`debug_test`) |
| 5 | Now the debug message prints | transcript |
| 6 | Logging to a file | `src/lib.rs` (`file_test`); writes `/tmp/rustdv_ch26_log.txt` |

All 3 tests end `REGRESSION: PASS`.
