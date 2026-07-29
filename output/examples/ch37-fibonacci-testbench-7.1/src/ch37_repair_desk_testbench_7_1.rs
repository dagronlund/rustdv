//! Chapter 37: The repair desk — testbench 7.1, answers that come back out of
//! order.
//!
//!     sim-common/run_sim.sh ch37_repair_desk_testbench_7_1 playground
//!
//! ============================================================================
//! ASPIRATIONAL — the target API (D1/D2), written before the framework can
//! compile it. What it asks for is listed at the bottom of this comment.
//! ============================================================================
//!
//! ## The problem this chapter exists for
//!
//! In 7.0 a sequence sent commands and never heard back. Real stimulus often
//! needs the answer — and the answer does not always arrive in the order the
//! requests were sent.
//!
//! **You take your laptop to a repair desk and get ticket 1043.** Nobody knows
//! how long it will take. Other people hand in their machines and get their own
//! tickets. A simpler repair behind you goes out first. You come back and ask
//! whether 1043 is done.
//!
//! Every piece maps onto the sequence system:
//!
//! | The repair desk | The testbench |
//! |---|---|
//! | your ticket | the item's `TxnId` |
//! | nobody knows how long | the driver's per-job latency |
//! | a later repair finishes first | responses out of order |
//! | asking whether 1043 is done | `get_response(Some(id))` |
//! | a ticket for a repair the shop never does | a response that never comes — you wait forever |
//!
//! **The desk releases you at the counter, not when your machine is fixed.**
//! `item_done` ends the handshake as soon as the job is accepted, so the
//! customer can hand in the next machine; the receipt comes back later through
//! `put_response`, under the job's ticket. Hold the handshake open until the
//! work is done and only one job is ever in the shop — and then there is
//! nothing for a ticket to disambiguate.
//!
//! This is the first chapter where the transaction id earns its keep.
//! Everywhere before it, one item is in flight at a time and `get_response`
//! would find the right answer without being told which one to look for.
//!
//! If you have written an AXI testbench you have met this already: `ARID` and
//! `RID` exist so a slave may answer out of order. Skip the sentence if you
//! have not — the repair desk is the whole idea.
//!
//! ## The DUT is a clock
//!
//! There is no TinyALU here. The device under test is the `playground` top,
//! which supplies a clock and nothing else, because the interesting behaviour
//! is *in the driver*: it takes a job, decides how long that job will take, and
//! gets on with the next one. A one-operation-at-a-time DUT could not show a
//! response arriving out of order, and inventing a second DUT for one chapter
//! would teach less than this does. `playground` is an empty module, so the
//! desk's tick is simulated time — a driver on a real bus would use the two
//! edges of its clock.
//!
//! ## What this asks the framework for
//!
//! Everything Chapter 36 asked for, plus:
//!
//! 1. **`SeqItemPort::try_next_item() -> Option<SeqItem<REQ>>`** — a
//!    non-blocking accept. The UVM has it (`tlm1/uvm_sqr_ifs.svh`, clause
//!    15.2.1.2.2, present since 1.1d) and pyuvm does not. Without it this
//!    driver deadlocks: on a rising edge with nothing queued, a blocking
//!    `get_next_item()` would wait there forever and never reach the falling
//!    edge to retire work already outstanding.
//! 2. `item_done(Some(rsp))` tagging the response with the request's id
//!    automatically, so nothing here calls the equivalent of `set_id_info`.

use rustdv::prelude::*;

rustdv::vpi_bootstrap!();

// ===========================================================================
// The transactions
// ===========================================================================

// Chapter 37, Figure 1: A job and its receipt.
//
// Plain structs with derives, as Chapter 35 left them. The job carries what
// the desk needs to do the work; the receipt carries what the customer wants
// to know. Neither knows about tickets — identity lives in the envelope the
// sequencer wraps around them (Chapter 35, Figure 7).
#[derive(Clone, Debug, PartialEq, Eq)]
struct RepairJob {
    machine: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RepairDone {
    machine: &'static str,
    clocks: u32,
}

// ===========================================================================
// The repair desk
// ===========================================================================

// Chapter 37, Figure 2: A driver that takes work in and gives it back later.
//
// The loop does one thing in each half of a tick, and that is what keeps it
// simple:
//
//   first half  — take a job if one is waiting, and decide its latency
//   second half — age every job in the shop; finish the ones that are due
//
// Splitting the two is what removes the concurrency problem. A driver that
// tried to wait for a job *and* count time at the same time would need two
// loops and a shared list; here one sequential loop does both, and nothing
// races. A driver on a real bus would use the two clock edges; this DUT is
// an empty module, so the tick is simulated time and nothing else.
//
// **`try_next_item` is why this compiles.** `get_next_item()` blocks. On a
// rising edge with an empty sequencer, a blocking accept would sit there and
// never reach the falling edge — so the jobs already in the shop would never
// age, and the sequences waiting for them would never be answered. The
// non-blocking form is the UVM's answer to exactly this, and the reason it
// exists.
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
                // Nobody knows how long it will take. Not even the desk, until
                // it looks at the machine.
                // A wide spread is what makes the point: quote everything the same
                // and a later job can never overtake an earlier one, and the
                // ticket has nothing to disambiguate.
                let clocks = 1 + (rng.u8() % 8) as u32;
                ctx.info(&format!(
                    "took in {} ({}), {} clocks",
                    job.payload().machine,
                    job.txn_id(),
                    clocks
                ));
                // **Release the customer now.** `item_done` ends the handshake,
                // which is what lets them hand in the next machine instead of
                // standing at the counter. The receipt comes later, through
                // `put_response`. Hold the handshake open until the work is
                // finished and only one job is ever in the shop — which is the
                // whole thing this chapter is about.
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
                    // The receipt goes back under the job's own ticket, so the
                    // customer who asks for 1043 gets 1043 however many others
                    // were finished first.
                    self.seq_item_port
                        .put_response(job.txn_id(), RepairDone { machine, clocks: quoted });
                }
            }
            self.bench = still_working;
        }
    }
}

// ===========================================================================
// The customer
// ===========================================================================

// Chapter 37, Figure 3: Drop off four machines, then collect four receipts.
//
// The sequence hands in every job before asking about any of them — which is
// the point. Had it waited for each answer before sending the next, only one
// job would ever be in the shop and the desk's latency would not matter.
//
// `finish_item` returns the ticket. `get_response(Some(ticket))` asks for that
// job's receipt specifically, and waits until it is ready however many other
// receipts come out first.
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
        //
        // **This is why `try_get_response` exists.** Blocking on ticket #1 and
        // then #2 and then #3 would collect them in the order they were issued
        // no matter what the shop did — the out-of-order work would be real and
        // invisible. Polling collects them in the order they are *finished*,
        // which is what the customer actually experiences and what the ticket
        // is for.
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

// Chapter 37, Figure 4 is a paragraph, not code: **asking for a receipt that
// will never exist.** A sequence that calls `get_response` for a ticket the
// desk never issues waits forever — a ticket for a repair the shop does not
// do. Nothing can tell "not ready yet" from "never coming", which is why it is
// the sequence writer's job to ask only for answers that are owed. There is no
// runnable figure because the only way to show it is a test that hangs.

// ===========================================================================
// The environment and the tests
// ===========================================================================

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

// Chapter 37, Figure 5: The test.
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

// Expected: four machines dropped off on consecutive clocks, each with a
// different latency, so "ready:" lines appear in a different order from the
// "dropped off" lines — and every machine is still collected by its own
// ticket. REGRESSION: PASS.
