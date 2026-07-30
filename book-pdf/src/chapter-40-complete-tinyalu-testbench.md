# Chapter 40: The Complete TinyALU Testbench

The Interlude showed you this testbench before you could read it, and asked only for recognition. Thirty-nine chapters later, the deal completes: the same `tinyalu_tb` crate, walked with nothing left on faith. This is also the chapter to use as a template — the shipped testbench in the rustdv repository, the one its regression runs, organized the way a real project's would be.

## Project layout

```text
# Figure 1: The testbench crate

tinyalu_tb/src/
├── tinyalu_tb.rs    the crate root: BaseTest, RandomTest, MaxTest
├── alu_item.rs      transactions, Ops, and the predictor — plus unit tests
├── alu_bfm.rs       the BFM: pins, protocol loops, queue-fed methods
├── sequences.rs     BaseSeq, RandomSeq, MaxSeq over one shared walk
├── components.rs    Driver, two monitors, Scoreboard, Coverage
└── env.rs           AluEnv: build, connect, and two ConfigDb knobs
```

One file per concern, and the crate root named after the crate — no file in this project is named `lib.rs`, so a stack trace or a log line always says *which* crate it came from. `alu_item.rs` ends with `#[cfg(test)]` unit tests: the predictor and the transaction derives are checked by `cargo test` on every build, no simulator anywhere — Chapter 14's capability, earning its keep in shipping code.

## The tests

```rust
// Figure 2: BaseTest — build files the BFM; run starts whatever the factory chose
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
```

Every line is a chapter. `build` constructs the BFM from the DUT handle and files it in the ConfigDb under `"*"` — the whole tree gets this one, and there is no singleton anywhere in the crate: the database asserts "one BFM under this name for this subtree," which is a promise a two-interface testbench can keep, where a singleton's "one BFM in the world" is not (Chapter 25). `run` finds the sequencer by name, builds its sequence *through the factory*, and starts it (Chapter 36).

Two details differ from the chapters, and each has its reason. First, `start_of_simulation` starts a `Clock` — the one place in the book's code that drives a clock rather than waits on one. The shipped testbench runs against `sim/hdl/tinyalu.sv`, the bare DUT, which takes `clk` as an input; the book's chapter examples run against a copy of the design that clocks itself, so their testbenches never touch a clock, and Chapter 19 told you why that discipline matters: a BFM that only ever *waits* on edges ports to an emulator unchanged, and one that drives them does not. Everything above this one line is that kind of BFM. The clock is the single place this testbench talks to a simulator rather than to a design.

Second, the end of stimulus is `bfm.wait_idle().await`, not the twenty-clock flush of Chapters 34 and 36. Counting clocks worked, but it encoded a magic number — twenty, because the multiply is slowest — that would quietly go stale if the DUT grew a slower operation. `wait_idle` asks the *protocol* instead: it watches for the driver queue empty and the handshake quiet for two consecutive falling edges (two, because a command already popped but not yet driven must not fool it), then gives the monitors one more edge to flush. Same job, no magic number, and it moves with the DUT.

```rust
// Figure 3: Two tests, one testbench, no new components
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

Each registered test is one override plus `BaseTest` — Chapter 36's pattern, shipping. `RandomSeq` runs every operation five times with seeded random operands, so coverage is guaranteed by construction rather than hoped for; `MaxSeq` drives the `0xff op 0xff` corner once each — the corner random stimulus is unlikely to find on its own. Adding a third stimulus pattern to this testbench is a sequence and a six-line test; no component changes, which is the measure the whole book has been building toward. The `timeout_time` attributes are the runner's safety net: a hung handshake fails loudly at 500 microseconds instead of running forever.

## The environment's two knobs

The Interlude showed `AluEnv` in full. The walk stops at its opening lines, because they are the ConfigDb doing structural work:

```rust
// Figure 4: Two choices a test can make from outside (env.rs, build)
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let activity: Active = ConfigDb::get(Some(ctx), "", "IS_ACTIVE").unwrap_or(Active::Active);
        self.is_active = activity == Active::Active;
        self.with_coverage = ConfigDb::get(Some(ctx), "", "WITH_COVERAGE").unwrap_or(true);

        self.seqr = Sequencer::new();
        ConfigDb::set(None, "*", "SEQR", self.seqr.handle());

        if self.is_active {
            self.driver = Driver::create_comp();
        }
        // ...
    }
```

`IS_ACTIVE` is the UVM's active/passive knob, typed: `Active` is an enum, so an illegal value cannot be filed, and `unwrap_or(Active::Active)` makes the ordinary case configure nothing. Look at what a passive env *is*: the driver slot is simply left empty — no driver constructed and told not to drive, no `None` checks downstream, just a component that does not exist and a `connect` that (three lines later) skips its wiring. This is why `build` had to be a phase: whether the driver exists is decided by configuration that must arrive *before* the children do. `WITH_COVERAGE` works the same way for the coverage collector. And this is also the reason every component takes no constructor arguments — the factory's makers cannot supply any (Chapter 29), so everything a component needs arrives by name after it exists, which is exactly what makes the whole tree overridable.

## The scoreboard's guards

The Interlude showed the scoreboard whole; the walk stops at the end of its `check`, on two guards that a lesser scoreboard omits:

```rust
// Figure 5: A scoreboard that cannot pass vacuously (components.rs, check)
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
```

The comparison loop zips commands against results, and a zip cannot complain about what never arrived — the shorter stream just ends the comparison, which is how a scoreboard passes while checking less than it saw (Chapter 34's warning). The first guard makes the count mismatch an error in its own right. The second refuses a clean pass with zero comparisons — the oldest trap in verification, a checker that never ran. Both guards were earned the hard way: when this testbench was refactored onto the restored framework, the storage moved from a hub into the subscriber (Chapter 32's rule — the scoreboard owns two `RustdvShared` logs behind two `SubscribePort`s), and a scoreboard that had quietly stopped receiving would have looked identical to one that passed. The project's answer was to corrupt the DUT's XOR into an OR and watch the scoreboard fail — verification of the verification — and these guards are what make that check stay meaningful.

## The run

```text
# Figure 6: The shipped testbench running

[TRANSCRIPT NEEDED — copy verbatim from a rerun of sim/run_rustdv.sh.
Expected: RandomTest 20 compared / 0 mismatches, coverage Add=5 And=5 Mul=5
Xor=5; MaxTest 4 / 0, all ops; every line under its component's path
(e.g. [RandomTest.inner.env.scoreboard]); REGRESSION: PASS.]
```

Twenty-four operations across two tests, each line stamped with the path of the component that wrote it, ending in the summary table. This is the same run the repository's regression asserts — the counts above are checked mechanically on every push, so the transcript you produce and the one in this book can only agree.

## What to take with you

Use this crate as the template it is. Transactions are plain structs with derives and a hand-written `Display`, plus unit tests beside them (Chapter 35, Chapter 14). The BFM owns the pins, speaks the protocol on falling edges, and exposes queue-fed async methods — and hides one hand-written `Debug` impl so `ConfigDb::dump` names it as `TinyAluBfm` instead of dumping eight signal handles (a small kindness Chapter 28 makes you glad of). Sequences share one walk and vary one method; components take nothing at construction and ask the ConfigDb for what they need; the env reads its knobs before building, builds through the factory, and wires everything in `connect` with the one idiom; the tests are overrides on a shared base. Every piece was a chapter; together they are a working, checked, regression-guarded testbench — and now they are yours.

The climb the Interlude promised is done. What follows are the appendices: the maps back to the earlier books, the idiom translations from both source languages, and the full catalogue of what rustdv provides.
