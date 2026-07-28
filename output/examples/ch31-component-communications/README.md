# Chapter 31: Component Communications — figure map

Run with:

```
sim-common/run_sim.sh ch31_component_communications playground
```

| Figure | Title | Where |
|---|---|---|
| 1 | A producer holds a put port and blocks on a full FIFO | `src/ch31_component_communications.rs` (`Producer`) |
| 2 | A consumer that peeks, then gets | `src/ch31_component_communications.rs` (`Consumer`) |
| 3 | The env builds the two components and a FIFO, then wires them | `src/ch31_component_communications.rs` (`PutGetPeekTest`) |
| 4 | A non-blocking producer never waits | `src/ch31_component_communications.rs` (`NbProducer`) |
| 5 | A non-blocking consumer | `src/ch31_component_communications.rs` (`NbConsumer`) |
| 6 | Same wiring, non-blocking components | `src/ch31_component_communications.rs` (`NonBlockingTest`) |
| 7 | A processing pipeline — y = 2x² | text + diagram |
| 8 | The first stage squares its input | `src/ch31_component_communications.rs` (`SquareIt`) |
| 9 | The second stage doubles what the first produced | `src/ch31_component_communications.rs` (`TimesTwo`) |
| 10 | The test drives the pipeline and checks the answer | `src/ch31_component_communications.rs` (`MathTest`) |
| 11 | A port left unconnected is an elaboration error | `src/ch31_component_communications.rs` (`UnconnectedTest`) |
| 12 | A FIFO's built-in analysis taps | `src/ch31_component_communications.rs` (`FifoTapTest`) |

Five tests, all ending `REGRESSION: PASS`.

## What this chapter proves

- **Connection reaches an erased child.** A parent holds children as
  `RustdvComp`, so `producer.put_port` does not exist to be written. Every
  connection here goes through `ComponentNode::port_slot` — a trait method,
  which is reachable through `dyn` where a cast is not (D83b).
- **A parent's `run` is concurrent with its children's.** `MathTest` sends `x`
  and waits for `y` while two worker components wait on it. Under a phaser that
  finishes the children first, this test cannot run at all (D82b).
- **Unconnected ports are found at elaboration, not at first use.**
  `UnconnectedTest` fails before any run phase, naming
  `UnconnectedTest.producer.put_port` (D22/D85).

## Transcript

Real Icarus output (`RUSTDV_RANDOM_SEED=1`):

```
      0.00ns INFO     rustdv: found 5 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running PutGetPeekTest (1/5)
      0.00ns INFO     [PutGetPeekTest.producer]: put 0
      0.00ns INFO     [PutGetPeekTest.consumer]: peeked 0
      0.00ns INFO     [PutGetPeekTest.consumer]: got 0
      0.00ns INFO     [PutGetPeekTest.producer]: put 1
      0.00ns INFO     [PutGetPeekTest.consumer]: peeked 1
      0.00ns INFO     [PutGetPeekTest.consumer]: got 1
      0.00ns INFO     [PutGetPeekTest.producer]: put 2
      0.00ns INFO     [PutGetPeekTest.consumer]: peeked 2
      0.00ns INFO     [PutGetPeekTest.consumer]: got 2
      0.00ns INFO     PutGetPeekTest PASSED
      0.00ns INFO     running NonBlockingTest (2/5)
      0.00ns INFO     [NonBlockingTest.producer]: put 0
      0.00ns INFO     [NonBlockingTest.producer]: FIFO full, retrying
      0.00ns INFO     [NonBlockingTest.consumer]: got 0
      1.00ns INFO     [NonBlockingTest.producer]: put 1
      1.00ns INFO     [NonBlockingTest.producer]: FIFO full, retrying
      1.00ns INFO     [NonBlockingTest.consumer]: got 1
      2.00ns INFO     [NonBlockingTest.producer]: put 2
      2.00ns INFO     [NonBlockingTest.consumer]: got 2
      2.00ns INFO     NonBlockingTest PASSED
      2.00ns INFO     running MathTest (3/5)
      2.00ns INFO     [MathTest.square_it]: 1² = 1
      2.00ns INFO     [MathTest.times_two]: 2 × 1 = 2
      2.00ns INFO     [MathTest]: PASSED: x=1, y=2
      2.00ns INFO     [MathTest.square_it]: 2² = 4
      2.00ns INFO     [MathTest.times_two]: 2 × 4 = 8
      2.00ns INFO     [MathTest]: PASSED: x=2, y=8
      2.00ns INFO     [MathTest.square_it]: 3² = 9
      2.00ns INFO     [MathTest.times_two]: 2 × 9 = 18
      2.00ns INFO     [MathTest]: PASSED: x=3, y=18
      2.00ns INFO     [MathTest.square_it]: 4² = 16
      2.00ns INFO     [MathTest.times_two]: 2 × 16 = 32
      2.00ns INFO     [MathTest]: PASSED: x=4, y=32
      2.00ns INFO     MathTest PASSED
      2.00ns INFO     running UnconnectedTest (4/5)
      2.00ns INFO     UnconnectedTest PASSED
      2.00ns INFO     running FifoTapTest (5/5)
      2.00ns INFO     [FifoTapTest.producer]: put 0
      2.00ns INFO     [FifoTapTest.consumer]: peeked 0
      2.00ns INFO     [FifoTapTest.consumer]: got 0
      2.00ns INFO     [FifoTapTest.producer]: put 1
      2.00ns INFO     [FifoTapTest.consumer]: peeked 1
      2.00ns INFO     [FifoTapTest.consumer]: got 1
      2.00ns INFO     [FifoTapTest.producer]: put 2
      2.00ns INFO     [FifoTapTest.consumer]: peeked 2
      2.00ns INFO     [FifoTapTest.consumer]: got 2
      2.00ns INFO     [FifoTapTest.watcher]: tap saw [0, 1, 2]
      2.00ns INFO     FifoTapTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** PutGetPeekTest                               PASS           0.00      **
** NonBlockingTest                              PASS           2.00      **
** MathTest                                     PASS           0.00      **
** UnconnectedTest                              PASS           0.00      **
** FifoTapTest                                  PASS           0.00      **
******************************************************************************
REGRESSION: PASS
```

`UnconnectedTest` passes *because* it fails: it is declared
`#[rustdv::test(expect_error = "tlm_unconnected_port")]`, and the elaboration
sweep reports

```
these TLM ports were declared but never connected:
  UnconnectedTest.producer.put_port (put)
```
