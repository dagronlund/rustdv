# Chapter 24: Components — figure map

Run with:

```
sim-common/run_sim.sh ch24_components playground
```

| Figure | Title | Where |
|---|---|---|
| 1 | pyuvm's nine phases, and where each went | text table |
| 2 | A component demonstrating the lifecycle methods | `src/lib.rs` (`PhaseComp`) |
| 3 | The test drives the lifecycle in order | `src/lib.rs` (`phase_test`) |
| 4 | The lifecycle runs in order | transcript |
| 5 | A three-level hierarchy: children are fields | `src/lib.rs` (`BottomComp`/`MiddleComp`/`TestTop`) |
| 6 | Constructors are the build phase | `src/lib.rs` (`hierarchy_test`) |
| 7 | The hierarchy, with names synthesized from field names | transcript |
| 8 | The predefined component taxonomy in rustdv | text table |

Both tests end `REGRESSION: PASS`. The WARNING in figure 4's transcript is
intentional — pyuvm's "you never objected" diagnostic, ported.
