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

pub fn critical(msg: &str) {
    log(Level::Critical, msg);
}

// ---------------------------------------------------------------------------
// Hierarchical targets and handlers (the pyuvm logging surface, ported)
// ---------------------------------------------------------------------------

use std::cell::RefCell;
use std::io::Write;

thread_local! {
    /// Per-target levels: (path prefix, level). Longest prefix wins;
    /// the global THRESHOLD is the fallback — pyuvm's per-component
    /// logger levels with hierarchy-wide setting (set_logging_level_hier).
    static TARGET_LEVELS: RefCell<Vec<(String, Level)>> = const { RefCell::new(Vec::new()) };
    /// Optional log file (pyuvm's FileHandler).
    static LOG_FILE: RefCell<Option<std::fs::File>> = const { RefCell::new(None) };
}

/// Set the level for a component path and everything under it
/// (port of set_logging_level_hier; a leaf path is set_logging_level).
pub fn set_level_for(path_prefix: &str, l: Level) {
    TARGET_LEVELS.with(|t| {
        let mut v = t.borrow_mut();
        v.retain(|(p, _)| p != path_prefix);
        v.push((path_prefix.to_string(), l));
    });
}

/// Also write every printed message to `path` (port of logging.FileHandler;
/// mode "w"). Pass append=true for mode "a".
pub fn log_to_file(path: &str, append: bool) -> std::io::Result<()> {
    let file =
        std::fs::OpenOptions::new().create(true).write(true).append(append).truncate(!append).open(path)?;
    LOG_FILE.with(|f| *f.borrow_mut() = Some(file));
    Ok(())
}

/// Stop writing to the log file (port of remove_logging_handler).
pub fn remove_log_file() {
    LOG_FILE.with(|f| *f.borrow_mut() = None);
}

fn emit(line: &str) {
    println!("{line}");
    LOG_FILE.with(|f| {
        if let Some(file) = f.borrow_mut().as_mut() {
            let _ = writeln!(file, "{line}");
        }
    });
}

/// A named logger: the pyuvm `self.logger`, with the component path
/// explicit. Components create one in their constructor.
#[derive(Clone)]
pub struct Logger {
    path: String,
}

impl Logger {
    pub fn new(path: &str) -> Logger {
        Logger { path: path.to_string() }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    fn enabled(&self, level: Level) -> bool {
        let per_target = TARGET_LEVELS.with(|t| {
            t.borrow()
                .iter()
                .filter(|(p, _)| self.path == *p || self.path.starts_with(&format!("{p}.")))
                .max_by_key(|(p, _)| p.len())
                .map(|(_, l)| *l)
        });
        match per_target {
            Some(l) => level >= l,
            None => THRESHOLD.with(|t| level >= t.get()),
        }
    }

    pub fn log(&self, level: Level, msg: &str) {
        if self.enabled(level) {
            emit(&format!(
                "{:>10.2}ns {:<8} [{}]: {}",
                sim_time_ns(),
                level.as_str(),
                self.path,
                msg
            ));
        }
    }

    pub fn debug(&self, msg: &str) {
        self.log(Level::Debug, msg);
    }
    pub fn info(&self, msg: &str) {
        self.log(Level::Info, msg);
    }
    pub fn warning(&self, msg: &str) {
        self.log(Level::Warning, msg);
    }
    pub fn error(&self, msg: &str) {
        self.log(Level::Error, msg);
    }
    pub fn critical(&self, msg: &str) {
        self.log(Level::Critical, msg);
    }
}
