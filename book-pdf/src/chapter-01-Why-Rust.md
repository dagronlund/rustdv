# Chapter 1: Why Rust?

Rust for RTL Verification is a book for verification engineers who have outgrown an interpreter and for Rust programmers who want to learn the Universal Verification Methodology (UVM). Mostly, though, it is a book for readers of *Python for RTL Verification* who are ready for a second language — one that trades a little of Python's ease for a lot of speed and an entirely new superpower: a compiler that finds testbench bugs before the simulator ever runs.

This book teaches you Rust the way the last book taught you Python: just enough of the language, arriving just in time, to build testbenches with rustvm-sim (our cocotb equivalent) and rustvm (our pyuvm equivalent). By the final chapter you will have rebuilt the TinyALU testbench — the same TinyALU, the same testbench architecture, versions 1.0 through 8.0 — in a language that compiles to native code and races through regressions.

## The book assumes you know the Python story

*Python for RTL Verification* assumed you knew how to program. This book assumes more: that you know how to program *testbenches*, the way that book taught them. When we meet a sequence in Chapter 36, I will not explain what a sequence is for — you know. I will explain what it looks like in Rust and why it looks that way. If you have not read the Python book but know cocotb and pyuvm well, you will be fine. If neither is true, read that book first; this one will still be here.

A note on what you do *not* need: any Rust. Not one line. If you have heard alarming rumors about a thing called the borrow checker, you have heard correctly, and we will make friends with it in Chapter 5.

## Two revolutions, thirty years apart

The Python book told the story of how we came to verify hardware with software — from e and SUPERLOG through the methodology wars to the UVM, and then sideways, off the simulator entirely, to Python. That story had a moral: testbenches are software, and software deserves a software language.

Rust's story rhymes with it. In 2006, a Mozilla engineer named Graydon Hoare started a personal project to answer an uncomfortable question: why, decades into the software era, were our foundational programs — browsers, kernels, the code we bet everything on — still written in languages that let one stray pointer corrupt everything? C and C++ were fast because they trusted the programmer completely, and every security bulletin showed what that trust cost.²

The conventional answer was garbage collection: let a runtime babysit memory, and accept the slowdown. That is Python's answer, and for testbenches it is a fine one — until it isn't. Rust proposed something genuinely new: what if the *compiler* proved memory safety, at compile time, and the finished program paid nothing at all? No garbage collector, no interpreter, no runtime babysitter. The rules that make this possible — ownership and borrowing — are the subject of Chapter 5, and they will bend your brain exactly once, after which you will wonder how you ever tracked object lifetimes in your head.

Rust 1.0 shipped in 2015. Since then the language has spent year after year at the top of developer-survey "most admired" lists, and it has done something no other language managed in half a century: it convinced the Linux kernel, Windows, and Android teams to admit a second systems language into their codebases. That is not fashion. That is an industry deciding that compile-time correctness is worth learning something hard.

> ² The security community eventually put numbers on it: both Microsoft and Google's Chrome team reported that roughly 70% of their serious security bugs were memory-safety bugs — the exact category Rust eliminates at compile time.

## Why Rust for verification?

It is fair to ask why a verification engineer with a working Python flow should care. I will give you the honest engineering answer in four parts.

**Speed.** Python is an interpreted language, and for all of cocotb's cleverness, every signal read, every transaction compare, every scoreboard update runs through the interpreter. For the TinyALU it does not matter. For a regression farm running thousands of seeds against a large SoC, testbench overhead is real money and real schedule. Rust compiles to the same kind of native code as the simulator itself. When the testbench stops being the bottleneck, you buy back regression time without touching your license count.

**Correctness before simulation.** This is the deeper reason, and the one this book keeps returning to. In Python, a typo'd signal name, a transaction handed to the wrong port, a scoreboard mutated by two tasks at once — all of these are discovered *at runtime*, which in our world means *after the simulator license is checked out, the design is elaborated, and forty minutes have passed*. Rust moves an astonishing fraction of these discoveries to the compile step, which takes seconds and costs nothing. A TLM connection mistake that pyuvm reports as a runtime `UVMTLMConnectionError` simply does not compile in rustvm. The Python book warned you about a race between two tasks sharing `transaction_data` and taught you to avoid it by discipline. The Rust compiler *rejects that code*. Discipline is good; proof is better.

