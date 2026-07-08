# Chapter 3 — figures

Source: `book-pdf/src/chapter-03-*.md`

| Figure | Title | Behavior | File | How to run |
|---|---|---|---|---|
| 1 | A playground for this chapter | shell transcript (see below) | `(chapter README)` | shell transcript |
| 2 | Assigning twice to an immutable binding | **compile error on purpose** (E0384) | `compile-fail/fig02_assigning_twice_immutable_binding/src/main.rs` | `cd compile-fail/fig02_assigning_twice_immutable_binding && cargo build` — expect E0384 |
| 3 | A mutable binding | runs | `src/bin/ch03_fig03_mutable_binding.rs` | `cargo run --bin ch03_fig03_mutable_binding` |
| 4 | The TinyALU's A leg really is a u8 | **compile error on purpose** | `compile-fail/fig04_tinyalu_s_leg_really/src/main.rs` | `cd compile-fail/fig04_tinyalu_s_leg_really && cargo build` — expect error |
| 5 | A float in operations means... a compile error | **compile error on purpose** (E0277) | `compile-fail/fig05_float_operations_means_compile/src/main.rs` | `cd compile-fail/fig05_float_operations_means_compile && cargo build` — expect E0277 |
| 6 | The same figure, with the conversions made explicit | runs | `src/bin/ch03_fig06_same_figure_conversions_made.rs` | `cargo run --bin ch03_fig06_same_figure_conversions_made` |
| 7 | Augmented assignments — the type never changes | runs | `src/bin/ch03_fig07_augmented_assignments_type_never.rs` | `cargo run --bin ch03_fig07_augmented_assignments_type_never` |
| 8 | Creating a number from a string, by shadowing | runs | `src/bin/ch03_fig08_creating_number_from_string.rs` | `cargo run --bin ch03_fig08_creating_number_from_string` |
| 9 | Format strings, next to the f-strings you know | runs | `src/bin/ch03_fig09_format_strings_next_f.rs` | `cargo run --bin ch03_fig09_format_strings_next_f` |
| 10 | if is an expression | runs | `src/bin/ch03_fig10_if_expression.rs` | `cargo run --bin ch03_fig10_if_expression` |
| 11 | A block is an expression; the semicolon is the switch | runs | `src/bin/ch03_fig11_block_expression_semicolon_switch.rs` | `cargo run --bin ch03_fig11_block_expression_semicolon_switch` |
| 12 | A TinyALU prediction function | runs | `src/bin/ch03_fig12_tinyalu_prediction_function.rs` | `cargo run --bin ch03_fig12_tinyalu_prediction_function` |

## Shell-transcript figures

### Figure 1: A playground for this chapter

```text

% cargo new basics
    Creating binary (application) `basics` package
```
