//! Template: minimal rustdv testbench entry (lib.rs). Replace <dut> parts.

use rustdv::prelude::*;

// Test executables need vpi_* symbol definitions (the simulator provides
// them for the real cdylib).
#[cfg(test)]
use rustdv_vpi_stubs as _;

// Export the VPI entry points from this cdylib. Exactly once, crate root.
rustdv::vpi_bootstrap!();

#[rustdv::test(timeout_time = 500, timeout_unit = "us")]
async fn smoke(ctx: TestCtx) -> Result<(), TestError> {
    let dut = ctx.dut();
    let clk = dut.signal("clk")?;
    Clock::new(&clk, SimDuration::ns(10)).start();

    // reset, build BFM/env, run stimulus, drain, then check:
    // let bfm = Rc::new(MyBfm::new(&dut)?);
    // bfm.start_tasks();
    // bfm.reset().await;
    // ...
    // run_extract_check_report(&mut env).map_err(TestError::from)

    clk.rising_edge().await;
    log::info("clock is toggling — scaffold works");
    Ok(())
}
