# Chapter 33: Components in Testbench 6.0

Configuration, variation points, logging, channels, analysis ports — the toolbox is full, and testbench 6.0 spends it. The 6.0 principle, quoted from the Python book because it cannot be improved: each component "either creates data and writes it to a port or gets data from a port and processes it." One job each. This chapter refactors the components to that standard; Chapter 34 wires them up.

> **In the UVM...** we split the testbench into a `BaseTester` that put command tuples into a `uvm_put_port`, a `Driver` that pulled from a `uvm_get_port` and drove the BFM, monitors publishing on analysis ports, a `Coverage` subscriber, and a `Scoreboard` buffering two `uvm_tlm_analysis_fifo`s for the check phase. The Python version had a flourish: one `Monitor` class taking a *method name* string, with `getattr` fetching `get_cmd` or `get_result` at runtime.

## The tester: stimulus and nothing else

```rust
// Figure 2: The tester writes to a Sender, not to the BFM

pub struct TesterComp<T: Tester + 'static> {
    tx: Sender<Cmd>,
    tester: Option<T>,
}

impl<T: Tester + 'static> Component for TesterComp<T> {
    fn start(&mut self, ctx: &mut RunCtx) {
        let tx = self.tx.clone();
        let mut tester = self.tester.take().expect("tester started twice");
        let obj = ctx.raise_objection("tester stimulus");
        spawn_named(
            async move {
                for op in Ops::ALL {
                    let (aa, bb) = tester.get_operands();
                    tx.send((aa, bb, op)).await.expect("driver dropped");
                }
                // twenty clocks: let the last operations drain through the
                // channel, the BFM queue, and the DUT (pyuvm waited ten;
                // our pipeline is one buffered stage deeper)
                Timer::ns(200).await;
                drop(obj);
            },
            "tester.run",
        );
    }
}
```

