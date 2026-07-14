# Chapter 36: Sequence Testbench: 7.0

Everything until now has generated stimulus *inside* the testbench — testers wired into the env, swapped by generics or makers. The UVM's crown jewel inverts that: stimulus becomes *data-generating objects* called sequences, started by tests, fed to drivers through a handshake with guaranteed ordering. Testbench 7.0 adopts the machinery whole, and this is the one place rustdv ports a pyuvm subsystem *event for event* — because the handshake's ordering semantics are methodology, not mechanism, and the book that taught you `start_item`/`finish_item` would like them to still be true.

> **In Python we...** extended `uvm_sequence` and wrote a `body()` that looped: create an `AluSeqItem`, `await self.start_item(cmd_tr)` (returns when the driver is ready), set the operands — *late generation* — then `await self.finish_item(cmd_tr)` (returns when the driver calls `item_done()`). The driver pulled with `self.seq_item_port.get_next_item()`, and the test started it all with `seq.start(self.seqr)`.

## The handshake, preserved event for event

Before any code, the contract — the same five steps the Python book diagrammed, with the rustdv spellings:

```text
# Figure 1: The sequencer handshake

 Sequence                    Sequencer                   Driver
 --------                    ---------                   ------
 1. start_item(&mut cmd) --> enqueue; block until
                             this item's turn      <---- 2. get_next_item()
                                                         grants; blocks until
 3. fill operands;                                       the item is ready
    finish_item(cmd)     --> hand off payload      ----> returns SeqItem<REQ>
    block until done                                     4. drive the DUT...
                                                            item_done(rsp)
 5. finish_item returns  <-- release                <---- (rsp tagged with the
    (and rsp is fetchable                                  envelope's txn id)
     via get_response)
```

Step 3 is the point of the whole design: the operands are filled **after** the grant, at the moment the driver is ready — *late generation*, so stimulus can depend on the freshest state of the system. Every ordering in this table is observable in pyuvm and observable here.

## The trait and the two ports

```rust
// Figure 2: The Sequence trait

pub trait Sequence<REQ, RSP = REQ> {
    fn body<'a>(
        &'a mut self,
        ctx: SeqCtx<REQ, RSP>,
    ) -> Pin<Box<dyn Future<Output = Result<(), SeqError>> + 'a>>;
}
```

`uvm_sequence` became a one-method trait: `body`, as before. The signature carries this book's only user-facing `Pin<Box<...>>`, promised back in Chapter 15, and the reason is honest: a sequencer stores *heterogeneous* sequences — any type implementing the trait — so `body`'s future must be boxed to have one runtime shape. Write `Box::pin(async move { ... })` around your body and read past it forever after. The two type parameters are the request and response transaction types; `RSP` defaults to `REQ`, and the TinyALU's sequences ignore it until Chapter 37.

The sequence talks through its `SeqCtx` — `start_item`, `finish_item`, `get_response` — and the driver through its `SeqItemPort`:

```rust
// Figure 3: The driver's side of the handshake (from Chapter 33's Driver,
// now in its final form)

impl Component for Driver {
    fn start(&mut self, _ctx: &mut RunCtx) {
        let bfm = self.bfm.clone();
        let mut port = self.seq_item_port.take().expect("Driver started twice");
        spawn_named(
            async move {
                loop {
                    let item = port.get_next_item().await;
                    bfm.send_op(item.payload().clone()).await;
                    port.item_done(None);
                }
            },
            "driver",
        );
    }
}
```

Three pyuvm rules survive with upgraded enforcement. `get_next_item` twice without `item_done` — a `UVMSequenceError` in pyuvm — panics here with the same diagnosis (a testbench bug, per the taxonomy). The item arrives as a `SeqItem<AluCommand>` — Chapter 35's envelope — with the payload inside and the transaction id on the wrapper, where the driver can't lose it. And `item_done(None)` declares "no response" in its argument; a response-bearing driver writes `item_done(Some(rsp))` and the envelope tags it automatically — `set_context`, retired.

For reference, the full surfaces of both sides:

```rust
// Figure 4: The sequence-side and driver-side APIs

impl<REQ, RSP> SeqCtx<REQ, RSP> {
    pub async fn start_item(&mut self, item: &mut REQ);
    pub async fn finish_item(&mut self, item: REQ) -> Result<TxnId, SeqError>;
    pub async fn get_response(&mut self, txn_id: Option<TxnId>) -> RSP;
}

impl<REQ, RSP> SeqItemPort<REQ, RSP> {
    pub async fn get_next_item(&mut self) -> SeqItem<REQ>;
    pub fn item_done(&mut self, rsp: Option<RSP>);
}
```

## The sequences

```rust
// Figure 5: RandomSeq — late generation at grant time

pub struct RandomSeq {
    pub n_per_op: usize,
    pub rng: Rng,
}

impl Sequence<AluCommand> for RandomSeq {
    fn body<'a>(
        &'a mut self,
        mut ctx: SeqCtx<AluCommand>,
    ) -> Pin<Box<dyn Future<Output = Result<(), SeqError>> + 'a>> {
        Box::pin(async move {
            for _ in 0..self.n_per_op {
                for op in Ops::ALL {
                    let mut cmd = AluCommand { a: 0, b: 0, op };
                    ctx.start_item(&mut cmd).await;
                    // Late generation: fill at grant time.
                    cmd.a = self.rng.u8();
                    cmd.b = self.rng.u8();
                    ctx.finish_item(cmd).await?;
                }
            }
            Ok(())
        })
    }
}
```

