# Chapter 39: Virtual Sequence Testbench: 8.0

The summit. Sequences generate stimulus; *virtual* sequences coordinate sequences — no items, no `start_item`, just a program that launches other sequences in whatever order and concurrency the test demands. Testbench 8.0 is the Python book's final version, and it arrives here with one upgrade the type system supplies for free.

> **In Python we...** wrote `TestAllSeq(uvm_sequence)` whose `body()` fetched the sequencer from the ConfigDB and ran `rand_seq.start(seqr)` then `max_seq.start(seqr)`; the test started the virtual sequence *without* a sequencer argument. A parallel variant used `start_soon` and `Combine`. And if a virtual sequence mistakenly called `start_item()`, pyuvm raised `UVMSequenceError` at runtime.

## A virtual sequence is a program

```rust
// Figure 1: A virtual sequence starts other sequences

pub struct TestAllSeq {
    pub seqr: Sequencer<AluCommand>,
    pub rng: Rng,
}

impl TestAllSeq {
    pub async fn body(&mut self) -> Result<(), SeqError> {
        // No start_item, no finish_item — no item context to call them on.
        let mut rand_seq = RandomSeq { rng: self.rng.clone() };
        let mut max_seq = MaxSeq;
        self.seqr.start(&mut rand_seq).await?;
        self.seqr.start(&mut max_seq).await?;
        Ok(())
    }
}
```

Look at what `TestAllSeq` is *not*: it does not implement `Sequence`, because `Sequence::body` receives a `SeqCtx` — an item channel — and a virtual sequence has no items. It is a plain struct holding the sequencer handle it will conduct, with an ordinary `async fn body`. That structural difference is the upgrade: pyuvm policed "virtual sequences must not call `start_item`" with a runtime `UVMSequenceError`; here the misuse is *unwritable* — there is no `ctx` in scope to call `start_item` on. The design documents call this making the illegal state unrepresentable, and this is its cleanest appearance in the book.

The sequencer handle arrives as a field — where pyuvm's virtual sequence pulled `"SEQR"` from the ConfigDB, ours is configured like everything else since Chapter 27: by construction. A virtual sequence coordinating *several* buses holds several sequencer fields, each typed to its transaction, and starting a sequence on the wrong bus is a compile error.

```rust
// Figure 2: The test starts the virtual sequence

    {
        let _obj = run_ctx.raise_objection("virtual sequence");
        let mut vseq = TestAllSeq { seqr: env.sequencer(), rng: ctx.rng() };
        vseq.body().await?;
        bfm.wait_idle().await;
    }
```

```text
# Figure 3: Running RandomSeq then MaxSeq
--
     75.00ns INFO     cmd_monitor: AluCommand { a: 193, b: 103, op: Add }
     95.00ns INFO     cmd_monitor: AluCommand { a: 94, b: 11, op: And }
    115.00ns INFO     cmd_monitor: AluCommand { a: 185, b: 128, op: Xor }
    135.00ns INFO     cmd_monitor: AluCommand { a: 165, b: 117, op: Mul }
    185.00ns INFO     cmd_monitor: AluCommand { a: 255, b: 255, op: Add }
    205.00ns INFO     cmd_monitor: AluCommand { a: 255, b: 255, op: And }
    225.00ns INFO     cmd_monitor: AluCommand { a: 255, b: 255, op: Xor }
    ...
    305.00ns INFO     scoreboard: 8 compared, 0 mismatches
    305.00ns INFO     coverage: Add=2 And=2 Mul=2 Xor=2
    305.00ns INFO     test_all PASSED
```

Random operands, then a wall of `0xFF` — the Python book's figure 3, in order, eight compares, all covered.

## Running sequences in parallel

