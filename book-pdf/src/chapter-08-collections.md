# Chapter 8: Collections: Vec, String, and HashMap

> **In Python we...** spent five chapters on sequences and dictionaries. We learned that "Python sequences are iterables that store ordered data and provide the data one at a time," that lists are "Python's workhorse" — mutable, sortable, able to hold anything — that strings are immutable sequences with 45 methods, and that dictionaries "allow us to store a value using a key and retrieve it later," returning their keys in creation order and raising `KeyError` when a key is missing. We counted the letters in `"Mississippi"` with a dictionary and sliced `"abcdefghi"[1:5]` to get `bcde`.

This chapter compresses those five Python chapters into one, and I want to be honest about why: most of what you learned transfers directly. Rust has a growable list (`Vec<T>`), a string type (`String`), and a hash-keyed store (`HashMap<K, V>`). You index with square brackets, you slice with ranges, you loop with `for`, you check membership, you sort. If I walked through every operation the way the Python book did, you would be bored and I would be padding.

So instead this chapter spends its words on the three places where your Python instincts will actively mislead you:

1. A `Vec` *owns* its elements — pushing a value into it is a move, and Chapter 5 comes due.
2. Rust has *two* string types, `String` and `&str`, and until you know which is which, every function signature involving text will look like a typo.
3. Iterating over a collection can either *borrow* it or *consume* it, and the difference is one ampersand.

Everything else — the operations that work the way you expect — gets a fast tour at the end.

## Vec: the list that owns its contents

A `Vec<T>` is Rust's growable array: the analog of the Python list, and every bit the workhorse the list was. The first difference is visible in the type: a Python list holds *anything* (`['a', 3, LL]` was a perfectly good list), while a `Vec<T>` holds values of exactly one type `T`. You have met this trade before — it is the same static-typing bargain from Chapter 3, and it is usually what a testbench wanted anyway. A history log of ALU transactions should hold transactions, all of them, and nothing else.

Let's build exactly that. Figure 1 creates a log of the `AluCommand` transactions we defined in Chapter 7 and pushes commands into it, the way a monitor might record everything it sees.¹

```rust
// Figure 1: A Vec<AluCommand> as a transaction history log

#[derive(Clone, Copy, Debug, PartialEq)]
enum Ops { Add = 1, And = 2, Xor = 3, Mul = 4 }

#[derive(Clone, Debug, PartialEq)]
struct AluCommand { a: u8, b: u8, op: Ops }

fn main() {
    let mut log: Vec<AluCommand> = Vec::new();
    log.push(AluCommand { a: 5, b: 3, op: Ops::Add });
    log.push(AluCommand { a: 2, b: 2, op: Ops::Mul });

    println!("{} commands logged", log.len());
    println!("first: {:?}", log[0]);
}
```

```text
--
2 commands logged
first: AluCommand { a: 5, b: 3, op: Add }
```

Familiar territory: `push` is `append`, `len()` is `len()`, `log[0]` is `log[0]`. Note that `log` must be `let mut` — Chapter 3's immutability-by-default applies to collections with no exceptions, which means a testbench data structure cannot be quietly modified by code you didn't expect to modify it. Also note `Vec::new()` gave us an empty vector; the `vec!` macro is the literal syntax, so `vec![1, 2, 3]` is Rust's `[1, 2, 3]`.

Now the part that is genuinely new. In Python, a list holds *references*. When you append a transaction to a list, the list gets one more name for an object that still has all its other names; the monitor keeps its handle, the scoreboard keeps its handle, and the GC sorts out the afterlife. A `Vec` holds *values*. When you push a transaction into a `Vec`, the transaction **moves** into the `Vec` — the vector becomes the owner, exactly as if you had assigned it to a new variable in Chapter 5. Figure 2 shows what happens when we forget.

```rust
// Figure 2: Pushing is a move

fn main() {
    let mut log: Vec<AluCommand> = Vec::new();
    let cmd = AluCommand { a: 5, b: 3, op: Ops::Add };
    log.push(cmd);
    println!("sent: {:?}", cmd);   // cmd moved into the Vec
}
```

```text
--
error[E0382]: borrow of moved value: `cmd`
  --> src/main.rs:12:28
   |
10 |     let cmd = AluCommand { a: 5, b: 3, op: Ops::Add };
   |         --- move occurs because `cmd` has type `AluCommand`,
   |             which does not implement the `Copy` trait
11 |     log.push(cmd);
   |              --- value moved here
12 |     println!("sent: {:?}", cmd);
   |                            ^^^ value borrowed here after move
   |
help: consider cloning the value if the performance cost is acceptable
   |
11 |     log.push(cmd.clone());
   |                 ++++++++
```

