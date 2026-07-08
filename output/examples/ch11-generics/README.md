# Chapter 11 — figures

Source: `book-pdf/src/chapter-11-*.md`

| Figure | Title | Behavior | File | How to run |
|---|---|---|---|---|
| 1 | A generic function, first attempt — the compiler wants proof | **compile error on purpose** (E0369) | `compile-fail/fig01_generic_function_first_attempt/src/main.rs` | `cd compile-fail/fig01_generic_function_first_attempt && cargo build` — expect E0369 |
| 2 | The bound is the fix — and the documentation | runs | `src/bin/ch11_fig02_bound_fix_documentation.rs` | `cargo run --bin ch11_fig02_bound_fix_documentation` |
| 3 | Multiple bounds with a where clause | runs | `src/bin/ch11_fig03_multiple_bounds_where_clause.rs` | `cargo run --bin ch11_fig03_multiple_bounds_where_clause` |
| 4 | A generic struct — one definition, many widths | runs | `src/bin/ch11_fig04_generic_struct_one_definition.rs` | `cargo run --bin ch11_fig04_generic_struct_one_definition` |
| 5 | What the compiler generates from figure 2 (conceptually — you never see this) | fragment — shown for shape, not runnable | `fragments/fig05_what_compiler_generates_from.rs` | not runnable — illustrative shape only |
| 6 | The shape of rustvm's driver (preview — signatures only) | fragment — shown for shape, not runnable | `fragments/fig06_shape_rustvm_s_driver.rs` | not runnable — illustrative shape only |
