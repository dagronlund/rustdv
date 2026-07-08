# Chapter 5 — figures

Source: `book-pdf/src/chapter-05-*.md`

| Figure | Title | Behavior | File | How to run |
|---|---|---|---|---|
| 1 | Python assignment: two names, one object | Python contrast (see `python/`) | `python/fig01_python_assignment_two_names.py` | `python3 python/fig01_python_assignment_two_names.py` |
| 2 | Assignment moves — and the old name is gone | **compile error on purpose** (E0382) | `compile-fail/fig02_assignment_moves_old_name/src/main.rs` | `cd compile-fail/fig02_assignment_moves_old_name && cargo build` — expect E0382 |
| 3 | Values die at the closing brace — every time, on time | runs | `src/bin/ch05_fig03_values_die_closing_brace.rs` | `cargo run --bin ch05_fig03_values_die_closing_brace` |
| 4 | Copy types don't move — small values are simply copied | runs | `src/bin/ch05_fig04_copy_types_don_t.rs` | `cargo run --bin ch05_fig04_copy_types_don_t` |
| 5 | The monitor hands off a transaction — and learns what "hands off" means | **compile error on purpose** (E0382) | `compile-fail/fig05_monitor_hands_off_transaction/src/main.rs` | `cd compile-fail/fig05_monitor_hands_off_transaction && cargo build` — expect E0382 |
| 6 | The monitor keeps a copy — explicitly | runs | `src/bin/ch05_fig06_monitor_keeps_copy_explicitly.rs` | `cargo run --bin ch05_fig06_monitor_keeps_copy_explicitly` |
