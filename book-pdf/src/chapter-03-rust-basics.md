# Chapter 3: Rust Basics

> **In Python we...** learned that everything is an object. When we wrote `xx = 5`, Python instantiated an `int` object of value 5 on the heap and handed `xx` a reference to it. There was no maximum `int`, no bit width, and no declared type — the *object* had a type, the *variable* was just a name, and we could point that name at anything else a line later. The Python book put it this way: when you say `byte xx = 5` in SystemVerilog, the program sets aside an 8-bit memory location; when you say `xx = 5` in Python, you get a handle to an `int` object. This chapter is about coming back from that trip.

Chapter 2 introduced the compiler as a collaborator. This chapter puts it to work on the smallest possible material: variables, numbers, strings of output, and functions. None of it is hard, but nearly all of it is *different* from Python in ways that will matter every day, so we will move through it the way the Python book moved through its basics chapter — with small figures you can run and diff against the ones you already know.

If you want to follow along (you should), make a playground the way we did in Chapter 1:

```text
# Figure 1: A playground for this chapter

% cargo new basics
    Creating binary (application) `basics` package
```

Everything in this chapter goes inside `fn main()` in `src/basics/src/main.rs` unless a figure shows its own function.

## `let`: bindings, not name tags

In Python, a variable is a name tag you can peel off one object and stick on another. In Rust, `let` creates a **binding**: it associates a name with a value, and — here is the first surprise — that binding is **immutable by default**. Assigning to it a second time is not merely discouraged. It does not compile.

```rust
# Figure 2: Assigning twice to an immutable binding

fn main() {
    let xx = 5;
    println!("xx: {xx}");
    xx = 6;
    println!("xx: {xx}");
}
```
--
```text
% cargo run
error[E0384]: cannot assign twice to immutable variable `xx`
 --> src/main.rs:4:5
  |
2 |     let xx = 5;
  |         -- first assignment to `xx`
3 |     println!("xx: {xx}");
4 |     xx = 6;
  |     ^^^^^^ cannot assign twice to immutable variable
  |
help: consider making this binding mutable
  |
2 |     let mut xx = 5;
  |         +++
```

Read that error the way Chapter 2 taught you: it names the rule, points at both the first assignment and the offending one, and then *tells you the fix*. If you want a variable that varies, you ask for one with `mut`:

```rust
# Figure 3: A mutable binding

fn main() {
    let mut xx = 5;
    println!("xx: {xx}");
    xx = 6;
    println!("xx: {xx}");
}
```
--
```text
xx: 5
xx: 6
```

Coming from Python, immutability-by-default feels backwards for about a week. Then you start reading other people's testbench code and discover what it buys you: every `mut` in a Rust program is a signpost saying *this value changes — watch it*. In Python, every variable carried that warning implicitly, which is the same as no variable carrying it at all. When you read a Rust monitor and see that only one binding in it is `mut`, you know where the state lives. The compiler is not restricting you; it is making your intentions legible.¹

> ¹ It will also nag you in the other direction: declare something `mut` and never mutate it, and the compiler warns you to take the `mut` off. It wants the signposts accurate in both directions.

## Scalar types: the bits are back

The Python book's basics chapter needed exactly three number types — `bool`, `int`, and `float` — and Python's `int` had no maximum value. Rust returns us to the world hardware people never really left: integers have widths, and the widths are in the names.

The scalar types you will actually use:

- **Integers, unsigned:** `u8`, `u16`, `u32`, `u64` — 8 to 64 bits, no sign bit. There is also `usize`, the pointer-width integer that Rust uses for indexing and lengths.
- **Integers, signed:** `i8`, `i16`, `i32`, `i64` — two's complement, exactly as your DUT stores them. Bare integer literals like `5` default to `i32`.
- **Floats:** `f32` and `f64`. Bare float literals like `2.0` default to `f64`, which is Python's `float`.
- **`bool`:** `true` and `false` — lowercase now, and the *only* things a condition accepts. Python's habit of treating `None`, `0`, and empty containers as falsy does not exist here; an `if` takes a `bool`, full stop.
- **`char`:** a single Unicode character, in single quotes: `'A'`. Not a one-character string — a distinct type, four bytes wide.

