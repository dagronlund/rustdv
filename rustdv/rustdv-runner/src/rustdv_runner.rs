//! # rustdv-runner
//!
//! The regression manager (design-doc D2.4, §4.5): port of cocotb's
//! `RegressionManager`. Owns the test registry (populated at link time by
//! `#[rustdv::test]`), runs tests sequentially, applies timeouts, scores
//! panics/errors vs. expectations, prints the summary table, and provides
//! the simulator entry point.
//!
//! **Bootstrap deviation from D3.2 (STATUS.md):** instead of exporting
//! cocotb's libpygpi entry symbols, the testbench cdylib is loaded as a
//! plain **VPI module** (`vvp -M<dir> -m<name>`): `rustdv::vpi_bootstrap!()`
//! exports `vlog_startup_routines`, whose startup routine registers a
//! cbStartOfSimulation callback that kicks off the regression.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use rustdv_gpi as gpi;
use rustdv_sim::combinators::{first2, Either};
use rustdv_sim::handle::top_module;
use rustdv_sim::log;
use rustdv_sim::time::{sim_time_ns, SimDuration};
use rustdv_sim::triggers::Timer;

// ===========================================================================
// Public test-facing types
// ===========================================================================

/// Test failure value — defined in `rustdv-uvm` since step 4, because
/// `Component::run` returns it and the UVM crate sits below this one
/// (D46/D47). Re-exported so `::rustdv::TestError` is unchanged.
pub use rustdv_uvm::TestError;

/// Handed to each test. Since step 4 this is the one universal context
/// (D47): the old `TestCtx` and `RunCtx` merged into `RustdvCtx`, which
/// lives in `rustdv-uvm` beside the `Component` trait that receives it.
pub use rustdv_uvm::RustdvCtx;

type TestFn =
    fn(RustdvCtx) -> Pin<Box<dyn Future<Output = Result<(), TestError>>>>;

/// One registered test (design-doc §6.1: the cocotb `Test` option set).
pub struct TestRegistration {
    pub name: &'static str,
    pub module: &'static str,
    pub file: &'static str,
    pub line: u32,
    pub run: TestFn,
    /// (time, unit), e.g. (100, "us").
    pub timeout: Option<(u64, &'static str)>,
    pub skip: bool,
    pub expect_fail: bool,
}

// ===========================================================================
// Link-time registry (design-doc §0.5/§6.1; OQ-4 — ELF section technique,
// with the sentinel guaranteeing the section exists)
// ===========================================================================

fn sentinel_shim(_ctx: RustdvCtx) -> Pin<Box<dyn Future<Output = Result<(), TestError>>>> {
    Box::pin(async { Ok(()) })
}

#[used]
// ELF (Linux) names sections freely; Mach-O (macOS) wants segment,section.
#[cfg_attr(not(target_vendor = "apple"), link_section = "rustdv_tests")]
#[cfg_attr(target_vendor = "apple", link_section = "__DATA,rustdv_tests")]
static SENTINEL: &TestRegistration = &TestRegistration {
    name: "__rustdv_sentinel",
    module: "rustdv_runner",
    file: file!(),
    line: line!(),
    run: sentinel_shim,
    timeout: None,
    skip: true,
    expect_fail: false,
};

// The linker-provided section bounds. ELF defines __start_/__stop_
// symbols automatically; Mach-O spells them section$start$/section$end$
// (reached via link_name — the \x01 prefix suppresses mangling).
#[cfg(not(target_vendor = "apple"))]
extern "C" {
    static __start_rustdv_tests: u8;
    static __stop_rustdv_tests: u8;
}

#[cfg(target_vendor = "apple")]
extern "C" {
    #[link_name = "\x01section$start$__DATA$rustdv_tests"]
    static __start_rustdv_tests: u8;
    #[link_name = "\x01section$end$__DATA$rustdv_tests"]
    static __stop_rustdv_tests: u8;
}

/// All registered tests, in (file, line) order.
pub fn collect_tests() -> Vec<&'static TestRegistration> {
    // Force the sentinel's object file into the link.
    std::hint::black_box(SENTINEL.name);
    let mut out: Vec<&'static TestRegistration> = Vec::new();
    unsafe {
        let start = std::ptr::addr_of!(__start_rustdv_tests) as usize;
        let stop = std::ptr::addr_of!(__stop_rustdv_tests) as usize;
        let entry = std::mem::size_of::<&TestRegistration>();
        let count = (stop - start) / entry;
        let base = start as *const &'static TestRegistration;
        for i in 0..count {
            let reg = *base.add(i);
            if reg.name != "__rustdv_sentinel" {
                out.push(reg);
            }
        }
    }
    out.sort_by_key(|r| (r.file, r.line));
    out
}

