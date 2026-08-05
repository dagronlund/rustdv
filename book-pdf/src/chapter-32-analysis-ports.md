# Chapter 32: Analysis Ports

Chapter 31 moved data from one component to one other component, with back-pressure: a full FIFO makes the producer wait, because a command that is not yet driven must not be dropped. A monitor lives in the opposite world. It observes traffic that has already happened, and it must tell *everyone who cares* — the scoreboard, the coverage collector, a logger — without being slowed by any of them, and without caring whether anyone is listening at all. That is broadcasting, the other TLM shape, and the UVM gives it its own machinery.

> **In the UVM...** a monitor holds a `uvm_analysis_port` and calls `ap.write(txn)`. Every connected subscriber's `write()` method runs — a subscriber extends `uvm_subscriber` and overrides `write()` — and the call returns immediately, in zero simulation time, no matter how many subscribers are connected, including none. Scoreboards typically route each incoming stream into a `uvm_tlm_analysis_fifo`, and a component that needed two streams reached for the `uvm_analysis_imp_decl` macros.

Analysis is a different mechanism from put/get, not a mode of it: one-to-many, non-blocking, no return value, no back-pressure. Be reassured up front: **rustdv's analysis layer is a copy of the UVM's.** The subscribers implement `write()`, the publisher calls it, and delivery obeys the UVM's strictest rule — `write` takes no simulation time *(SV: `write` is a `function`, never a `task`; pyuvm: a plain `def`)*. What you know carries over. This chapter builds the layer up in order — the idea, the two ports, the `write` contract, one shared-state helper — and then runs it, because the parts arrive together in every listing and are easier to read once each has been met alone.

## One publisher, many subscribers

Start with the shape, because everything else in the chapter is machinery for it. One component — the **publisher** — has something to announce. In a real testbench it is a monitor: it has just decoded a bus transaction, and its job is to say so. Several components — the **subscribers** — want to hear it: a scoreboard to compare it against a prediction, a coverage collector to bin it, perhaps a logger to file it. Each does something *different* with the *same* item.

Two properties define the relationship. The publisher does not know its subscribers — not how many there are, not what they do, not whether there are any at all. It announces and moves on. And no subscriber can slow the publisher down or dictate to another: each is handed the item, does its own thing, and has no channel back. That is why analysis has no back-pressure and no return value; a broadcast is not a conversation.

Everything below is the rustdv spelling of that shape, and the spelling is the UVM's: a publisher port, subscriber ports, and a `write()` that fans out in zero time.

## The two ports

The publisher declares a `PublishPort<T>`; each subscriber declares a `SubscribePort<T>`. Both are `#[port(...)]` fields, exactly as in Chapter 31, and the attribute does the same two jobs: it generates the typed name constant the wiring uses, and it enrolls the port in the elaboration report.

```rust,ignore
#[port(publish)]
ap: PublishPort<CmdTuple>,        // the publisher announces here

#[port(subscribe)]
input: SubscribePort<CmdTuple>,   // a subscriber listens here
```

The publisher's side is the simpler of the two: it calls `self.ap.write(&item)` and is done. The subscriber's side needs two more ideas, and they are the only two new ideas in the chapter.

**The first is `WriteSink` — the trait that carries `write()`.** In the UVM, a subscriber extends `uvm_subscriber` and overrides its `write()` method. rustdv keeps the method and swaps the base class for a trait: `WriteSink<T>` has exactly one method, `fn write(&mut self, item: &T)`, and implementing the trait is how you say what an arriving item should do. Same word, same job, same rule — it returns nothing and must take no time.

There is one twist, and it is deliberate: you implement `WriteSink` on the **state** the write updates — a small struct holding a count, a `Vec`, whatever the subscriber keeps — not on the subscriber component itself. The reason is ownership, and it gets the next section to itself.

**The second is `on_write` — and how it relates to `connect`.** A subscriber's port needs two attachments, made by two different parties, in two different phases:

- `connect`, in the *parent's* `connect` phase, attaches the port to a stream — *which broadcast this subscriber hears*. That is topology, decided from outside, exactly as it was for every port in Chapter 31.
- `on_write`, in the *subscriber's own* `build` phase, hands the port the sink an arriving item should be delivered to — *what hearing it does*. That is behavior, decided by the component that owns the port, and no one else can decide it: only the subscriber holds a handle to its own state.

