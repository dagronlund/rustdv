# Chapter 9 — figures

Source: `book-pdf/src/chapter-09-*.md`

| Figure | Title | Behavior | File | How to run |
|---|---|---|---|---|
| 1 | A HashMap lookup returns Option | runs | `src/bin/ch09_fig01_hashmap_lookup_returns_option.rs` | `cargo run --bin ch09_fig01_hashmap_lookup_returns_option` |
| 2 | You still can't divide by zero | runs, **panics on purpose** | `src/bin/ch09_fig02_still_can_t_divide.rs` | `cargo run --bin ch09_fig02_still_can_t_divide` |
| 3 | nice_div returns a Result instead of raising | runs | `src/bin/ch09_fig03_nice_div_returns_result.rs` | `cargo run --bin ch09_fig03_nice_div_returns_result` |
| 4 | The TypeError scenario, ported to Rust | **compile error on purpose** (E0308) | `compile-fail/fig04_typeerror_scenario_ported_rust/src/main.rs` | `cd compile-fail/fig04_typeerror_scenario_ported_rust && cargo build` — expect E0308 |
| 5 | The ? operator sends the error up the stack | runs | `src/bin/ch09_fig05_operator_sends_error_up.rs` | `cargo run --bin ch09_fig05_operator_sends_error_up` |
| 6 | A custom error enum for the TinyALU | runs | `src/bin/ch09_fig06_custom_error_enum_tinyalu.rs` | `cargo run --bin ch09_fig06_custom_error_enum_tinyalu` |
| 7 | assert! guards an invariant | runs, **panics on purpose** | `src/bin/ch09_fig07_assert_guards_invariant.rs` | `cargo run --bin ch09_fig07_assert_guards_invariant` |
