# Interlude: The Complete TinyALU Testbench

> **In the UVM...** the destination was always the same summit: a TinyALU driven by sequences, checked by a scoreboard, measured by coverage, held together by the methodology — whether *The UVM Primer* built it in SystemVerilog or *Python for RTL Verification* built it in pyuvm, both ended there. This interlude shows you that summit in Rust — the complete, running rustdv testbench — *before* the climb. Nothing here is pseudocode: every line below is the shipped `tinyalu_tb` crate, and it runs on Icarus Verilog to `REGRESSION: PASS`.

Part I handed you fourteen chapters of language and kept saying they were load-bearing. This is the load. Read it the way you would walk through a finished house before studying the blueprints: do not try to understand it — try to *recognize* it. You know this testbench. You have built it in another language. The point of the next few pages is that when you squint, it is the tool you already own, spelled in the language you just learned — and the parts you cannot read yet each have a chapter with their name on it. The map of those chapters closes the interlude. Chapter 40 walks this same code with everything explained.

## The transactions

```rust
// Figure 1: The TinyALU transactions (tinyalu_tb/src/alu_item.rs)

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Ops {
    Add = 1,
    And = 2,
    Xor = 3,
    Mul = 4,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AluCommand {
    pub a: u8,
    pub b: u8,
    pub op: Ops,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AluResult {
    pub result: u16,
}

/// Golden model (the scoreboard's predictor).
pub fn predict(cmd: &AluCommand) -> AluResult {
    let a = cmd.a as u16;
    let b = cmd.b as u16;
    let result = match cmd.op {
        Ops::Add => a + b,
        Ops::And => a & b,
        Ops::Xor => a ^ b,
        Ops::Mul => a * b,
    };
    AluResult { result }
}
```

This figure you can read completely — it is Chapters 4, 6, 7, and 10 doing their jobs. A transaction is a plain struct; no `uvm_sequence_item` base class, and the jobs the base class did arrive as derives: `Clone` is `do_copy`, `PartialEq` is `do_compare`, `Debug` is the printable form. The predictor is a function and a `match`.

## The stimulus

```rust
// Figure 2: A sequence — stimulus as a program (tinyalu_tb/src/sequences.rs)

/// How the operands get filled, once the driver is committed.
trait Operands {
    fn set_operands(&mut self, rng: &mut Rng, cmd: &mut AluCommand);
}

/// Every operation, `n` times each — the walk all three sequences share.
async fn all_ops<S: Operands>(
    seq: &mut S,
    ctx: &mut SeqCtx<AluCommand, AluResult>,
    n: usize,
) -> Result<(), SeqError> {
    let mut rng = ctx.rng();
    for _ in 0..n {
        for op in Ops::ALL {
            let mut cmd = AluCommand { a: 0, b: 0, op };
            ctx.start_item(&mut cmd).await?;
            // Late generation: the driver is waiting, so decide now.
            seq.set_operands(&mut rng, &mut cmd);
            // Ownership moves to the driver here. A sequence that needed the
            // command afterward would clone it first; this one does not.
            ctx.finish_item(cmd).await?;
        }
    }
    Ok(())
}

/// Random operands across every operation, five times each.
#[derive(Default)]
pub struct RandomSeq;

impl Operands for RandomSeq {
    fn set_operands(&mut self, rng: &mut Rng, cmd: &mut AluCommand) {
        cmd.a = rng.u8();
        cmd.b = rng.u8();
    }
}

impl Sequence for RandomSeq {
    type Req = AluCommand;
    type Rsp = AluResult;

    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        all_ops(self, ctx, 5).await
    }
}
```

`start_item`, `finish_item`, a body that loops the operations: the sequence idiom you know, and the comment about ownership moving is Chapter 5 speaking. What `SeqCtx` and `Sequence` are, and why the rendezvous has two calls, is Chapter 36's whole subject.

## A component

```rust
// Figure 3: The driver (tinyalu_tb/src/components.rs)

#[derive(Component, Default)]
pub struct Driver {
    #[port(seq_item)]
    seq_item_port: SeqItemPort<AluCommand, AluResult>,
}

impl Component for Driver {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        bfm.reset().await;
        loop {
            let item = self.seq_item_port.get_next_item().await;
            bfm.send_op(item.payload().clone()).await;
            self.seq_item_port.item_done(None);
        }
    }
}
```

`get_next_item`, drive, `item_done` — `uvm_driver`'s loop, recognizable at a glance. Two things to merely notice, not yet understand: the driver takes *no constructor arguments* — the BFM arrives from something called the `ConfigDb`, by name — and its `run` returns a `Result`, with `?` doing what Chapter 9 taught.

## The scoreboard

