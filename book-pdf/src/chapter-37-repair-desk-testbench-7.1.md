# Chapter 37: The Repair Desk: Testbench 7.1

In testbench 7.0 a sequence sent commands and never heard back. Real stimulus often needs the answer — and the answer does not always arrive in the order the requests were sent. That second clause is the hard part, and this chapter teaches it on a device built to make it vivid, before Chapter 38 needs it on the TinyALU.

Here is the whole idea. **You take your laptop to a repair desk and get ticket 1043.** Nobody knows how long the repair will take — not even the desk, until it looks at the machine. Other people hand in their machines and get their own tickets. A simpler repair behind you goes out first. You come back and ask whether 1043 is done. Every piece maps onto the sequence system:

| The repair desk | The testbench |
|---|---|
| your ticket | the item's `TxnId` |
| nobody knows how long | the driver's per-job latency |
| a later repair finishes first | responses out of order |
| asking whether 1043 is done | `get_response(Some(id))` |
| a ticket for a repair the shop never does | a response that never comes — you wait forever |

And the detail that makes the whole system work: **the desk releases you at the counter, not when your machine is fixed.** `item_done` ends the handshake as soon as the job is accepted, so the customer can hand in the next machine; the receipt comes back later, through `put_response`, under the job's ticket. Hold the handshake open until the work is done, and only one job is ever in the shop — and then there is nothing for a ticket to disambiguate. This is the first chapter where the transaction id earns its keep; everywhere before it, one item was in flight at a time and "give me whatever comes next" would have found the right answer without being told which one to look for. (If you have written an AXI testbench, you have met all of this: `ARID` and `RID` exist so a slave may answer out of order. If you have not, the repair desk is the whole idea.)

> **In the UVM...** the driver called `set_id_info(req)` on its response so the sequencer could route it, then `put_response(rsp)`; the sequence called `get_response(rsp, req.get_transaction_id())` to claim a specific answer. Forgetting `set_id_info` was a classic run-time failure.

One note on the cast: there is no TinyALU in this chapter. The DUT is the empty `playground` module, because the interesting behavior is *in the driver* — it takes a job, decides how long that job will take, and gets on with the next one — and a one-operation-at-a-time DUT could not show a response overtaking another. The TinyALU could not teach this chapter; that is precisely why the repair desk exists.

## The transactions

```rust
// Chapter 37, Figure 1: A job and its receipt
#[derive(Clone, Debug, PartialEq, Eq)]
struct RepairJob {
    machine: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RepairDone {
    machine: &'static str,
    clocks: u32,
}
```

Plain structs with derives, as Chapter 35 left them. The job carries what the desk needs to do the work; the receipt carries what the customer wants to know. Neither mentions a ticket — identity lives in the envelope the sequencer wraps around them, not in your data.

## The repair desk

```rust
// Chapter 37, Figure 2: A driver that takes work in and gives it back later
#[derive(Component, Default)]
struct RepairDesk {
    #[port(seq_item)]
    seq_item_port: SeqItemPort<RepairJob, RepairDone>,
    /// Jobs in the shop: the envelope, how long it was quoted, and how many
    /// ticks are left on it.
    bench: Vec<(SeqItem<RepairJob>, u32, u32)>,
}

impl Component for RepairDesk {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let mut rng = ctx.rng();

        loop {
            // --- take in whatever is waiting -----------------------------
            Timer::ns(5).await;
            if let Some(job) = self.seq_item_port.try_next_item() {
                let clocks = 1 + (rng.u8() % 8) as u32;
                ctx.info(&format!(
                    "took in {} ({}), {} clocks",
                    job.payload().machine,
                    job.txn_id(),
                    clocks
                ));
                // Release the customer now. The receipt comes later.
                self.seq_item_port.item_done(None);
                self.bench.push((job, clocks, clocks));
            }

            // --- age the shop, and hand back anything that is done --------
            Timer::ns(5).await;
            let mut still_working = Vec::new();
            for (job, quoted, left) in self.bench.drain(..) {
                if left > 1 {
                    still_working.push((job, quoted, left - 1));
                } else {
                    let machine = job.payload().machine;
                    ctx.info(&format!("ready: {} ({}) after {} ticks", machine, job.txn_id(), quoted));
                    self.seq_item_port
                        .put_response(job.txn_id(), RepairDone { machine, clocks: quoted });
                }
            }
            self.bench = still_working;
        }
    }
}
```