So `connect` says *whose traffic*, `on_write` says *what happens to it*, and both must occur. Forgetting `connect` is legal — an unheard subscriber simply never runs, and the elaboration report will show the port unbound. Forgetting `on_write` is a wiring error, and it is loud: `connect` fails naming the port and telling you which component's `build` phase owes the call.

## A digression: `RustdvShared`

Now the ownership twist, because the first listing is unreadable without it.

`write` takes `&mut self` — a tally must be incremented, a `Vec` pushed. But look at *when* delivery happens: the publisher is inside its `run` phase, which holds `&mut` on the publisher, and the item must reach a sibling component's data in the same instant. Chapter 24 built the tree so that a parent owns its children and siblings cannot reach into each other — a rule that has served every chapter since, and analysis delivery is the one place it would forbid the whole mechanism.

The way out is to share the **state**, not the component. The subscriber keeps its data in its own small struct — the one `WriteSink` is implemented on — behind a `RustdvShared<T>`:

- **What it is.** A cloneable handle to shared state: Chapter 13's `Rc<RefCell<T>>`, wrapped in a framework type. `clone()` produces another handle to the *same* data, not a copy of it; `get()` borrows the state to read and `get_mut()` to modify, checked at run time as `RefCell` always is.
- **How it is used.** The component keeps one handle as a field. In `build`, it clones a second handle and passes the clone to `on_write`. From then on the port can deliver into the state directly — no `&mut` on any component, no sibling reaching into sibling — while the component reads the same state whenever it likes, typically in `check` or `report`.
- **Why the name.** It wears the `Rustdv` prefix for the same reason `RustdvComp` and `RustdvCtx` do: it is the framework's type, not the language's. A reader who goes looking for `Shared<T>` in the standard library will find nothing; the name says where to look instead.

Keep the borrows short — take `get()`'s guard for a line, not across an `await` — and the pattern has no sharp edges. You will write these three lines (a state struct, a `WriteSink` impl, an `on_write` in `build`) in every subscriber from here to the end of the book, so the first listing is worth a slow read.

## A counter and a collector

The chapter's example is deliberately not a TinyALU testbench. It is the publisher/subscriber shape with nothing else in the room: a number generator that publishes `0, 1, 2`, and two subscribers that hear the same three numbers and do different things with them — a **counter** that keeps a tally, and a **collector** that keeps the values. Chapter 33 will put monitors and scoreboards in these roles; today the data is plain `u32`s so the machinery has your whole attention.

Here is the counter, all three pieces of the pattern in one place:

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

Read it against the last two sections. `ItemCount` is the state, and `WriteSink` is implemented there — `write` bumps the count, instantly, nothing awaited. `Counter` is the component: it declares the `SubscribePort`, keeps one `RustdvShared` handle in `tally`, and in `build` hands a clone of that handle to `on_write`. In `report`, it reads the same state back through `get()`. The component never sees an item arrive; arrival goes straight into `ItemCount`, and the component and the port simply share it.

The collector is the same pattern with different state — a `Vec` where the counter had a number:

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

What connects a publisher to its subscribers is an `AnalysisBus` — the hub that brokers the broadcast:

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

The wiring reads exactly like Chapter 31's: a concrete `#[component]` child, a named export, `connect(component, PORT_NAME)`. The publisher's port goes to `pub_export()`; every subscriber goes to the *same* `sub_export()`, and connecting several is what makes the write fan out. One connection idiom for both TLM shapes is a deliberate rustdv choice — the UVM broadcasts straight from port to subscribers with no intermediary, but with both endpoints factory-erased, neither side could drive the call, and one idiom for the reader to learn beats two.

```text
# Figure 5: One write, every subscriber hears it — all in zero time

      0.00ns INFO     running BroadcastTest (1/3)  [ch32-analysis-ports/src/ch32_analysis_ports.rs:192]
      0.00ns INFO     [BroadcastTest.source]: wrote 0
      0.00ns INFO     [BroadcastTest.source]: wrote 1
      0.00ns INFO     [BroadcastTest.source]: wrote 2
      0.00ns INFO     [BroadcastTest.counter]: counted 3 items
      0.00ns INFO     [BroadcastTest.collector]: collected [0, 1, 2]
      0.00ns INFO     BroadcastTest PASSED
```

