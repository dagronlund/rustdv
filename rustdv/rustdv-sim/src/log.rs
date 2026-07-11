//! Sim-time-stamped logging in the book's format: `  2.00ns INFO ...`
//! (design-doc OQ-8: the `tracing` mapping is deferred; this minimal
//! zero-dependency logger reproduces the output format the book teaches).

use std::cell::Cell;

use crate::time::sim_time_ns;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Debug = 0,
    Info = 1,
    Warning = 2,
    Error = 3,
    Critical = 4,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warning => "WARNING",
            Level::Error => "ERROR",
            Level::Critical => "CRITICAL",
        }
    }
}

thread_local! {
    static THRESHOLD: Cell<Level> = const { Cell::new(Level::Info) };
}

pub fn set_level(l: Level) {
    THRESHOLD.with(|t| t.set(l));
}

pub fn log(level: Level, msg: &str) {
    let enabled = THRESHOLD.with(|t| level >= t.get());
    if enabled {
        println!("{:>10.2}ns {:<8} {}", sim_time_ns(), level.as_str(), msg);
    }
}

pub fn debug(msg: &str) {
    log(Level::Debug, msg);
}
pub fn info(msg: &str) {
    log(Level::Info, msg);
}
pub fn warning(msg: &str) {
    log(Level::Warning, msg);
}
pub fn error(msg: &str) {
    log(Level::Error, msg);
}
