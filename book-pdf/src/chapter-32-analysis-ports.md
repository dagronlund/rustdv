# Chapter 32: Analysis Ports

Chapter 31 moved data from one component to one other component, with back-pressure: a full FIFO makes the producer wait, because a command that is not yet driven must not be dropped. A monitor lives in the opposite world. It observes traffic that has already happened, and it must tell *everyone who cares* — the scoreboard, the coverage collector, a logger — without being slowed by any of them, and without caring whether anyone is listening at all. That is broadcasting, the other TLM shape, and the UVM gives it its own machinery.

> **In the UVM...** a monitor holds a `uvm_analysis_port` and calls `ap.write(txn)`. Every connected subscriber's `write()` method runs — a subscriber extends `uvm_subscriber` and overrides `write()` — and the call returns immediately, in zero simulation time, no matter how many subscribers are connected, including none. Scoreboards typically route each incoming stream into a `uvm_tlm_analysis_fifo`, and a component that needed two streams reached for the `uvm_analysis_imp_decl` macros.

Analysis is a different mechanism from put/get, not a mode of it: one-to-many, non-blocking, no return value, no back-pressure. This chapter ports it — and teaches the one place where a UVM engineer's habits will mislead them, which is the question of *where the traffic goes*.

## A subscriber, and where its state lives

```rust
// Chapter 32, Figure 1: A subscriber counts what it sees
#[derive(Default)]
struct ItemCount {
    count: u32,
}

impl WriteSink<u32> for ItemCount {
    fn write(&mut self, _item: &u32) {
        self.count += 1;
    }
}

#[derive(Component, Default)]
struct Counter {
    #[port(subscribe)]
    input: SubscribePort<u32>,
    tally: RustdvShared<ItemCount>,
}

impl Component for Counter {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        let my_sink = self.tally.clone();
        self.input.on_write(my_sink);
    }

    fn report(&mut self, ctx: &mut RustdvCtx) {
        let tally = self.tally.get();
        ctx.info(&format!("counted {} items", tally.count));
    }
}
```

The pieces:

- `WriteSink<u32>` is the port of `uvm_subscriber`'s `write()`, expressed as a trait rather than a base class. The method is `write(&mut self, item)` — ordinary Rust, and the same word the UVM engineer already knows.
- The state `write` touches lives in its *own* struct, `ItemCount`, not directly on the component — and the reason is ownership, worth one paragraph because the pattern repeats in every subscriber you will ever write. Delivery must reach the subscriber's data while the publisher's `run` holds `&mut` on the publisher, and sibling components cannot reach into each other. So the port is handed a handle to the subscriber's **state**, not to the component: `tally` is a `RustdvShared<ItemCount>` — the cloneable shared handle from the Toolkit page — the component keeps one handle, `on_write` gives the port another, and both see the same data.
- `self.input.on_write(my_sink)` in `build` is the enrollment: when anything arrives on this port, call `my_sink.write`.

A second subscriber proves the fan-out — this one keeps the values instead of counting them:

```rust
// Chapter 32, Figure 2: A second subscriber on the same stream
#[derive(Default)]
struct SeenList {
    items: Vec<u32>,
}

impl WriteSink<u32> for SeenList {
    fn write(&mut self, item: &u32) {
        self.items.push(*item);
    }
}

#[derive(Component, Default)]
struct Collector {
    #[port(subscribe)]
    input: SubscribePort<u32>,
    seen: RustdvShared<SeenList>,
}

impl Component for Collector {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        let my_sink = self.seen.clone();
        self.input.on_write(my_sink);
    }

    fn report(&mut self, ctx: &mut RustdvCtx) {
        let seen = self.seen.get();
        ctx.info(&format!("collected {:?}", seen.items));
    }
}
```

A tally in one, a `Vec` in the other. Keep that difference in mind; it is about to become the chapter's thesis.

## The source and the broadcast

```rust
// Chapter 32, Figure 3: A source holds an analysis port and writes to it
#[derive(Component, Default)]
struct NumberGen {
    #[port(publish)]
    ap: PublishPort<u32>,
}

impl Component for NumberGen {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("generating");
        for n in 0..3 {
            self.ap.write(&n); // broadcast; non-blocking
            ctx.info(&format!("wrote {n}"));
        }
        Ok(())
    }
}
```

`write` takes a reference, returns immediately, and is not `async` — there is nothing to await, because delivery is synchronous and takes zero simulation time. The publisher does not block, does not learn how many subscribers heard it, and does not care.

