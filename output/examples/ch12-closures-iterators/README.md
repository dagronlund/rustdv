# Chapter 12 — figures

Source: `book-pdf/src/chapter-12-*.md`

| Figure | Title | Behavior | File | How to run |
|---|---|---|---|---|
| 1 | A closure is an unnamed function in a variable | runs | `src/bin/ch12_fig01_closure_unnamed_function_variable.rs` | `cargo run --bin ch12_fig01_closure_unnamed_function_variable` |
| 2 | A closure that reads captures by shared borrow | runs | `src/bin/ch12_fig02_closure_that_reads_captures.rs` | `cargo run --bin ch12_fig02_closure_that_reads_captures` |
| 3 | A closure that mutates captures by exclusive borrow | runs | `src/bin/ch12_fig03_closure_that_mutates_captures.rs` | `cargo run --bin ch12_fig03_closure_that_mutates_captures` |
| 4 | A move closure takes ownership of its captures | **compile error on purpose** (E0382) | `compile-fail/fig04_move_closure_takes_ownership/src/main.rs` | `cd compile-fail/fig04_move_closure_takes_ownership && cargo build` — expect E0382 |
| 5 | The list comprehension, as an iterator chain | runs | `src/bin/ch12_fig05_list_comprehension_iterator_chain.rs` | `cargo run --bin ch12_fig05_list_comprehension_iterator_chain` |
| 6 | The dictionary comprehension, collected into a HashMap | runs | `src/bin/ch12_fig06_dictionary_comprehension_collected_into.rs` | `cargo run --bin ch12_fig06_dictionary_comprehension_collected_into` |
| 7 | fold reduces a stream to one value | runs | `src/bin/ch12_fig07_fold_reduces_stream_one.rs` | `cargo run --bin ch12_fig07_fold_reduces_stream_one` |
| 8 | The Fibonacci generator, as an Iterator implementation | runs | `src/bin/ch12_fig08_fibonacci_generator_iterator_implementation.rs` | `cargo run --bin ch12_fig08_fibonacci_generator_iterator_implementation` |
| 9 | A TinyALU operand-pair stream, replacing a generator function | runs | `src/bin/ch12_fig09_tinyalu_operand_pair_stream.rs` | `cargo run --bin ch12_fig09_tinyalu_operand_pair_stream` |
| 10 | A struct that carries its behavior as a closure | runs | `src/bin/ch12_fig10_struct_that_carries_behavior.rs` | `cargo run --bin ch12_fig10_struct_that_carries_behavior` |