The 4.0/5.0 tester held a BFM handle; this one holds a `Sender<Cmd>` and has never heard of pins. `RandomTester` and `MaxTester` are untouched — still the Chapter 20 `get_operands` implementations, still selecting behavior through the generic parameter. Two inheritances from the pyuvm original are worth naming. The end-of-stimulus drain (pyuvm's ten `ClockCycles`) survives as a timer, with the count honestly adjusted for our extra buffer stage — and yes, "wait long enough" is a smell; testbench 7.0's sequencer handshake replaces it with actual completion knowledge, which is part of why sequences exist. And where pyuvm's `BaseTester.get_operands` raised `RuntimeError` if you ran it unextended, the Rust version has no such method to forget: `TesterComp<T>` requires a `T: Tester` at the type level.

## The driver

```rust
// Figure 3: The Driver: from the Receiver to the pins

#[derive(rustdv::Component)]
pub struct Driver {
    bfm: Rc<TinyAluBfm>,
    rx: Option<Receiver<Cmd>>,
}

impl Component for Driver {
    fn start(&mut self, _ctx: &mut RunCtx) {
        let bfm = self.bfm.clone();
        let rx = self.rx.take().expect("driver started twice");
        spawn_named(
            async move {
                bfm.reset().await;
                bfm.start_tasks();
                while let Ok((aa, bb, op)) = rx.recv().await {
                    bfm.send_op(aa, bb, op).await;
                }
            },
            "driver.run",
        );
    }
}
```

pyuvm's `Driver` figure, nearly line for line: reset, start the BFM tasks, then loop — get a command, send an operation. The channel's blocking `recv`/`send_op` pair is what synchronizes stimulus generation with the DUT's actual pace, exactly as the blocking get port did. The `while let Ok(...)` is Chapter 31's end-of-stream idiom: if the tester drops its `Sender`, the driver retires instead of blocking forever. No objection guard — the driver is a servant of stimulus, not a source of it.

## One Monitor, not two

Here the Python book showed off: one `Monitor` class taking a *method name*, with `getattr(self.bfm, self.method_name)` fetching `get_cmd` or `get_result` at runtime — "a Python feature that does not exist in SystemVerilog." It does not exist in Rust either, and the translation is instructive: you cannot pass a method's *name*, but you can pass the *method*.

```rust
// Figure 5: One Monitor type; the function is the argument

pub type GetFn<T> = fn(Rc<TinyAluBfm>) -> Pin<Box<dyn std::future::Future<Output = T>>>;

pub struct Monitor<T: 'static> {
    name: &'static str,
    bfm: Rc<TinyAluBfm>,
    get: GetFn<T>,
    ap: AnalysisPort<T>,
}

impl<T: std::fmt::Debug + 'static> Component for Monitor<T> {
    fn start(&mut self, _ctx: &mut RunCtx) {
        let (name, bfm, get, ap) = (self.name, self.bfm.clone(), self.get, self.ap.clone());
        spawn_named(
            async move {
                loop {
                    let datum = get(bfm.clone()).await;
                    log::info(&format!("{name}: {datum:?}"));
                    ap.write(&datum);
                }
            },
            "monitor.run",
        );
    }
}
```

```rust
// Figure 6: The two BFM methods, as passable functions

pub fn get_cmd(bfm: Rc<TinyAluBfm>) -> Pin<Box<dyn std::future::Future<Output = CmdTuple>>> {
    Box::pin(async move { bfm.get_cmd().await })
}

pub fn get_result(bfm: Rc<TinyAluBfm>) -> Pin<Box<dyn std::future::Future<Output = u64>>> {
    Box::pin(async move { bfm.get_result().await })
}
```

Instantiation reads like pyuvm's figure 5 with the string quotes removed: `Monitor::new("cmd_monitor", bfm, get_cmd, cmd_ap)` versus `Monitor("cmd_monitor", self, "get_cmd")`. The differences are the point. `getattr` with a typo'd string was a runtime `AttributeError`; a typo'd function name here doesn't resolve at compile time. And the *type parameter* rides along: a `Monitor<CmdTuple>` publishes commands and a `Monitor<u64>` publishes results, so connecting the result monitor to the command scoreboard is a type error — dynamism's flexibility, minus dynamism's trapdoors. The `Pin<Box<dyn Future>>` in `GetFn` is the one place the plumbing shows (an `fn` pointer must name a concrete return type, and async methods don't have one); the two four-line adapter functions pay that tax once.²

> ² A team preferring two ten-line monitors over one generic monitor plus adapters would get no argument from this book — the Interlude's testbench chose exactly that. The figure exists because the Python book's `getattr` deserves an honest translation, not because the translation is mandatory.

## Coverage: a Subscriber with a check

```rust
// Figure 7: Coverage is a Subscriber and a Component

struct CovCollector {
    cvg: HashMap<Ops, usize>,
}

impl Subscriber<CmdTuple> for CovCollector {
    fn write(&mut self, cmd: &CmdTuple) {
        if let Some(op) = Ops::from_u64(cmd.2) {
            *self.cvg.entry(op).or_insert(0) += 1;
        }
    }
}

pub struct Coverage {
    collector: Rc<RefCell<CovCollector>>,
}

impl Component for Coverage {
    fn check(&mut self, errors: &mut CheckSink) {
        let cvg = &self.collector.borrow().cvg;
        if Ops::ALL.iter().any(|op| !cvg.contains_key(op)) {
            errors.error("Functional coverage error: missed operations".to_string());
        } else {
            log::info("Covered all operations");
        }
    }
}
```

pyuvm's `Coverage` extended `uvm_analysis_export` and did both jobs in one class; Rust splits the two *roles* into two types wearing one trench coat: the inner `CovCollector` is the `Subscriber` the analysis port talks to (via the `Rc<RefCell>` from Chapter 32), and the outer `Coverage` is the `Component` the hierarchy talks to, checking at end of test. The scoreboard's job — comparing — moved out of coverage entirely, completing the one-job-each refactor.

## The scoreboard: FIFOs in, verdicts out

```rust
// Figure 8: The Scoreboard drains its analysis FIFOs in check

#[derive(rustdv::Component)]
pub struct Scoreboard {
    cmd_fifo: AnalysisFifo<CmdTuple>,
    result_fifo: AnalysisFifo<u64>,
}

impl Component for Scoreboard {
    fn check(&mut self, errors: &mut CheckSink) {
        while let Some(cmd) = self.cmd_fifo.try_get() {
            let (aa, bb, op_int) = cmd;
            let op = Ops::from_u64(op_int).expect("illegal op captured");
            let Some(actual) = self.result_fifo.try_get() else {
                errors.error(format!("Missing result for command {cmd:?}"));
                break;
            };
            let actual = actual as u16;
            let prediction = alu_prediction(aa as u8, bb as u8, op);
            if actual == prediction {
                log::info(&format!("PASSED: {aa:02x} {op:?} {bb:02x} = {actual:04x}"));
            } else {
                errors.error(format!(
                    "FAILED: {aa:02x} {op:?} {bb:02x} = {actual:04x} - predicted {prediction:04x}"
                ));
            }
        }
    }
}
```

Gone at last: the `Rc<RefCell<Vec<...>>>` lists that versions 2.0 through 5.0 dragged along. The analysis FIFOs *are* the storage — the monitors' broadcasts accumulate in them all run long, and `check` drains with `try_get` until empty, pyuvm's figure 12 loop with the `(got_next, value)` tuples replaced by `Option`s and the missing-result `RuntimeError` replaced by a check error. The scoreboard owns its FIFOs as plain fields; nothing is shared, nothing is `RefCell`ed, and the 2.0 chapter's promised cure has arrived.

## Summary

Testbench 6.0's components each do one thing. The tester generates and sends (`Sender<Cmd>`); the driver receives and drives (`Receiver<Cmd>` to BFM); one generic `Monitor<T>` observes and broadcasts, taking the BFM accessor as a passed *function* where Python passed a method name; `Coverage` splits into a `Subscriber` collector and a checking `Component`; and the `Scoreboard` buffers broadcasts in owned `AnalysisFifo`s and drains them in `check`, shared-mutable-state-free. Every component names its inputs and outputs in its field types.

None of them, you will notice, knows any other exists. That is Chapter 34's whole subject: the environment that introduces them — where every connection is a constructor argument, and the wiring diagram *is* the `new()` function.
