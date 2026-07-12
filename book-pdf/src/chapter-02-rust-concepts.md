# Chapter 2: Rust Concepts

> **In Python we...** learned that everything is an object, and every object is an instance of a class. We called `type(5)` and Python told us, at runtime, that `5` was an instance of `int`. Variables held handles to objects, not bits, so we never worried about data sizes, and dynamic typing let us ask forgiveness rather than permission: try any operation on any object, and Python would raise an `AttributeError` if the object couldn't oblige. *(Python for RTL Verification, "Python concepts")*

The Python book opened its concepts chapter with a little language history: programming began with engineers pushing bits around, types were invented so that a 16-bit `int` couldn't silently trample an 8-bit `char`, and languages arranged themselves along a spectrum of strictness. VHDL and Pascal demanded permission for everything. C and SystemVerilog were permissive to a fault — SystemVerilog would happily chop the top eight bits off a 16-bit value to cram it into a byte. Python opted out of the argument entirely: since variables hold handles to objects rather than bits, there is nothing to chop, and the question of type compatibility gets answered at runtime, one operation at a time.

Rust takes a position on that spectrum you have seen before — it is statically typed, like VHDL — but it occupies the position so differently that the comparison will mislead you if you stop there. This chapter is about the difference. Not the syntax (that starts in Chapter 3), but the worldview: where the types live, when the checking happens, and why the strictest compiler you have ever met is going to become the most useful colleague you have.

## Where the objects went

In Python, the number `5` is an object of class `int`, and it knows it. The type information travels with the object at runtime; that is what makes `type(5)` possible, and it is what makes dynamic typing work — every operation on every object begins with the interpreter looking up, right then, whether the object can do the thing.

Rust's `5` has a type too — but the type exists only in the compiler's head. Every value in a Rust program has exactly one type, known completely before the program runs. The compiler uses that knowledge to check every operation in the program, and then it throws the types away. The compiled binary contains no type tags, no class objects, no lookup tables — just the machine code that the types proved correct. There is no `type()` function in Rust for the same reason there is no ghost in a finished building: by the time the program runs, the types have done their work and left.¹

This means the checking that Python spreads across the whole runtime, Rust performs all at once, up front. The Python book demonstrated dynamic typing with a program that called a string method on an integer — and the key detail was *when* the program failed. Let's port that exact example. In figure 1, `ends_with` is a real method on Rust strings, just as `endswith()` was in Python, and calling a method looks exactly the way it looked in Python: `mystring.ends_with("orld")`. Then we try it on an integer.

```rust
// Figure 1: Calling an undefined method

fn main() {
    let mystring = "Hello, World";
    println!("{}", mystring.ends_with("orld"));
    let myint: u8 = 42;
    println!("{}", myint.ends_with("a whimper"));
}
```

```text
--
% cargo run
   Compiling concepts v0.1.0
error[E0599]: no method named `ends_with` found for type `u8` in the current scope
 --> src/main.rs:5:26
  |
5 |     println!("{}", myint.ends_with("a whimper"));
  |                          ^^^^^^^^^ method not found in `u8`

error: could not compile `concepts` (bin "concepts") due to 1 previous error
```

Compare this to the Python book's version of the same mistake. There, the program *ran*: it printed `True` for the string, created the integer, and only then died with an `AttributeError` at line 4. The first three lines executed; the trouble was discovered in the act of transgressing.

Here, nothing ran. Not the broken line, and — look carefully — not the correct lines either. Rust refused to produce a program at all. That is the trade in its purest form: Python checks each operation the moment it happens, so a mistake costs you a run; Rust checks every operation before any of them happen, so a mistake costs you a compile. In Chapter 1 we noted that for verification work this trade is nearly a gift, because in our world "a run" is not a millisecond of interpreter time — it is a simulator license, an elaboration, and a lunch break. Delete the offending line, as in figure 2, and the program compiles and runs.

```rust
// Figure 2: The corrected program

fn main() {
    let mystring = "Hello, World";
    println!("{}", mystring.ends_with("orld"));
}
```

```text
--
true
```

One reassurance before we move on: statically typed does not mean verbosely typed. You may remember pages of SystemVerilog declarations and be bracing for their return. Rust's compiler performs *type inference* — in figure 1 we never told it that `mystring` was a string; it worked that out from the value. You will write far fewer type annotations than you did in SystemVerilog, and the ones you do write (like the `u8` above) tend to be the ones a verification engineer *wants* to write, because in our business the sizes are the specification. The TinyALU's A port really is eight bits wide, and Chapter 3 will make that `u8` official.

> ¹ Rust does keep a sliver of type information around for one feature we'll meet much later (trait objects, Chapter 10), but the rule to internalize is: no runtime type lookup, because no runtime types.

## The compiler as collaborator

Everything in this book now depends on a mental adjustment, so let's make it explicitly.

When the Python interpreter raised an exception, it was reporting an accident that had already happened: the program was running, it hit something it couldn't do, and it stopped. When the Rust compiler rejects your program, nothing has happened yet. A rejection is not a failure report — it is a *prediction*, delivered while the mistake is still free. The compiler is not an adversary blocking the door to the simulator. It is a reviewer who reads every line of your testbench, every time, in seconds, and never gets bored or skims.