Read that error the way Chapter 2 taught you: the compiler names the move, points at the line where it happened, and offers the fix. If you want to log the command *and* keep using it, `push(cmd.clone())` makes the copy explicit — the choice Python's list made for you silently (share the reference) is now a decision you make on the line where it matters. If the log is the transaction's final destination, push the value and let the `Vec` own it. This is the monitor-hands-a-transaction-to-the-scoreboard question from Chapter 5, answered by a container: *whoever holds the `Vec` owns everything in it*, and when the `Vec` is dropped, every transaction inside is dropped with it. No cycles, no GC, no doubt about who frees what.

One more `Vec` note before we move on: indexing past the end panics, where Python raised `IndexError`. There is also `log.get(7)`, which does not panic but instead returns a value that is either something or nothing — a type called `Option` that keeps appearing in this chapter's corners and gets the full treatment in Chapter 9.

## Iteration: borrow or consume, your choice

The `for` loop over a `Vec` looks exactly like Python — and hides the chapter's second lesson in a single character. Figure 3 loops over the log the obvious way and then tries to use it afterward.

```rust
// Figure 3: A for loop can consume the collection

fn main() {
    let log = vec![
        AluCommand { a: 5, b: 3, op: Ops::Add },
        AluCommand { a: 2, b: 2, op: Ops::Mul },
    ];
    for cmd in log {
        println!("{:?}", cmd);
    }
    println!("{} commands", log.len());   // log is gone
}
```

```text
--
error[E0382]: borrow of moved value: `log`
  --> src/main.rs:14:29
   |
9  |     for cmd in log {
   |                --- `log` moved due to this implicit call
   |                    to `.into_iter()`
...
14 |     println!("{} commands", log.len());
   |                             ^^^ value borrowed here after move
   |
help: consider borrowing to avoid moving into the for loop
   |
9  |     for cmd in &log {
   |                +
```

`for cmd in log` *consumes* the vector: ownership of the whole `Vec` moves into the loop, each element moves into `cmd` in turn, and when the loop ends there is nothing left. That is occasionally exactly what you want — a scoreboard draining its queue at end of test, say. But most of the time you want Python's behavior, looking at the elements while leaving the collection intact, and the compiler's `help` line hands you the idiom: borrow it.

```rust
// Figure 4: Borrowing iteration leaves the Vec intact

fn main() {
    let log = vec![
        AluCommand { a: 5, b: 3, op: Ops::Add },
        AluCommand { a: 2, b: 2, op: Ops::Mul },
    ];
    for cmd in &log {
        println!("{:?}", cmd);
    }
    println!("{} commands still logged", log.len());
}
```

```text
--
AluCommand { a: 5, b: 3, op: Add }
AluCommand { a: 2, b: 2, op: Mul }
2 commands still logged
```

With `for cmd in &log`, each `cmd` is a `&AluCommand` — a shared borrow, read-only, governed by Chapter 6's rules. The third form, `for cmd in &mut log`, borrows each element mutably so you can edit in place. The whole story is three spellings:

- `for x in &v` — borrow each element; the loop reads. *This is your Python `for` loop.*
- `for x in &mut v` — mutably borrow each element; the loop edits.
- `for x in v` — take ownership of each element; the loop consumes, and `v` is gone.

Chapter 6's aliasing rule follows you into the loop body, and it quietly retires a classic Python bug: with `for cmd in &log` active, you cannot also `log.push(...)` inside the loop, because that would mutate a collection you are currently borrowing. The Python book could only warn you never to modify a list while iterating over it; the borrow checker rejects the program.

## String and &str: the two-string problem

Here is the single most confusing thing this chapter has to teach, so let's take it slowly. Python has one string type. Rust, from where you sit, appears to have two, and every Rust learner arriving from Python spends a bewildered week discovering which functions want which.

The two types are:

- **`String`** — an owned, growable string, allocated on the heap. This is the closest thing to a mutable Python `str`-builder: you can push characters onto it, and whoever owns it is responsible for it, per Chapter 5.
- **`&str`** (pronounced "string slice") — a *borrowed view* of string data that lives somewhere else. It doesn't own anything; it points at characters and knows how many of them it covers. It is Chapter 6's `&T`, specialized for text.

The reason beginners meet the confusing one first: **every string literal is a `&str`.** When you write `"TinyALU"`, those seven bytes are baked into your compiled program itself, and the literal is a borrowed slice pointing into that program memory. It costs nothing, it lives forever, and it is read-only. Figure 5 shows both types and the traffic between them.

```rust
// Figure 5: String literals are borrowed; String is owned

fn main() {
    let dut: &str = "TinyALU";            // borrowed slice into program memory
    let mut name: String = String::from("Tiny");
    name.push_str("ALU");                 // owned and growable
    let banner = format!("*** Testing the {} ***", name);
    println!("{}", dut);
    println!("{}", banner);
}
```

