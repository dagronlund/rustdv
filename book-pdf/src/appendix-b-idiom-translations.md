# Appendix B: Python → Rust Idiom Translations

For readers coming from cocotb and pyuvm (and *Python for RTL Verification*): the working translations this book used, gathered for reference. SystemVerilog readers want Appendix C, this table's twin. Legend: **[C]** cocotb, **[P]** pyuvm.

## Language and runtime

| Python | Rust | Chapter |
|---|---|---|
| `async def` coroutine, resumed via `send(None)` | `async fn` → `Future`, resumed via `poll()` | 15 |
| `@cocotb.test()` | `#[rustdv::test]` | 15, 21 |
| exceptions fail the test | `Result<(), TestError>`; panics = testbench bugs | 9, 18 |
| `cocotb.start_soon(coro)` | `spawn(future) -> TaskHandle<T>` | 16 |
| `await task` | `task.await` → `Result<T, TaskError>` | 16 |
| `task.kill()` | `handle.cancel()` — the future is dropped; cleanup in `Drop` | 16 |
| `Combine(...)` / `First(...)` | `join2`/`join!` / `first2`/`first!` | 16 |
| `try/except QueueFull` | `try_send` → `Result<(), TlmFull<T>>` (item returned) | 16, 31 |
| decorator registration at import time | link-section registration at compile time | 21 |
| metaclass class registration | not needed — constructor injection | 21, 29 |
| `getattr(obj, name)` dispatch | pass the function/closure itself | 33 |
| `logging` levels + handlers | `log::` levels, `set_level_for(prefix)`, `log_to_file` | 26 |

## cocotb layer

| Python (cocotb) | Rust (rustdv-sim) | Chapter |
|---|---|---|
| `Timer(2, units="ns")` | `Timer::ns(2).await` | 15 |
| `RisingEdge(sig)` / `FallingEdge(sig)` | `sig.rising_edge().await` / `sig.falling_edge().await` | 17 |
| `ClockCycles(clk, n)` | `for _ in 0..n { clk.rising_edge().await; }` | 17 |
| `dut.sig` attribute magic | `dut.signal("sig")?` → `Result<LogicHandle, _>` | 17 |
| `sig.value = x` / `int(sig.value)` | `sig.set_u64(x)` / `sig.get_u64()?` | 17 |
| `Clock(dut.clk, 10, units="ns").start()` | `Clock::new(&clk, SimDuration::ns(10)).start()` | 17 |
| `cocotb.queue.Queue(maxsize=1)` | `Queue::new(Some(1))`; `Queue::unbounded()` | 16 |
| `Event` / `Lock` | `sim::Event` / `sim::Lock` (FIFO-fair, RAII guard) | 16 |

## pyuvm layer

| Python (pyuvm) | Rust (rustdv) | Chapter |
|---|---|---|
| `uvm_test` class, `uvm_test_top` | the `#[rustdv::test]` fn owns the env | 23 |
| `raise_objection()`/`drop_objection()` | `ObjectionGuard` (RAII) + `all_objections_dropped().await` | 23 |
| `uvm_component(name, parent)` tree | children are struct fields; `#[derive(Component)]` | 24 |
| nine phases | constructors (build/connect) + `start`/`extract`/`check`/`report`/`final_phase` | 24 |
| `self.logger`, `[uvm_test_top.comp]` | `Logger::new("path")`, same bracket format | 26 |
| `ConfigDB().set/get`, wildcards, globals | typed config structs; repeated fields; `..Default::default()` | 27 |
| `create()` + `set_type_override_by_type` | maker closures in config structs | 29 |
| TLM-1 put/get/peek port classes | `channel()` → `Sender<T>`/`Receiver<T>`, six methods | 31 |
| `UVMTLMConnectionError` | a compile error (E0308) at the construction site | 31 |
| `uvm_analysis_port.write()` | `AnalysisPort<T>::write(&T)` — non-blocking broadcast | 32 |
| `uvm_subscriber` | `trait Subscriber<T> { fn write(&mut self, &T); }` | 32 |
| `uvm_tlm_analysis_fifo` | `AnalysisBus<T>` via `ap.connect_fifo()` | 32 |
| `uvm_object` do_copy/do_compare/convert2string | `#[derive(Clone, PartialEq, Debug)]` | 35 |
| `do_compare` overrides | a comparator closure, owned by the scoreboard | 35 |
| `uvm_sequence.body()` | `Sequence<REQ, RSP>` trait, boxed-future `body` | 36 |
| `start_item`/`finish_item` | `ctx.start_item(&mut req)` / `ctx.finish_item(req)? -> TxnId` | 36 |
| `seq_item_port.get_next_item()` | `port.get_next_item().await -> SeqItem<REQ>` | 36 |
| `item_done()` / `item_done(rsp)` + `set_id_info` | `item_done(None)` / `item_done(Some(rsp))` — auto-tagged | 36–38 |
| `get_response()` | `ctx.get_response(None)` FIFO / `Some(id)` cherry-pick | 37–38 |
| virtual sequence (no sequencer `start()`) | plain struct + `async fn body`; no `SeqCtx` to misuse | 39 |
| `is_active` from ConfigDB | `Active` enum + `Option<Driver>` children | 34 |
