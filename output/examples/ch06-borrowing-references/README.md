# Chapter 6 — figures

Source: `book-pdf/src/chapter-06-*.md`

| Figure | Title | Behavior | File | How to run |
|---|---|---|---|---|
| 1 | Shared references — everyone may look, nobody may touch | runs | `src/bin/ch06_fig01_shared_references_everyone_may.rs` | `cargo run --bin ch06_fig01_shared_references_everyone_may` |
| 2 | An exclusive reference grants write access | runs | `src/bin/ch06_fig02_exclusive_reference_grants_write.rs` | `cargo run --bin ch06_fig02_exclusive_reference_grants_write` |
| 3 | Two tasks' worth of access to one value -- the borrow checker objects | **compile error on purpose** (E0502) | `compile-fail/fig03_two_tasks_worth_access/src/main.rs` | `cd compile-fail/fig03_two_tasks_worth_access && cargo build` — expect E0502 |
| 4 | The same actors, with the ordering made real | runs | `src/bin/ch06_fig04_same_actors_ordering_made.rs` | `cargo run --bin ch06_fig04_same_actors_ordering_made` |