```rust
// Figure 4: The scoreboard — two streams in, verdicts in check
//           (tinyalu_tb/src/components.rs)

#[derive(Default)]
struct CmdLog {
    cmds: Vec<AluCommand>,
}

impl WriteSink<AluCommand> for CmdLog {
    fn write(&mut self, cmd: &AluCommand) {
        self.cmds.push(cmd.clone());
    }
}

#[derive(Default)]
struct ResultLog {
    results: Vec<AluResult>,
}

impl WriteSink<AluResult> for ResultLog {
    fn write(&mut self, res: &AluResult) {
        self.results.push(res.clone());
    }
}

#[derive(Component, Default)]
pub struct Scoreboard {
    #[port(subscribe)]
    cmd_in: SubscribePort<AluCommand>,
    #[port(subscribe)]
    result_in: SubscribePort<AluResult>,
    cmd_log: RustdvShared<CmdLog>,
    result_log: RustdvShared<ResultLog>,
    compared: usize,
    mismatches: usize,
}

impl Component for Scoreboard {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.cmd_in.on_write(self.cmd_log.clone());
        self.result_in.on_write(self.result_log.clone());
    }

    fn check(&mut self, ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        let cmd_log = self.cmd_log.get();
        let result_log = self.result_log.get();

        for (cmd, actual) in cmd_log.cmds.iter().zip(result_log.results.iter()) {
            let expected = predict(cmd);
            self.compared += 1;
            if expected != *actual {
                self.mismatches += 1;
                ctx.info(&format!(
                    "scoreboard: in={cmd:?} out={actual:?} expected={expected:?} check=FAIL"
                ));
                errors.error(format!(
                    "scoreboard mismatch: {cmd:?} -> got {actual:?}, expected {expected:?}"
                ));
            } else {
                ctx.info(&format!(
                    "scoreboard: in={cmd:?} out={actual:?} expected={expected:?} check=PASS"
                ));
            }
        }

        if cmd_log.cmds.len() != result_log.results.len() {
            errors.error(format!(
                "scoreboard: saw {} commands and {} results",
                cmd_log.cmds.len(),
                result_log.results.len()
            ));
        }
        if self.compared == 0 {
            errors.error("scoreboard: nothing was compared".to_string());
        }
    }

    fn report(&mut self, ctx: &mut RustdvCtx) {
        ctx.info(&format!(
            "scoreboard: {} compared, {} mismatches",
            self.compared, self.mismatches
        ));
    }
}
```

A scoreboard that subscribes to two streams — commands and results — predicts with the figure-1 function, compares with `PartialEq`, and files its failures somewhere called a `CheckSink` inside a phase called `check`. Notice, without yet knowing why, that it holds a plain `Vec` for each stream, and that the last two error checks refuse to let "nothing arrived" look like "nothing failed." There are also two monitors publishing onto the buses the scoreboard reads, and a coverage collector counting ops as a second subscriber on the command stream — the same shapes, not reprinted here.

## The environment

```rust
// Figure 5: The environment — build creates, connect wires
//           (tinyalu_tb/src/env.rs)

#[derive(Component, Default)]
pub struct AluEnv {
    #[component(sequencer)]
    seqr: Sequencer<AluCommand, AluResult>,
    #[component(child)]
    driver: RustdvComp,
    #[component(child)]
    cmd_mon: RustdvComp,
    #[component(child)]
    result_mon: RustdvComp,
    #[component(child)]
    scoreboard: RustdvComp,
    #[component(child)]
    coverage: RustdvComp,
    #[component(fifo)]
    cmd_bus: AnalysisBus<AluCommand>,
    #[component(fifo)]
    result_bus: AnalysisBus<AluResult>,
    is_active: bool,
    with_coverage: bool,
}

impl Component for AluEnv {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let activity: Active = ConfigDb::get(Some(ctx), "", "IS_ACTIVE").unwrap_or(Active::Active);
        self.is_active = activity == Active::Active;
        self.with_coverage = ConfigDb::get(Some(ctx), "", "WITH_COVERAGE").unwrap_or(true);

        self.seqr = Sequencer::new();
        ConfigDb::set(None, "*", "SEQR", self.seqr.handle());

        if self.is_active {
            self.driver = Driver::create_comp();
        }
        self.cmd_mon = CmdMonitor::create_comp();
        self.result_mon = ResultMonitor::create_comp();
        self.scoreboard = Scoreboard::create_comp();
        if self.with_coverage {
            self.coverage = Coverage::create_comp();
        }

        self.cmd_bus = AnalysisBus::new();
        self.result_bus = AnalysisBus::new();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        if self.is_active {
            self.seqr.seq_item_export().connect(&self.driver, Driver::SEQ_ITEM_PORT);
        }

        self.cmd_bus.pub_export().connect(&self.cmd_mon, CmdMonitor::AP);
        self.cmd_bus.sub_export().connect(&self.scoreboard, Scoreboard::CMD_IN);
        if self.with_coverage {
            self.cmd_bus.sub_export().connect(&self.coverage, Coverage::CMD_IN);
        }

        self.result_bus.pub_export().connect(&self.result_mon, ResultMonitor::AP);
        self.result_bus.sub_export().connect(&self.scoreboard, Scoreboard::RESULT_IN);
    }

    fn start_of_simulation(&mut self, ctx: &mut RustdvCtx) {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM").expect("the test sets BFM");
        bfm.start_tasks();
    }
}
```

