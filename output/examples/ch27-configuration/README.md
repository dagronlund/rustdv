# Chapter 27: Configuration — figure map

Run with:

```
sim-common/run_sim.sh ch27_configuration playground
```

| Figure | Title | Where |
|---|---|---|
| 1 | A component that needs configuration | `src/lib.rs` (`MsgLogger`) |
| 2 | The config struct mirrors the hierarchy | `src/lib.rs` (`MsgEnvConfig`/`MsgEnv`) |
| 3 | The test builds the config and hands it over | `src/lib.rs` (`msg_test`) |
| 4 | The loga and logb components have different things to say | transcript |
| 5 | "Wildcards" become one field used twice | `src/lib.rs` (`MultiMsgConfig`) |
| 6 | The "talk" components get the same message | transcript |
| 7 | "Global data" becomes a Default implementation | `src/lib.rs` (`GlobalConfig`) |
| 8 | Overriding some fields, defaulting the rest | `src/lib.rs` (`global_test`) |
| 9 | The default is matched only where nothing overrides it | transcript |
| 10 | The parent/child conflict has nowhere to live | `compile-fail/fig10_the_conflict_has_nowhere_to_live/` (E0062) |
| 11 | The shape of a real config tree (testbench 6.0's, previewed) | fragment — realized in ch34 |

All 3 tests end `REGRESSION: PASS`.
