# Chapter 37: Fibonacci Testbench: 7.1

Testbench 7.0 sent commands and ignored the results — monitors caught them downstream. That is fine until the *next* command depends on the *last* answer, which real stimulus often does. Testbench 7.1 makes the dependency vivid the traditional way: the TinyALU computes Fibonacci numbers, each ADD consuming the previous ADD's sum. In pyuvm the result traveled back through a shared handle — the driver wrote `cmd.result`, and the sequence, still holding the same object, read it. Rust does not do shared-handle telepathy, and this chapter's quiet thesis is that the honest alternative — the response path — was the better design all along.

> **In the UVM...** we wrote `FibonacciSeq`, whose body reused one sequence item: `start_item`, set the operands to the last two numbers, `finish_item` — "and when the coroutine returns, a miracle happens: `cmd.result` contains the sum." The driver performed the miracle by writing through the shared handle: it got the result from the BFM and copied it into the sequence item before `item_done()` — a classic move in SystemVerilog and pyuvm alike.

## The sequence

```rust
// Figure 1: The Fibonacci sequence — each command needs the previous result

pub struct FibonacciSeq;

impl Sequence<AluCommand, AluResult> for FibonacciSeq {
    fn body<'a>(
        &'a mut self,
        mut ctx: SeqCtx<AluCommand, AluResult>,
    ) -> Pin<Box<dyn Future<Output = Result<(), SeqError>> + 'a>> {
        Box::pin(async move {
            let mut prev_num: u8 = 0;
            let mut cur_num: u8 = 1;
            let mut fib_list: Vec<u16> = vec![prev_num as u16, cur_num as u16];
            for _ in 0..7 {
                let mut cmd = AluCommand { a: 0, b: 0, op: Ops::Add };
                ctx.start_item(&mut cmd).await;
                cmd.a = prev_num;
                cmd.b = cur_num;
                ctx.finish_item(cmd).await?;

                // Figure 2: the miracle, made explicit

                let rsp = ctx.get_response(None).await;
                fib_list.push(rsp.result);
                prev_num = cur_num;
                cur_num = rsp.result as u8;
            }
            log::info(&format!("Fibonacci Sequence: {fib_list:?}"));
            Ok(())
        })
    }
}
```

Two changes from Chapter 36's sequences carry the whole story. The trait is now `Sequence<AluCommand, AluResult>` — the second parameter, defaulted away until now, names the response type. And after `finish_item` comes `ctx.get_response(None).await`: an explicit request for the answer, blocking until the driver supplies it. Where the old style narrated a miracle — the result *appearing* in the item you were still holding — the Rust sequence *asks*, and the type system explains why it must: `finish_item(cmd)` consumed the command (Chapter 36 called this closing a hazard), so there is no shared handle for anyone to write a result into. The response path isn't a workaround for missing dynamism; it makes the data flow visible in the code — request goes down, response comes back, each with a type.

## The driver

```rust
// Figure 3: The 7.1 driver returns responses through item_done

impl Component for RspDriver {
    fn start(&mut self, _ctx: &mut RunCtx) {
        let bfm = self.bfm.clone();
        let ap = self.result_ap.clone();
        let mut port = self.seq_item_port.take().expect("driver started twice");
        spawn_named(
            async move {
                loop {
                    let item = port.get_next_item().await;
                    bfm.send_op(item.payload().clone()).await;
                    // Wait for THIS operation's result before moving on...
                    let result = bfm.get_result().await;
                    log::info(&format!("driver: {result:?}"));
                    ap.write(&result);
                    // ...and hand it back to the sequence.
                    port.item_done(Some(result));
                }
            },
            "rsp_driver",
        );
    }
}
```

