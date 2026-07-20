# Chapter 1: Why Rust?

*Rust for RTL Verification* is a book for verification engineers who want to write testbenches in Rust. It teaches the language from zero — not one line of prior Rust is assumed — and then uses it to build rustdv, a UVM-style verification framework, culminating in a complete, running testbench for a small ALU. If you have written UVM testbenches in any language, this book was written for you.

Verification engineers come to Rust from two directions. Most write SystemVerilog UVM testbenches and have spent years with `uvm_config_db`, `type_id::create()`, and the sequencer handshake. A growing number write Python testbenches with cocotb and pyuvm, and know the same methodology by its Python names. This book addresses both of you, usually at the same time, because you share the thing that matters: you know what a driver is for, why a scoreboard subscribes to a monitor, and what the factory buys you. That shared knowledge — the UVM, not any particular language — is the ground this book builds on.

What the book assumes, then, is verification: the UVM's concepts and vocabulary. If you have that from work, you are ready. If you want to build it first, two earlier books in this series teach it from first principles: [*The UVM Primer*](https://www.uvmprimer.com) in SystemVerilog and [*Python for RTL Verification*](https://a.co/d/0hTKAJvh) in Python. Either one prepares you for this book. Neither is required.

A note on what you do *not* need: any Rust. Not one line. If you have heard alarming rumors about a thing called the borrow checker, you have heard correctly, and we will make friends with it in Chapter 5.

## Two revolutions, thirty years apart

The first revolution is the one you lived through, or inherited. In the 1990s, verification engineers realized that testbenches are not test fixtures but *software* — software that happens to talk to a simulated design. That realization gave us the verification languages (e, Vera, SUPERLOG), then SystemVerilog, then a decade of methodology wars that ended with every EDA vendor blessing a single winner: the Universal Verification Methodology. Later the same realization pushed further — if testbenches are software, why not write them in a general-purpose software language? — and produced cocotb and pyuvm, which moved testbenches off the simulator and into Python. Every step followed the same moral: testbenches are software, and software deserves a software language.

Rust's story rhymes with it. In 2006, a Mozilla engineer named Graydon Hoare started a personal project to answer an uncomfortable question: why, decades into the software era, were our foundational programs — browsers, kernels, the code we bet everything on — still written in languages that let one stray pointer corrupt everything? C and C++ were fast because they trusted the programmer completely, and every security bulletin showed what that trust cost.¹

The conventional answer was garbage collection: let a runtime babysit memory, and accept the slowdown. That is Python's answer, and SystemVerilog's too — class objects in SV live until the last handle drops, collected automatically, exactly like Python objects. Rust proposed something genuinely new: what if the *compiler* proved memory safety, at compile time, and the finished program paid nothing at all? No garbage collector, no interpreter, no runtime babysitter. The rules that make this possible — ownership and borrowing — are the subject of Chapter 5, and they will bend your brain exactly once, after which you will wonder how you ever tracked object lifetimes in your head.

Rust 1.0 shipped in 2015. Since then the language has spent year after year at the top of developer-survey "most admired" lists, and it has done something no other language managed in half a century: it convinced the Linux kernel, Windows, and Android teams to admit a second systems language into their codebases. That is not fashion. That is an industry deciding that compile-time correctness is worth learning something hard.

> ¹ The security community eventually put numbers on it: both Microsoft and Google's Chrome team reported that roughly 70% of their serious security bugs were memory-safety bugs — the exact category Rust eliminates at compile time.

## Why Rust for verification?

It is fair to ask why a verification engineer with a working flow should care. The honest answer depends on which flow you are working in, so here it is twice.

**If you come from Python:** speed and proof. Python is an interpreted language, and for all of cocotb's cleverness, every signal read, every transaction compare, every scoreboard update runs through the interpreter. For a small ALU it does not matter; for a regression farm running thousands of seeds against a large SoC, testbench overhead is real money and real schedule. Rust compiles to the same kind of native code as the simulator itself. And where Python discovers a typo'd signal name or a mis-wired analysis port *at runtime* — after the license checks out, the design elaborates, and forty minutes pass — Rust discovers it at compile time, in seconds.

**If you come from SystemVerilog:** you already live the typed life, so nobody needs to sell you on declarations. Here is the part your compiler has not been telling you. SystemVerilog is statically typed, yet its methodology defers an embarrassing number of checks to runtime: a `uvm_config_db#(T)::get()` with the wrong type fails at runtime. A `$cast` of the wrong transaction fails at runtime. A null virtual interface fails at runtime. A factory override by string fails — silently or loudly, but at runtime. A TLM port left unconnected reports at `connect_phase`, which is to say, at runtime. In rustdv, every one of those is a compile error, produced in seconds, before any simulator license is touched. Your typed language stops checking too early; Rust's compiler goes the distance. This book demonstrates that claim rather than repeating it: throughout Parts II through V you will find figures showing testbench mistakes — a mis-connected port, a conflicting configuration, a typo'd field — caught as compile errors, with the compiler frequently suggesting the fix.

