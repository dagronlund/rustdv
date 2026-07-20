# Chapter 32: Analysis Ports

Channels are point-to-point: one sender, one receiver, backpressure by design. A monitor's needs are the opposite — it publishes what it saw to *whoever cares*, must never block on a slow listener, and shouldn't fail if nobody is listening at all. That is the **analysis port**, the UVM's broadcast primitive, and the piece that finally lets the 6.0 testbench give the scoreboard and coverage their own independent feeds.

> **In the UVM...** monitors owned a `uvm_analysis_port` and called `ap.write(item)`; components extending `uvm_subscriber` overrode `write()` to receive the broadcast, and `uvm_tlm_analysis_fifo` buffered the stream for components that wanted to consume it at their own pace. Forgetting to override `write()` raised pyuvm's `UVMFatalError` at call time — SystemVerilog, to its credit, made `write()` pure virtual and caught the omission at compile time.

## The Subscriber trait

The receiving side first, because it is the smallest piece of UVM machinery to survive as *itself*: a trait with one required method.

```rust
// Figure 1: A Subscriber counts what it sees

struct OpCounter {
    counts: HashMap<Ops, u32>,
}

impl Subscriber<Ops> for OpCounter {
    fn write(&mut self, item: &Ops) {
        *self.counts.entry(*item).or_insert(0) += 1;
    }
}
```

This is `uvm_subscriber` with the enforcement moved: pyuvm's `write()` was "abstract" in the raise-at-runtime sense — forget to override it and the first broadcast produced `UVMFatalError`. A rustdv type claiming `Subscriber<Ops>` without a `write` does not compile. Note also the argument type: `&Ops`, a borrow. The port lends each subscriber the item; a subscriber that needs to *keep* it clones explicitly, so the cost of ownership is visible and paid only by those who incur it — pyuvm cloned defensively for everyone, or didn't and hoped.

## The port, and the fan-out

```rust
// Figure 2: One write, every subscriber hears it

#[rustdv::test]
async fn fan_out_test(_ctx: TestCtx) -> Result<(), TestError> {
    let ap: AnalysisPort<Ops> = AnalysisPort::new();

    let coverage = Rc::new(RefCell::new(OpCounter::new()));
    let logger_sub = Rc::new(RefCell::new(OpLogger));
    ap.connect(coverage.clone());
    ap.connect(logger_sub);
    log::info(&format!("{} subscribers connected", ap.subscriber_count()));

    for op in [Ops::Add, Ops::Mul, Ops::Add] {
        ap.write(&op); // non-blocking, no matter how many listen
    }
    coverage.borrow().report();
    Ok(())
}
```

```rust
// Figure 3: A second subscriber, three lines long

struct OpLogger;

impl Subscriber<Ops> for OpLogger {
    fn write(&mut self, item: &Ops) {
        log::info(&format!("saw {item:?}"));
    }
}
```

```text
# Figure 4: The broadcast reaches both
--
      0.00ns INFO     2 subscribers connected
      0.00ns INFO     saw Add
      0.00ns INFO     saw Mul
      0.00ns INFO     saw Add
      0.00ns INFO     coverage: Add=2 Mul=1
```

Three `write()` calls, two listeners, six deliveries, zero awaits — `write` is a plain function, not a coroutine, because a broadcast that could block would let one slow subscriber stall the monitor and, behind it, the protocol being monitored. That non-negotiable is why the analysis port survives as a named type instead of dissolving into `channel()`: **a channel is a queue with backpressure; an analysis port is a megaphone.** Semantically different, so nominally different.

One Rust honesty note, because you can see it in the figure: subscribers connect as `Rc<RefCell<dyn Subscriber<T>>>`. The port and the test both need to reach the coverage object — genuinely shared, genuinely mutated — and Chapter 13's escape hatch is the correct spelling of that. This is the pattern's cost, and Chapter 33 shows the alternative that most scoreboard-shaped components prefer: don't subscribe an object at all; attach a FIFO and pull.

## AnalysisFifo: broadcast to stream

The `write()`-callback style suits collectors that react item by item. A scoreboard would rather *consume* — await items in its own task, at its own pace. `connect_fifo()` bridges the two worlds:

```rust
// Figure 5: An AnalysisFifo turns broadcast into a stream

#[rustdv::test]
async fn analysis_fifo_test(_ctx: TestCtx) -> Result<(), TestError> {
    let ap: AnalysisPort<Ops> = AnalysisPort::new();
    let fifo: AnalysisFifo<Ops> = ap.connect_fifo();

    ap.write(&Ops::Xor);
    ap.write(&Ops::And);
    log::info(&format!("fifo holds {} items", fifo.len()));

    // A task drains the fifo at its own pace — write() never waited for it.
    let first = fifo.get().await;
    let second = fifo.get().await;
    log::info(&format!("drained {first:?} then {second:?}"));
    Ok(())
}
```

```text
--
      0.00ns INFO     fifo holds 2 items
      0.00ns INFO     drained Xor then And
```

`uvm_tlm_analysis_fifo`, ported: unbounded (a bounded buffer would reintroduce the blocking the port exists to forbid), fed by the broadcast, drained by `get().await` or `try_get()`. The unboundedness is the honest trade — a subscriber that never drains will grow its FIFO without limit — and it is the same trade pyuvm made for the same reason.

And the degenerate case, which is not degenerate at all in a passive testbench:

```rust
// Figure 6: Zero subscribers is not an error

    let ap: AnalysisPort<Ops> = AnalysisPort::new();
    ap.write(&Ops::Add); // fire and forget: nobody listening, nobody hurt
```

Zero-or-more subscribers is the analysis port's contract. A monitor publishes identically whether the env attached a scoreboard, a coverage collector, both, or — in some bring-up configuration — nothing. The monitor cannot know, which is exactly the decoupling Chapter 22 ordered.

## Summary

The analysis port ported as the one communication primitive that stays itself: `AnalysisPort<T>` broadcasts with a non-blocking, non-awaiting `write(&T)` to zero or more listeners; `Subscriber<T>` is `uvm_subscriber` with `write` enforced at compile time instead of by `UVMFatalError`; and `AnalysisFifo<T>` (via `connect_fifo()`) buffers the broadcast for consumers that prefer to pull, unbounded so the publisher never waits. Items travel by borrow, cloning only where a subscriber keeps them; callback-style subscribers ride in `Rc<RefCell<...>>`, the visible price of genuine sharing.

The toolbox is complete: lifecycle, environments, configs, variation points, channels, broadcasts. Testbench 6.0 now assembles all of it into the architecture the Interlude previewed — driver, two monitors, scoreboard, coverage, every connection an endpoint passed at construction — across the next two chapters: the components first, then the wiring.
