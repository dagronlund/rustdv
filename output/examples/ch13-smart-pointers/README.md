# Chapter 13 — figures

Source: `book-pdf/src/chapter-13-*.md`

| Figure | Title | Behavior | File | How to run |
|---|---|---|---|---|
| 1 | A Box owns its value on the heap — everything else is Chapter 5 | runs | `src/bin/ch13_fig01_box_owns_value_heap.rs` | `cargo run --bin ch13_fig01_box_owns_value_heap` |
| 2 | The refcount Python never showed you | Python contrast (see `python/`) | `python/fig02_refcount_python_never_showed.py` | `python3 python/fig02_refcount_python_never_showed.py` |
| 3 | Rc::clone increments a refcount — on purpose, where you can see it | runs | `src/bin/ch13_fig03_rc_clone_increments_refcount.rs` | `cargo run --bin ch13_fig03_rc_clone_increments_refcount` |
| 4 | Shared owners are readers — Rc will not hand out write access | **compile error on purpose** (E0594) | `compile-fail/fig04_shared_owners_are_readers/src/main.rs` | `cd compile-fail/fig04_shared_owners_are_readers && cargo build` — expect E0594 |
| 5 | Interior mutability — mutation through an immutable binding | runs | `src/bin/ch13_fig05_interior_mutability_mutation_through.rs` | `cargo run --bin ch13_fig05_interior_mutability_mutation_through` |
| 6 | The borrow checker at runtime — a panic replaces the compile error | runs, **panics on purpose** | `src/bin/ch13_fig06_borrow_checker_runtime_panic.rs` | `cargo run --bin ch13_fig06_borrow_checker_runtime_panic` |
| 7 | Chapter 5's Python experiment, finally legal in Rust — with its costs itemized | runs | `src/bin/ch13_fig07_chapter_5_s_python.rs` | `cargo run --bin ch13_fig07_chapter_5_s_python` |