Why should a verification engineer care, beyond nostalgia for SystemVerilog's `byte`? Because *your DUT already thinks this way*, and now your testbench can agree with it. The TinyALU's A and B legs are eight bits wide. In the Python book we drove them from an `int` and relied on discipline (and the BFM) to keep values in range — nothing in the language stopped a careless test from generating `a = 300`. In Rust, a TinyALU operand is a `u8`, and 300 *is not a value that type can hold*. The type system now knows something true about the hardware, and it never forgets it:

```rust
# Figure 4: The TinyALU's A leg really is a u8

fn main() {
    let aa: u8 = 0xFF;
    let bb: u8 = 300;
    println!("aa: {aa}, bb: {bb}");
}
```
--
```text
% cargo run
error: literal out of range for `u8`
 --> src/main.rs:3:18
  |
3 |     let bb: u8 = 300;
  |                  ^^^
  |
  = note: the literal `300` does not fit into the type `u8`
    whose range is `0..=255`
```

Notice the `let aa: u8 = 0xFF;` syntax: a colon and a type after the name is a **type annotation**. Most of the time you can leave it off and Rust infers the type from context — this is why Figures 2 and 3 didn't need one — but when you mean *eight bits*, say so.

One honest wrinkle while we are here: what happens when arithmetic *overflows* a `u8` at runtime — say, `200 + 100` where both values arrived from a random generator? In a debug build, the program panics (halts with an error) at the overflowing operation; in a release build, the value wraps around modulo 256, the way the hardware would. If wrapping is what you *mean* — and in ALU-prediction code it often is — Rust provides methods like `wrapping_add` that say so explicitly. We will use exactly that when we write the TinyALU predictor, because "the model overflows exactly like the DUT, on purpose, in writing" is the kind of sentence verification sign-off meetings love.

## No implicit conversions — diffing against the Python book

Here is the Python book's very first figure, which showed that a `float` in an operation means a `float` result — `ii + ff` quietly promoted the `int`, and `ii/ii` produced a `float` even from two `int`s. Let's port it one-for-one and watch Rust refuse to play:

```rust
# Figure 5: A float in operations means... a compile error

fn main() {
    let ii: i32 = 1;
    let ff: f64 = 2.0;
    let ss = ii + ff;
    println!("ss: {ss}");
}
```
--
```text
% cargo run
error[E0277]: cannot add a `f64` to `i32`
 --> src/main.rs:4:17
  |
4 |     let ss = ii + ff;
  |                 ^ no implementation for `i32 + f64`
```

Rust performs **no implicit numeric conversions**. Not int-to-float, not `u8`-to-`u16`, nothing. If you want a conversion, you write one, using the `as` keyword — and once you do, the rest of the ported figure behaves recognizably, with one telling difference in the last line:

```rust
# Figure 6: The same figure, with the conversions made explicit

fn main() {
    let ii: i32 = 1;
    let ff: f64 = 2.0;
    let ss = ii as f64 + ff;
    println!("ss: {ss}");
    let dd = ii / ii;
    println!("dd: {dd}");
}
```
--
```text
ss: 3
dd: 1
```

In the Python original, `dd = ii/ii` printed `1.0` — division *always* returned a `float`. In Rust, dividing two integers is integer division: `dd` is `1`, an `i32`, and `7 / 2` would be `3`. If you want the fractional answer, convert to floats first. Neither behavior is right or wrong, but they are different, and scoreboard math is exactly where that difference bites — so it is worth one figure now instead of one confused afternoon later.

The same strictness rewrites the Python book's augmented-assignment figure. In Python, `xx` started as an `int` at 1, and after `xx /= 4` it had silently become a `float` holding 1.5 — the variable changed *type* mid-flight. Watch the Rust version:

```rust
# Figure 7: Augmented assignments — the type never changes

fn main() {
    let mut xx = 1;
    println!("xx: {xx}");
    xx += 1;
    println!("xx += 1: {xx}");
    xx *= 3;
    println!("xx *= 3: {xx}");
    xx /= 4;
    println!("xx /= 4: {xx}");
}
```
--
```text
xx: 1
xx += 1: 2
xx *= 3: 6
xx /= 4: 1
```

