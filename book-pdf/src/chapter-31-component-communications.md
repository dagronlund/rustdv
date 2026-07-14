# Chapter 31: Component Communications

Chapter 22 planted the problem: the 4.0/5.0 scoreboard calls `bfm.get_cmd()`, and whatever it takes, nobody else gets. Components need a standard way to send data to *each other* — the UVM's TLM (Transaction Level Modeling) system. This chapter ports it, and the port is the design documents' proudest compression: pyuvm implemented roughly thirty port/export classes; rustdv does the job with **two types and six methods**.

> **In Python we...** connected components with TLM-1 machinery: `uvm_blocking_put_port` and its export on the far side, `uvm_get_port`, `uvm_nonblocking_put_port`, peek variants, each with `connect()` calls in `connect_phase` — and a runtime `UVMTLMConnectionError` when the wiring was wrong. `uvm_tlm_fifo` sat between a producer's put port and a consumer's get port.

## The taxonomy, and what it was for

Strip TLM-1 to its questions and there are only three: does the operation *block or not*? Is it a *put, get, or peek*? And which side *initiates*? pyuvm answered with a class per combination — blocking/nonblocking × put/get/peek, times port/export duality, plus composites. rustdv answers with a channel:

```text
# Figure 1: Thirty classes, six methods

pyuvm                                   rustdv
-----                                   ------
uvm_blocking_put_port.put(item)         tx.send(item).await
uvm_nonblocking_put_port.try_put(item)  tx.try_send(item)     -> Result<(), TlmFull<T>>
uvm_blocking_get_port.get()             rx.recv().await
uvm_nonblocking_get_port.try_get()      rx.try_recv()         -> Result<T, TlmEmpty>
uvm_blocking_peek_port.peek()           rx.peek().await        (T: Clone)
uvm_nonblocking_peek_port.try_peek()    rx.try_peek()
port/export duality + connect()         the two ends of channel(capacity)
uvm_tlm_fifo                            TlmFifo<T> (a component wrapping a channel)
```

`channel::<T>(capacity)` returns `(Sender<T>, Receiver<T>)` — and the port/export bureaucracy dissolves into those two names. A port was "I initiate"; an export was "I am initiated upon"; `connect()` matched them at runtime. A `Sender` *is* the sending end, a `Receiver` *is* the receiving end, and "connecting" them is the act of handing each to its component — at construction, per Chapter 25's connect convention.

## Blocking communication

```rust
// Figure 2: A producer holds the Sender, a consumer the Receiver

async fn blocking_producer(tx: Sender<u32>) {
    for nn in 1..=3 {
        tx.send(nn).await.expect("receiver dropped");
        log::info(&format!("Sent {nn}"));
    }
}

async fn blocking_consumer(rx: Receiver<u32>) {
    while let Ok(datum) = rx.recv().await {
        log::info(&format!("Received {datum}"));
    }
    log::info("sender dropped — consumer retires");
}
```

```rust
// Figure 3: Blocking put/get is send/recv on a channel of size 1

#[rustdv::test]
async fn blocking_test(_ctx: TestCtx) -> Result<(), TestError> {
    let (tx, rx) = channel::<u32>(1);
    spawn_named(blocking_consumer(rx), "consumer");
    spawn_named(blocking_producer(tx), "producer").await.ok();
    Timer::ns(1).await;
    Ok(())
}
```

```text
# Figure 4: Alternating, as a size-1 channel must
--
      0.00ns INFO     Sent 1
      0.00ns INFO     Received 1
      0.00ns INFO     Sent 2
      0.00ns INFO     Received 2
      0.00ns INFO     Sent 3
      0.00ns INFO     Received 3
      0.00ns INFO     sender dropped — consumer retires
```

Chapter 16's queue ping-pong, upgraded with types and one genuinely new behavior. Look at the consumer's `while let Ok(...)` and the final log line: `send` and `recv` return `Result` because a channel end can be *dropped* — when the producer finishes and its `Sender` dies, the consumer's next `recv` returns `Err(Disconnected)`, and the consumer retires cleanly instead of blocking forever on a channel nobody will ever fill again. pyuvm had no equivalent, because Python references never die; the ownership system just handed us end-of-stream detection for free. (A driver whose sequencer has gone away can notice. File that thought.)

## Nonblocking communication

```rust
// Figure 5: Nonblocking put/get: the failure is a value

    tx.try_send(1).expect("channel was empty");
    log::info("try_send(1) succeeded");

    if let Err(TlmFull(rejected)) = tx.try_send(2) {
        log::info(&format!("channel full: {rejected} came back"));
    }

    let got = rx.try_recv().expect("channel had data");
    log::info(&format!("try_recv() -> {got}"));

    if rx.try_recv().is_err() {
        log::info("channel empty: try_recv() returned TlmEmpty");
    }
```