```rust
// Chapter 32, Figure 4: One publisher, two subscribers, one hub
#[rustdv::test]
#[derive(Component, Default)]
struct BroadcastTest {
    #[component]
    source: RustdvComp,
    #[component]
    counter: RustdvComp,
    #[component]
    collector: RustdvComp,
    #[component]
    analysis_fifo: AnalysisBus<u32>,
}

impl Component for BroadcastTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.source = NumberGen::new_comp();
        self.counter = Counter::new_comp();
        self.collector = Collector::new_comp();
        self.analysis_fifo = AnalysisBus::new();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        self.analysis_fifo.pub_export().connect(&self.source, NumberGen::AP);
        self.analysis_fifo.sub_export().connect(&self.counter, Counter::INPUT);
        self.analysis_fifo.sub_export().connect(&self.collector, Collector::INPUT);
    }
}
```

The `AnalysisBus` is the hub that brokers the broadcast, and its wiring reads exactly like Chapter 31's: a concrete `#[component]` child, a named export, `connect(component, PORT_NAME)`. The publisher's port goes to `pub_export()`; every subscriber goes to the *same* `sub_export()`, and connecting several is what makes the write fan out. One connection idiom for both TLM shapes is a deliberate rustdv choice — the UVM broadcasts straight from port to subscribers with no intermediary, but with both endpoints factory-erased, neither side could drive the call, and one idiom for the reader to learn beats two.

```text
# Figure 5: One write, every subscriber hears it — all in zero time

      0.00ns INFO     running BroadcastTest (1/3)
      0.00ns INFO     [BroadcastTest.source]: wrote 0
      0.00ns INFO     [BroadcastTest.source]: wrote 1
      0.00ns INFO     [BroadcastTest.source]: wrote 2
      0.00ns INFO     [BroadcastTest.counter]: counted 3 items
      0.00ns INFO     [BroadcastTest.collector]: collected [0, 1, 2]
      0.00ns INFO     BroadcastTest PASSED
```

Every line is at `0.00ns`. Three writes, both subscribers fully served, and the simulation clock never moved — that is the contract `write` keeps.

## The hub holds nothing

Now the habit this chapter exists to correct. Despite living in a `#[component]` slot, **an `AnalysisBus` is not a FIFO and stores no items**. `write` calls every subscribed sink and returns. There is no queue in the hub; a datum broadcast to nobody is *gone*.

```rust
// Chapter 32, Figure 6: A hub with no subscribers is legal
#[rustdv::test]
#[derive(Component, Default)]
struct NoSubscribersTest {
    #[component]
    source: RustdvComp,
    #[component]
    analysis_fifo: AnalysisBus<u32>,
}

impl Component for NoSubscribersTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.source = NumberGen::new_comp();
        self.analysis_fifo = AnalysisBus::new();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        self.analysis_fifo.pub_export().connect(&self.source, NumberGen::AP);
        // no sub_export() connection — legal for analysis
    }
}
```

```text
# Figure 7: Broadcasting into the void

      0.00ns INFO     running NoSubscribersTest (2/3)
      0.00ns INFO     [NoSubscribersTest.source]: wrote 0
      0.00ns INFO     [NoSubscribersTest.source]: wrote 1
      0.00ns INFO     [NoSubscribersTest.source]: wrote 2
      0.00ns INFO     NoSubscribersTest PASSED
```

Where Chapter 31's unconnected put port failed elaboration, an analysis `sub_export()` has minimum cardinality zero: broadcasting to nobody is a valid state — a monitor in a block-level environment reused at chip level may well have no one listening — so this elaborates and runs clean. And the storing-nothing rule is the mechanism, not a limitation of it: a broadcast hub that stored what nobody wanted would grow forever, with the monitor paying for listeners it does not have.

