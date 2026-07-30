# Chapter 37: The Repair Desk: Testbench 7.1 — figure map

Run with:

```
sim-common/run_sim.sh ch37_repair_desk_testbench_7_1 playground
```

Figure numbers are the **book's** (D110: one numbering space per chapter, code
and transcripts drawn from the same sequence); the `.rs` captions carry the same
numbers. The hanging-`get_response` passage is prose in the book, not a numbered
figure, and the crate's comment for it carries no number.

| Figure | Title | Where |
|---|---|---|
| 1 | A job and its receipt | `src/ch37_repair_desk_testbench_7_1.rs` (`RepairJob`, `RepairDone`) |
| 2 | A driver that takes work in and gives it back later | `src/ch37_repair_desk_testbench_7_1.rs` (`RepairDesk`) |
| 3 | Drop off four machines, then collect four receipts | `src/ch37_repair_desk_testbench_7_1.rs` (`RepairSeq`) |
| 4 | The shop and its test | `src/ch37_repair_desk_testbench_7_1.rs` (`ShopEnv`, `RepairTest`) |
| 5 | Four repairs, finished out of order | transcript — `RepairTest` |

One test, ending `REGRESSION: PASS`.

## Transcript

**Figure 5.** Verbatim from `sim-common/run_sim.sh
ch37_repair_desk_testbench_7_1 playground`, `RUSTDV_RANDOM_SEED=1`.

This is the chapter's whole argument in one log. Four machines go in — laptop,
desktop, tablet, server — and they come back **in a different order**: the
laptop first (2 clocks), then the desktop and server together at 115ns, then
the tablet last at 125ns even though it arrived third. Each is matched to its
owner by ticket number, not by arrival order.

```
      0.00ns INFO     rustdv: found 1 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running RepairTest (1/1)  [ch37-repair-desk-testbench-7.1/src/ch37_repair_desk_testbench_7_1.rs:269]
     15.00ns INFO     [RepairTest.env.desk]: took in laptop (#1), 2 clocks
     15.00ns INFO     [RepairSeq]: dropped off laptop, ticket #1
     30.00ns INFO     [RepairTest.env.desk]: ready: laptop (#1) after 2 ticks
     35.00ns INFO     [RepairTest.env.desk]: took in desktop (#2), 8 clocks
     35.00ns INFO     [RepairSeq]: dropped off desktop, ticket #2
     55.00ns INFO     [RepairTest.env.desk]: took in tablet (#3), 7 clocks
     55.00ns INFO     [RepairSeq]: dropped off tablet, ticket #3
     75.00ns INFO     [RepairTest.env.desk]: took in server (#4), 4 clocks
     75.00ns INFO     [RepairSeq]: dropped off server, ticket #4
     85.00ns INFO     [RepairSeq]: collected laptop (#1) after 2 ticks
    110.00ns INFO     [RepairTest.env.desk]: ready: desktop (#2) after 8 ticks
    110.00ns INFO     [RepairTest.env.desk]: ready: server (#4) after 4 ticks
    115.00ns INFO     [RepairSeq]: collected desktop (#2) after 8 ticks
    115.00ns INFO     [RepairSeq]: collected server (#4) after 4 ticks
    120.00ns INFO     [RepairTest.env.desk]: ready: tablet (#3) after 7 ticks
    125.00ns INFO     [RepairSeq]: collected tablet (#3) after 7 ticks
    125.00ns INFO     RepairTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** RepairTest                                   PASS         125.00      **
******************************************************************************
REGRESSION: PASS
```

## What this chapter proves

- **Answers can come back out of order, and the ticket is what matches them.**
  The TinyALU cannot show this — it runs one operation at a time, so nothing
  ever returns out of sequence. That is why the mechanism is taught here first,
  on a device that does nothing but wait, before Chapter 38 applies it to the
  real DUT.
- **`try_next_item` lets the driver take new work while old work is still in
  flight.** The desk accepts the desktop at 35ns while the laptop is still
  being repaired.
- **Asking for a receipt that will never exist hangs forever.** Nothing can
  distinguish "not ready yet" from "never coming", so it is the sequence
  writer's job to ask only for answers that are owed. There is no runnable
  figure for it, because the only way to show it is a test that never ends.