```rust
// Figure 4: Running sub-sequences in parallel

impl TestAllParallelSeq {
    pub async fn body(&mut self) -> Result<(), SeqError> {
        let seqr_a = self.seqr.clone();
        let seqr_b = self.seqr.clone();
        let rng = self.rng.clone();
        let random_task = spawn_named(
            async move { seqr_a.start(&mut RandomSeq { rng }).await },
            "random_seq",
        );
        let max_task = spawn_named(async move { seqr_b.start(&mut MaxSeq).await }, "max_seq");
        let (r1, r2) = join2(random_task, max_task).await;
        r1.map_err(|e| SeqError(format!("{e:?}")))??;
        r2.map_err(|e| SeqError(format!("{e:?}")))??;
        Ok(())
    }
}
```

`cocotb.start_soon` plus `Combine` became `spawn_named` plus `join2` — Chapter 16 vocabulary, because a virtual sequence's body is ordinary async code and *all* the task machinery applies. The sequencer handle clones freely (clones share the one queue), each spawned task owns its clone and its sequence, and the sequencer's FIFO arbitration interleaves the two item streams:

```text
# Figure 5: The two sequences interleave at the sequencer
--
    320.00ns INFO     cmd_monitor: AluCommand { a: 206, b: 66, op: Add }
    340.00ns INFO     cmd_monitor: AluCommand { a: 255, b: 255, op: Add }
    360.00ns INFO     cmd_monitor: AluCommand { a: 47, b: 100, op: And }
    380.00ns INFO     cmd_monitor: AluCommand { a: 255, b: 255, op: And }
    400.00ns INFO     cmd_monitor: AluCommand { a: 41, b: 179, op: Xor }
    420.00ns INFO     cmd_monitor: AluCommand { a: 255, b: 255, op: Xor }
    ...
    610.00ns INFO     scoreboard: 8 compared, 0 mismatches
    610.00ns INFO     coverage: Add=2 And=2 Mul=2 Xor=2
    610.00ns INFO     test_all_parallel PASSED
```

Random and max commands alternating, exactly as pyuvm's parallel figure showed — each sequence's own items stay in order (the handshake guarantees it), and the interleaving *between* sequences is the sequencer's FIFO fairness at work. (Two double-`?` lines in figure 4 deserve their gloss: awaiting a spawned task yields `Result<_, TaskError>` — Chapter 16 — wrapping the sequence's own `Result`. One `?` per layer of fallibility; nothing is silently dropped, including a sub-sequence that failed.)

The Python book's chapter continued into pyuvm's `fork`-style helpers and sequence priorities; rustdv, like pyuvm, ships FIFO arbitration only — grab/lock/priority remain unported on both sides of the language divide, a gap all the design documents record rather than hide.

## The ladder, complete

Version 8.0 closes the climb that began with one `while` loop:

```text
# Figure 6: Ten testbenches, one DUT

1.0  one loop, everything mixed             (Ch. 18)
     + the BFM: pins extracted              (Ch. 19)
2.0  testers and scoreboard as structs      (Ch. 20)
3.0  the methodology's test discipline      (Ch. 23)
4.0  components + environment               (Ch. 25)
5.0  one env, variation points              (Ch. 30)
6.0  single-purpose components, channels,
     analysis fan-out                       (Ch. 33–34)
7.0  sequences: stimulus as data            (Ch. 36)
7.1  the response path                      (Ch. 37)
7.2  transaction ids, cherry-picking        (Ch. 38)
8.0  virtual sequences: stimulus programs   (this chapter)
```

Same summit as the Python book, same version numbers meaning the same steps — by the steeper, more scenic route Chapter 1 promised.

## Summary

Testbench 8.0 added the coordination layer: virtual sequences as plain structs with async `body` methods and sequencer-handle fields — not `Sequence` implementors, so the no-items rule that pyuvm enforced with `UVMSequenceError` is enforced by there being nothing to misuse. Sequential composition is two awaited `start` calls; parallel composition is `spawn_named` plus `join2` with cloned sequencer handles, the sequencer's FIFO arbitration interleaving item streams while each stream keeps its internal order. Configuration reached the virtual sequence the same way it reaches everything: through a constructor.

Part IV is complete — every pyuvm chapter has its rustdv companion, and the TinyALU has been verified eleven ways. What remains is to see it all in one place: the capstone testbench, whole, end to end, as a reference you can build from. Chapter 40.
