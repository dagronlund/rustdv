//! Chapter 31: Component communications — channels replace TLM-1.
//!
//!     sim-common/run_sim.sh ch31_component_communications playground

use rustdv::prelude::*;
use rustdv::TlmFull;

rustdv::vpi_bootstrap!();

// Chapter 31, Figure 2: A producer holds the Sender, a consumer the Receiver

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

// Chapter 31, Figure 3: Blocking put/get is send/recv on a channel of size 1
#[rustdv::test]
async fn blocking_test(_ctx: TestCtx) -> Result<(), TestError> {
    let (tx, rx) = channel::<u32>(1);
    spawn_named(blocking_consumer(rx), "consumer");
    spawn_named(blocking_producer(tx), "producer").await.ok();
    Timer::ns(1).await;
    Ok(())
}

// Chapter 31, Figure 5: Nonblocking put/get: the failure is a value
#[rustdv::test]
async fn nonblocking_test(_ctx: TestCtx) -> Result<(), TestError> {
    let (tx, rx) = channel::<u32>(1);

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
    Ok(())
}

// Chapter 31, Figure 6: peek reads without removing (needs T: Clone)
#[rustdv::test]
async fn peek_test(_ctx: TestCtx) -> Result<(), TestError> {
    let (tx, rx) = channel::<u32>(1);
    tx.try_send(42).ok();
    let peeked = rx.peek().await.expect("sender alive");
    let gotten = rx.recv().await.expect("sender alive");
    log::info(&format!("peeked {peeked}, then got {gotten}, channel now empty"));
    Ok(())
}

// Chapter 31, Figure 7: TlmFifo — a FIFO that lives in the hierarchy
#[rustdv::test]
async fn fifo_test(_ctx: TestCtx) -> Result<(), TestError> {
    let fifo: TlmFifo<u32> = TlmFifo::new(Some(2));
    log::info(&format!("size={:?} used={} empty={}", fifo.size(), fifo.used(), fifo.is_empty()));
    fifo.put(7).await;
    fifo.put(8).await;
    log::info(&format!("size={:?} used={} full={}", fifo.size(), fifo.used(), fifo.is_full()));
    let x = fifo.get().await;
    log::info(&format!("got {x}; used={}", fifo.used()));
    fifo.flush();
    log::info(&format!("flushed; empty={}", fifo.is_empty()));
    Ok(())
}