Same operators, same rhythm — Rust has `+=`, `*=`, `/=`, and friends, though like Python it has no `++`. But `xx` was born an `i32` and will die an `i32`; `6 /= 4` gives 1, not 1.5. A binding's type is fixed for the binding's whole life. Which raises an obvious question: what do we do when we genuinely want the Python pattern — same *idea*, new *type*?

## Shadowing: same name, new binding

The Python book's constructor figure turned the string `"3.14159"` into a `float` by creating a new object: `pi = float("3.14159")`. Rust's version of that pattern is **shadowing**: declaring a *new* binding, with `let`, that reuses an old name.

```rust
# Figure 8: Creating a number from a string, by shadowing

fn main() {
    let pi = "3.14159";
    let pi: f64 = pi.parse().expect("not a number");
    println!("pi: {pi}");
}
```
--
```text
pi: 3.14159
```

The first `pi` is a string; the second `pi` is a brand-new binding, a `f64`, whose value came from parsing the first. From that line on, the name `pi` means the number; the string version is shadowed — inaccessible, retired with honors. This is not mutation (nothing was `mut`) and it is not a type change (each binding kept its type); it is the "same idea, new type" idiom, done with two immutable bindings instead of one shape-shifting variable. You will see it constantly in testbench code: parse a string into a number, convert raw bits into a transaction, and keep the natural name at every step.

Two small notes on Figure 8. First, `parse` can fail — `"pi".parse()` has nowhere good to go — so it returns a `Result`, Rust's replacement for exceptions; `.expect("...")` says "give me the value, and halt with this message if it failed." That is a blunt instrument we will trade for proper tools in Chapter 9; the Python original had the same rough edge, raising `ValueError` on `int("3.14159")`. Second, the annotation `: f64` is doing real work: it is how `parse` knows *what* to parse the string into.

## `println!` and format strings

You have been reading `println!` output all chapter; now let's look at the format strings themselves, next to the f-strings you know. `{}` is the placeholder, arguments fill placeholders in order, and — the part that makes Rust feel almost Pythonic — a variable name can go directly inside the braces:

```rust
# Figure 9: Format strings, next to the f-strings you know

fn main() {
    let aa: u8 = 0x2A;
    let bb: u8 = 7;
    println!("aa is {} and bb is {}", aa, bb);  // like str.format()
    println!("aa is {aa} and bb is {bb}");      // like an f-string
    println!("aa in hex: {aa:#04x}");
    println!("aa in binary: {aa:#010b}");
    println!("sum: {}", aa + bb);
}
```
--
```text
aa is 42 and bb is 7
aa is 42 and bb is 7
aa in hex: 0x2a
aa in binary: 0b00101010
sum: 49
```

The format specifiers after the colon will feel familiar from both Python and `$display`: `{aa:#04x}` means hexadecimal, `#` for the `0x` prefix, padded to width 4. The hex and binary forms in Figure 9 are the ones you will reach for when a scoreboard mismatch needs to be read against a waveform.

One genuine difference from f-strings: the braces capture *names only*, not arbitrary expressions. Python lets you write `f"{aa + bb}"`; Rust makes you write the expression as an argument, as in the last line of Figure 9. And the exclamation point still means what Chapter 1 said it means: `println!` is a macro, which is precisely *why* it can type-check your format string against your arguments at compile time — pass one argument too few and the program does not build, where Python's `"{} {}".format(x)` waited until runtime to complain.

## Expressions vs. statements

Here is the concept in this chapter most likely to be genuinely new, rather than a stricter spelling of something Python had. Python divides the world into statements (`if`, `for`, assignments) and expressions (things with values), and mostly keeps them apart. In Rust, nearly everything is an **expression** — nearly everything *has a value* — and the language leans on this constantly.

Two demonstrations. First, `if` is an expression, which means it can sit on the right-hand side of a `let`:

```rust
# Figure 10: if is an expression

fn main() {
    let count: u8 = 42;
    let parity = if count % 2 == 0 { "even" } else { "odd" };
    println!("count is {parity}");
}
```
--
```text
count is even
```

Python has the ternary form `"even" if count % 2 == 0 else "odd"` for exactly this job; Rust simply has no separate ternary, because ordinary `if` already returns a value. The compiler checks that both arms produce the same type — an `if` that gives you a string on Mondays and an integer on Tuesdays does not compile — and an `else` is required when you use the value, because the value must exist either way.