The catch is that this reviewer communicates in a format you have to learn to read, and reading it well is a genuine skill — the single most valuable skill in this book, which is why our figures will so often show error messages rather than output. Let's dissect one. Figure 3 makes a mistake with TinyALU flavor: an ADD of two 8-bit operands can carry into nine bits, so the result belongs in a `u16` — but we try to store it back into a `u8` register. This is precisely the assignment the Python book warned about in its history lesson, the one where SystemVerilog "will happily chop off the top eight bits."

```rust
// Figure 3: The mistake SystemVerilog would have allowed

fn main() {
    let a: u8 = 0xFF;
    let b: u8 = 0x01;
    let result: u16 = a as u16 + b as u16;
    let reg: u8 = result;
    println!("{}", reg);
}
```

```text
--
error[E0308]: mismatched types
 --> src/main.rs:5:19
  |
5 |     let reg: u8 = result;
  |              --   ^^^^^^ expected `u8`, found `u16`
  |              |
  |              expected due to this
  |
help: you can convert a `u16` to a `u8` and panic if the converted value
      doesn't fit
  |
5 |     let reg: u8 = result.try_into().unwrap();
  |                         ++++++++++++++++++++
For more information about this error, try `rustc --explain E0308`.
```

Read it from the top, because the compiler writes for readers who do:

- **The headline**: `error[E0308]: mismatched types`. Every error has a code, and the last line of the output tells you that `rustc --explain E0308` will print a small essay about this class of error, with examples. That command is the book behind the book.
- **The location and the arrows**: file, line, column, then your own code quoted back with carets under the guilty expression. The compiler doesn't just point at the line — it points at the exact expression, and the `expected due to this` arrow points at the *other* place involved, the annotation that created the expectation. Two pointers, because a type mismatch always has two ends.
- **The `help` block**: a suggested fix, often as a ready-to-paste diff (those `+` signs mark what to insert). And notice what this particular suggestion says out loud: converting a `u16` to a `u8` *can lose data*, so the suggested conversion is one that checks at runtime and panics if the value doesn't fit. Rust will let you chop bits — with `result as u8`, exactly the truncation SystemVerilog performs silently — but you must write the chop yourself, in ink, where a reviewer can see it.

Figure 4 takes the honest fix: our register was simply too small for an ADD result, so we widen it. The point of the figure is how little drama the fix involves once the message has told you both ends of the mismatch.

```rust
// Figure 4: The corrected program

fn main() {
    let a: u8 = 0xFF;
    let b: u8 = 0x01;
    let result: u16 = a as u16 + b as u16;
    let reg: u16 = result;
    println!("{}", reg);
}
```

```text
--
256
```

The compiler's helpfulness extends past types into plain proofreading. The Python book admitted that a typo'd name in Python surfaces as a runtime `AttributeError`, possibly weeks later, possibly on the one branch of the testbench that only executes when the DUT misbehaves. In Rust:

```rust
// Figure 5: The compiler as proofreader

fn main() {
    let result = 42;
    println!("{}", resutl);
}
```

```text
--
error[E0425]: cannot find value `resutl` in this scope
 --> src/main.rs:3:20
  |
3 |     println!("{}", resutl);
  |                    ^^^^^^ help: a local variable with a similar name
  |                            exists: `result`
```

It found the typo, and then it found the variable you meant. This is the texture of working in Rust: the error messages are not walls, they are directions. When this book's later chapters claim that a misconnected TLM port or a mistyped configuration "becomes a compile error," figures like these are what that will look like — and by then you will read them at a glance.

A word of honest preparation, though. The errors in this chapter are the friendly kind, because the mistakes were simple. Starting in Chapter 5, the compiler will begin rejecting programs for *ownership* reasons — code that looks obviously fine to a Python-trained eye — and those messages take real practice to read. The habit to build now, on easy errors, is the one that will carry you through the hard ones: read the first error first (later errors are often echoes of it), read the *whole* message including the help block, and trust that the compiler is describing a real problem even when you don't yet see it. In several years of Rust's existence, the compiler has been wrong about this far less often than its users have.²

> ² The Rust project treats confusing error messages as bugs and accepts bug reports about them. It is the only compiler I know with a customer-service department.

## No interpreter, no garbage collector, no GIL

Three pieces of Python machinery are simply absent from a running Rust program, and each absence will matter to us.

**No interpreter.** Python source is executed by a program that reads it — every signal comparison, every scoreboard update pays the interpreter's overhead, and every deployment needs the right interpreter version installed. Rust source is *compiled away*: `cargo run` in Chapter 1 produced a native binary, the same species of artifact as the simulator itself, and the source code is not consulted again. This is where the speed comes from, and it is also where the "no environment" benefit from Chapter 1 comes from — there is nothing to install on the farm machines because the program is already finished.