```rust
// Figure 6: MaxSeq — same protocol, different data

impl Sequence<AluCommand> for MaxSeq {
    fn body<'a>(
        &'a mut self,
        mut ctx: SeqCtx<AluCommand>,
    ) -> Pin<Box<dyn Future<Output = Result<(), SeqError>> + 'a>> {
        Box::pin(async move {
            for op in Ops::ALL {
                let mut cmd = AluCommand { a: 0xFF, b: 0xFF, op };
                ctx.start_item(&mut cmd).await;
                ctx.finish_item(cmd).await?;
            }
            Ok(())
        })
    }
}
```

The bodies read like pyuvm's figures 9 and 10 merged: the Python design used a `BaseSeq` with an overridable `set_operands()` hook; the rustdv sequences just write their operand code between `start_item` and `finish_item`, because with sequences as plain values there is no env to protect from the difference — a base-with-hook remains available if a family of sequences shares a skeleton, but two ten-line sequences don't need a hierarchy. Note `finish_item(cmd).await?` *consumes* the command — after handoff, the sequence provably cannot touch the in-flight item, closing a subtle pyuvm hazard (any holder of the item handle could fire its events).

## Starting sequences

```rust
// Figure 7: The test starts a sequence on the sequencer

    let config = AluEnvConfig { bfm: bfm.clone(), is_active: Active::Active, enable_coverage: true };
    let mut env = AluEnv::new(config);

    let mut run_ctx = RunCtx::new();
    start_all(&mut env, &mut run_ctx);
    {
        let _obj = run_ctx.raise_objection(description);
        env.sequencer().start(seq).await?;
        bfm.wait_idle().await; // drain by knowledge, not by clock-counting
    }
    run_ctx.all_objections_dropped().await;
```

The env here is the Interlude's: sequencer, `Option<Driver>` (active/passive from the config enum), monitors, scoreboard, `Option<Coverage>` — the full 6.0 architecture with the sequencer replacing the tester-to-driver channel. Three details deserve the ink. `env.sequencer()` hands the test a clonable handle, doing the job pyuvm routed through `ConfigDB().get(self, "", "SEQR")` — the handle is typed (`Sequencer<AluCommand>`), so starting a sequence of the wrong transaction type is a compile error. `start(seq).await` is `seq.start(seqr)` with the receiver flipped, returning the sequence's own `Result` — a sequence can fail, and the `?` forwards it. And `bfm.wait_idle()` finally retires the drain hack: where pyuvm waited fifty `ClockCycles` "to do last transaction" and testbench 6.0 guessed twenty, the BFM now reports when its queue is empty and the handshake is quiet. Sequences know when they're done; the testbench should too.

```text
# Figure 8: Testbench 7.0 running
--
     75.00ns INFO     cmd_monitor: AluCommand { a: 193, b: 103, op: Add }
     75.00ns INFO     result_monitor: AluResult { result: 296 }
     95.00ns INFO     cmd_monitor: AluCommand { a: 94, b: 11, op: And }
     95.00ns INFO     result_monitor: AluResult { result: 10 }
    115.00ns INFO     cmd_monitor: AluCommand { a: 185, b: 128, op: Xor }
    115.00ns INFO     result_monitor: AluResult { result: 57 }
    135.00ns INFO     cmd_monitor: AluCommand { a: 165, b: 117, op: Mul }
    165.00ns INFO     result_monitor: AluResult { result: 19305 }
    185.00ns INFO     scoreboard: 4 compared, 0 mismatches
    185.00ns INFO     coverage: Add=1 And=1 Mul=1 Xor=1
    185.00ns INFO     random_test PASSED
```

The Interlude's transcript, earned line by line: struct transactions in the monitor narration, the scoreboard's counted verdict, coverage's tally — and underneath it, the handshake ticking through its five events per operation.

## Summary

Testbench 7.0 adopted the sequence machinery: `Sequence<REQ, RSP>` with a boxed-future `body` (the design's one visible `Pin`), `SeqCtx::start_item`/`finish_item` preserving pyuvm's grant-then-fill ordering — late generation intact — and the driver's `get_next_item`/`item_done` loop with the double-get error preserved as a panic. Transactions travel in `SeqItem` envelopes that own identity; `finish_item` consumes the payload, ending shared-handle mutation of in-flight items; tests reach the sequencer through a typed handle and start sequences as plain values — the per-test variation the factory chapters predicted would need no machinery at all. And `wait_idle` replaced clock-count draining with actual completion knowledge.

The response path — `item_done(Some(...))` and `get_response` — sat unused today. Testbench 7.1 needs it: stimulus that depends on the DUT's answers, demonstrated the traditional way, by making the TinyALU compute Fibonacci numbers.