The pyuvm 7.1 driver's three additions, all present: it gets the result from the BFM, writes it to an analysis port (the scoreboard still needs its feed — the driver has absorbed the result monitor's job, exactly as in pyuvm), and returns it to the sequence — `item_done(Some(result))` where 7.0 wrote `item_done(None)`. The port's type grew to match: `SeqItemPort<AluCommand, AluResult>`, and a driver that promised responses but passed `None` would still compile — but the *sequence's* `get_response` would wait forever, and the objection timeout would name the hang. One thing the driver does **not** do: call `set_context` or touch a transaction id. The response is tagged with the in-flight envelope's id by the port itself — the bookkeeping pyuvm asked the driver to remember is now unforgettable.

Note also what awaiting the result does to pacing: this driver *serializes* — no new command until the last answer is back. For Fibonacci that is the requirement, not a cost. A pipelined DUT wants the 7.0 driver shape plus out-of-order response matching, which is precisely Chapter 38's subject.

## The test

The environment swaps in the response-bearing pieces — `Sequencer<AluCommand, AluResult>`, the `RspDriver`, command monitor and scoreboard as before (figure 4 in the chapter crate) — and the test is Chapter 36's with a different sequence:

```rust
// Figure 5: The Fibonacci test

    let mut env = FibEnv::new(bfm.clone());

    let mut run_ctx = RunCtx::new();
    start_all(&mut env, &mut run_ctx);
    {
        let _obj = run_ctx.raise_objection("fibonacci sequence");
        env.sequencer().start(&mut FibonacciSeq).await?;
        bfm.wait_idle().await;
    }
    run_ctx.all_objections_dropped().await;
```

```text
# Figure 6: The TinyALU computes Fibonacci numbers
--
     75.00ns INFO     cmd_monitor: AluCommand { a: 0, b: 1, op: Add }
     75.00ns INFO     driver: AluResult { result: 1 }
     95.00ns INFO     cmd_monitor: AluCommand { a: 1, b: 1, op: Add }
     95.00ns INFO     driver: AluResult { result: 2 }
    115.00ns INFO     cmd_monitor: AluCommand { a: 1, b: 2, op: Add }
    115.00ns INFO     driver: AluResult { result: 3 }
    135.00ns INFO     cmd_monitor: AluCommand { a: 2, b: 3, op: Add }
    135.00ns INFO     driver: AluResult { result: 5 }
    155.00ns INFO     cmd_monitor: AluCommand { a: 3, b: 5, op: Add }
    155.00ns INFO     driver: AluResult { result: 8 }
    175.00ns INFO     cmd_monitor: AluCommand { a: 5, b: 8, op: Add }
    175.00ns INFO     driver: AluResult { result: 13 }
    195.00ns INFO     cmd_monitor: AluCommand { a: 8, b: 13, op: Add }
    195.00ns INFO     driver: AluResult { result: 21 }
    195.00ns INFO     Fibonacci Sequence: [0, 1, 1, 2, 3, 5, 8, 13, 21]
    215.00ns INFO     scoreboard: 7 compared, 0 mismatches
    215.00ns INFO     fibonacci_test PASSED
```

The same nine numbers the earlier books logged, computed the same way — each command's operands visibly built from the previous result in the monitor narration — and the scoreboard, fed by the driver's analysis port, confirms the DUT did the arithmetic honestly. The generator from Chapter 12's ancient history has become hardware-in-the-loop.

## Summary

Testbench 7.1 exercised the response path. A sequence that needs answers declares it in its type — `Sequence<AluCommand, AluResult>` — and asks with `ctx.get_response(None).await` after `finish_item`; the driver supplies answers with `item_done(Some(result))`, the envelope tagging each response with its request's id automatically. The shared-handle miracle became an explicit request/response round trip, because `finish_item` consumes the item — the data flow the old style performed offstage now appears in the code, typed in both directions. The 7.1 driver serializes by awaiting each result, which is what result-dependent stimulus wants and what pipelined stimulus does not.

`get_response(None)` took whatever response came first, which was safe because exactly one item was ever in flight. Run sequences *concurrently* — or pipeline one — and "whatever came first" stops being safe. That is what the transaction ids in the envelopes are for, and testbench 7.2 finally spends them.