// ===========================================================================
// Regression execution (§4.5)
// ===========================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
enum Outcome {
    Pass,
    Fail(String),
    Skip,
}

struct TestResult {
    name: &'static str,
    outcome: Outcome,
    sim_ns: f64,
}

thread_local! {
    /// "A panic in any task fails the current test" (cocotb:
    /// TestManager._task_done_callback).
    static CURRENT_FAILURE: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
}

fn take_background_failure() -> Option<String> {
    CURRENT_FAILURE.with(|f| f.borrow_mut().take())
}

fn seed_from_env() -> u64 {
    std::env::var("RUSTDV_RANDOM_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(1)
        })
}

async fn run_one(reg: &'static TestRegistration, seed: u64) -> Outcome {
    // Each test starts clean — pyuvm's run_test clears singletons and
    // resets the default logging level, so a test never inherits the
    // previous test's BFM (with its half-drained queues) or its logging
    // configuration. D16's rule, applied beyond the ConfigDb.
    rustdv_uvm::clear_singletons();
    log::reset_config();

    let dut = match top_module() {
        Ok(d) => d,
        Err(e) => return Outcome::Fail(format!("no DUT: {e}")),
    };
    // D49: the root path is the test's registered name, so `ctx.info(..)`
    // logs `[random_test]` where UVM logs `uvm_test_top`.
    let ctx = RustdvCtx::new(reg.name, dut, seed);

    let ex = rustdv_sim::executor::current();
    let watermark = ex.watermark();

    // UVM's end-of-test consensus: the body finishes, then the test waits
    // for every outstanding objection. The clone shares the registry, and
    // folding the wait into the same future keeps it under the timeout.
    // A test that never objected is not made to wait — that is the Part II
    // front door (D46), and pyuvm's "you never objected" warning would
    // otherwise fire on every cocotb-shaped test.
    let body = {
        let watcher = ctx.clone();
        let fut = (reg.run)(ctx);
        async move {
            let result = fut.await;
            if watcher.objections().ever_raised() {
                watcher.all_objections_dropped().await;
            }
            result
        }
    };

    let handle = ex.spawn_named(body, Some(reg.name));

    // handle.await → Result<Result<(), TestError>, TaskError>
    let raw = match reg.timeout {
        Some((n, unit)) => {
            let d = SimDuration::from_unit(n, unit);
            match first2(handle, Timer::new(d)).await {
                Either::First(r) => Some(r),
                Either::Second(()) => None, // timeout
            }
        }
        None => Some(handle.await),
    };

    // Kill surviving tasks spawned during the test (§4.5).
    ex.cancel_after(watermark);

    let mut outcome = match raw {
        None => Outcome::Fail(format!(
            "timeout after {}{}",
            reg.timeout.unwrap().0,
            reg.timeout.unwrap().1
        )),
        Some(Err(e)) => Outcome::Fail(format!("test task: {e}")),
        Some(Ok(Err(e))) => Outcome::Fail(e.to_string()),
        Some(Ok(Ok(()))) => Outcome::Pass,
    };

    // A panic in a child task fails the test even if the body passed.
    if let Some(bg) = take_background_failure() {
        if outcome == Outcome::Pass {
            outcome = Outcome::Fail(bg);
        }
    }

    if reg.expect_fail {
        outcome = match outcome {
            Outcome::Pass => Outcome::Fail("expected failure but test passed".into()),
            Outcome::Fail(_) => Outcome::Pass,
            s => s,
        };
    }
    outcome
}

