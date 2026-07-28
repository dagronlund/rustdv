# Chapter 34: Connections — Testbench 6.0 — figure map

Run with:

```
sim-common/run_sim.sh ch34_connections_testbench_6_0 tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

| Figure | Title | Where |
|---|---|---|
| 1 | The Tester puts commands into a FIFO | `src/ch34_connections_testbench_6_0.rs` (`Tester`) |
| 2 | The Driver gets commands and drives the BFM | `src/ch34_connections_testbench_6_0.rs` (`Driver`) |
| 3 | The command monitor watches the bus and broadcasts | `src/ch34_connections_testbench_6_0.rs` (`CmdMonitor`) |
| 4 | The result monitor broadcasts results | `src/ch34_connections_testbench_6_0.rs` (`ResultMonitor`) |
| 5 | The Scoreboard subscribes to BOTH streams | `src/ch34_connections_testbench_6_0.rs` (`Scoreboard`) |
| 6 | Coverage subscribes to the command stream only | `src/ch34_connections_testbench_6_0.rs` (`Coverage`) |
| 7 | Build the components and the FIFOs; connect in one place | `src/ch34_connections_testbench_6_0.rs` (`AluEnv`) |
| 8 | The test is just the env | `src/ch34_connections_testbench_6_0.rs` (`AluTest`) |

One test, ending `REGRESSION: PASS`.

## Architecture

```
  Tester --put--> [cmd_fifo] --get--> Driver --> BFM --> DUT

  CmdMonitor --pub--> [cmd_bus] --sub--> Scoreboard
                           \----sub----> Coverage

  ResultMonitor --pub--> [result_bus] --sub--> Scoreboard
```

Every connection is the same shape — a concrete FIFO, a named export,
`connect(component, PORT_NAME)` — whether the traffic is point-to-point
(`TlmFifo`) or broadcast (`AnalysisFifo`).

## The Rust win worth noting

The Scoreboard needs **two** analysis streams. SystemVerilog cannot give one
class two `write` methods, so it needs the `uvm_analysis_imp_decl` macros;
pyuvm cannot express it at all with one `write` per class. Here each stream gets
its own `SubscribePort` and its own sink struct — and it works the same way when
both streams carry the same type, which is the case the SV macros exist for
(D20/D88).

## Transcript

Real Icarus output (`RUSTDV_RANDOM_SEED=1`):

```
      0.00ns INFO     rustdv: found 1 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running AluTest (1/1)
    240.00ns INFO     [AluTest.env.scoreboard]: PASSED: c1 Add 67 = 0128
    240.00ns INFO     [AluTest.env.scoreboard]: PASSED: 5e And 0b = 000a
    240.00ns INFO     [AluTest.env.scoreboard]: PASSED: b9 Xor 80 = 0039
    240.00ns INFO     [AluTest.env.scoreboard]: PASSED: a5 Mul 75 = 4b69
    240.00ns INFO     [AluTest.env.scoreboard]: Covered all operations
    240.00ns INFO     [AluTest.env.coverage]: coverage saw 4 of 4 ops
    240.00ns INFO     AluTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** AluTest                                      PASS         240.00      **
******************************************************************************
REGRESSION: PASS
```

## Why the Tester waits before dropping its objection

`put` returns when the FIFO accepts a command, not when the DUT has answered it.
Dropping the objection at that point ends the run phase with commands still in
the pipeline, and the scoreboard checks fewer results than it saw commands — it
does not fail, it silently checks less. The Python testbench waits ten clocks
after its last put; this one waits twenty, because the multiply is the last
operation and takes the longest to come back.

This chapter is what caught D82c: before it, the objection race dropped the whole
run tree, taking the components with it, and `check` never ran at all.
