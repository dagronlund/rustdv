# Chapter 26: Logging

With a hierarchy to hang them on, we can tour the features that live on it, starting with the one you've been reading all book: logging. Large testbenches generate more output than humans can read, so the game is filtering and directing — per component, per subtree, per destination — and the pyuvm surface for that game ports over nearly method-for-method.

> **In Python we...** logged through `self.logger`, inherited from `uvm_report_object`: six levels from DEBUG to CRITICAL, INFO as the default threshold, `set_logging_level_hier(DEBUG)` to open a whole subtree, and handlers — `StreamHandler`, `FileHandler` — to say where messages went. The path between square brackets, `[uvm_test_top.comp]`, told us who was talking.

## Creating log messages

rustdv's logger has two layers, and you have been using the anonymous one — `log::info(...)`, plain messages with the simulated timestamp — since Chapter 15. Components graduate to the named layer: a `Logger` carrying the component's hierarchy path, created in the constructor, playing exactly the role of pyuvm's `self.logger`:

```rust
// Figure 1: Logging messages of all levels

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
```

```text
# Figure 2: The default level is Info
--
      0.00ns INFO     [uvm_test_top.comp]: This is info
      0.00ns WARNING  [uvm_test_top.comp]: This is warning
      0.00ns ERROR    [uvm_test_top.comp]: This is error
      0.00ns CRITICAL [uvm_test_top.comp]: This is critical
```

Five calls, four lines — the same demonstration, the same missing line. `debug` ranks below the default INFO threshold and is filtered, precisely as in the Python book's figure. The output format carries the pyuvm signature into the rustdv house style: simulated time, level, `[hierarchy.path]:`, message.

One honest deviation to flag while it is visible in figure 2: pyuvm's logger got its path *for free* — the component knew its parent, so `uvm_test_top.comp` materialized without your help. A rustdv component's logger takes the path as a constructor argument, because a struct field does not know what field name it lives in. The convention is the one you'd guess (the logger path matches the field path: the env constructs `Scoreboard::new(...)` whose logger is `"env.scoreboard"`), and the derive-macro wiring that would automate it is on rustdv's roadmap rather than in its present. One string per constructor is the current price of the feature.¹

> ¹ Design-doc watchers: this is OQ-15's "logging-span wiring," deferred. The `Logger` API is shaped so the automation, when it lands, changes who *calls* `Logger::new` — not any code that logs.

## Logging levels

The six levels come straight across, minus one that Python's `logging` module defined but nobody used:

```text
# Figure 3: Logging levels

Level     rustdv               filtered by default?
-----     ------               --------------------
CRITICAL  log::critical / .critical()   no
ERROR     log::error / .error()         no
WARNING   log::warning / .warning()     no
INFO      log::info / .info()           no  <- default threshold
DEBUG     log::debug / .debug()         yes
(NOTSET)  — not ported; set an explicit level instead
```

A message prints when its level is at or above the governing threshold. "Governing" is where the hierarchy earns its keep: each named logger answers to the most specific level set on any *prefix* of its path, falling back to the global threshold (`log::set_level`) when nobody has spoken for it.

## Setting logging levels

pyuvm offered `set_logging_level` (one component) and `set_logging_level_hier` (the component and everything below). rustdv folds the pair into one function whose argument decides the scope — a path names one logger; a path *prefix* names a subtree:

```rust
// Figure 4: Setting the logging level for a hierarchy

#[rustdv::test]
async fn debug_test(_ctx: TestCtx) -> Result<(), TestError> {
    let mut comp = LogComp::new("uvm_test_top.comp");
    set_level_for("uvm_test_top", Level::Debug); // ...and everything below it

    let mut run_ctx = RunCtx::new();
    start_all(&mut comp, &mut run_ctx);
    run_ctx.all_objections_dropped().await;
```

```text
# Figure 5: Now the debug message prints
--
      0.00ns DEBUG    [uvm_test_top.comp]: This is debug
      0.00ns INFO     [uvm_test_top.comp]: This is info
      0.00ns WARNING  [uvm_test_top.comp]: This is warning
      0.00ns ERROR    [uvm_test_top.comp]: This is error
      0.00ns CRITICAL [uvm_test_top.comp]: This is critical
```

`set_level_for("uvm_test_top", Level::Debug)` opens the whole tree under `uvm_test_top` — the `set_logging_level_hier(DEBUG)` of the Python book's DebugTest, called from the same moment in the schedule (after construction, before `start_all`: the lines that replaced `end_of_elaboration_phase`). Longest prefix wins, so the debugging move you will actually make on a bad day is surgical: leave the world at INFO and open one suspect:

```rust
set_level_for("env.agent.driver", Level::Debug);   // just the driver chatters
```

The pyuvm caution — you can only set hierarchical levels after the hierarchy is built — dissolves rather than ports: levels attach to path prefixes, not to component objects, so there is no object that must exist first. Set them whenever; they govern whoever logs.

## Handlers: where the messages go

pyuvm inherited Python's handler zoo and taught two: the screen (StreamHandler, default) and a file. rustdv's screen handler is likewise just *on*, and the file handler is one call, mirroring `logging.FileHandler("log.txt", mode="w")` with the mode spelled as a boolean:

```rust
// Figure 6: Logging to a file

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
```

`false` is mode `"w"` (start fresh), `true` is mode `"a"` (append); `remove_log_file()` is `remove_logging_handler` for the one removable handler. And note the `?` on `log_to_file` — opening a file can fail, and where Python's `FileHandler` would raise `PermissionError` from inside the logging machinery, the rustdv call returns `Result` at the call site, converted here into a test error with the path in the message. After the run:

```text
$ cat /tmp/rustdv_ch26_log.txt
--
      0.00ns INFO     [uvm_test_top.comp]: This is info
      0.00ns WARNING  [uvm_test_top.comp]: This is warning
      0.00ns ERROR    [uvm_test_top.comp]: This is error
      0.00ns CRITICAL [uvm_test_top.comp]: This is critical
```

The screen output and the file agree line for line — one stream, two destinations, which is all a testbench usually wants. (pyuvm's further reaches — per-*component* handlers via `add_logging_handler`, custom formatters — have no rustdv port today; if your flow needs the log sliced per component, the pragmatic answer is grep over the path brackets, which is what the brackets are for.)

## Summary

Logging ported as a two-layer system over the format you already knew. The anonymous layer (`log::info` and friends) serves quick tests; the named layer gives each component a `Logger` carrying its hierarchy path — pyuvm's `self.logger` with the path made explicit, printed between the familiar square brackets. Six levels became five useful ones with INFO as the default gate; `set_level_for(prefix, level)` does the work of both `set_logging_level` and `set_logging_level_hier`, with longest-prefix-wins enabling the open-one-driver debugging move; and the file handler is `log_to_file`/`remove_log_file`, mode "w" or "a", with fallibility in the signature instead of the stack trace.

Next, the feature pyuvm hung on the hierarchy with string paths and a database: configuration. Chapter 25's constructor arguments were the quiet preview; Chapter 27 makes the argument in full — the ConfigDB's job, done by types.