**Testbench refactoring without fear.** Verification code lives longer than we admit and gets modified by more hands than we would like. In Python, renaming a field means grepping and praying; the interpreter will tell you what you missed, one AttributeError at a time, over the following month. In Rust, the compiler produces the complete list of every place that must change, and the testbench does not build until you have addressed all of them. This changes how boldly you can improve old testbenches.

**One binary, no environment.** A Rust testbench compiles to a single library that the simulator loads. There is no interpreter version to match, no virtual environment to activate, no `pip install` on the farm machines. If it built, it runs.

And one more, which deserves its own paragraph: **unit testing without a simulator.** Because Rust testbench components are ordinary structs, the pure-software parts of your testbench — predictors, transaction operations, coverage logic — can be tested with `cargo test` in milliseconds, on your laptop, with no simulator license anywhere in sight. The Python stack could do some of this; the Rust toolchain makes it so frictionless that this book does it constantly, starting in Chapter 14.

## What it costs

I owe you the other side of the ledger, because there is one.

Rust is harder to learn than Python. Not a little harder — the ownership system is a genuinely new idea, and for your first weeks the compiler will reject code you are certain is fine. (It is almost never fine. This is the maddening part. Later it becomes the endearing part.) The Python book could teach its whole language in fourteen short chapters because Python works hard to be unsurprising; Rust holds opinions, and it holds them at compile time.

The edit-run loop is slower. Python starts instantly; Rust compiles first. For testbench work the compile is usually seconds, not minutes, and it replaces the forty-minute runtime failure — but the rhythm is different and you will feel it.

The verification ecosystem is younger. Python has cocotb, pyuvm, and years of conference papers; Rust verification is early. That is part of why this book exists — someone gets to write the early chapters of that story, and it may as well be us.

There is no REPL. Python let you poke at ideas interactively; Rust asks you to write a small program. The playground projects in Part I are this book's substitute, and honestly, `cargo` makes them cheap enough that you will not miss the prompt much.

If your testbenches are small, your regressions short, and your team fluent in Python, Python remains a fine answer, and I will not pretend otherwise. This book is for when one of those three stops being true.

## Code examples

The conventions are the ones you know. Every example has a figure number; code is followed by `--` and then its output. Examples live in the `rust4uvm_examples` repository, in directories named after their chapters, with instructions in each `README.md`. Early chapters use standalone cargo projects the way the Python book used Jupyter notebooks; from Chapter 17 on, examples are simulation directories.

We should not break a two-book tradition. In figure 1, we create a program with `cargo new`, Rust's project generator — meet `cargo` now, because like `pip`, `venv`, `make`, and `pytest` fused into one tool, it will be everywhere.

```text
# Figure 1: Creating our first program

% cargo new hello
    Creating binary (application) `hello` package
```

`cargo new` writes a tiny project containing `src/main.rs`, which is where figure 2 lives. Where Python let us type `print("Hello, world.")` naked at a prompt, Rust asks for a function — `fn main()` is where every Rust program begins. The exclamation point on `println!` marks it as a *macro* rather than a function, a distinction that will matter a great deal in Chapter 21 and not at all before then.

```rust
# Figure 2: The classic first program

fn main() {
    println!("Hello, world.");
}
```

```text
% cargo run
   Compiling hello v0.1.0
    Finished `dev` profile
     Running `target/debug/hello`
Hello, world.
--
Hello, world.
```

Notice what happened between `cargo run` and the greeting: a compile. Nothing about our two-line program was checked until we asked to run it — but *everything* about it was checked before it ran. Hold on to that trade; it is the whole book in miniature.

## The plan

Part I (Chapters 1–14) teaches the Rust you need, always against the Python you know: ownership where Python had garbage collection, traits where Python had inheritance, `Result` where Python had exceptions, `match` where Python had `if` chains and envy. Part II (Chapters 15–20) reaches the simulator: async/await — whose engine, you will be pleased to hear, works on the same principles as the cocotb scheduler you already understand — then triggers, signal handles, and testbenches 1.0 and 2.0. Part III (Chapter 21) is a single load-bearing chapter on macros, Rust's answer to decorators and metaclasses. Part IV (Chapters 22–39) rebuilds the UVM: components, the lifecycle, configuration, the factory's job (done, you may be surprised to learn, by closures), component communication, and sequences, ending at testbench 8.0 — the same summit as last time, by a steeper and more scenic route. Part V wraps the complete testbench into a reference template and looks ahead.

The TinyALU is waiting. It has not gotten any bigger, and this time, neither will our tolerance for runtime errors.

