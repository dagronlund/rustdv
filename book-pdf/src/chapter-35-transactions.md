# Chapter 35: Transactions: The uvm_object Problem, Solved by std Derives

The 6.0 testbench passes `(u64, u64, u64)` tuples, and everyone is tired of remembering that `op` is the thing at index 2. The UVM's answer was `uvm_object`: named-field transaction classes with standard copy, compare, and print machinery. This chapter gives the TinyALU its real transactions — and the porting story is the shortest in Part IV, because everything `uvm_object` labored to provide, the language derives.

> **In the UVM...** we extended `uvm_sequence_item`, and got the machinery of `uvm_object`: `clone()` backed by a `do_copy()` that walked the fields; equality backed by `do_compare()`; printing via `convert2string()`, which we overrode by hand for every class; plus `get_name()`, IDs for request/response matching, and the long tail — pack, unpack, record — that the specification demands and most testbenches quietly ignore.

## The transactions

```rust
// Figure 1: The TinyALU transactions, final form

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Ops {
    Add = 1,
    And = 2,
    Xor = 3,
    Mul = 4,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AluCommand {
    pub a: u8,
    pub b: u8,
    pub op: Ops,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AluResult {
    pub result: u16,
}
```

The Interlude showed you these; now you own every line. Plain structs — no base class, no trait from rustdv, nothing to extend — because there is nothing left for a base class to do:

```rust
// Figure 2: Three derives, three dunder families

fn main() {
    let cmd = AluCommand { a: 0xA5, b: 0x75, op: Ops::Mul };

    // Clone is do_copy: a field-wise deep copy
    let copy = cmd.clone();

    // PartialEq is do_compare: field-wise equality
    println!("copy == cmd: {}", copy == cmd);

    // Debug is convert2string: a readable rendering, for free
    println!("{cmd:?}");

    // ...and mutation of the copy proves it was a copy
    let mut tweaked = cmd.clone();
    tweaked.a = 0;
    println!("tweaked == cmd: {}", tweaked == cmd);
}
```

```text
--
copy == cmd: true
AluCommand { a: 165, b: 117, op: Mul }
tweaked == cmd: false
```

The classic table maps `uvm_object`'s methods to their Python dunders. Here it is with its third column, which is mostly one word:

*Figure 3: The uvm_object surface, dispositioned*

| UVM method | Python (pyuvm) | Rust (rustdv) |
|---|---|---|
| `clone()`/`do_copy()` | `__deepcopy__` / override | `#[derive(Clone)]` |
| `compare()`/`do_compare` | `__eq__` / override | `#[derive(PartialEq)]` |
| `convert2string()` | `__str__` / override | `#[derive(Debug)]` |
| `print()` | `print(obj)` | `println!("{obj:?}")` |
| `get_name()` | stored name string | `std::any::type_name` / none needed |
| `get_inst_id()` | `id(self)` | no identity on data (see below) |
| `pack()`/`unpack()` | raised `UVMNotImplemented` | not ported (same cut) |
| `record()` | stub | not ported (same cut) |

Two rows repay a closer look. The derives are not conveniences over the pyuvm way — they are the *same technique moved to compile time*. pyuvm's `do_copy` walked `self.__dict__` at runtime to copy whatever fields it found; `#[derive(Clone)]` walks the field list at compile time and emits exactly the member-wise code you'd write by hand (Chapter 21's derive story). Add a field to `AluCommand` and copy, compare, and print all update themselves — the field-macros problem SystemVerilog solved with `\`uvm_field_int` and pyuvm solved with reflection, solved a third way, with no runtime cost and no macros in *your* code. And the bottom rows record an agreement across all three books: pack, unpack, and recording were stubs in pyuvm (`UVMNotImplemented`), and rustdv makes the same cut on the same reasoning.

## Where comparison policy went

One pyuvm capability looks missing: overriding `do_compare` so that "equal" ignores some fields — a timestamp, a don't-care flag. rustdv's position: *comparison policy is checker policy, not data-type property* — bake one DUT's notion of matching into the type and every other user of that type inherits it silently. So the scoreboard takes the policy as a value:

```rust
// Figure 4: Comparison policy lives in the checker

#[derive(Clone, Debug, PartialEq)]
pub struct BusResult {
    pub data: u16,
    pub timestamp_ns: u64, // interesting to the log, irrelevant to correctness
}

/// A scoreboard that takes its notion of "matches" as a value.
fn check(expected: &BusResult, actual: &BusResult,
         matches: impl Fn(&BusResult, &BusResult) -> bool) {
    if matches(expected, actual) {
        println!("PASSED: {actual:?}");
    } else {
        println!("FAILED: {actual:?} - expected {expected:?}");
    }
}

fn main() {
    let expected = BusResult { data: 0x4B69, timestamp_ns: 100 };
    let actual = BusResult { data: 0x4B69, timestamp_ns: 130 };

    // Structural equality says no — the timestamps differ:
    check(&expected, &actual, |e, a| e == a);

    // The policy this DUT needs: compare the data, ignore the clock.
    check(&expected, &actual, |e, a| e.data == a.data);
}
```

```text
--
FAILED: BusResult { data: 19305, timestamp_ns: 130 } - expected BusResult { data: 19305, timestamp_ns: 100 }
PASSED: BusResult { data: 19305, timestamp_ns: 130 }
```

`PartialEq` remains the default policy — the TinyALU scoreboard uses `expected != actual` and never thinks about it — and a closure carries the exceptions, visibly, in the checker that owns them. What pyuvm spelled as a `do_compare` override plus field-exclusion conventions is here two lines of `|e, a|`, local to the one scoreboard that wants them.

## Where identity went

The other survivor of the `uvm_sequence_item` lineage is *identity*: matching responses to requests. pyuvm stored transaction IDs — and, tellingly, the sequence-synchronization events — on the transaction itself, which meant any code holding an item could fiddle with machinery that rightly belonged to the sequencer. rustdv relocates identity into an envelope the infrastructure owns:

```rust
// Figure 5: Identity belongs to the envelope, not the data

pub struct SeqItem<REQ> { /* txn id + payload */ }

impl<REQ> SeqItem<REQ> {
    pub fn txn_id(&self) -> TxnId;
    pub fn payload(&self) -> &REQ;
    pub fn payload_mut(&mut self) -> &mut REQ;
}
```

Your `AluCommand` stays plain data; the sequencer wraps it in a `SeqItem` on its way to the driver, the id travels in the wrapper, and responses get tagged automatically — pyuvm's `set_context()` call, which you could forget, has no equivalent because there is nothing to remember. The envelope is Chapter 36's on-ramp, and we will watch it work there.

## Summary

`uvm_object` ported as three derives on a plain struct: `Clone` is `do_copy`, `PartialEq` is `do_compare`, `Debug` is `convert2string`, each generated from the field list at compile time — reflection's job, done by the language, updating itself when fields change. Comparison policy moved out of the data type and into the scoreboard as a closure, with structural equality as the undemanding default; transaction identity moved out of the data type and into the `SeqItem` envelope the sequencer owns; and pack/unpack/record stay unported, a cut all three books agree on. The TinyALU's tuples are dead; `AluCommand` and `AluResult` take their places in every remaining chapter.

Which clears the runway. Stimulus-as-data, late generation, the driver handshake — the sequence machinery is the UVM's crown, pyuvm ported it event for event, and rustdv does the same. Testbench 7.0.
