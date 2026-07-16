# Chapter 38: get_response Testbench: 7.2

The Python book's 7.2 chapter taught `get_response()` as the *alternative* to shared-handle results: the driver built a response item, called `set_id_info(cmd)` to link it to its request, and sent it back through `item_done()`. rustdv's Chapter 37 already crossed that bridge — with no shared handles to write through, the response path *is* the way results come back, and the id bookkeeping vanished into the envelope. So this chapter teaches what remains of 7.2, which is the part pyuvm's ids existed for all along: **retrieving responses by transaction id**, in whatever order the test wants them.

> **In the UVM...** the driver created a response item and linked it to its command with `rsp.set_id_info(req)` before `item_done(rsp)`; the sequence awaited `get_response()`. The classic teaching closed with pitfalls — chiefly, that `get_response()` hangs when a driver never sends a response — and recommended the shared-handle style where it sufficed.

## The id you already had

Look back at Chapter 36's figure 4: `finish_item` returns `Result<TxnId, SeqError>`. Every handoff has been minting transaction ids since testbench 7.0; the Fibonacci sequence just ignored them, and `get_response(None)` meant "give me responses in FIFO order," which was safe with one item in flight. Collect the ids instead, and responses stop being a stream and become an *addressable set*:

```rust
// Figure 1: finish_item returns the transaction's id

pub struct CherryPickSeq {
    pub rng: Rng,
}

impl Sequence<AluCommand, AluResult> for CherryPickSeq {
    fn body<'a>(
        &'a mut self,
        mut ctx: SeqCtx<AluCommand, AluResult>,
    ) -> Pin<Box<dyn Future<Output = Result<(), SeqError>> + 'a>> {
        Box::pin(async move {
            let mut ids: Vec<(TxnId, Ops)> = Vec::new();
            for op in Ops::ALL {
                let mut cmd = AluCommand { a: 0, b: 0, op };
                ctx.start_item(&mut cmd).await;
                cmd.a = self.rng.u8();
                cmd.b = self.rng.u8();
                let id = ctx.finish_item(cmd).await?;
                ids.push((id, op));
            }

            // Figure 2: Cherry-picking responses by id,
            // in reverse order — arrival order no longer matters

            for (id, op) in ids.iter().rev() {
                let rsp = ctx.get_response(Some(*id)).await;
                log::info(&format!("response for {op:?} (txn {id:?}): {rsp:?}"));
            }
            Ok(())
        })
    }
}
```

The first loop sends all four operations without collecting a single response — they pile up in the sequencer's response queue as the driver produces them, each tagged with its request's id (the tagging pyuvm asked the driver to do with `set_id_info`, done by the port in `item_done`). The second loop retrieves them **in reverse**: `get_response(Some(id))` searches the queue for that id, blocking until it appears. FIFO order (`None`) and cherry-picking (`Some(id)`) are the two modes of pyuvm's `ResponseQueue`, preserved exactly.

```text
# Figure 3: Responses retrieved in the opposite of arrival order
--
     75.00ns INFO     cmd_monitor: AluCommand { a: 193, b: 103, op: Add }
     95.00ns INFO     cmd_monitor: AluCommand { a: 94, b: 11, op: And }
    115.00ns INFO     cmd_monitor: AluCommand { a: 185, b: 128, op: Xor }
    135.00ns INFO     cmd_monitor: AluCommand { a: 165, b: 117, op: Mul }
    165.00ns INFO     response for Mul (txn TxnId(4)): AluResult { result: 19305 }
    165.00ns INFO     response for Xor (txn TxnId(3)): AluResult { result: 57 }
    165.00ns INFO     response for And (txn TxnId(2)): AluResult { result: 10 }
    165.00ns INFO     response for Add (txn TxnId(1)): AluResult { result: 296 }
    185.00ns INFO     scoreboard: 4 compared, 0 mismatches
    185.00ns INFO     cherry_pick_test PASSED
```

Commands went down Add-And-Xor-Mul; responses came back Mul-Xor-And-Add, each correctly paired with its operation by id. The driver and env are Chapter 37's, unchanged — only the sequence's retrieval strategy differs, which is the point: response ordering is *sequence policy*, not testbench structure. This is the machinery that scales to the situations the TinyALU is too polite to create — several sequences sharing one sequencer, or a genuinely out-of-order DUT — where "whatever response comes first" would pair answers with the wrong questions.

## The pitfalls, ported honestly

One classic warning survives translation intact: **`get_response` on a request that will never get a response hangs forever.** Its example was a RAM whose writes produce no reply — a sequence calling `get_response()` after a write waits for a response the driver never sends. The rustdv failure mode is identical (the await parks on the response queue; eventually the test's timeout fires and the objection report names the survivor), and so is the discipline: *the sequence must know which requests respond*, calling `get_response` only for those. Where the response-per-request contract is real, encode it in the driver — `item_done(Some(...))` unconditionally — and where it is conditional, the honest shape is often a response type that says so: a `RSP = Option<ReadData>`, so even "no data" is a response and nothing hangs.

The other classic conclusion — prefer shared handles over `get_response` where possible — does not port, because the premise doesn't: there is no shared-handle option, and Chapter 37 showed the response path costing one `.await`. What *does* port is the underlying advice, restated for Rust: use `get_response(None)` when one item is in flight and order is obvious; collect ids the moment more than one item can be outstanding.

## Summary

Testbench 7.2 spent the transaction ids the envelope had been minting all along. `finish_item` returns each request's `TxnId`; the driver's `item_done(Some(rsp))` tags responses automatically — `set_id_info`, retired in Chapter 37, stayed retired — and `get_response(Some(id))` cherry-picks from the response queue regardless of arrival order, with `None` remaining the FIFO-order mode for single-item-in-flight sequences. The hang-on-missing-response pitfall ports unchanged, and its mitigations are contracts: respond to everything, or make "nothing" a response.

One rung remains on the ladder: sequences that coordinate *other sequences* — stimulus of stimulus — and the 8.0 testbench that runs them. The summit is next.