```text
--
TinyALU
*** Testing the TinyALU ***
```

`String::from("Tiny")` copies the literal's characters into a fresh, owned, heap-allocated `String` that we can grow with `push_str`. And `format!` is your f-string: `format!("Testing the {}", name)` builds a new `String` the way `f"Testing the {name}"` built a new `str` — same job, and like Python's strings, Rust's are immutable-by-default; growing one requires `mut`, and replacing text builds a new string rather than editing in place.²

Now the question that actually bites: when you write a function that takes text, which type goes in the signature? The rule of thumb is worth memorizing, because it appears throughout rustdv's own API:

> **Store `String`, pass `&str`.** Struct fields that keep text own it as `String`; function parameters that read text borrow it as `&str`.

The reasoning is pure Chapter 6. A function that only *reads* a name has no business demanding ownership of it — that would force every caller to give up (or clone) their string just so you could look at it. A parameter of type `&str` says "lend me a view," and it is maximally accepting: a `&String` coerces to a `&str` automatically, so callers can pass a literal, a slice, or a borrowed `String`, all with no copying. Figure 6 shows the shape.

```rust
// Figure 6: Take &str; accept everything

fn report_pass(test_name: &str) {
    println!("PASSED: {}", test_name);
}

fn main() {
    let owned = String::from("random_ops");
    report_pass("smoke_test");    // a literal is already a &str
    report_pass(&owned);          // a &String coerces to &str
    println!("still have {}", owned);
}
```

```text
--
PASSED: smoke_test
PASSED: random_ops
still have random_ops
```

If instead the function is going to *keep* the text — storing a component's name in a struct field, say — it should take a `String` and own it outright, and the move at the call site honestly documents the handoff. When rustdv asks for `&str` in a signature, it is promising to look and not keep; when it asks for `String`, it is telling you the name is moving in permanently.

One Python habit does not survive the crossing at all: indexing into a string. `line[45]` was everyday Python; in Rust, `s[0]` on a `String` does not compile. Rust strings are UTF-8 encoded, so a character can occupy anywhere from one to four bytes, and Rust refuses to guess whether you want the byte or the character.³ When you need the characters, say so — `for ch in s.chars()` iterates over them, mirroring the Python book's "string as an iterable" figure — and methods like `split`, `trim` (Python's `strip`), `replace`, and `contains` cover the daily string chores you already know by their Python names.

## HashMap: the dictionary, minus the ordering promise

`HashMap<K, V>` is the Python dict: store a value under a key, get it back later. It lives in the standard library's collections module rather than the language itself, so it needs an import — your first `use` statement doing real work.

The Python book's signature dictionary example counted the letters in `"Mississippi"`, using `KeyError` (and then `setdefault`) to handle the first time each letter appeared. Our version counts something a verification engineer actually tallies: how many times the testbench has exercised each ALU operation — the raw material of functional coverage. Rust's replacement for the whole `try`/`except KeyError`/`setdefault` dance is the *entry API*, and it is one line.

```rust
// Figure 7: A HashMap<Ops, u32> op-frequency counter

use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Ops { Add = 1, And = 2, Xor = 3, Mul = 4 }

fn main() {
    let op_stream = vec![Ops::Add, Ops::Mul, Ops::Add,
                         Ops::Xor, Ops::Add, Ops::Mul];
    let mut freq: HashMap<Ops, u32> = HashMap::new();
    for op in &op_stream {
        *freq.entry(*op).or_insert(0) += 1;
    }
    for (op, count) in &freq {
        println!("{:?}: {}", op, count);
    }
}
```

```text
--
Mul: 2
Xor: 1
Add: 3
```

