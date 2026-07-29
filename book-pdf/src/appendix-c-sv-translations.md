# Appendix C: SystemVerilog-UVM → rustdv Translations

For readers coming from SystemVerilog UVM (and *The UVM Primer*): where each piece of your working vocabulary went. Python readers want Appendix B, this table's twin.

## Language level

| SystemVerilog | Rust | Chapter |
|---|---|---|
| `byte`, `shortint`, `int` | `u8`/`i8`, `u16`/`i16`, `u32`/`i32` — no silent truncation | 3 |
| `logic` four-state values | `Logic` enum / `LogicArray` — no x in arithmetic | 7, 17 |
| `typedef enum` (an int in disguise) | `enum` — a real type; exhaustively matched | 7 |
| `case` + `default` (+ `unique` warnings) | `match` — missing cases are compile errors | 4 |
| `class ... extends`, `virtual`, `super.new()` | traits, default methods, composition + delegation | 10 |
| `pure virtual function` in a virtual class | a required trait method, checked at the `impl` | 10 |
| parameterized class `#(type T = int)` | generics `<T: Bound>`, checked at definition | 11 |
| `class ... #(type REQ, type RSP = REQ)` | `SeqItemPort<REQ, RSP = REQ>` — same convention | 11 |
| `local` / `protected` | private-by-default, `pub` to export | 14 |
| `null` handle, `$cast` | `Option<T>`, exhaustive `match` — no null, no cast | 9 |
| status flags and sentinel returns | `Result<T, E>` + `?` — failure in the signature | 9 |
| `$sformatf` | `format!` | 8 |
| `fork` / `join_none` / `disable` | `spawn(future)` → `TaskHandle`; `handle.cancel()` | 16 |
| `forever` | `loop` (an expression — it can `break` with a value) | 4 |
| `mailbox #(T)`, `try_put`/`try_get` | `sim::Queue<T>` — same names, `Result`/`Option` answers | 16 |
| named `event`, `->done`, `@(done)` | `sim::Event` — `set()` / `wait().await` | 16 |
| `semaphore` (one key) | `sim::Lock` — FIFO-fair, RAII guard | 16 |
| `@(posedge clk)`, `#2ns` | `clk.rising_edge().await`, `Timer::ns(2).await` | 15, 17 |
| `` `define ``-style codegen (`` `uvm_*_utils ``) | attribute + derive macros — syntax trees, not text | 21 |
| package + `.f` file + vendor tarball | crate + `Cargo.toml` + crates.io | 14 |
| *(no equivalent)* | `cargo test` — unit tests with no simulator | 14 |

## Methodology level

| SystemVerilog UVM | rustdv | Chapter |
|---|---|---|
| `class my_test extends uvm_test` + `run_test()` | the `#[rustdv::test]` fn owns the env | 23 |
| `phase.raise_objection(this)` / `drop_objection` | `ObjectionGuard` (RAII) + `all_objections_dropped().await` | 23 |
| `uvm_component(name, parent)` tree | children are struct fields; `#[derive(Component)]` | 24 |
| `build_phase` / `connect_phase` | constructors — building is what `new()` does | 24 |
| run/extract/check/report phases | `start` / `extract` / `check` / `report` / `final_phase` | 24 |
| `` `uvm_info(id, msg, verbosity) `` | `Logger` — same time/level/`[path]` line format | 26 |
| `set_report_verbosity_level_hier()` | `set_level_for(prefix)` — longest prefix wins | 26 |
| `uvm_config_db#(T)::set/get`, wildcards | typed config structs; fields route by nesting | 27 |
| virtual interface via config database | `Rc<TinyAluBfm>` in the config struct | 13, 27 |
| a failed `get()` (silent default) | unrepresentable — a missing field doesn't compile | 27, 28 |
| `type_id::create()` + `set_type_override_by_type` | maker closures in config structs | 29, 30 |
| `uvm_factory::get().print()` | no registry to print | 29 |
| TLM-1 put/get/peek port + export + `connect()` | `channel()` → `Sender<T>`/`Receiver<T>`, six methods | 31 |
| connection errors at elaboration | a compile error (E0308) at the construction site | 31 |
| `uvm_analysis_port.write()` | `AnalysisPort<T>::write(&T)` — non-blocking broadcast | 32 |
| `uvm_subscriber` (pure virtual `write`) | `trait Subscriber<T> { fn write(&mut self, &T); }` | 32 |
| `uvm_tlm_analysis_fifo` | `AnalysisBus<T>` via `ap.connect_fifo()` | 32 |
| `uvm_agent` + `is_active` | env plays the agent; `Active` enum + `Option<Driver>` | 34 |
| `do_copy` / `do_compare` / `convert2string` | `#[derive(Clone, PartialEq, Debug)]` + `Display` | 10, 35 |
| `uvm_field_*` macros (runtime field walking) | `derive` — the same generation, at compile time | 21, 35 |
| `uvm_sequence #(REQ, RSP)`, `body()` | `Sequence<REQ, RSP>` trait, boxed-future `body` | 36 |
| `start_item` / `finish_item` | `ctx.start_item(&mut req)` / `ctx.finish_item(req)?` | 36 |
| `seq_item_port.get_next_item()` / `item_done()` | `port.get_next_item().await` / `item_done(None)` | 36 |
| driver writes result through the `req` handle | `item_done(Some(rsp))` — an explicit response | 37 |
| `rsp.set_id_info(req)` + `get_response()` | auto-tagged envelope; `ctx.get_response(None or id)` | 37–38 |
| virtual sequence (no `p_sequencer` needed here) | plain struct + `async fn body`; no `SeqCtx` to misuse | 39 |
| sequencer grab/lock/priority arbitration | unported (FIFO only) — recorded gap | 39 |
| `assert` (prints, simulation continues) | `assert!` (panic = test fails, on the spot) | 9 |
| `uvm_error` vs `uvm_fatal` (convention) | `Result::Err` = DUT check; `panic!` = testbench bug | 9 |
