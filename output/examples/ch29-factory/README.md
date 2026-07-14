# Chapter 29: The Factory Problem — figure map

Run with:

```
sim-common/run_sim.sh ch29_factory playground
```

| Figure | Title | Where |
|---|---|---|
| 1 | A tiny example component | `src/lib.rs` (`TinyComponent`) |
| 2 | Instantiating the component by calling new() directly | `src/lib.rs` (`TinyEnv`) |
| 3 | The expected log message | transcript (`tiny_test`) |
| 4 | A designed variation point: the maker closure | `src/lib.rs` (`FlexEnvConfig`/`FlexEnv`) |
| 5 | The default maker builds the original component | transcript (`tiny_factory_test`) |
| 6 | The component a test will swap in | `src/lib.rs` (`MediumComponent`) |
| 7 | The override is an assignment, visible in the test | `src/lib.rs` (`medium_test`) |
| 8 | The environment builds the substitute | transcript |
| 9 | The factory, dispositioned | text table |

All 3 tests end `REGRESSION: PASS`.