Three things to unpack. First, the derive line grew: a type used as a `HashMap` key must support hashing and full equality, so `Ops` now derives `Eq` and `Hash` alongside the derives from Chapter 7. In Python, hashability was a runtime property you discovered when a `dict` rejected your key; in Rust it is a capability you declare on the type, and using a non-hashable key is a compile error. (What deriving really does, and the family of traits behind it, is Chapter 10's story.)

Second, the entry line: `freq.entry(*op).or_insert(0)` means "find this key's slot, filling it with 0 if it's empty, and hand me access to the value" — `setdefault`, near-verbatim, with the leading `*` saying "increment the value the entry points at," Chapter 6's dereference doing honest work. For simple lookup, `freq.get(&Ops::Add)` plays the role of `players.get(4)`: where Python's `get` returned the value or `None`, Rust's returns that same something-or-nothing `Option` type that `Vec::get` gave us — Chapter 9 is circling closer.

Third — and this one is a genuine behavioral difference, worth a flag in your notes — **look at the output order.** We inserted Add first; the printout led with Mul. The Python book was careful to note (as of Python 3.7) that dicts return keys in insertion order, and if you have written Python since then you have surely leaned on it. `HashMap` makes *no* ordering promise: iteration order is arbitrary, and it is deliberately randomized from run to run, so the same program can print `Add, Mul, Xor` today and `Xor, Add, Mul` tomorrow.⁴ Any testbench logic that quietly depends on dict order — golden log files compared line-by-line are the classic offender — must either sort the keys before printing or use `BTreeMap`, `HashMap`'s sibling that keeps keys in sorted order at a small cost. Iterating `for (op, count) in &freq` borrows, exactly per this chapter's rules, and destructures each key-value pair into two variables the way `for number, item in count.items()` did.

## The rest of the toolbox, briefly

Here is the promised fast tour of what transfers without drama — the material of the Python book's tuples, ranges, sets, and "commonly used sequence operations" chapters, each in a sentence or two.

**Tuples** exist and you have already used them: `let pair = (5, Ops::Add);` groups values of different types, `pair.0` indexes them, and `let (a, op) = pair;` destructures — so functions return multiple values exactly as they did in Python. **Ranges** you met in Chapter 4: `0..5` is `range(5)`, `0..=4` includes the end. **Sets** are `HashSet<T>`, with `insert`, `contains`, and the union/intersection/difference operations — and the same no-ordering caveat as `HashMap`, which shares its machinery.

Of the common sequence operations: `v.len()` is `len(v)`; `v.contains(&x)` is `x in v`; `v.sort()` sorts in place like `list.sort()`, with `sort_by_key` playing the `key=` role; `v.iter().max()` and `.min()` return — you can guess by now — an `Option`, because the vector might be empty, a case Python handled by raising `ValueError`. Slicing carries over almost keystroke-for-keystroke: `&log[1..3]` is `log[1:3]`, a borrowed view of elements one and two, and the same range syntax slices strings and arrays. The borrowed-ness is the Rust twist: a slice is a reference into the original, so Chapter 6's rules apply while you hold it. What you will *not* find are `+` and `*` on vectors; Rust spells concatenation and repetition through methods (`extend`, `concat`, `"-".repeat(15)` for the Python book's horizontal bar). And list comprehensions have no literal syntax — their job is done by iterator chains like `map` and `filter`, which are Chapter 12's whole subject and worth the wait.

## Summary

Rust's collections are Python's collections with ownership made visible. A `Vec<T>` is the list, but it owns its elements: pushing a transaction moves it in, dropping the `Vec` drops everything inside, and the who-frees-this question of Chapter 5 has a container-sized answer. Iteration comes in three spellings — `&v` borrows, `&mut v` borrows mutably, bare `v` consumes — and the borrow checker enforces the never-mutate-while-iterating rule Python could only put in a warning box. Text comes in two types: owned, growable `String` and borrowed `&str`, with string literals being `&str` slices into your program's own memory, and the rule of thumb *store `String`, pass `&str`*. `HashMap<K, V>` is the dict, with the entry API replacing the `KeyError` dance and one honest regression from Python 3.7: no insertion-order promise, so sort your keys before comparing log files. Tuples, ranges, sets, slices, sorting, and membership tests all transfer nearly unchanged.

Along the way, one type kept appearing in the corners of figures and refusing to explain itself: `Vec::get`, `HashMap::get`, `max`, and `min` all returned an `Option`, Rust's way of saying "there might be nothing here" — in the type, where the compiler can see it. Python answered the same question with `None` checks and `KeyError`; Rust's answer is better, and it comes with a partner named `Result` that replaces exceptions entirely. That is Chapter 9.

---

> ¹ Standalone playground project, per our Part I convention: `cargo new collections` and everything in this chapter runs in `src/main.rs`. And yes, `vec!` has the exclamation point of a macro, like `println!` — Chapter 21 explains why; until then, enjoy the enthusiasm.

> ² The Python book proved `str` immutability by showing `replace()` returning a new object with a new `id()`. Rust's `s.replace("real", "mall")` likewise returns a fresh `String` and leaves the original alone — same lesson, no `id()` required, because ownership tells you there are two strings.

> ³ The moment this stops feeling like pedantry is the moment a signal name, a file path, or a log message contains its first non-ASCII character. Python 2 programmers can tell you stories; Rust decided at birth never to be in those stories.

> ⁴ The randomization is a deliberate defense — a hash table with predictable layout invites pathological (even malicious) key patterns — but the practical takeaway for us is simpler: if your test passes or fails depending on `HashMap` iteration order, the bug is in the test.

*Next: Chapter 9, where `Option` finally introduces itself properly, `Result` retires the exception, and the `?` operator makes honest error handling almost as terse as ignoring errors used to be.*
