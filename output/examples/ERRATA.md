# Errata found while making the figures runnable

**Status: all three fixed in `book-pdf/src` on 2026-07-08.** The examples
now match the manuscript verbatim again (no marked deviations remain). Kept
as a record of what changed and why. Remember to rebuild the book
(`mdbook build`) and regenerate the PDF.

## 1. Chapter 9, Figure 2 — the divide-by-zero doesn't panic; it doesn't compile

The book shows `let divisor = 0;` followed by `3 / divisor` panicking at
runtime. rustc const-propagates the zero and rejects the program at *compile
time* with `error: this operation will panic at runtime` (the deny-by-default
`unconditional_panic` lint).

That is arguably an even better story for this chapter (the compiler catches
what Python/SystemVerilog leave to runtime), but the shown transcript is
wrong for the shown code. Options for the book: (a) keep the code and show
the compile error instead, or (b) make the divisor opaque so the panic really
happens — the example uses `let divisor: i32 = "0".parse().unwrap();`
(`parse` was introduced in Chapter 3, Figure 8).

## 2. Chapter 12, Figure 6 — `ii.pow(3)` fails type inference (E0689)

Despite the `HashMap<u32, u32>` annotation on `cubes`, rustc cannot resolve
`.pow` on the closure parameter: method calls need the receiver's concrete
type at check time, and inference hasn't flowed back through
`collect()`/`map` yet. The figure as printed fails with `error[E0689]: can't
call method pow on ambiguous numeric type {integer}`.

Fix for the book: annotate the closure parameter — `.map(|ii: u32| (ii,
ii.pow(3)))` — or write the range as `(0u32..4)`. The example uses the former.

## 3. Chapter 8, Figures 1 and 4 — the printed Debug output is wrong

The transcripts show `op: Ops::Add`, but `#[derive(Debug)]` prints enum
variants *without* the type path — the real output is `op: Add`. Three lines
to fix in `chapter-08-collections.md`, all in transcripts, none in code:

- line 43 (Figure 1's output): `first: AluCommand { a: 5, b: 3, op: Ops::Add }`
- lines 140–141 (Figure 4's output): `op: Ops::Add` and `op: Ops::Mul`

Everything else that writes `Ops::` is correct: source code constructs
variants with the full path, and rustc itself prints full paths in error
messages (Figure 2's E0382 quote, Chapter 7's E0004 `Ops::Sub`). Figure 7's
transcript (`Mul: 2`) and Chapter 10 Figure 6's (`op=Xor`) already have it
right.

## Not errata, but noted

- Chapters 4 and 12 figures that print with `print!("{} ", n)` emit a
  trailing space the book's transcripts trim. Harmless; `check.sh` ignores it.
- Several figures produce `dead_code` warnings (unused enum variants/fields).
  Expected — the figures define more than they use.
