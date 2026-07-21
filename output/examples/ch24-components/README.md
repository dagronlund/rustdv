# Chapter 24: Components — figure map

Run with:

```
sim-common/run_sim.sh ch24_components playground
```

No DUT: structure is the subject. `playground` is an empty top module.

## First half — running the phases (done)

The nine UVM phases are real `Component` methods again. `build` and
`connect` were destroyed in the first rustdv pass — demoted to "constructor
conventions" — and are restored here (D5/D6/D51). The test is a component;
the runner drives every phase in order, the way `@pyuvm.test()` hands the
class to its phaser. Each phase logs under the path the walk derived (D7),
so the lines read `[PhaseTest]`.

| Figure | Title | Where |
|---|---|---|
| 1 | A uvm_test demonstrating the nine phase methods | `src/ch24_components.rs` (`PhaseTest`) |
| 2 | The lifecycle runs in order | transcript below |

Port of the Python book's chapter 28 Figure 1. Transcript (seed 1):

```
      0.00ns INFO     rustdv: found 1 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running PhaseTest (1/1)  [ch24-components/src/ch24_components.rs:41]
      0.00ns INFO     [PhaseTest]: 1 build
      0.00ns INFO     [PhaseTest]: 2 connect
      0.00ns INFO     [PhaseTest]: 3 end_of_elaboration
      0.00ns INFO     [PhaseTest]: 4 start_of_simulation
      0.00ns INFO     [PhaseTest]: 5 run
      0.00ns INFO     [PhaseTest]: 6 extract
      0.00ns INFO     [PhaseTest]: 7 check
      0.00ns INFO     [PhaseTest]: 8 report
      0.00ns INFO     [PhaseTest]: 9 final
      0.00ns INFO     PhaseTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** PhaseTest                                    PASS           0.00      **
******************************************************************************
REGRESSION: PASS
```

Phase order is pyuvm's, not SV UVM's (D34): build top-down, connect
bottom-up, run bottom-up, the elaboration and post-run phases top-down. A
single component does not show direction — the three-level hierarchy does.

## Second half — building the hierarchy (next)

The Python book's Figures 4-6 build `TestTop -> mc -> bc`, where each parent
**creates its children in its own build phase** and the phaser recurses into
them. That is D6's two-stage construction (children as `Option<T>`), the
real restoration of late binding, and it is the next increment.
