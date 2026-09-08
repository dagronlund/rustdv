//! `sig_` — reading and writing the design's signals.
//!
//! The boundary between Rust and the simulator. Above it everything is a
//! plain value; below it a signal is four-valued, has a width, and may be
//! neither 0 nor 1. Chapter 17 tells the reader `x` becomes an `Err` rather
//! than a surprise; this is where that stops being a claim.

use rustdv::prelude::*;

// Widths come from the design, not from the Rust type.
#[rustdv::test]
async fn sig_widths_match_the_design(ctx: RustdvCtx) -> Result<(), TestError> {
    let dut = ctx.dut();
    for (name, want) in [
        ("byte_sig", 8u32),
        ("word_sig", 16),
        ("nibble", 4),
        ("flag", 1),
    ] {
        let size = dut.signal(name)?.size();
        check!(
            size == want,
            "{name} reports {size} bits, the design declares {want}"
        );
    }
    Ok(())
}

// A value written is the value read back, at every width.
#[rustdv::test]
async fn sig_round_trips(ctx: RustdvCtx) -> Result<(), TestError> {
    let dut = ctx.dut();
    for (name, value) in [("byte_sig", 0xA5u64), ("word_sig", 0xBEEF), ("nibble", 0xC)] {
        let sig = dut.signal(name)?;
        sig.set_u64(value);
        read_write().await;
        let got = sig
            .get_u64()
            .map_err(|e| TestError::new(format!("{name}: {e}")))?;
        check!(
            got == value,
            "{name} read back {got:#x} after writing {value:#x}"
        );
    }
    Ok(())
}

// A write wider than the signal keeps the bits that fit.
//
// Not a rule anyone would want to rely on, but it is the behaviour, and a
// silent change to it would corrupt drivers rather than fail them.
#[rustdv::test]
async fn sig_truncates_to_width(ctx: RustdvCtx) -> Result<(), TestError> {
    let nibble = ctx.dut().signal("nibble")?;
    nibble.set_u64(0xFF);
    read_write().await;
    let got = nibble.get_u64().unwrap_or(0xDEAD);
    check!(
        got == 0xF,
        "writing 0xFF to a 4-bit signal read back {got:#x}, not 0xf"
    );
    Ok(())
}

// X is an `Err`, not a zero.
//
// Chapter 17's Figure 3 has the reader write `unwrap_or(0)` and explains that
// the choice to treat x as zero is *theirs*, made visible by the `Result`.
// That lesson only holds if an undriven signal really does come back `Err`.
#[rustdv::test]
async fn sig_x_is_an_error_not_a_zero(ctx: RustdvCtx) -> Result<(), TestError> {
    let dut = ctx.dut();

    let x = dut.signal("never_driven")?;
    check!(
        x.get_u64().is_err(),
        "an undriven bit converted to an integer"
    );
    check!(
        x.get_binstr().contains('x'),
        "an undriven bit reads as {:?}",
        x.get_binstr()
    );
    check!(
        !x.is_high() && !x.is_low(),
        "an undriven bit claimed to be high or low"
    );

    let bus = dut.signal("never_driven_bus")?;
    check!(
        bus.get_u64().is_err(),
        "an undriven bus converted to an integer"
    );
    check!(
        bus.get_binstr().chars().all(|c| c == 'x'),
        "an undriven bus reads as {:?}",
        bus.get_binstr()
    );

    // And the escape hatch the book teaches still works.
    check!(
        x.get_u64().unwrap_or(0) == 0,
        "unwrap_or(0) did not yield 0"
    );
    Ok(())
}

// A partly-unknown bus is still an error: one x poisons the integer.
#[rustdv::test]
async fn sig_partial_x_is_still_an_error(ctx: RustdvCtx) -> Result<(), TestError> {
    let sig = ctx.dut().signal("byte_sig")?;

    // Drive a known value, then write a pattern with unknown bits through
    // the four-valued path.
    sig.set_u64(0);
    read_write().await;
    check!(sig.get_u64().is_ok(), "a fully driven byte did not convert");

    let arr = LogicArray::from_binstr("0000x000");
    check!(
        arr.len() == 8,
        "the pattern is {} bits wide, not 8",
        arr.len()
    );
    check!(
        !arr.is_resolvable(),
        "a pattern containing x claimed to be resolvable"
    );
    sig.set(&arr);
    read_write().await;
    check!(
        sig.get_u64().is_err(),
        "a byte with one x bit converted to {:?}",
        sig.get_u64()
    );
    check!(
        sig.get_binstr().contains('x'),
        "the x bit vanished: {:?}",
        sig.get_binstr()
    );
    Ok(())
}

// The design's own signal changes underneath us, and we see it.
#[rustdv::test]
async fn sig_reads_track_the_design(ctx: RustdvCtx) -> Result<(), TestError> {
    let clk = ctx.dut().signal("clk")?;
    let counted = ctx.dut().signal("counted")?;
    Clock::new(&clk, SimDuration::ns(2)).start();

    clk.rising_edge().await;
    read_only().await;
    let a = counted.get_u64().unwrap_or(0);
    clk.rising_edge().await;
    read_only().await;
    let b = counted.get_u64().unwrap_or(0);

    check!(
        b.wrapping_sub(a) == 1,
        "the counter went {a} -> {b} across one clock"
    );
    Ok(())
}

// A signal name the design does not have is an `Err` naming it, not a panic.
#[rustdv::test]
async fn sig_missing_name_is_an_error(ctx: RustdvCtx) -> Result<(), TestError> {
    let dut = ctx.dut();
    check!(
        dut.signal("byte_sig").is_ok(),
        "the control signal is missing from probe.sv"
    );

    let bad = dut.signal("byte_sgi"); // the classic transposition
    check!(
        bad.is_err(),
        "a signal that does not exist was found anyway"
    );
    let msg = format!("{:?}", bad.err().unwrap());
    check!(
        msg.contains("byte_sgi"),
        "the error does not name the signal that was not found: {msg}"
    );
    Ok(())
}