Here is the whole restored vocabulary on one page: a `build` phase creating children top-down, a `connect` phase wiring them bottom-up, components built through something called a factory (`create_comp()`), an `IS_ACTIVE` knob read from the ConfigDb that decides whether a driver exists at all, and one broadcast bus per observed stream. If you have written a `uvm_env`, every line has a shape you have seen — down to the passive env that simply does not build its driver.

## The tests

```rust
// Figure 6: Two tests, one testbench (tinyalu_tb/src/tinyalu_tb.rs)

#[derive(Component, Default)]
pub struct BaseTest {
    #[component(child)]
    env: RustdvComp,
}

impl Component for BaseTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let bfm = TinyAluBfm::new(&ctx.dut()).expect("TinyALU signals");
        ConfigDb::set(None, "*", "BFM", Rc::new(bfm));
        self.env = AluEnv::new_comp();
    }

    fn start_of_simulation(&mut self, ctx: &mut RustdvCtx) {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM").expect("build filed the BFM");
        Clock::new(bfm.clk(), SimDuration::ns(10)).start();
    }

    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("stimulus");

        let seqr: Sequencer<alu_item::AluCommand, alu_item::AluResult> =
            ConfigDb::get(Some(ctx), "", "SEQR")?;

        let mut seq = create_seq::<BaseSeq>();
        seq.start(&seqr).await?;

        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        bfm.wait_idle().await;

        ctx.info("sequence complete");
        Ok(())
    }
}

#[rustdv::test(timeout_time = 500, timeout_unit = "us")]
#[derive(Component, Default)]
struct RandomTest {
    #[component(child)]
    inner: RustdvComp,
}

impl Component for RandomTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        set_seq_override::<BaseSeq, RandomSeq>();
        self.inner = BaseTest::new_comp();
    }
}

#[rustdv::test(timeout_time = 500, timeout_unit = "us")]
#[derive(Component, Default)]
struct MaxTest {
    #[component(child)]
    inner: RustdvComp,
}

impl Component for MaxTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        set_seq_override::<BaseSeq, MaxSeq>();
        self.inner = BaseTest::new_comp();
    }
}
```

Two tests, and neither adds a component. Each names a different sequence for the factory to substitute and reuses everything else — the whole point of the methodology, visible in six lines of difference.

```text
# Figure 7: The testbench running

[TRANSCRIPT NEEDED — copy verbatim from a rerun of sim/run_rustdv.sh. The
counts to expect: RandomTest 20 compared / 0 mismatches, coverage
Add=5 And=5 Mul=5 Xor=5; MaxTest 4 / 0, all ops covered; REGRESSION: PASS,
every line carrying its component's path.]
```

Twenty-four operations driven, predicted, compared, and counted, with every log line stamped with the path of the component that wrote it — and the run ends in the summary table and `REGRESSION: PASS`.

## What you could already read, and where the rest is taught

Tally what Part I just let you read fluently: enums and `match` in the transactions and the predictor (Chapters 4, 7); ownership moving the command into `finish_item`, with `clone()` as the alternative (Chapter 5); `Rc` where one BFM is truly shared (Chapter 13); `Result` and `?` threading every fallible step (Chapter 9); traits standing in for base classes, derives doing `uvm_object`'s jobs (Chapter 10); `SeqItemPort<AluCommand, AluResult>` and the other typed plumbing (Chapter 11); `Vec` and `HashMap` holding what the subscribers keep (Chapter 8); and a crate you could build and unit-test with `cargo` (Chapter 14).

What you took on faith is exactly the rest of the book. The page after this one — the rustdv Toolkit — names every framework identifier you just squinted at. Then: `async`/`await` and the executor underneath every `run` (Chapter 15), tasks and queues (Chapter 16), the simulator connection and the BFM (Chapters 17–19), the macros behind `#[rustdv::test]` and `#[derive(Component)]` (Chapter 21), tests as components (Chapter 23), the nine phases and the growing tree (Chapter 24), the ConfigDb that delivered the BFM (Chapters 25, 27–28), the factory behind `create_comp` and `create_seq` (Chapter 29), ports, FIFOs and the connect idiom (Chapter 31), the analysis buses and why the scoreboard owns its own `Vec`s (Chapter 32), testbench 6.0 wiring this very architecture (Chapters 33–34), transactions in full (Chapter 35), and the sequencer handshake (Chapter 36) with its response machinery (Chapters 37–38) and virtual sequences (Chapter 39). Chapter 40 then returns here, to this exact crate, and walks it with nothing left on faith.

The climb starts on the next page. It is worth it: at the top, this testbench is yours.
