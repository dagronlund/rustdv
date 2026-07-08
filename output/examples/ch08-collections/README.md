# Chapter 8 — figures

Source: `book-pdf/src/chapter-08-*.md`

| Figure | Title | Behavior | File | How to run |
|---|---|---|---|---|
| 1 | A Vec<AluCommand> as a transaction history log | runs | `src/bin/ch08_fig01_vec_alucommand_transaction_history.rs` | `cargo run --bin ch08_fig01_vec_alucommand_transaction_history` |
| 2 | Pushing is a move | **compile error on purpose** (E0382) | `compile-fail/fig02_pushing_move/src/main.rs` | `cd compile-fail/fig02_pushing_move && cargo build` — expect E0382 |
| 3 | A for loop can consume the collection | **compile error on purpose** (E0382) | `compile-fail/fig03_loop_can_consume_collection/src/main.rs` | `cd compile-fail/fig03_loop_can_consume_collection && cargo build` — expect E0382 |
| 4 | Borrowing iteration leaves the Vec intact | runs | `src/bin/ch08_fig04_borrowing_iteration_leaves_vec.rs` | `cargo run --bin ch08_fig04_borrowing_iteration_leaves_vec` |
| 5 | String literals are borrowed; String is owned | runs | `src/bin/ch08_fig05_string_literals_are_borrowed.rs` | `cargo run --bin ch08_fig05_string_literals_are_borrowed` |
| 6 | Take &str; accept everything | runs | `src/bin/ch08_fig06_take_str_accept_everything.rs` | `cargo run --bin ch08_fig06_take_str_accept_everything` |
| 7 | A HashMap<Ops, u32> op-frequency counter | runs | `src/bin/ch08_fig07_hashmap_ops_u32_op.rs` | `cargo run --bin ch08_fig07_hashmap_ops_u32_op` |