So "where does the traffic go?" has a different answer here than a UVM engineer expects: **wherever the subscriber decides to put it.** A tally (figure 1), a `Vec` (figure 2), a comparison against a prediction (Chapter 34's scoreboard) — the subscriber owns its storage, chosen to fit its job. If you find yourself looking for the analysis FIFO, this paragraph is the answer: there isn't one, and nothing is missing.

It is worth being precise about what that replaces, because the UVM's scoreboards buffer for a reason that is real *there*. A SystemVerilog class gets exactly one `write()` method. A scoreboard watching two streams — commands and results — therefore needs the `uvm_analysis_imp_decl` macros to mint two differently-named writes, and routing each stream into its own `uvm_tlm_analysis_fifo` is the standard way around the whole problem; pyuvm, with one `write` per class, routes into FIFOs for the same reason. A rustdv component declares two `SubscribePort`s and writes two `WriteSink` impls, one per stream — you will see it done in Chapter 34's scoreboard — so the workaround has nothing to work around, and the buffer that lived in every UVM scoreboard is simply absent. (This is also the story behind the name `AnalysisBus`: the type is the broadcast hub, a thing the UVM has no class for at all, and an earlier name borrowed from `uvm_tlm_analysis_fifo` — a subscriber-side buffer, the one piece this design decided rustdv does not need — confused everyone who knew the original.)

## When the subscriber needs time

One legitimate reason to buffer remains, and it has nothing to do with `imp_decl`: **`write` cannot take simulation time.** It is synchronous, called from the publisher's `run`, and it is not `async` — no `await` is possible inside it. Bumping a counter fits. Consulting a slow reference model, driving a bus, waiting on the DUT does not. A subscriber whose real work takes time splits the job in two:

```rust
// Chapter 32, Figure 8: When the subscriber needs *time*
#[derive(Default)]
struct Inbox {
    queue: TlmFifo<u32>,
}

impl WriteSink<u32> for Inbox {
    fn write(&mut self, item: &u32) {
        let _ = self.queue.try_put(*item);
    }
}

#[derive(Component, Default)]
struct SlowChecker {
    #[port(subscribe)]
    input: SubscribePort<u32>,
    inbox: RustdvShared<Inbox>,
}

impl Component for SlowChecker {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.inbox = RustdvShared::new(Inbox { queue: TlmFifo::unbounded() });
        let my_inbox = self.inbox.clone();
        self.input.on_write(my_inbox);
    }

    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("checking");
        let my_queue = self.inbox.get().queue.handle();
        for _ in 0..3 {
            let n = my_queue.get().await;
            Timer::ns(5).await; // the slow work `write` could not have done
            ctx.info(&format!("checked {n}"));
        }
        Ok(())
    }
}
```

`write` does the one thing it can do instantly — `try_put` into a `TlmFifo` the subscriber owns — and the component's `run`, which may await all it likes, takes it from there. Three details repay attention:

- **The FIFO is connected to no port.** A reader fresh from Chapter 31 will expect every `TlmFifo` to be wired in `connect`; this one is an ordinary handoff *inside* one component, between a synchronous method and an asynchronous one. It is not part of the testbench topology.
- **The inbox must be unbounded.** `write` has no way to wait for space, and analysis has no back-pressure to push back with — so a *bounded* inbox here would be a bug that could only drop items. `TlmFifo::unbounded()` is the honest declaration of what analysis traffic is.
- **The handle is taken once, before the loop.** Holding the shared borrow (`self.inbox.get()`) across an `await` would keep the state locked exactly when `write` needs it; taking a queue handle first keeps the two halves out of each other's way.

```rust
// Chapter 32, Figure 9: The publisher does not wait for the slow subscriber
#[rustdv::test]
#[derive(Component, Default)]
struct SlowSubscriberTest {
    #[component]
    source: RustdvComp,
    #[component]
    checker: RustdvComp,
    #[component]
    analysis_fifo: AnalysisBus<u32>,
}

impl Component for SlowSubscriberTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.source = NumberGen::new_comp();
        self.checker = SlowChecker::new_comp();
        self.analysis_fifo = AnalysisBus::new();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        self.analysis_fifo.pub_export().connect(&self.source, NumberGen::AP);
        self.analysis_fifo.sub_export().connect(&self.checker, SlowChecker::INPUT);
    }
}
```

Nothing in the wiring says this subscriber buffers — the same `sub_export()` as any other. That is the checker's own business, which is the point.

```text
# Figure 10: Writes at 0ns; checks at 5, 10, 15

      0.00ns INFO     running SlowSubscriberTest (3/3)
      0.00ns INFO     [SlowSubscriberTest.source]: wrote 0
      0.00ns INFO     [SlowSubscriberTest.source]: wrote 1
      0.00ns INFO     [SlowSubscriberTest.source]: wrote 2
      5.00ns INFO     [SlowSubscriberTest.checker]: checked 0
     10.00ns INFO     [SlowSubscriberTest.checker]: checked 1
     15.00ns INFO     [SlowSubscriberTest.checker]: checked 2
     15.00ns INFO     SlowSubscriberTest PASSED
```

The transcript is the argument. All three writes land at `0.00ns` — the publisher is never held up by what a subscriber does with an item — and the checker's results come out at 5, 10, and 15ns as it works through its own queue in its own time.

## Summary

Analysis is broadcast: a publisher `write`s, every subscriber's sink runs, delivery is synchronous and free, and zero listeners is legal. A subscriber implements `WriteSink` per stream — two streams means two ports and two impls, no macros — and hands the port a shared handle to its state, because delivery must reach the data while the component itself is borrowed elsewhere. The `AnalysisBus` brokers the fan-out with the same connect idiom as every other wiring in the book, and it stores nothing: the subscriber owns the storage, shaped to its job, and the only reason to make that storage a queue is time — `write` cannot await, so a slow subscriber front-ends its `run` with an unbounded inbox and lets the transcript show writes at zero and checks at leisure.

Testbench 6.0 now has everything it needs: components that talk point-to-point, monitors that broadcast, and a scoreboard that subscribes to two streams at once. Chapter 33 builds those components; Chapter 34 wires them to the TinyALU.