Every line is at `0.00ns`. Three writes, both subscribers fully served, and the simulation clock never moved — that is the contract `write` keeps. And note where the results came from when `report` ran: the counter read its `ItemCount` and the collector its `SeenList`, each through the same `RustdvShared` handle whose clone its port had been delivering into all along. The shared state is the join between the zero-time world of `write` and the component that eventually wants the answer.

## The bus stores nothing

Despite living in a `#[component]` slot, **an `AnalysisBus` is not a FIFO and stores no items.** It is a subscriber list and nothing more: `write` calls every enrolled sink and returns, connecting function calls rather than holding data, and a datum broadcast to nobody is *gone*.

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

      0.00ns INFO     running NoSubscribersTest (2/3)  [ch32-analysis-ports/src/ch32_analysis_ports.rs:230]
      0.00ns INFO     [NoSubscribersTest.source]: wrote 0
      0.00ns INFO     [NoSubscribersTest.source]: wrote 1
      0.00ns INFO     [NoSubscribersTest.source]: wrote 2
      0.00ns INFO     NoSubscribersTest PASSED
```

Where Chapter 31's unconnected put port failed elaboration, an analysis `sub_export()` has minimum cardinality zero: broadcasting to nobody is a valid state — a monitor in a block-level environment reused at chip level may well have no one listening — so this elaborates and runs clean. And the storing-nothing rule is the mechanism, not a limitation of it: a broadcast hub that stored what nobody wanted would grow forever, with the monitor paying for listeners it does not have.

So "where does the traffic go?" has a simple answer: **wherever the subscriber decides to put it.** A tally (figure 1), a `Vec` (figure 2), a comparison against a prediction (Chapter 34's scoreboard) — the subscriber owns its storage, held in its `RustdvShared` state and shaped to its job. If you find yourself looking for the analysis FIFO, this paragraph is the answer: there isn't one, and nothing is missing.

It is worth being precise about what that replaces, because the UVM's scoreboards buffer for a reason that is real *there*. A SystemVerilog class gets exactly one `write()` method. A scoreboard watching two streams — commands and results — therefore needs the `uvm_analysis_imp_decl` macros to mint two differently-named writes, and routing each stream into its own `uvm_tlm_analysis_fifo` is the standard way around the whole problem; pyuvm, with one `write` per class, routes into FIFOs for the same reason. A rustdv component declares two `SubscribePort`s and writes two `WriteSink` impls, one per stream — you will see it done in Chapter 34's scoreboard — so the workaround has nothing to work around, and the buffer that lived in every UVM scoreboard is simply absent. (Hence the name `AnalysisBus`: the type is the broadcast hub, a thing the UVM has no class for at all — emphatically not an analysis FIFO, which in the UVM names the subscriber-side buffer this design does without.)

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

      0.00ns INFO     running SlowSubscriberTest (3/3)  [ch32-analysis-ports/src/ch32_analysis_ports.rs:324]
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

Analysis is one publisher and many subscribers: the publisher `write`s, every subscriber's `write()` runs, delivery is synchronous and free, and zero listeners is legal — the UVM's analysis layer, carried over whole, including the rule that `write` takes no time. A subscriber implements `WriteSink` on the state its write updates, shares that state between component and port with a `RustdvShared` handle, and makes two attachments: `on_write` in its own `build` says what an arriving item does, `connect` in the parent's `connect` phase says whose traffic it hears. Two streams means two ports and two `WriteSink` impls, no macros. The `AnalysisBus` brokers the fan-out with the same connect idiom as every other wiring in the book, and it stores nothing: the subscriber owns the storage, shaped to its job, and the only reason to make that storage a queue is time — `write` cannot await, so a slow subscriber front-ends its `run` with an unbounded inbox and lets the transcript show writes at zero and checks at leisure.

Testbench 6.0 now has everything it needs: components that talk point-to-point, monitors that broadcast, and a scoreboard that subscribes to two streams at once. Chapter 33 builds those components; Chapter 34 wires them to the TinyALU.
