# Chapter 30: Variation-Point Testbench: 5.0

Testbench 4.0 had a flaw both earlier books flagged the moment it shipped: two tests needed two environments — `AluEnv<RandomTester>` and `AluEnv<MaxTester>` in our version, `RandomEnv` and `MaxEnv` before — even though the environments differed in exactly one component. Version 5.0 fixes it the way the factory always promised: **one environment**, with the difference carried in from the tests. In the UVM the carrier was a factory override; here it is Chapter 29's maker closure, doing its first day of real testbench work.

> **In the UVM...** we kept one `AluEnv` that created its tester through the factory — `base_tester::type_id::create("tester", this)`, `BaseTester.create("tester", self)` — and each test registered an override in `build_phase`: `set_type_override_by_type(BaseTester, RandomTester)`. Three lines that changed what the env built without the env knowing.

## The variation point, for real

The slot's contract and the maker type, exactly as Chapter 29 designed them, now sized for the TinyALU:

```rust
// Figure 1: The slot's contract, and the maker that fills it

/// What must be true of anything standing in the tester slot.
pub trait TesterCompLike: ComponentNode {}
impl<T: ComponentNode> TesterCompLike for T {}

/// A stored constructor: give it the BFM, get a tester component.
pub type TesterMaker = Box<dyn FnOnce(Rc<TinyAluBfm>) -> Box<dyn TesterCompLike>>;
```

One refinement over Chapter 29's toy: the maker *takes an argument*. A tester component cannot exist without its BFM, so the stored constructor's signature says so — `FnOnce(Rc<TinyAluBfm>) -> ...` — and the env, which owns the BFM, supplies it at the moment of creation. pyuvm's `create("tester", self)` passed name-and-parent to whatever the override table produced and trusted the ConfigDB to deliver everything else; the rustdv maker's parameter list *is* the delivery manifest.

```rust
// Figure 2: One environment with a designed variation point

pub struct AluEnvConfig {
    pub bfm: Rc<TinyAluBfm>,
    pub make_tester: TesterMaker,
}

pub struct AluEnv {
    tester: Box<dyn TesterCompLike>,
    scoreboard: Scoreboard,
}

impl AluEnv {
    pub fn new(config: AluEnvConfig) -> AluEnv {
        AluEnv {
            // The variation point: the env builds whatever the test sent.
            tester: (config.make_tester)(config.bfm.clone()),
            scoreboard: Scoreboard::new(config.bfm),
        }
    }
}
```

Compare with 4.0's env line by line. The generic parameter `<T: Tester>` is gone; the `tester` field is a `Box<dyn TesterCompLike>` — the env genuinely does not know, at compile time, what will stand there, which is the entire point. The scoreboard is untouched: variation points cost only the slots that vary. And the env is *closed*: nothing outside can change what it builds except through the config it declares, which is both the discipline (Chapter 29's honest ledger) and the reuse story — this env works for every TinyALU test anyone will ever write, because the thing tests want to change is exactly the thing its config exposes.

## Two tests, one environment

```rust
// Figure 4: random_test picks its tester with three visible lines

#[rustdv::test]
async fn random_test(ctx: TestCtx) -> Result<(), TestError> {
    // Run with random operands
    let rng = ctx.rng();
    run_test(
        &ctx,
        Box::new(move |bfm| Box::new(TesterComp::new(bfm, RandomTester { rng }))),
    )
    .await
}
```

```rust
// Figure 5: max_test differs only in the maker it sends

#[rustdv::test]
async fn max_test(ctx: TestCtx) -> Result<(), TestError> {
    // Run with maximum operands
    run_test(&ctx, Box::new(|bfm| Box::new(TesterComp::new(bfm, MaxTester)))).await
}
```

Set these beside pyuvm's 5.0 tests and the symmetry is exact: pyuvm's tests were `build_phase` plus one `set_type_override_by_type` line; ours are one maker expression. The `move` on `random_test`'s closure is Chapter 12 remembering its manners — the closure captures the seeded `rng` by value and carries it into the tester it will someday build. The shared `run_test` body (figure 3 in the chapter's example crate) is Chapter 25's skeleton with the maker threaded through; nothing else changed, and the transcript proves it:

```text
# Figure 6: One env, two behaviors
--
    145.00ns INFO     PASSED: c1 Add 67 = 0128
    145.00ns INFO     PASSED: 5e And 0b = 000a
    145.00ns INFO     PASSED: b9 Xor 80 = 0039
    145.00ns INFO     PASSED: a5 Mul 75 = 4b69
    145.00ns INFO     Covered all operations
    145.00ns INFO     random_test PASSED
    290.00ns INFO     PASSED: ff Add ff = 01fe
    290.00ns INFO     PASSED: ff And ff = 00ff
    290.00ns INFO     PASSED: ff Xor ff = 0000
    290.00ns INFO     PASSED: ff Mul ff = fe01
    290.00ns INFO     Covered all operations
    290.00ns INFO     max_test PASSED
```

Identical results to 4.0 — same seed, same operands, down to the hex — from half the environment code.

## Which tool, when

Part IV has now shown two ways to make one env serve many tests, and a word on choosing is owed. **Generics** (`AluEnv<T>`, testbench 4.0) resolve the variation at compile time: zero dispatch cost, full inlining, and the set of variants is closed — each is a distinct type. **Maker closures** (testbench 5.0) resolve it at construction time: one env type, open to any conforming substitute, at the price of a vtable call nobody will ever measure. The rule of thumb the rest of the book follows: when the *test* is the thing choosing, and choice is the feature — use the closure in the config; when a component is generic over its transaction or port types as an internal matter — use generics. And keep Chapter 29's spoiler in mind: the most common per-test variation of all, *what stimulus runs*, will shortly need neither, because sequences (Chapter 36) are plain values the test starts directly. The factory's dominion shrinks to the cases where testbench *structure* truly varies — which is why this chapter's pattern, though load-bearing, appears in real testbenches less often than a SystemVerilog veteran would guess.

## Summary

Testbench 5.0 collapsed 4.0's parallel environments into one `AluEnv` with a designed variation point: a `TesterMaker` closure in the config, typed to receive the BFM and return anything satisfying the slot's trait. The env builds whatever the test sent; the tests differ by one expression; the override is visible at the construction site, checked by the compiler, and incapable of leaking between tests through a global table. The same demonstration as pyuvm's 5.0 — same version number, same one-env victory — with the factory's job done by values.

The env's two components still talk to the DUT through one shared BFM, though, and the scoreboard still hoards `get_cmd()` — the very problem Chapter 22 flagged. Standard component-to-component communication is the next stop: channels, and the two-type answer to TLM's thirty classes.