**No garbage collector.** Python's GC is why the Python book could show two variables holding handles to one object and never ask who was responsible for it — memory management was somebody else's job, done invisibly, at times of the runtime's choosing. Rust has no such somebody. Memory is reclaimed at points the compiler determines *while compiling*, according to rules about which variable owns which value. Those rules are the famous part of Rust — ownership — and they are the subject of Chapter 5, where we will need a full chapter to replace the instinct that "assignment copies a handle and both names stay alive." For now, one sentence of preview: in Rust, assignment usually *moves* a value to its new owner, and that single idea eliminates the garbage collector, the pauses, and — more interesting to us — a whole family of "two tasks touched one transaction" testbench bugs.

**No GIL.** Python's Global Interpreter Lock serializes threads, which is why cocotb never pretended to give you parallelism — everything ran cooperatively on one thread. Rust has no GIL; its compiler can prove threaded code free of data races, and "fearless concurrency" is a genuine Rust selling point. I mention it mostly to lower your expectations: our testbenches will still run cooperatively on the simulator's single thread, because the simulator interface demands it, and the design of our libraries enforces that rule at compile time rather than in documentation. The GIL's absence is real; our use for it, in this book, is modest.

## The toolchain

The Python book could assume you had Python and then teach `pip` when packages arrived. Rust's tooling deserves a proper introduction, because it is unusually good and because you will touch all of it in the next few chapters. The pleasant surprise is that the whole kit arrives as a set with well-defined jobs — a verification engineer who has juggled `pip`, `venv`, `pyenv`, `black`, and `pylint` will recognize every silhouette here, minus the juggling.

**`rustup`** installs and manages Rust itself — the job `pyenv` did for Python versions. One command installs the whole toolchain; `rustup update` moves you to the latest release. Rust ships a new stable version every six weeks, and — this matters — the project holds a strong backward-compatibility promise, so updating is routine rather than an event.

Which raises the obvious worry for anyone who lived through Python 2-to-3: what happens when the language needs to change incompatibly? Rust's answer is **editions**. Every few years (2015, 2018, 2021, 2024) the project bundles its rare breaking changes into a named edition, and each crate — Rust's word for a package, official introductions in Chapter 14 — declares in its manifest which edition it speaks. The compiler supports all of them, forever, and crates from different editions link together in one program. It is Python 3 without the decade of schism: old code keeps compiling, new code gets the improvements, and nobody writes a farewell blog post. The projects in this book use the 2024 edition, which is what `cargo new` selects for you.

**`cargo`** you met in Chapter 1 — `pip`, `venv`, `make`, and `pytest` fused into one tool. It creates projects (`cargo new`), builds them (`cargo build`), builds-and-runs them (`cargo run`), fetches dependencies declared in `Cargo.toml`, and runs tests (`cargo test` — the star of Chapter 14, where we will unit-test testbench components with no simulator in sight). Because every Rust project is a cargo project with the same layout, there is no per-project incantation to learn; every example in this book builds the same way.

**`rustfmt`** reformats your source into the community-standard style, the job `black` did in Python: `cargo fmt`, and the formatting debate is over. This book's figures are all `rustfmt`-clean, so what you read is what your editor will produce.

**`clippy`** is the linter — `pylint`'s job — but with a compiler's leverage, since it analyzes the same typed, checked view of your program the compiler sees. It catches correctness hazards, but its everyday gift to a Rust learner is idiom: clippy knows what fluent Rust looks like and will tell you, kindly and specifically, when you have written Python in Rust syntax. Run it as `cargo clippy`; a representative complaint (output abridged) looks like this:

```rust
// Figure 6: Clippy teaching idiom

fn main() {
    let done = true;
    if done == true {
        println!("finished");
    }
}
```

```text
--
% cargo clippy
warning: equality checks against true are unnecessary
 --> src/main.rs:3:8
  |
3 |     if done == true {
  |        ^^^^^^^^^^^^ help: try simplifying it as shown: `done`
```

Notice the shape: the same location-caret-help anatomy as a compiler error, because it comes from the same machinery. Reading one teaches you to read the other. Make `cargo clippy` a habit early — while you are learning, it is the closest thing to a Rust mentor watching over your shoulder, and unlike a mentor it never sighs.

## Summary

Python's core idea was that everything is an object carrying its type at runtime, checked operation by operation as the program runs. Rust's core idea is that every value has a type known *before* the program runs, checked exhaustively by a compiler that then erases the types entirely — leaving a native binary with no interpreter, no garbage collector, and no GIL. The cost is that the compiler rejects programs Python would have happily started; the payoff is that it rejects them in seconds, with error messages that name the problem, point at both ends of it, and usually propose the fix. Learning to read those messages is the skill this book will exercise constantly, and the toolchain — `rustup` for the compiler, `cargo` for everything, `rustfmt` for style, `clippy` for idiom — is the same set of jobs your Python toolbelt did, consolidated and sharpened.

Concepts in hand, it is time to write some actual Rust. Chapter 3 starts where the Python book's basics chapter started — variables, numbers, and printing — and the TinyALU's 8-bit operands are about to get types that say so.
