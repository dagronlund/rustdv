# Chapters 33–34: Testbench 6.0 — figure map

Components (ch33 figures 2–8) live in `../tinyalu-utils/src/tb6.rs`; the
environment and tests (ch34 figures 1–2) in `src/lib.rs` here. Run with:

```
sim-common/run_sim.sh ch34_connections_testbench_6_0 tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

Ch33: fig 2 TesterComp, fig 3 Driver, fig 5 Monitor<T>, fig 6 get_cmd/get_result,
fig 7 Coverage, fig 8 Scoreboard (all in tb6.rs). Ch34: fig 1 AluEnv::new wiring,
fig 2 run_test, fig 3 transcript, fig 4 agent pattern (fragment; realized in ch40).

Both tests end `REGRESSION: PASS` (RUSTDV_RANDOM_SEED=1).