async fn regression() {
    let tests = collect_tests();
    let seed = seed_from_env();
    log::info(&format!(
        "rustdv: found {} test(s), RUSTDV_RANDOM_SEED={seed}",
        tests.len()
    ));

    let mut results: Vec<TestResult> = Vec::new();
    let total = tests.len();

    for (i, reg) in tests.iter().enumerate() {
        if reg.skip {
            log::info(&format!("skipping {} ({}/{})", reg.name, i + 1, total));
            results.push(TestResult { name: reg.name, outcome: Outcome::Skip, sim_ns: 0.0 });
            continue;
        }
        log::info(&format!(
            "running {} ({}/{})  [{}:{}]",
            reg.name,
            i + 1,
            total,
            reg.file,
            reg.line
        ));
        let t0 = sim_time_ns();
        let outcome = run_one(reg, seed.wrapping_add(i as u64)).await;
        let dt = sim_time_ns() - t0;
        match &outcome {
            Outcome::Pass => log::info(&format!("{} PASSED", reg.name)),
            Outcome::Fail(m) => log::error(&format!("{} FAILED: {m}", reg.name)),
            Outcome::Skip => {}
        }
        results.push(TestResult { name: reg.name, outcome, sim_ns: dt });
    }

    print_summary(&results);
    write_xunit(&results);

    let failed = results.iter().any(|r| matches!(r.outcome, Outcome::Fail(_)));
    println!("REGRESSION: {}", if failed { "FAIL" } else { "PASS" });
    gpi::finish();
}

fn print_summary(results: &[TestResult]) {
    // Port of cocotb's summary table shape (regression.py _log_test_summary).
    println!("{}", "*".repeat(78));
    println!("** {:<40} {:>8} {:>14}      **", "TEST", "STATUS", "SIM TIME (ns)");
    println!("{}", "*".repeat(78));
    for r in results {
        let status = match &r.outcome {
            Outcome::Pass => "PASS",
            Outcome::Fail(_) => "FAIL",
            Outcome::Skip => "SKIP",
        };
        println!("** {:<40} {:>8} {:>14.2}      **", r.name, status, r.sim_ns);
    }
    println!("{}", "*".repeat(78));
}

/// xUnit XML (cocotb: _xunit_reporter.py) — written only if
/// RUSTDV_RESULTS_XML names a path.
fn write_xunit(results: &[TestResult]) {
    let Ok(path) = std::env::var("RUSTDV_RESULTS_XML") else { return };
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let failures = results.iter().filter(|r| matches!(r.outcome, Outcome::Fail(_))).count();
    let skipped = results.iter().filter(|r| matches!(r.outcome, Outcome::Skip)).count();
    xml.push_str(&format!(
        "<testsuites>\n<testsuite name=\"rustdv\" tests=\"{}\" failures=\"{}\" skipped=\"{}\">\n",
        results.len(),
        failures,
        skipped
    ));
    for r in results {
        xml.push_str(&format!(
            "  <testcase name=\"{}\" time=\"{:.2}\"",
            r.name, r.sim_ns
        ));
        match &r.outcome {
            Outcome::Pass => xml.push_str("/>\n"),
            Outcome::Skip => xml.push_str("><skipped/></testcase>\n"),
            Outcome::Fail(m) => xml.push_str(&format!(
                "><failure message=\"{}\"/></testcase>\n",
                m.replace('"', "'").replace('<', "(").replace('>', ")")
            )),
        }
    }
    xml.push_str("</testsuite>\n</testsuites>\n");
    if let Err(e) = std::fs::write(&path, xml) {
        log::warning(&format!("could not write {path}: {e}"));
    }
}

// ===========================================================================
// Bootstrap (§3.2, deviated: VPI-module loading — see crate docs)
// ===========================================================================

/// Called from `vlog_startup_routines` at VPI module load time (before
/// elaboration). Registers the start-of-simulation hook; everything else
/// happens from simulator callbacks.
pub fn vpi_startup() {
    let cb = gpi::register_start_of_simulation(Box::new(|| {
        on_start_of_simulation();
    }));
    cb.forget();
}

fn on_start_of_simulation() {
    let ex = rustdv_sim::init();

    // Route panics — from tasks and from raw GPI callbacks — into
    // "fail the current test".
    let flag = CURRENT_FAILURE.with(|f| f.clone());
    ex.set_failure_sink(Box::new(move |msg| {
        let mut slot = flag.borrow_mut();
        if slot.is_none() {
            *slot = Some(msg.to_string());
        }
    }));
    let flag2 = CURRENT_FAILURE.with(|f| f.clone());
    gpi::set_panic_sink(Box::new(move |msg| {
        let mut slot = flag2.borrow_mut();
        if slot.is_none() {
            *slot = Some(format!("panic in simulator callback: {msg}"));
        }
    }));

    ex.spawn_named(regression(), Some("rustdv_regression"));
    ex.run_until_idle();
}
