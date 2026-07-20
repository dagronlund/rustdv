# Chapter 23: uvm_test Testbench: 3.0

Testbench 2.0 was modular; now we make it methodological. This chapter writes the first rustdv tests in the UVM style — and immediately meets the largest single difference between pyuvm and rustdv, which is what happened to the test *class*. Tradition first, though.

> **In the UVM...** the test was a class extending `uvm_test`, with an objection-guarded `run_phase()` that raised, said hello, and dropped — `class hello_world extends uvm_test` selected by `run_test()` in SV; `HelloWorldTest(uvm_test)` marked `@pyuvm.test()` in Python — and the framework instantiated it under the name `uvm_test_top`. Then testbench 3.0 refactored 2.0: `BaseTest` carried the shared `run_phase()`, while `RandomTest` and `MaxTest` overrode `build_phase()` to pick a tester.

## Hello, world, in the methodology

```rust
// Figure 1: The basic rustdv-UVM use model in hello_world

#[rustdv::test]
async fn hello_world_test(_ctx: TestCtx) -> Result<(), TestError> {
    let run_ctx = RunCtx::new();
    {
        let _obj = run_ctx.raise_objection("saying hello");
        log::info("Hello, world.");
    } // the guard drops here: the objection is released
    run_ctx.all_objections_dropped().await;
    Ok(())
}
```

```text
# Figure 2: Hello, world!
--
      0.00ns INFO     running hello_world_test (1/3)  [ch23-uvm-test-testbench-3.0/src/lib.rs:18]
      0.00ns INFO     Hello, world.
      0.00ns INFO     hello_world_test PASSED
```

Set this beside the pyuvm original and take inventory. `@pyuvm.test()` on a class became `#[rustdv::test]` on a function — the same attribute we have used since Chapter 15, because in rustdv there is only one kind of test. `run_phase(self)` became the test body itself. And `raise_objection()`/`drop_objection()` became a **guard**: `raise_objection` returns an `ObjectionGuard` whose `Drop` *is* the drop call. The two new lines are the methodology showing through: `RunCtx` is the run-phase context that Part IV's components will all share, and `all_objections_dropped().await` is the UVM's end-of-run consensus — the run continues until every raised objection has been released. Here there is one objection and it dies at the closing brace, so the await returns immediately; in three chapters, when free-running drivers and monitors hold guards of their own, that await becomes the thing deciding when your test ends.

Notice what the guard pattern deletes: the *forgot to drop the objection* bug, which in pyuvm hangs the run phase until the timeout fires, is unwritable — you would have to deliberately `std::mem::forget` the guard. Scope ends, objection drops. (Chapter 16's `LockGuard`, Chapter 13's RAII, third verse.)

## Where uvm_test went

Both earlier books paused here for the UML tower every UVM engineer has climbed: `uvm_void` → `uvm_object` → `uvm_report_object` → `uvm_component` → `uvm_test`, with your tests extending the top. It is worth reprinting the tower just to watch what happens to it in Rust:

```text
# Figure 3: The uvm_test tower, and its rustdv equivalent

pyuvm                          rustdv
-----                          ------
uvm_void                       (nothing — served the factory; see Ch. 29)
uvm_object                     plain structs + derives (Ch. 35)
uvm_report_object              log:: targets on components (Ch. 26)
uvm_component                  the Component trait (Ch. 24)
uvm_test                       the #[rustdv::test] fn itself
YourTest(uvm_test)             — the function body IS the test
```

`uvm_test` existed to answer "which class does the framework instantiate to start everything?" — and pyuvm instantiated it for you, named it `uvm_test_top`, and dispatched its phases. rustdv's answer: the framework does not instantiate anything. **The test function constructs and owns the testbench.** There is no `uvm_test_top`, no test class, and no moment where a factory conjures your test by name — the registry from Chapter 21 finds the *function*, and everything after that is ordinary ownership. The test-by-name capability that `uvm_root.run_test("RandomTest")` provided lives in the runner's registry instead, where Chapter 21 put it.

What survives, because it is methodology rather than mechanism: the *shape* — a shared base holding the common run logic, with tests varying only in what they build.

## Refactoring testbench 2.0

pyuvm expressed base-and-variants as `BaseTest` (abstract, providing `run_phase`) extended by `RandomTest` and `MaxTest` (each overriding `build_phase` to pick a tester). We already know Rust's spelling of that idea from Chapter 20 — and here it is doing the whole job:

