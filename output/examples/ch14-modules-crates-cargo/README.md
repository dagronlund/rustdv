# Chapter 14 — figures

Source: `book-pdf/src/chapter-14-*.md`

| Figure | Title | Behavior | File | How to run |
|---|---|---|---|---|
| 1 | A module declared inline, in the middle of main.rs | runs | `src/bin/ch14_fig01_module_declared_inline_middle.rs` | `cargo run --bin ch14_fig01_module_declared_inline_middle` |
| 2 | use brings names into scope, like Python's from-import | runs | `src/bin/ch14_fig02_use_brings_names_into.rs` | `cargo run --bin ch14_fig02_use_brings_names_into` |
| 3 | Private by default — the underscore convention, enforced | **compile error on purpose** (E0603) | `compile-fail/fig03_private_default_underscore_convention/src/main.rs` | `cd compile-fail/fig03_private_default_underscore_convention && cargo build` — expect E0603 |
| 4 | The file-to-module mapping | shell transcript (see below) | `(chapter README)` | shell transcript |
| 5 | Adding a dependency with cargo add | shell transcript (see below) | `(chapter README)` | shell transcript |
| 6 | Unit tests live beside the code they test | unit tests (`cargo test`) | `src/lib.rs` | `cargo test -p ch14_modules_crates_cargo` |
| 7 | Running the unit tests | shell transcript (see below) | `(chapter README)` | shell transcript |
| 8 | A failing test names the culprit | shell transcript (see below) | `(chapter README)` | shell transcript |

## Shell-transcript figures

### Figure 4: The file-to-module mapping

```text

alu_playground/
├── Cargo.toml
└── src/
    ├── main.rs          <-- contains the line: mod predictor;
    └── predictor.rs     <-- the body of the module: Ops, alu_prediction
--
% cargo run
   Compiling alu_playground v0.1.0
    Finished `dev` profile
     Running `target/debug/alu_playground`
AND: 0x0030
XOR: 0x00cc
```

### Figure 5: Adding a dependency with cargo add

```text

% cargo add rand
    Updating crates.io index
      Adding rand v0.9.1 to dependencies
--
# Cargo.toml, after:

[package]
name = "alu_playground"
version = "0.1.0"
edition = "2024"

[dependencies]
rand = "0.9.1"
```

### Figure 7: Running the unit tests

```text

% cargo test
   Compiling alu_playground v0.1.0
    Finished `test` profile [unoptimized + debuginfo]
     Running unittests src/main.rs
--
running 4 tests
test predictor::tests::add_carries_into_bit_eight ... ok
test predictor::tests::and_masks_operands ... ok
test predictor::tests::mul_needs_the_full_result_bus ... ok
test predictor::tests::xor_finds_differing_bits ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### Figure 8: A failing test names the culprit

```text

% cargo test
--
running 4 tests
test predictor::tests::add_carries_into_bit_eight ... FAILED
test predictor::tests::and_masks_operands ... ok
test predictor::tests::mul_needs_the_full_result_bus ... ok
test predictor::tests::xor_finds_differing_bits ... ok

failures:

---- predictor::tests::add_carries_into_bit_eight stdout ----
assertion `left == right` failed
  left: 254
 right: 510

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured
```
