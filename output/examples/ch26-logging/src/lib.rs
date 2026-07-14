//! Chapter 26: Logging.
//!
//!     sim-common/run_sim.sh ch26_logging playground

use rustdv::prelude::*;
use rustdv::sim::log::{set_level_for, Level, Logger};

rustdv::vpi_bootstrap!();

// Chapter 26, Figure 1: Logging messages of all levels
struct LogComp {
    logger: Logger,
}

impl LogComp {
    fn new(path: &str) -> LogComp {
        LogComp { logger: Logger::new(path) }
    }
}

impl Component for LogComp {
    fn start(&mut self, ctx: &mut RunCtx) {
        let obj = ctx.raise_objection("logging");
        let logger = self.logger.clone();
        spawn_named(
            async move {
                logger.debug("This is debug");
                logger.info("This is info");
                logger.warning("This is warning");
                logger.error("This is error");
                logger.critical("This is critical");
                drop(obj);
            },
            "log_comp.run",
        );
    }
}

impl ComponentNode for LogComp {
    fn node_name(&self) -> &'static str {
        "LogComp"
    }
    fn visit_children(&mut self, _f: &mut dyn FnMut(&str, &mut dyn ComponentNode)) {}
}

// Chapter 26, Figure 2: The default level is Info
#[rustdv::test]
async fn log_test(_ctx: TestCtx) -> Result<(), TestError> {
    let mut comp = LogComp::new("uvm_test_top.comp");
    let mut run_ctx = RunCtx::new();
    start_all(&mut comp, &mut run_ctx);
    run_ctx.all_objections_dropped().await;
    run_extract_check_report(&mut comp).map_err(TestError::from)
}

// Chapter 26, Figure 4: Setting the logging level for a hierarchy
#[rustdv::test]
async fn debug_test(_ctx: TestCtx) -> Result<(), TestError> {
    let mut comp = LogComp::new("uvm_test_top.comp");
    set_level_for("uvm_test_top", Level::Debug); // ...and everything below it

    let mut run_ctx = RunCtx::new();
    start_all(&mut comp, &mut run_ctx);
    run_ctx.all_objections_dropped().await;

    set_level_for("uvm_test_top", Level::Info); // restore for the next test
    run_extract_check_report(&mut comp).map_err(TestError::from)
}

// Chapter 26, Figure 6: Logging to a file
#[rustdv::test]
async fn file_test(_ctx: TestCtx) -> Result<(), TestError> {
    log::log_to_file("/tmp/rustdv_ch26_log.txt", false).map_err(|e| TestError(e.to_string()))?;

    let mut comp = LogComp::new("uvm_test_top.comp");
    let mut run_ctx = RunCtx::new();
    start_all(&mut comp, &mut run_ctx);
    run_ctx.all_objections_dropped().await;

    log::remove_log_file();
    log::info("messages above are also in /tmp/rustdv_ch26_log.txt");
    run_extract_check_report(&mut comp).map_err(TestError::from)
}