The rest of the ledger serves both readers:

**Testbench refactoring without fear.** Verification code lives longer than we admit and gets modified by more hands than we would like. Rename a transaction field, and the Rust compiler produces the complete list of every place that must change; the testbench does not build until you have addressed all of them. This changes how boldly you can improve old testbenches.

**Unit testing without a simulator.** Because Rust testbench components are ordinary structs, the pure-software parts of your testbench — predictors, transaction operations, coverage logic — can be tested with `cargo test` in milliseconds, on your laptop, with no simulator license anywhere in sight. If you have ever queued for a license to test a scoreboard change that never touches a signal, this feature alone may justify the book.

**One binary, no environment.** A Rust testbench compiles to a single library that the simulator loads. There is no interpreter version to match, no virtual environment to activate, no `pip install` on the farm machines — and no vendor-specific class-library quirks, because the testbench's correctness was settled by the compiler before any vendor tool saw it. If it built, it runs.

**An open toolchain.** The Rust compiler, the cargo build system, the package ecosystem, and rustdv itself are free and open source. The simulator is the only licensed tool left in the loop — and this book's examples run on Icarus Verilog, which is also free.

## What it costs

I owe you the other side of the ledger, because there is one.

Rust is hard to learn. Not a little hard — the ownership system is a genuinely new idea, and for your first weeks the compiler will reject code you are certain is fine. (It is almost never fine. This is the maddening part. Later it becomes the endearing part.) If you come from Python, you are trading a language that works hard to be unsurprising for one that holds opinions and holds them at compile time. If you come from SystemVerilog, take heart: you have already mastered a language assembled from Verilog, Vera, and SUPERLOG by committee, and Rust is smaller, more consistent, and better documented than what you already know. Different, though. Chapter 2 maps what carries over and what must be unlearned, for both of you.

The edit-run loop includes a compile. For testbench work the compile is usually seconds, not minutes, and it replaces the forty-minute runtime failure — but the rhythm is different and you will feel it.

The verification ecosystem is younger. SystemVerilog has two decades of UVM infrastructure; Python has cocotb, pyuvm, and years of conference papers; Rust verification is early. That is part of why this book exists — someone gets to write the early chapters of that story, and it may as well be us.

If your testbenches are small, your regressions short, and your team fluent in its current language, that language remains a fine answer, and I will not pretend otherwise. This book is for when one of those stops being true.

## Code examples

Every example has a figure number; code is followed by `--` and then its output, and every transcript in this book is genuine tool output. Examples live in the book's `rustdv-examples` repository, in directories named after their chapters, with instructions in each `README.md`. Early chapters use small standalone cargo projects as playgrounds; from Chapter 17 on, examples are simulation directories.

Our running example is the TinyALU: a two-input ALU with a `start`/`done` handshake, small enough that the testbench, not the design, stays the subject. Readers of the earlier books will recognize it; Chapter 18 specifies it fully, so newcomers lose nothing. By the final chapter you will have built its testbench eight times, versions 1.0 through 8.0, each version adding one architectural idea — the same climb the earlier books made, now with a compiler on the rope team.

We begin where every programming book begins. In figure 1, we create a program with `cargo new`, Rust's project generator — meet `cargo` now, because it is the package manager, build system, test runner, and project generator fused into one tool, and it will be everywhere.

```text
# Figure 1: Creating our first program

% cargo new hello
    Creating binary (application) `hello` package
```

`cargo new` writes a tiny project containing `src/main.rs`, which is where figure 2 lives. Rust asks even the smallest program for a function — `fn main()` is where every Rust program begins. The exclamation point on `println!` marks it as a *macro* rather than a function, a distinction that will matter a great deal in Chapter 21 and not at all before then.

```rust
// Figure 2: The classic first program

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

Part I (Chapters 1–14) teaches the Rust you need, always against the languages you know: ownership where they had automatic collection, traits where they had inheritance, `Result` where they had exceptions or error signals, `match` where they had `case` and `if` chains. Part II (Chapters 15–20) reaches the simulator: async/await — the same coroutine idea that powers both SystemVerilog's tasks and cocotb's scheduler, except that Rust hands you the engine — then triggers, signal handles, and testbenches 1.0 and 2.0. Part III (Chapter 21) is a single load-bearing chapter on macros, Rust's answer to `uvm_component_utils` and decorators alike. Part IV (Chapters 22–39) rebuilds the UVM: components, the lifecycle, configuration, the factory's job (done, you may be surprised to learn, by closures), component communication, and sequences, ending at testbench 8.0. Part V wraps the complete testbench into a reference template and looks ahead.

The TinyALU is waiting. It is a very small design, and this book holds a very small tolerance for runtime errors.
