# Chapter 40: The Complete TinyALU Testbench

The Python book never needed this chapter — its final testbench accreted in place across the sequence chapters. This book gets one anyway, for the readers who will use it as a template: the full 8.0-era testbench, end to end, as it ships in the rustdv repository's `tinyalu_tb` crate — the same code the Interlude showed you before the climb, now with no line unexplained. If you read the Interlude first (you did — it was the deal), this chapter is the victory lap with map annotations.

## Project layout

```text
# Figure 1: The testbench crate

tinyalu_tb/
├── Cargo.toml            # [lib] crate-type = ["cdylib"]; depends on rustdv
├── src/
│   ├── lib.rs            # vpi_bootstrap!, test scaffolding, the tests
│   ├── alu_item.rs       # AluCommand / AluResult / predict  (Ch. 35)
│   ├── alu_bfm.rs        # TinyAluBfm                        (Ch. 19)
│   ├── components.rs     # Driver, monitors, Scoreboard, Coverage (Ch. 33, 36)
│   ├── env.rs            # AluEnv + AluEnvConfig             (Ch. 25, 27, 34)
│   └── sequences.rs      # RandomSeq, MaxSeq                 (Ch. 36)
└── (sim/run_rustdv.sh drives the build-and-simulate flow)
```

One crate, six files, one file per concern — and the file list *is* the chapter index for anyone maintaining it. `Cargo.toml`'s one interesting line is `crate-type = ["cdylib"]`: the testbench compiles to the shared library the simulator loads (Chapter 17).

## The pieces, and where you learned them

**Transactions** (`alu_item.rs`) — plain structs, three derives, the `predict` golden model as a free function, with unit tests beside it that run in `cargo test` with no simulator (Chapters 14, 35). **BFM** (`alu_bfm.rs`) — one owner of the pins, three falling-edge loops, queue-fed async surface, shared as `Rc` (Chapter 19), plus the `wait_idle` drain that ends tests on knowledge instead of clock-counting (Chapter 36). **Components** (`components.rs`) — the `Driver` pulling `SeqItem<AluCommand>` envelopes from its typed port; two monitors narrating to analysis ports; the `Scoreboard` draining its analysis FIFOs in `check` with `PartialEq` as comparison policy; `Coverage` as a `Subscriber` that counts and a `Component` that checks (Chapters 32, 33, 36). **Environment** (`env.rs`) — the config struct with the `Rc<TinyAluBfm>`, the `Active` enum, and `enable_coverage`; children as fields with `Option` for the conditional ones; the constructor doing build-and-connect in one pass (Chapters 24, 25, 27, 34). **Sequences** (`sequences.rs`) — `RandomSeq` and `MaxSeq`, late generation at grant time (Chapter 36).

The test scaffolding in `lib.rs` is the two helpers the whole book has been converging on:

```rust
// Figure 2: The scaffolding every test shares (tinyalu_tb/src/lib.rs)

async fn build_testbench(
    ctx: &TestCtx,
    enable_coverage: bool,
) -> Result<(Rc<TinyAluBfm>, AluEnv), TestError> {
    let dut = ctx.dut();
    let bfm = Rc::new(TinyAluBfm::new(&dut)?);
    Clock::new(bfm.clk(), SimDuration::ns(10)).start();
    bfm.start_tasks();
    bfm.reset().await;

    let config = AluEnvConfig { bfm: bfm.clone(), is_active: Active::Active, enable_coverage };
    let env = AluEnv::new(config);
    Ok((bfm, env))
}

async fn run_sequence(
    bfm: &Rc<TinyAluBfm>,
    env: &mut AluEnv,
    seq: &mut dyn Sequence<alu_item::AluCommand>,
    description: &str,
) -> Result<(), TestError> {
    let mut run_ctx = RunCtx::new();
    start_all(env, &mut run_ctx);

    {
        // Every stimulus task holds an objection guard (§7.3 convention 2).
        let _obj = run_ctx.raise_objection(description);
        env.sequencer().start(seq).await?;
        bfm.wait_idle().await;
    }
    run_ctx.all_objections_dropped().await;

    run_extract_check_report(env).map_err(TestError::from)
}
```