```rust
// Figure 4: base_test — the shared run phase of every test

async fn base_test(ctx: &TestCtx, tester: &mut impl Tester) -> Result<(), TestError> {
    let run_ctx = RunCtx::new();
    let _obj = run_ctx.raise_objection("base_test stimulus");

    Clock::new(&ctx.dut().signal("clk")?, SimDuration::ns(10)).start();
    let bfm = Rc::new(TinyAluBfm::new(&ctx.dut())?);
    let mut scoreboard = Scoreboard::new(bfm.clone());
    bfm.reset().await;
    bfm.start_tasks();
    scoreboard.start_tasks();

    tester.execute(&bfm).await;
    let passed = scoreboard.check_results();

    drop(_obj);
    run_ctx.all_objections_dropped().await;

    if passed {
        Ok(())
    } else {
        Err(TestError::from("scoreboard saw failing comparisons"))
    }
}
```

This is Chapter 20's `execute_test` promoted to methodology: it raises an objection before stimulus, releases it when checking is done (the explicit `drop(_obj)` — we want the objection gone *before* awaiting the consensus), and returns `Result` instead of a boolean. The 2.0 classes themselves — `Tester`, `RandomTester`, `MaxTester`, `Scoreboard` — moved into `tinyalu_utils`, the graduation shared code earns; the chapter crate just imports them.

And the two tests are exactly as thin as pyuvm's:

```rust
// Figure 5: The tests build a tester and share base_test

#[rustdv::test]
async fn random_test(ctx: TestCtx) -> Result<(), TestError> {
    // Run with random operations
    base_test(&ctx, &mut RandomTester { rng: ctx.rng() }).await
}

#[rustdv::test]
async fn max_test(ctx: TestCtx) -> Result<(), TestError> {
    // Run with maximum operations
    base_test(&ctx, &mut MaxTester).await
}
```

Read these against pyuvm's figure 6. `RandomTest.build_phase()` set `self.tester = RandomTester()`; our `random_test` builds a `RandomTester` and passes it in. The *build phase* became *the argument list* — a sentence that Chapter 25 will spend a whole testbench version unpacking. One pyuvm subtlety got promoted from convention to law along the way: pyuvm's `BaseTest` was abstract only in the sense that nobody decorated it with `@pyuvm.test()`; nothing but discipline stopped a teammate from running it. Rust's `base_test` takes a `&mut impl Tester` argument, and a function with a required argument *cannot be a test* — the signature enforces its abstractness.

```text
# Figure 6: RandomTest passes
--
    145.00ns INFO     PASSED: ce Add 42 = 0110
    145.00ns INFO     PASSED: 2f And 64 = 0024
    145.00ns INFO     PASSED: 29 Xor b3 = 009a
    145.00ns INFO     PASSED: 86 Mul 83 = 4492
    145.00ns INFO     Covered all operations
```

```text
# Figure 7: max_test maxes all the operands
--
    290.00ns INFO     PASSED: ff Add ff = 01fe
    290.00ns INFO     PASSED: ff And ff = 00ff
    290.00ns INFO     PASSED: ff Xor ff = 0000
    290.00ns INFO     PASSED: ff Mul ff = fe01
    290.00ns INFO     Covered all operations
```

Same behavior as 2.0, now wearing the methodology's two first garments: objections deciding when the run ends, and a shared base with per-test variation at a designed point.

## Summary

Testbench 3.0 brought the UVM's test discipline to rustdv. `#[rustdv::test]` plays `@pyuvm.test()`; the test function plays `uvm_test` — constructing and owning the testbench rather than being conjured as `uvm_test_top` — and the base-class pattern became a shared `async fn` taking `&mut impl Tester`, with its abstractness enforced by its signature. Objections arrived as RAII guards: raise returns a guard, scope-exit drops it, `all_objections_dropped().await` is the end-of-run consensus, and the forgot-to-drop hang is unwritable. The pyuvm class tower mostly dissolved into language features, with each floor's forwarding address recorded for the chapters ahead.

The next chapter takes on the tower's load-bearing floor: `uvm_component`, phasing, and the question of how a testbench gets *structure* — where pyuvm built a runtime tree of parent-child references, and rustdv is about to claim the ownership tree was the component tree all along.
