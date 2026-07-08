# Chapter 7 — figures

Source: `book-pdf/src/chapter-07-*.md`

| Figure | Title | Behavior | File | How to run |
|---|---|---|---|---|
| 1 | Defining and instantiating a struct | runs | `src/bin/ch07_fig01_defining_instantiating_struct.rs` | `cargo run --bin ch07_fig01_defining_instantiating_struct` |
| 2 | Forgetting a field is now a compile error | **compile error on purpose** (E0063) | `compile-fail/fig02_forgetting_field_now_compile/src/main.rs` | `cd compile-fail/fig02_forgetting_field_now_compile && cargo build` — expect E0063 |
| 3 | A method in an impl block | runs | `src/bin/ch07_fig03_method_impl_block.rs` | `cargo run --bin ch07_fig03_method_impl_block` |
| 4 | A method that mutates takes &mut self | runs | `src/bin/ch07_fig04_method_that_mutates_takes.rs` | `cargo run --bin ch07_fig04_method_that_mutates_takes` |
| 5 | The new() associated function | runs | `src/bin/ch07_fig05_new_associated_function.rs` | `cargo run --bin ch07_fig05_new_associated_function` |
| 6 | Associated constants and functions replace class variables and static methods | runs | `src/bin/ch07_fig06_associated_constants_functions_replace.rs` | `cargo run --bin ch07_fig06_associated_constants_functions_replace` |
| 7 | The Ops enumeration and an exhaustive match | runs | `src/bin/ch07_fig07_ops_enumeration_exhaustive_match.rs` | `cargo run --bin ch07_fig07_ops_enumeration_exhaustive_match` |
| 8 | The compiler finds every match the new variant breaks | **compile error on purpose** (E0004) | `compile-fail/fig08_compiler_finds_every_match/src/main.rs` | `cd compile-fail/fig08_compiler_finds_every_match && cargo build` — expect E0004 |
| 9 | A four-state Logic enum | runs | `src/bin/ch07_fig09_four_state_logic_enum.rs` | `cargo run --bin ch07_fig09_four_state_logic_enum` |
| 10 | An enum whose variants carry payloads | runs | `src/bin/ch07_fig10_enum_whose_variants_carry.rs` | `cargo run --bin ch07_fig10_enum_whose_variants_carry` |
