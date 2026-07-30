You wrote *Rust for RTL Verification* — 40 chapters, the Interlude, the Toolkit
page and Appendices A–D, in `book-pdf/src/`. That pass is complete and you
declared it so. You are being brought back to close it out.

**Read these three, in this order, before touching anything:**

1. **`book-pdf/HANDOFF.md`** — your own handoff. Go to **"Next action"**; it has
   been updated by a code thread and now carries the true status of your three
   closing steps plus one new item. The "Progress" table above it is still your
   per-chapter record.
2. **`book-pdf/chapter-notes.md`** — the per-chapter notes. A **cross-cutting
   note now sits at the head of the Part II table**; read it before ch21 or ch24.
3. **`book-pdf/FABLE.md`** — the rules. Unchanged. The banned framings still
   apply, and "Do not sell types, and do not disparage what came before" still
   governs everything.

You do not need `fable-prompt.md` — its router would send you here anyway.

## What changed while you were away

**The caption pass ran.** `renumbering-spec.md` was applied to the `.rs` captions
in ch27, ch28, ch31, ch32, ch34, ch36, ch37, ch39. Every edit was line-count
neutral, so **every `file:line` your manuscript embeds is still correct.** Your
step 2 is done.

**The child attribute lost its argument (D114).** `#[component(child)]`,
`#[component(fifo)]` and `#[component(sequencer)]` are gone; it is bare
`#[component]`. The derive never read the word. Your manuscript prints the old
form in 17 files.

## What to do

**1. The `#[component]` sweep — 17 files.** Fifteen are substitution inside code
blocks: the Interlude and ch25, 26, 27, 28, 29, 30, 31, 32, 34, 36, 37, 38, 39,
40. **Two are real writing:**

- **ch21** teaches `#[derive(Component)]`. Whatever it says the argument
  selects, that explanation has lost its subject.
- **ch24** introduces the attribute to the reader. The rule is now the whole
  syntax: `#[component]` marks a field as a child in the component tree.

Do not mention this. The reader never knew it existed.
The reasoning, if you want it, is `output/.design-decisions.md` §42.

**2. The end-to-end read** against rendered output — your step 3, and the last
thing. `mdbook build book-pdf` works in the sandbox; PDF rendering does not
(no Chromium), so read the HTML.

**3. Fill the 13 `[TRANSCRIPT NEEDED]` markers.** The sims have been rerun
(2026-07-30) and **every transcript you were owed is now in the chapter's
README**, verbatim, under a "Transcript(s)" heading with the figure number on
it. Copy them across character for character. Do not compose, approximate, or
reconstruct one from STATUS.md.

| Chapter | Markers | Where the output now lives |
|---|---|---|
| ch27 | 4 (figs 4, 7, 10, 13) | `output/examples/ch27-configuration/README.md` |
| ch28 | 1 (fig 7) | `output/examples/ch28-config-debugging/README.md` — figs 7, 10, 12 all there |
| ch36 | 1 (fig 9) | `output/examples/ch36-sequence-testbench-7.0/README.md` |
| ch37 | 1 (fig 5) | `output/examples/ch37-repair-desk-testbench-7.1/README.md` |
| ch38 | 1 (fig 6) | `output/examples/ch38-fibonacci-testbench-7.2/README.md` |
| ch39 | 3 (figs 3, 5, 9) | `output/examples/ch39-virtual-sequence-testbench-8.0/README.md` |
| Interlude, ch40 | 1 each | `sim/run_rustdv.sh` output — ask Ray; it is the shipped testbench and its README is the crate itself |

**The figure maps in those READMEs were rebuilt too**, and they now use the
**book's** numbering rather than the crate's. So a README row number is the
figure number you should be printing. That was not true this morning.

**ch15–21 have been regenerated too**, so every README in the book is now
current. Two things there may affect text you already wrote:

- Their `src/lib.rs` citations are gone — the crate roots are named after the
  crate (D29). If any ch15–21 prose names a path, check it.
- **ch18, ch19 and ch20's transcripts changed.** Their simulated times were 5ns
  early throughout (ch18's `35.00ns` is now `40.00ns`, and so on). Same
  operands, same results — the DUT self-clocks now (D112), so the first edge
  lands later. If those numbers appear in your prose, they are stale.

## Standing constraints

- **You edit `book-pdf/src` and nothing else.** No code, no `output/`, no
  crates. If you find something the code must fix, write it in
  `chapter-notes.md` and say so out loud.
- **ch1–14 listings are frozen.** `book-sync` compares them byte-for-byte
  against the example files and the pre-push hook runs it. Prose around the
  blocks is yours; a character inside one turns the suite red.
- **Code blocks come from the crate, never from the old manuscript.** The crates
  under `output/examples/` and `rustdv/tinyalu_tb/` are the truth.
- Keep `HANDOFF.md` current as you go — it is how the next reader, including a
  future you, knows where this stands.