The loop does one thing in each half of a tick, and that split is what keeps it simple: first half, take a job if one is waiting and quote it a latency; second half, age every job in the shop and hand back the ones that are due. A driver that tried to wait for a job *and* count time simultaneously would need two loops and a shared list; one sequential loop does both, and nothing races. (A driver on a real bus would use its clock's two edges; the playground is empty, so the tick is simulated time.)

Three lines to dwell on:

- **`try_next_item()` is why this compiles — and why it exists.** `get_next_item()` blocks. On a tick with an empty sequencer, a blocking accept would sit there forever and never reach the second half of the loop — the jobs already in the shop would never age, and the sequences waiting for them would never be answered. The non-blocking accept is the UVM's own answer to exactly this situation (`try_next_item` has been in the standard since 1.1d; pyuvm does not carry it, and rustdv follows the UVM here because this driver cannot be written without it).
- The latency is quoted with a wide spread — one to eight — on purpose: quote everything the same and a later job can never overtake an earlier one, and the ticket has nothing to prove.
- `put_response(job.txn_id(), ...)` returns the receipt *under the job's own ticket*, so the customer who asks for 1043 gets 1043, however many other machines went out first. The framework tags responses with the request's identity; nothing here plays the part of `set_id_info`, so nothing can forget to.

## The customer

```rust
// Chapter 37, Figure 3: Drop off four machines, then collect four receipts
#[derive(Default)]
struct RepairSeq;

impl Sequence for RepairSeq {
    type Req = RepairJob;
    type Rsp = RepairDone;

    async fn body(&mut self, ctx: &mut SeqCtx<RepairJob, RepairDone>) -> Result<(), SeqError> {
        let machines = ["laptop", "desktop", "tablet", "server"];

        // Drop everything off first.
        let mut tickets = Vec::new();
        for machine in machines {
            let mut job = RepairJob { machine };
            ctx.start_item(&mut job).await?;
            let ticket = ctx.finish_item(job).await?;
            ctx.info(&format!("dropped off {machine}, ticket {ticket}"));
            tickets.push((machine, ticket));
        }

        // Then come back and check the board.
        while !tickets.is_empty() {
            Timer::ns(10).await; // walk back to the counter and look
            let mut i = 0;
            while i < tickets.len() {
                let (machine, ticket) = tickets[i];
                match ctx.try_get_response(Some(ticket)) {
                    Some(done) => {
                        ctx.info(&format!(
                            "collected {machine} ({ticket}) after {} ticks",
                            done.clocks
                        ));
                        tickets.remove(i);
                    }
                    None => i += 1,
                }
            }
        }
        Ok(())
    }
}
```

The sequence hands in every job before asking about any of them — which is the point. Had it waited for each answer before sending the next, only one job would ever be in the shop, and the desk's varying latency would be invisible. `finish_item` returns the ticket; the collection loop then polls with `try_get_response(Some(ticket))`, and the polling is not laziness — it is the honest shape of the errand. Blocking on ticket 1, then 2, then 3 would collect the receipts in the order they were *issued* no matter what the shop did, and the out-of-order work would be real but invisible. Polling collects them in the order they are *finished*, which is what the customer experiences and what the ticket is for.

One failure mode deserves its paragraph, because no figure can show it: **asking for a receipt that will never exist.** A sequence that calls `get_response` for a ticket the desk never answers waits forever — a ticket for a repair the shop does not do. Nothing can tell "not ready yet" from "never coming"; that is inherent to asking for a specific answer, in every UVM. It is the sequence writer's job to ask only for answers that are owed, and the only way to demonstrate the mistake is a test that hangs, which is why this book does not run one.

## The environment and the test

```rust
// Chapter 37, Figure 4: The shop, assembled
#[derive(Component, Default)]
struct ShopEnv {
    #[component(sequencer)]
    seqr: Sequencer<RepairJob, RepairDone>,
    #[component(child)]
    desk: RustdvComp,
}

impl Component for ShopEnv {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.seqr = Sequencer::new();
        ConfigDb::set(None, "*", "SEQR", self.seqr.handle());
        self.desk = RepairDesk::new_comp();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        self.seqr.seq_item_export().connect(&self.desk, RepairDesk::SEQ_ITEM_PORT);
    }
}

#[rustdv::test]
#[derive(Component, Default)]
struct RepairTest {
    #[component(child)]
    env: RustdvComp,
}

impl Component for RepairTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.env = ShopEnv::new_comp();
    }

    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("repairs in progress");
        let seqr: Sequencer<RepairJob, RepairDone> = ConfigDb::get(Some(ctx), "", "SEQR")?;
        let mut seq = RepairSeq::default();
        seq.start(&seqr).await?;
        Ok(())
    }
}
```

The smallest environment since Chapter 25 — a sequencer and a desk — because the chapter is about the handshake, not the architecture. Note what the test does *not* have: a flush. `RepairSeq`'s body does not return until every receipt is collected, so when `start` returns, nothing is in flight and the objection can drop immediately.

```text
# Figure 5: Later repairs overtake earlier ones; every ticket still claims its own

[TRANSCRIPT NEEDED — ch37's README predates the conversion; copy verbatim
from a rerun of `sim-common/run_sim.sh ch37_repair_desk_testbench_7_1
playground`. Expected shape: four "dropped off" lines in order, "ready:"
lines in a different order, four "collected" lines matching tickets to
machines. REGRESSION: PASS.]
```

## Summary

Responses are the second half of the sequencer handshake, and identity is what makes them safe under concurrency. The driver accepts a job with `try_next_item` — the non-blocking accept that exists so a driver can keep serving the work it already holds — releases the sequence at the counter with `item_done`, and returns each answer later with `put_response` under the request's own `TxnId`. The sequence holds its tickets and claims each receipt with `get_response`/`try_get_response(Some(ticket))`, in whatever order the work finishes. The mechanism was taught on a clock-only device deliberately: nothing about a repair desk can confuse the protocol with the arithmetic.

Chapter 38 puts the response path to work on the real DUT — where each command cannot even be written until the previous answer is in hand.