And the tests are the Interlude's — `random_ops` and `max_ops`, each a config decision and a sequence choice, differing in one line.

## Build, run, read

```text
# Figure 3: One command, zero to regression

$ sim/run_rustdv.sh
--
      0.00ns INFO     rustdv: found 2 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running random_ops (1/2)  [tinyalu_tb/src/lib.rs:67]
     75.00ns INFO     cmd_monitor: AluCommand { a: 193, b: 103, op: Add }
     75.00ns INFO     result_monitor: AluResult { result: 296 }
    ...
    635.00ns INFO     scoreboard: 20 compared, 0 mismatches
    635.00ns INFO     coverage: Add=5 And=5 Mul=5 Xor=5
    635.00ns INFO     random_ops PASSED
    ...
    830.00ns INFO     max_ops PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** random_ops                                   PASS         635.00      **
** max_ops                                      PASS         195.00      **
******************************************************************************
REGRESSION: PASS
```

The script's five jobs, each one line of shell: `cargo build --release` the cdylib; copy it as a `.vpi` module; `iverilog` the DUT; set the seed and results path; `vvp -M build -m tinyalu_tb`. Test selection and the seed ride on environment variables (`RUSTDV_RANDOM_SEED` to replay a failure), and the runner writes xUnit XML for your CI to ingest alongside the console table.

Reading the report is a skill worth the paragraph. The `[file:line]` on each `running` line is the test's source location (Chapter 21's macro captured it). The per-test SIM TIME column is each test's *own* duration, while log timestamps are absolute — the regression is one simulation, and the clock does not reset. On failure, the FAILED line carries the check messages from every component's `CheckSink` deposits, and a timed-out test reports the objections still held, by description — which is why convention 2 says every stimulus task takes a guard *with a description*.

## The two verifications of the verification

First, the one that needs no simulator:

```text
# Figure 4: cargo test — the testbench's own test suite

$ cd rustdv && cargo test -p tinyalu_tb
--
running 4 tests
test alu_item::tests::predict_add_carries_into_bit8 ... ok
test alu_item::tests::predict_and_masks ... ok
test alu_item::tests::predict_mul_uses_full_bus ... ok
test alu_item::tests::predict_xor ... ok
test result: ok. 4 passed; 0 failed
```

The predictor — the component whose bugs masquerade as DUT bugs — has its own unit tests, run on every build, in milliseconds (Chapter 14's promise, kept for the last time).

Second, the sabotage run, because a checker you have never seen fail is a checker you should not trust. Corrupt the DUT's XOR into an OR and rerun:

```text
# Figure 5: The testbench catching a planted bug

--
    830.00ns ERROR    scoreboard mismatch: AluCommand { a: 255, b: 255, op: Xor }
                      -> got AluResult { result: 255 }, expected AluResult { result: 0 }
    830.00ns ERROR    max_ops FAILED: 1 check failure(s): ...
REGRESSION: FAIL
```

Every affected transaction flagged, both tests failed, regression red. The checking has teeth; the repository's status log keeps this run's record as standing evidence.

## Using this as a template

Renaming the pieces for your DUT is the intended workflow, and the order of work is the order of the files: transactions first (plain structs + a predictor + its unit tests), then the BFM (own the pins, expose async verbs, `wait_idle`), then components (driver and monitors are twenty lines each), then the env (fields, config, one constructor), then sequences, then tests. The one design decision per stage that matters: transactions — what is the *unit of intent* on this interface; BFM — what are the protocol's verbs; env config — what will tests want to vary (expose it now; Chapter 29's ledger explains why later is harder); sequences — what orderings does the protocol make meaningful. Everything else is this chapter with the names changed — which is precisely what a methodology is for.

The TinyALU has nothing left to fear from us. One chapter remains: what all this adds up to, and where Rust in verification goes from here.
