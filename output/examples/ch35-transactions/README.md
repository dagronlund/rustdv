# Chapter 35: Transactions — figure map

Not a simulation chapter. Each figure is a standalone binary, like Part I:

```
cargo run --bin ch35_fig01_two_string_forms
```

| Figure | Title | Where |
|---|---|---|
| 1 | A transaction is a plain struct with two string forms | `src/bin/ch35_fig01_two_string_forms.rs` |
| 2 | You decide what "the same" means | `src/bin/ch35_fig02_you_decide_what_same_means.rs` |
| 3 | Why equality comes in two traits | `src/bin/ch35_fig03_partialeq_versus_eq.rs` |
| 4 | Clone is deep for what you own | `src/bin/ch35_fig04_clone_is_deep_for_what_you_own.rs` |
| 5 | If you want the shallow copy, you ask for it | `src/bin/ch35_fig05_sharing_is_asked_for.rs` |
| 6 | `clone_from` is the UVM's `copy()` | `src/bin/ch35_fig06_clone_from_is_uvm_copy.rs` |
| 7 | The TinyALU transactions, final form | `src/bin/ch35_fig07_the_tinyalu_transactions.rs` |

Every figure carries its expected output in a header comment, checked against a
real run. **Every figure is runnable** — the previous version of this chapter
had two of its five figures marked "fragment", code printed in the book with
nothing behind it.

## Structure

This follows the Python book's *uvm_object in Python*, which teaches the four
transaction operations on deliberately non-TinyALU objects: a person with an
ID, and a student who is a person plus a list of grades. That choice does work
— the grades list is the field that makes shallow-versus-deep *visible*, and
three scalars cannot show it.

The TinyALU transactions arrive last, as the thing the reader now knows how to
write, rather than first as a definition to accept.

## The four operations, and where they land

| UVM / pyuvm | rustdv |
|---|---|
| `convert2string()` / `__str__()` | `impl Display` — **written by hand** |
| `compare()` / `do_compare()` / `__eq__()` | `#[derive(PartialEq)]`, or hand-written when "same" is a choice |
| `copy(other)` | `Clone::clone_from(&mut self, source)` |
| `clone()` | `Clone::clone(&self)` |
| field-by-field print | `#[derive(Debug)]` — free |

## What this chapter proves

- **`Debug` and `Display` are different jobs.** `Debug` is the developer dump
  and comes free; `Display` is the readable form and must be written, because
  only the author knows which fields matter. Figure 1.
- **"The same" is a decision.** The derive compares every field; the UVM makes
  you write `do_compare()` for exactly the cases where that is wrong. Batman
  equals Bruce Wayne when only the ID counts. Figure 2.
- **Equality comes in two traits, for a reason a verification engineer meets.**
  `PartialEq` does not promise `a == a`, because IEEE 754 says NaN equals
  nothing. `Eq` adds that promise, and `HashSet` requires it — so a transaction
  carrying a measured delay cannot be a coverage key. Figure 3.
- **Shallow versus deep is settled by the type, not by the call.** Python needs
  `copy.copy()` and `copy.deepcopy()` because sharing is the default. Rust has
  one `clone()`: owned data is copied (Figure 4), and sharing has to be asked
  for with `Rc` (Figure 5).

## What is deliberately *not* here

An earlier version of this chapter moved comparison policy out of the
transaction and into the checker, as a closure passed to a compare function.
It is expressive, and it is wrong for this book. The UVM does not pass
behaviour around for equality and neither does `__eq__`; a reader meeting
Rust and the UVM at the same time would learn a pattern from neither. Equality
belongs on the transaction, as `do_compare()` puts it there — derived when
every field counts, hand-written when "the same" means something narrower
(Figure 2).

## What has no counterpart

`super().do_copy(other)` — the UVM's discipline of copying up an inheritance
chain, and the "always call super first" rule with it. Rust has no inheritance;
the derive walks the field list, so there is no first step to forget.

One false friend: Rust's `Copy` trait is unrelated to the UVM's `copy()`.
`Copy` is about ownership — duplicating is a memcpy and the original stays
usable — so a transaction owning a `String` or a `Vec` cannot be `Copy` at all.
Chapter 31 shows what goes wrong when a `Copy` stand-in lets the wrong code
compile.
