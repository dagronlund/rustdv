---
name: write-a-bfm
description: Use when writing a rustdv BFM (bus functional model) — the struct that owns a DUT's signal handles and speaks its pin protocol. Covers signal lookup, sampling/driving conventions, x/z handling during reset, edge detection, multi-cycle handshakes, and the end-of-stimulus drain. Getting these wrong produces "flaky scoreboard" symptoms.
---

# Write a rustdv BFM

One struct owns every pin. Nothing else in the testbench touches a signal.
The working reference is `rustdv/tinyalu_tb/src/alu_bfm.rs`.

## Shape

```rust
pub struct MyBfm {
    clk: LogicHandle,          // handles are Copy — cheap to move into tasks
    // ... every DUT pin, plus Queue<T>s feeding/fed by the loops
}

impl MyBfm {
    pub fn new(dut: &HierarchyHandle) -> Result<MyBfm, HandleError> {
        Ok(MyBfm { clk: dut.signal("clk")?, /* ... */ })
    }
}
```

- `dut.signal("name")?` fails at time zero with scope+name in the message —
  never unwrap-and-hope; propagate the `Result`.
- **Every method takes `&self`.** The BFM is shared as `Rc<MyBfm>`; interior
  state lives in `Queue<T>`s (already shareable). No `RefCell` needed.
- Free-running loops go in a `start_tasks(&self)` that `spawn_named`s each
  loop. Capture `Copy` handles and `Clone`d queues by `move`.

## Timing conventions (the craft)

1. **Sample and drive on falling edges** (`clk.falling_edge().await`) when
   the DUT acts on rising edges. Reads in the callback see pre-write
   values; `set_u64()`/`set()` writes are *scheduled* — applied at the
   simulator's read-write phase of the same timestep. Driving at the
   falling edge means the DUT samples stable values at the next rising.
2. **`set()` vs `set_now()`.** Protocol pins: scheduled `set_u64()`.
   Clocks/reset-style immediate forcing: `set_u64_now()`.
3. **x/z during reset.** `get_u64()` returns `Err` if any bit is x/z.
   Monitors must treat that as "not now, skip this cycle":
   ```rust
   let (Ok(a), Ok(b)) = (sig_a.get_u64(), sig_b.get_u64()) else { continue };
   ```
   For 1-bit pins, compare `get_binstr()` against `"0"`/`"1"` (x matches
   neither) or use `is_high()`/`is_low()`.
4. **Edge detection in a sampled loop** needs previous-value tracking:
   ```rust
   let now = start.is_high();
   if now && !prev { /* capture the command */ }
   prev = now;
   ```
5. **Multi-cycle handshake:** hold the request asserted until the DUT
   acknowledges; clear it in the state where both are high:
   ```rust
   match (start_high, done_high) {
       (false, false) => { /* idle: launch next queued command */ }
       (true,  true)  => start.set_u64(0),
       _ => {}
   }
   ```
6. **Reset:** drive inactive values, wait N falling edges, deassert, wait
   one more — then **flush the monitor queues** so reset-time garbage never
   reaches the scoreboard.
7. **Drain before checking.** Provide `wait_idle()`: loop on falling edges
   until the command queue is empty and the handshake is idle, then one
   extra edge so monitors flush. Tests call it after the sequence and
   before `run_extract_check_report`. Skipping this yields "orphaned
   command has no result" scoreboard errors on the last transaction.
8. **Never synchronize with `Timer`-and-hope.** Use queues and `Event`s;
   `NullTrigger` exists for parity but is a smell (design-doc §7.3).