```text
--
      1.00ns INFO     try_send(1) succeeded
      1.00ns INFO     channel full: 2 came back
      1.00ns INFO     try_recv() -> 1
      1.00ns INFO     channel empty: try_recv() returned TlmEmpty
```

The pyuvm nonblocking ports returned a success boolean and made you check it (or forgot to); the rustdv signatures make ignoring failure a compiler warning and — the touch worth savoring — `TlmFull(rejected)` hands your item *back* on failure, so a full channel cannot eat a transaction. `peek`/`try_peek` (figure 6 in the chapter crate) round out the family: read without removing, requiring `T: Clone` because two parties will now hold the value — a bound pyuvm couldn't state and therefore couldn't enforce.

## Wiring mistakes, relocated

The Python book's chapter ended, as TLM chapters must, with the runtime failure: connect a port to the wrong thing and `UVMTLMConnectionError` arrives during elaboration — if you're lucky, and during a confused simulation if you're not. The rustdv equivalent, promised since the design documents:

```rust
// Figure 8: A direction mismatch is a type error

struct Driver {
    items: Receiver<u32>, // the driver consumes items
}

fn main() {
    let (tx, _rx) = channel::<u32>(1);
    // pyuvm: connecting a port to a port raised UVMTLMConnectionError at
    // run time. Here, handing the driver the wrong end doesn't compile:
    let _driver = Driver { items: tx };
}
```

```text
--
error[E0308]: mismatched types
  --> src/main.rs:14:35
   |
14 |     let _driver = Driver { items: tx };
   |                                   ^^ expected `Receiver<u32>`, found `Sender<u32>`
```

Direction mismatch: type error. Transaction-type mismatch (a `Sender<AluCommand>` into a slot wanting `Sender<AluResult>`): type error. Forgot to connect at all: missing constructor argument, type error. The whole `UVMTLMConnectionError` genus is extinct here, and `connect_phase` went with it — you cannot *have* an unconnected export when the export is a field that must be initialized.

## TlmFifo: when the FIFO belongs in the hierarchy

A bare channel is invisible plumbing. Sometimes the buffer should be a *citizen* — visible in the hierarchy, introspectable, flushable — which is `uvm_tlm_fifo`'s role, and `TlmFifo<T>` keeps it with the same surface:

```rust
// Figure 7: TlmFifo — a FIFO that lives in the hierarchy

    let fifo: TlmFifo<u32> = TlmFifo::new(Some(2));
    log::info(&format!("size={:?} used={} empty={}", fifo.size(), fifo.used(), fifo.is_empty()));
    fifo.put(7).await;
    fifo.put(8).await;
    log::info(&format!("size={:?} used={} full={}", fifo.size(), fifo.used(), fifo.is_full()));
    let x = fifo.get().await;
    log::info(&format!("got {x}; used={}", fifo.used()));
    fifo.flush();
    log::info(&format!("flushed; empty={}", fifo.is_empty()));
```

```text
--
      1.00ns INFO     size=Some(2) used=0 empty=true
      1.00ns INFO     size=Some(2) used=2 full=true
      1.00ns INFO     got 7; used=1
      1.00ns INFO     flushed; empty=true
```

`size()`, `used()`, `is_empty()`, `is_full()`, `flush()` — pyuvm's `uvm_tlm_fifo` surface, on a type that implements `ComponentNode` and can therefore sit as a `#[component(child)]` field, appearing in `print_hierarchy` like any other component. (pyuvm's default fifo size was 1; `TlmFifo::new(None)` is the unbounded spelling.) The transport/master/slave composites from the far end of the TLM-1 matrix are not ported: the Python book never taught them, pyuvm's own sources describe them as complexity inherited rather than chosen, and nothing in forty more chapters of TinyALU will miss them.

## Summary

TLM-1 ported as subtraction. One constructor — `channel::<T>(capacity)` — yields a `Sender` and a `Receiver` whose six methods cover blocking (`send`/`recv`/`peek`, awaited) and nonblocking (`try_send`/`try_recv`/`try_peek`, returning `Result` with the rejected item preserved) communication; direction and transaction type live in the type names, so every member of the `UVMTLMConnectionError` family is now a compile error at the construction site. Dropped endpoints turn into clean end-of-stream errors rather than eternal blocks. `TlmFifo<T>` keeps the hierarchy-visible FIFO with pyuvm's exact surface.

One communication pattern doesn't fit a channel, though: a monitor with *many* listeners — scoreboard, coverage, whoever else subscribes — where the sender must never block and never care who's listening. That is the analysis port, it is how the 6.0 testbench finally pries the scoreboard off the BFM, and it is next.
