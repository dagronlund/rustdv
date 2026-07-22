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
    /// Above every real level, so nothing passes the threshold — the port
    /// of pyuvm's `disable_logging()`.
    Off = 5,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warning => "WARNING",
            Level::Error => "ERROR",
            Level::Critical => "CRITICAL",
            Level::Off => "OFF",
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
        // Through `emit`, so a global log file captures un-pathed messages
        // too. Subtree handlers do not apply: this message has no path.
        emit(&format!("{:>10.2}ns {:<8} {}", sim_time_ns(), level.as_str(), msg));
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
use std::rc::Rc;

/// Does `path` name this component or one below it? `"a.b"` is under
/// `"a"`, but `"ab"` is not — the dot matters.
fn under(path: &str, prefix: &str) -> bool {
    path == prefix || path.starts_with(&format!("{prefix}."))
}

thread_local! {
    /// Per-target levels: (path prefix, level). Longest prefix wins;
    /// the global THRESHOLD is the fallback — pyuvm's per-component
    /// logger levels with hierarchy-wide setting (set_logging_level_hier).
    static TARGET_LEVELS: RefCell<Vec<(String, Level)>> = const { RefCell::new(Vec::new()) };
    /// Optional global log file (pyuvm's FileHandler on the root).
    static LOG_FILE: RefCell<Option<std::fs::File>> = const { RefCell::new(None) };
    /// Per-subtree file handlers (add_logging_handler_hier): every message
    /// from a path under the prefix is also written here.
    static TARGET_FILES: RefCell<Vec<(String, Rc<RefCell<std::fs::File>>)>> =
        const { RefCell::new(Vec::new()) };
    /// Per-subtree console suppression (remove_streaming_handler_hier).
    /// Longest matching prefix wins; absent means "console on".
    static TARGET_CONSOLE: RefCell<Vec<(String, bool)>> = const { RefCell::new(Vec::new()) };
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
    let file = open_log(path, append)?;
    LOG_FILE.with(|f| *f.borrow_mut() = Some(file));
    Ok(())
}

/// Stop writing to the global log file (port of remove_logging_handler).
pub fn remove_log_file() {
    LOG_FILE.with(|f| *f.borrow_mut() = None);
}

fn open_log(path: &str, append: bool) -> std::io::Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(path)
}

/// Send this component's subtree to a file as well — the port of
/// `add_logging_handler_hier(logging.FileHandler(...))`.
pub fn add_file_for(path_prefix: &str, path: &str, append: bool) -> std::io::Result<()> {
    let file = Rc::new(RefCell::new(open_log(path, append)?));
    TARGET_FILES.with(|t| t.borrow_mut().push((path_prefix.to_string(), file)));
    Ok(())
}

/// Turn console output on or off for a subtree — the port of
/// `remove_streaming_handler_hier()` (and its undo).
pub fn set_console_for(path_prefix: &str, enabled: bool) {
    TARGET_CONSOLE.with(|t| {
        let mut v = t.borrow_mut();
        v.retain(|(p, _)| p != path_prefix);
        v.push((path_prefix.to_string(), enabled));
    });
}

/// Drop all logging configuration: levels, handlers, console suppression.
///
/// The runner calls this before every test, so configuration set by one
/// test cannot leak into the next — pyuvm's `run_test` does the same via
/// `set_default_logging_level(INFO)`. Without it, a chapter that logs to a
/// file or disables logging would silently reshape every later test.
pub fn reset_config() {
    THRESHOLD.with(|t| t.set(Level::Info));
    TARGET_LEVELS.with(|t| t.borrow_mut().clear());
    TARGET_FILES.with(|t| t.borrow_mut().clear());
    TARGET_CONSOLE.with(|t| t.borrow_mut().clear());
    LOG_FILE.with(|f| *f.borrow_mut() = None);
}

fn console_enabled_for(path: &str) -> bool {
    TARGET_CONSOLE
        .with(|t| {
            t.borrow()
                .iter()
                .filter(|(p, _)| under(path, p))
                .max_by_key(|(p, _)| p.len())
                .map(|(_, on)| *on)
        })
        .unwrap_or(true)
}

/// Emit a line attributed to `path`: console unless this subtree's console
/// was removed, plus every file handler covering the path.
fn emit_for(path: &str, line: &str) {
    if console_enabled_for(path) {
        println!("{line}");
    }
    TARGET_FILES.with(|t| {
        for (prefix, file) in t.borrow().iter() {
            if under(path, prefix) {
                let _ = writeln!(file.borrow_mut(), "{line}");
            }
        }
    });
    LOG_FILE.with(|f| {
        if let Some(file) = f.borrow_mut().as_mut() {
            let _ = writeln!(file, "{line}");
        }
    });
}

fn emit(line: &str) {
    println!("{line}");
    LOG_FILE.with(|f| {
        if let Some(file) = f.borrow_mut().as_mut() {
            let _ = writeln!(file, "{line}");
        }
    });
}

/// A named logger: the pyuvm `self.logger`. The path is **not** typed by
/// hand — [`crate::log`] users get one from `RustdvCtx`, which carries the
/// path the phase walk derived (D7), so it cannot go stale when a
/// component moves.
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
                .filter(|(p, _)| under(&self.path, p))
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
            emit_for(
                &self.path,
                &format!(
                    "{:>10.2}ns {:<8} [{}]: {}",
                    sim_time_ns(),
                    level.as_str(),
                    self.path,
                    msg
                ),
            );
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
