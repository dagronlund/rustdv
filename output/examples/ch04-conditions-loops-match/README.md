# Chapter 4 — figures

Source: `book-pdf/src/chapter-04-*.md`

| Figure | Title | Behavior | File | How to run |
|---|---|---|---|---|
| 1 | A Rust if statement | runs | `src/bin/ch04_fig01_rust_if_statement.rs` | `cargo run --bin ch04_fig01_rust_if_statement` |
| 2 | Rust has no truthiness | **compile error on purpose** (E0308) | `compile-fail/fig02_rust_has_no_truthiness/src/main.rs` | `cd compile-fail/fig02_rust_has_no_truthiness && cargo build` — expect E0308 |
| 3 | else if as a switch (for now) | runs | `src/bin/ch04_fig03_else_if_switch_now.rs` | `cargo run --bin ch04_fig03_else_if_switch_now` |
| 4 | Conditional assignment — no ternary needed | runs | `src/bin/ch04_fig04_conditional_assignment_no_ternary.rs` | `cargo run --bin ch04_fig04_conditional_assignment_no_ternary` |
| 5 | A while loop in action | runs | `src/bin/ch04_fig05_while_loop_action.rs` | `cargo run --bin ch04_fig05_while_loop_action` |
| 6 | loop — the intentional infinite loop | runs | `src/bin/ch04_fig06_loop_intentional_infinite_loop.rs` | `cargo run --bin ch04_fig06_loop_intentional_infinite_loop` |
| 7 | Looping through numbers using a range | runs | `src/bin/ch04_fig07_looping_through_numbers_using.rs` | `cargo run --bin ch04_fig07_looping_through_numbers_using` |
| 8 | Stepping through a range | runs | `src/bin/ch04_fig08_stepping_through_range.rs` | `cargo run --bin ch04_fig08_stepping_through_range` |
| 9 | match as a switch | runs | `src/bin/ch04_fig09_match_switch.rs` | `cargo run --bin ch04_fig09_match_switch` |
| 10 | The compiler catches missing cases | **compile error on purpose** (E0004) | `compile-fail/fig10_compiler_catches_missing_cases/src/main.rs` | `cd compile-fail/fig10_compiler_catches_missing_cases && cargo build` — expect E0004 |
| 11 | Matching on ranges | runs | `src/bin/ch04_fig11_matching_ranges.rs` | `cargo run --bin ch04_fig11_matching_ranges` |
| 12 | Matching and destructuring a tuple | runs | `src/bin/ch04_fig12_matching_destructuring_tuple.rs` | `cargo run --bin ch04_fig12_matching_destructuring_tuple` |