Second, a block — any `{ ... }` — is an expression whose value is its **last expression, written without a semicolon**:

```rust
# Figure 11: A block is an expression; the semicolon is the switch

fn main() {
    let nn = {
        let doubled = 2 * 3;
        doubled + 1
    };
    println!("nn: {nn}");
}
```
--
```text
nn: 7
```

Look hard at `doubled + 1` — no semicolon. That is not sloppy punctuation; it is load-bearing syntax. A trailing expression without a semicolon is the block's value; add a semicolon and you have turned it into a statement, the block's value becomes the empty "unit" value `()`, and the compiler will greet you with an error message that — helpfully — points straight at the semicolon and suggests removing it. Every Rust programmer alive has been rescued by that message.² Once this clicks, you will start to see Rust code as a tree of nested expressions, each yielding a value to the one above it, and the language's whole shape gets simpler.

> ² Usually within the first hour.

## Functions

Which brings us, with suspicious convenience, to functions — because a function body is just another block, and returning a value is just the block-expression rule again.

```rust
# Figure 12: A TinyALU prediction function

fn predict_add(aa: u8, bb: u8) -> u16 {
    aa as u16 + bb as u16
}

fn main() {
    let sum = predict_add(0xFF, 0xFF);
    println!("predicted sum: {sum:#06x}");
}
```
--
```text
predicted sum: 0x01fe
```

Everything Python left optional is now required, and everything required is now checked. Each parameter declares its type; the `-> u16` arrow declares the return type; and the last expression of the body — `aa as u16 + bb as u16`, no semicolon — is the return value. (An explicit `return` keyword exists for bailing out early, but idiomatic Rust lets the final expression speak for itself.) A function with no `->` returns `()`, Rust's cousin of Python's implicit `None`.

Figure 12 also carries the chapter's TinyALU payload. The Python book's prediction function took two Python `int`s and returned a Python `int`, and the fact that the TinyALU's result port is *sixteen* bits while its operands are *eight* lived only in prose and in the DUT. Here it lives in the signature: `fn predict_add(aa: u8, bb: u8) -> u16` *is* the TinyALU's ADD operation, as a type. `0xFF + 0xFF` overflows a `u8` — which is exactly why the hardware has a wide result port, and exactly why the function converts each operand to `u16` before adding. Try deleting the two `as u16` conversions and read the error you get; the compiler will explain the TinyALU datasheet to you.

And that signature pays one more dividend Python could not offer: the compiler checks every *call*. Pass `predict_add` a 16-bit value, or three arguments, or use its result where a `u8` is expected, and the testbench does not build. In the Python book, a mis-called prediction function was a runtime discovery; here it never gets as far as the simulator.

## Comments, briefly

Line comments are `//` to end of line, matching Python's `#` in spirit. Block comments `/* ... */` exist but are rare in practice. What Rust has that Python approximated with docstrings is **doc comments**: lines beginning `///` above a function or type are documentation the toolchain actually compiles into browsable HTML (`cargo doc`). We will start writing them when we write code worth documenting, which is soon.

## Summary

This chapter covered Rust's nuts and bolts, each one a deliberate diff against the Python book's basics chapter:

- **`let` bindings** — immutable by default; `mut` is an explicit, visible request for mutability
- **scalar types** — `u8` through `i64`, `f32`/`f64`, `bool`, `char`; bit widths are back, and the TinyALU's operands are honest `u8`s at last
- **no implicit conversions** — mixed-type arithmetic is a compile error; `as` makes conversions visible; integer division stays integer
- **shadowing** — the "same idea, new type" pattern, done with a fresh binding instead of a shape-shifting variable
- **`println!` and format strings** — f-string-like `{name}` captures, plus compile-time checking of the format string itself
- **expressions vs. statements** — `if` and blocks have values; the trailing-expression-without-semicolon rule
- **functions** — typed parameters, declared return types, and signatures the compiler enforces at every call site

Every figure here was straight-line code — no branches worth mentioning, no loops at all. A testbench that never loops is not much of a testbench, and besides, Rust is holding back its best conditional construct: `match`, which Python never had and which the rest of this book will use on nearly every page. Both are waiting in Chapter 4.
