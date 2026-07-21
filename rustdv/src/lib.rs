//! # rustdv
//!
//! A hardware verification framework in Rust: cocotb-style simulator
//! coroutines plus a UVM-analog testbench library, rethought for Rust's
//! ownership model.
//!
//! This is the facade crate (design-doc D2.5): it re-exports the public
//! API so a testbench needs one dependency and one import:
//!
//! ```ignore
//! use rustdv::prelude::*;
//!
//! rustdv::vpi_bootstrap!();
//!
//! #[rustdv::test]
//! async fn my_test(ctx: RustdvCtx) -> Result<(), TestError> {
//!     let dut = ctx.dut();
//!     // ...
//!     Ok(())
//! }
//! ```

// --- sub-crates, re-exported whole for power users -------------------------
pub use rustdv_gpi as gpi;
pub use rustdv_runner as runner;
pub use rustdv_sim as sim;
pub use rustdv_uvm as uvm;

// --- macros -----------------------------------------------------------------
/// `#[rustdv::test]` — port of `@cocotb.test()` (design-doc §6.1).
pub use rustdv_macros::test;
/// `#[derive(Component)]` — ComponentNode traversal (design-doc §6.3).
pub use rustdv_macros::Component;

// first!/join! are #[macro_export]ed by rustdv-sim at its crate root.
pub use rustdv_sim::{first, join};

// --- the curated surface ----------------------------------------------------
pub use rustdv_runner::TestRegistration;

pub use rustdv_sim::{
    first2, join2, next_time_step, read_only, read_write, sim_time_ns, sim_time_steps, spawn,
    spawn_named, with_timeout, AnyHandle, Clock, Either, Event, Executor, HandleError,
    HierarchyHandle, Lock, LockGuard, Logic, LogicArray, LogicHandle, NullTrigger, Queue, Rng,
    SimDuration, TaskError, TaskHandle, TaskState, TimeoutError, Timer, ValueError,
};
pub use rustdv_sim::handle::top_module;
pub use rustdv_sim::log;

pub use rustdv_uvm::{
    channel, check_all, extract_all, final_all, print_hierarchy, report_all, run_extract_check_report, start_all,
    Active, AnalysisFifo, AnalysisPort, CheckSink, Component as ComponentTrait, ComponentNode,
    DynPhases, ObjectionGuard, ObjectionRegistry, Receiver, ResponseQueue, Sender, SeqCtx, SeqError,
    SeqItem, SeqItemPort, Sequence, Sequencer, Subscriber, TestError, TlmEmpty, TlmError, TlmFifo,
    TlmFull, TxnId,
};

// The lifecycle trait under its design-doc name, in the type namespace.
// (The derive macro of the same name lives in the macro namespace; Rust
// resolves them independently.)
pub use rustdv_uvm::Component;

/// One-line import for testbenches (the `from pyuvm import *` analog).
pub mod prelude {
    pub use crate::{
        channel, first2, join2, next_time_step, print_hierarchy, read_only, read_write, run_extract_check_report,
        sim_time_ns, spawn, spawn_named, start_all, with_timeout, Active, AnalysisFifo,
        AnalysisPort, CheckSink, Clock, Component, ComponentNode, Either, Event, HandleError,
        HierarchyHandle, Lock, Logic, LogicArray, LogicHandle, NullTrigger, ObjectionGuard, Queue,
        Receiver, Rng, RustdvCtx, Sender, SeqCtx, SeqError, SeqItem, SeqItemPort, Sequence,
        Sequencer, SimDuration, Subscriber, TaskHandle, TestError, Timer, TlmFifo, TxnId,
    };
    pub use crate::log;
}

/// The one context every testbench is handed (D47: `TestCtx` and `RunCtx`
/// merged). Named for the framework, not for a phase, because Part II
/// teaches testbenches that have no phases.
pub use rustdv_uvm::RustdvCtx;

/// Export the VPI entry points from the testbench cdylib. The simulator
/// (vvp) dlopens the library and calls each routine in
/// `vlog_startup_routines`; ours registers the start-of-simulation
/// callback that launches the regression (design-doc §3.2, as deviated —
/// see rustdv-runner docs and STATUS.md).
#[macro_export]
macro_rules! vpi_bootstrap {
    () => {
        #[no_mangle]
        pub extern "C" fn __rustdv_vpi_entry() {
            $crate::runner::vpi_startup();
        }

        #[no_mangle]
        #[allow(non_upper_case_globals)]
        pub static vlog_startup_routines: [::core::option::Option<extern "C" fn()>; 2] =
            [::core::option::Option::Some(__rustdv_vpi_entry), ::core::option::Option::None];
    };
}
