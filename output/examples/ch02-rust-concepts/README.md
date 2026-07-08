# Chapter 2 — figures

Source: `book-pdf/src/chapter-02-*.md`

| Figure | Title | Behavior | File | How to run |
|---|---|---|---|---|
| 1 | Calling an undefined method | **compile error on purpose** (E0599) | `compile-fail/fig01_calling_undefined_method/src/main.rs` | `cd compile-fail/fig01_calling_undefined_method && cargo build` — expect E0599 |
| 2 | The corrected program | runs | `src/bin/ch02_fig02_corrected_program.rs` | `cargo run --bin ch02_fig02_corrected_program` |
| 3 | The mistake SystemVerilog would have allowed | **compile error on purpose** (E0308) | `compile-fail/fig03_mistake_systemverilog_would_have/src/main.rs` | `cd compile-fail/fig03_mistake_systemverilog_would_have && cargo build` — expect E0308 |
| 4 | The corrected program | runs | `src/bin/ch02_fig04_corrected_program.rs` | `cargo run --bin ch02_fig04_corrected_program` |
| 5 | The compiler as proofreader | **compile error on purpose** (E0425) | `compile-fail/fig05_compiler_proofreader/src/main.rs` | `cd compile-fail/fig05_compiler_proofreader && cargo build` — expect E0425 |
| 6 | Clippy teaching idiom | runs | `src/bin/ch02_fig06_clippy_teaching_idiom.rs` | `cargo run --bin ch02_fig06_clippy_teaching_idiom` |
