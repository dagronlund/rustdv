# The prompt for the prose pass

*Paste everything below the line into a fresh Fable session opened on this repo.*

---

You are writing the prose for *Rust for RTL Verification* — 41 chapters, an
interlude, and appendices, in `book-pdf/src/`. The framework the book teaches
(`rustdv/`) is finished, every example runs, and the manuscript was written
against an earlier design that has since been reversed. Your job is the words.

## You wrote this book

The manuscript is yours. An earlier Fable session wrote all 41 chapters — and an
earlier Fable session also wrote the first version of the framework, the one that
removed the UVM's dynamic build/connect phases and its TLM FIFOs and then argued
in the book that they were unnecessary. Undoing that is why this pass exists.

Two consequences, one useful and one to guard against.

**The voice is your own, so preserving it costs you nothing.** You are not
imitating a stranger. When a paragraph reads well it reads well because you wrote
it well — keep it. This is the strongest reason to read a chapter before revising
it, and the reason the three modes below lean so hard on harvesting.

**The arguments you have to reverse are also your own, and you made them with
conviction.** Chapters 24, 27, 28, 29, 31, 32, 34, 36 and 39 argue for the design
that turned out to be wrong. The belief behind it was that the UVM's runtime
indirection was unsophistication to be compiled away; what it produced was a
testbench compiler rather than a verification framework, because removing late
binding is what made everything look statically decidable. Expect a pull to
defend those passages. When you feel it, the rule is the one in `FABLE.md`: the
code wins, and the code no longer does what the chapter claims.

Two ways that goes wrong, and both are worse than the original error:

- **Defending it.** "I argued this carefully" is not evidence. The framework
  changed; the argument lost. Reverse it cleanly.
- **Apologising for it.** Do not narrate the book's history, do not confess, and
  do not add a sentence explaining that an earlier version said otherwise. The
  reader does not know and has no reason to care. Write the correct argument as
  though it had always been the argument. The record of what went wrong belongs
  in the repo, and it is already there.

One specific blind spot: **your own verbal habits are invisible to you.**
`FABLE.md` bans "honestly", "genuinely" and "straightforward" because they were
overused, and when you meet one in your own prose you will read straight past it.
Grep for them instead of trusting your ear — and do the same for any phrasing you
catch yourself reaching for twice.

## The tone that ruined the last draft

The previous draft was obnoxious in its joy at how a heavily typed language
catches errors at compile time. Do not go back down that path. `FABLE.md` has the
rule under "Do not sell types, and do not disparage what came before" — read it
carefully, because this is the single thing Ray most wants changed.

The short version, and the reason it matters: **the reader already knows.** A
SystemVerilog engineer has lived with types for their entire career and hears
nothing new. A Python engineer either left types on purpose and can tell you why,
or is agitating for them right now — hints, `mypy`, gradual typing. Either way
they have thought about the trade-off longer than the paragraph you are about to
write. A book that keeps announcing the discovery is a book talking to itself.

Worse, the enthusiasm curdled into disparagement — of Python, of SystemVerilog,
of the UVM, of pyuvm. **Do not shit on what came before.** The UVM's runtime
indirection is not primitive: a statically-typed language chose it three separate
times with the static option sitting right there, and this project spent a whole
branch proving that judgement was correct. pyuvm's lack of typing was a
deliberate decision by the author of this book and it removed a bug class SV
still has. SystemVerilog's awkward corners follow from its object model, not from
anyone being slow. And the two earlier books are sources, not the baseline this
one beats.

Foils are fine — the book needs them, and two sharp comparisons beat none. The
banned move is the scoreboard: any sentence whose real payload is "and that is
why this is better." A useful test before you keep a comparison: would it leave a
pyuvm user feeling their tool is a toy, or a SystemVerilog engineer feeling
patronised? Then cut it.

This is also the one place your authorship works against you hardest. You wrote
the enthusiasm, so it will read to you as simply true rather than as a tone. Trust
the grep in the book map over your ear.

## Read first, and only these

1. `book-pdf/FABLE.md` — the rules, the reader, the argument, the voice, the
   claims that must not survive. Read it once, now, in full.
2. `book-pdf/chapter-notes.md` — one row per chapter. Read the header now; read
   a row when you reach that chapter, not before.

Those two files were written for you and are complete. **Do not read
`output/.design-decisions.md`** — it is 3,000 lines of internal history, and
everything in it you need has already been lifted into the two files above. If
you find yourself wanting it, that means the two files have a gap: say so, and
ask.

You may read anything under `rustdv/` and `output/examples/` — that is where the
truth is — and `../rustdv-reference/` for cocotb, pyuvm, four releases of the
SystemVerilog UVM, and the example code from both earlier books.

## Read the chapter you are about to work on — always

**This is a revision, not a from-scratch write.** The manuscript is 85,700 words
in an established voice — Ray's book, in your drafting: first-person, warm,
concrete, with jokes and worked pedagogy that took a long time to get right. A
chapter written without reading the existing one throws that away and replaces it
with competent generic prose, which is a much worse book. So: **before you write a word of a chapter, read the current
`book-pdf/src/chapter-NN-*.md` in full.**

Read it one chapter at a time, when you reach it. **Never read the manuscript in
bulk** — it does not fit in a budget, and a chapter you read six chapters early
you will have forgotten by the time you need it.

That rule is right for *writing* and wrong for *planning*, which is why Phase 0
below has you survey the whole book mechanically — grep and headings, not prose —
before you write anything. Some facts only exist across chapters, and you cannot
find them one chapter at a time.

Three modes, and `chapter-notes.md` tells you which applies:

- **Edit in place (all of Part I).** The technical content stands and the
  listings are frozen. You are making targeted changes to existing sentences —
  the recap blockquote, the Python references, the banned framings — and leaving
  everything else alone. Most of the chapter should survive verbatim. If you find
  yourself rewriting a paragraph that was not wrong, stop.
- **Rewrite, but harvest first (the reversed-argument chapters).** The argument
  is wrong at the premise, so it cannot be patched sentence by sentence. Read the
  whole chapter anyway, and take from it everything that is still good: the
  jokes, the analogies, the order the ideas arrive in, the sentence that explains
  the hard part well. Then build the new argument around what you kept. You wrote
  these chapters too — harvest generously, and defend nothing.
- **Update (the ordinary Part II chapters).** The prose mostly stands and the
  code moved under it. Read the chapter, read the example, and change what
  disagrees.

In all three: what you are preserving is voice and pedagogy; what you are hunting
is the reversed claims listed in `FABLE.md` and the banned framings. Preserving
a good sentence costs nothing and is the whole reason a human wrote it.

## The absolute rule

**You change no code. Ever.** You write only in `book-pdf/`. Off limits without
exception: `rustdv/`, `output/`, `sim/`, `skills/`, `toolchain-drop/`, and every
`.rs`, `.toml`, `.json`, `.sh`, `.py` and HDL file anywhere.

When code and manuscript disagree, **the code wins** — the examples are verified
running and the manuscript is known stale. If an example looks wrong, stop and
tell Ray. Do not "fix" it. Transcripts are copied verbatim from the chapter
READMEs in `output/examples/` (and from `STATUS.md` for the shipped testbench);
never retype, tidy, invent, or regenerate one, and never run a simulator.

Do not run the regression. Do not try to render the book — mdBook is not
installed here.

## The budget, and why it shapes the order

There is a fixed budget for this pass, and it may run out before the book is
finished. If it does, **Opus 5 takes over from your notes.** So the work is
ordered to put every judgement call early and every mechanical repetition late:

- **Front-load the thinking.** Decide the voice, settle the arguments, and write
  one full-quality *exemplar* for each kind of chapter before touching the bulk.
- **Leave patterns, not problems.** Opus is good at "do to ch6 what was done to
  ch5." It should never have to re-derive what the book is arguing.
- **Never leave a chapter half-written.** Finish the one you are in before
  starting the next, so a takeover always begins at a clean boundary.
- **Keep `book-pdf/HANDOFF.md` current after every single chapter.** One
  paragraph is enough. This is the only thing standing between a mid-sentence
  stop and a wasted week. Assume you will not get a warning.

## Phase 0 — survey and plan, then ask for the figure numbers (before any prose)

Produce three files.

**A. `book-pdf/figure-plan.md`** — the inventory and the numbering request.

Today `Figure N` means a code listing, so the book has no free word for an
actual drawing, which is why the pipeline in ch31, the architecture in ch34 and
the sequencer handshake all live as ASCII art inside code comments. The likely
answer is to split the two numbering spaces — **Example N** for a code listing,
**Figure N** for a drawing, numbered independently — so a chapter can say
"Example 5 wires the FIFO; Figure 5 shows the topology."

Collect the inventory with `grep`, not by reading chapters. The captions are
mechanical — `// Chapter N, Figure M:` in the example crates, `Figure N` in the
manuscript, and a figure table in each `output/examples/*/README.md` — so sweep
them with a pattern and count. Phase 0 is the one part of this job where reading
prose would be waste; you will read every chapter properly when you write it.

Include:

1. Every chapter, its code listings, and their current caption numbers.
2. Your recommendation on the split, including the honest option of leaving it.
3. Whether Part II+ wants drawings at all, and where each one would go.
4. **The exact renumbering you want**, precise enough to execute: file, current
   caption, new caption.
5. Anything else you want renamed on the code side while someone is in there —
   for instance, `SUMMARY.md` still calls ch37 "Fibonacci Testbench: 7.1" and
   ch38 "get_response Testbench: 7.2", which is backwards: ch37 is the repair
   desk and ch38 is Fibonacci. Chapter titles are yours to fix; the manuscript
   *filenames* still carry the old order, and renaming those is a request.

Hand the figure plan and the book map to Ray and **wait**. He will have the code
captions renumbered for you. You never perform the rename — captions live in `.rs` comments. Do not
begin a chapter whose numbers are about to move.

**B. `book-pdf/BOOK-MAP.md`** — the cross-chapter survey, and the reason a
successor can work one chapter at a time.

Everything in it is findable with `grep` and a look at each chapter's headings.
**Do not read chapters in full for this.** What it must contain:

1. **Every banned framing, with file and line.** The list is in `FABLE.md`:
   "the Python book taught you", "as you learned in", "you remember" + a
   Python/pyuvm/cocotb referent, "the last book", "your Python testbench", "In
   Python we", second-person Python nostalgia. This is the punch list, and it is
   the single most mechanical thing in the job — which makes it exactly what a
   successor should inherit rather than re-derive.
2. **Every recap blockquote**, with its chapter and what it currently recaps.
   They all become `> **In the UVM...**`, and knowing how many there are and
   where tells you the size of that job.
3. **Every "the Python book" / "In Python" passage, classified** keep / pair /
   neutralise, one line of reasoning each. There are roughly 204 of them. This
   is a judgement call that depends on the *balance* across Part I — too many
   keeps and the book still assumes a Python reader, too many neutralisations
   and it reads as list-making with no foil at all. It cannot be decided one
   chapter at a time, and it is the decision most likely to be made
   inconsistently by a successor who was not told.
4. **Callbacks and running examples that cross chapters** — a joke set up in one
   chapter and paid off in another, a promise made in ch24 and kept in ch34, an
   example that recurs. These are what break silently when chapters are revised
   in isolation, and nothing in the code will warn you.
5. **What ch1's typing pitch has to cover**, derived from the claims later
   chapters actually make. Write ch1 against this list, not from memory.
6. **Word count per chapter**, so a successor can tell a 1,200-word chapter from
   a 4,000-word one before opening it.
7. **Your own tics, counted.** Grep the manuscript for "honestly", "genuinely",
   "straightforward" and any other phrasing that turns up far more often than a
   human would choose it, and record the counts and locations. You wrote this
   book, so these are invisible to you from the inside — a count is the only
   honest way to see them, and a successor with the list can clear them without
   having to develop an ear for your habits.
8. **Every place the book sells types or talks down about Python, SystemVerilog,
   pyuvm, cocotb or the UVM.** Sweep for the vocabulary this tone travels in —
   "compile time", "compiler catches", "at compile time rather than", "type
   safety", "unlike Python", "SystemVerilog cannot", "primitive", "clumsy",
   "awkward", "verbose", "boilerplate", "finally", "no longer have to", "for
   free" — and list each hit with its chapter and a one-word verdict: *keep*
   (a real, specific, load-bearing observation), *soften* (true but pleased with
   itself), or *cut* (a scoreboard sentence). This is the punch list Ray cares
   most about, it is the one you are least able to see by reading, and it is
   perfectly suited to a successor working from a list.

Review this with Ray alongside the figure plan. It is the game plan; the
per-chapter routine is only the execution of it.

**C. `book-pdf/HANDOFF.md`** — start it now, in this shape, and keep it alive:

```
# Handoff

## Durable decisions (append-only; a successor obeys these)
- <every judgement call you make, one line each, with the reason>

## Exemplars (the pattern to copy)
| Archetype | Exemplar chapter | What to imitate in it |

## Progress
| Chapter | State | Notes |
(state = untouched / drafted / done)

## Next action
<the single next thing to do, specific enough to start cold>

## Where the cross-chapter answers live
<`BOOK-MAP.md` sections a successor must consult before touching a chapter, and
anything you have since learned that belongs in it — keep it a live document,
not a Phase 0 artifact that rots>

## Open questions for Ray
<anything you could not settle; never guess and write it as settled>
```

## Phase 1 — the expensive thinking (do all of this before Phase 2)

Work in this order. Each item either fixes the book's argument or creates a
pattern the rest of the book copies.

1. **Chapter 1.** It carries the typing theme once, for the whole book, as a
   dual pitch. Everywhere else: show, don't sermonize. Get this right and every
   later chapter gets shorter, because it can stop re-arguing.
2. **The rustdv catalogue** — the `prelude::*` debt in `FABLE.md`. A chapter (or
   a substantial opening section) before the first example that uses rustdv's
   surface, plus `ctx` its own section. This gates all of Part II.
3. **Appendix D: What rustdv Provides** — at least its structure and the entries
   for everything ch15–ch20 touch. Fill it in as you go afterward.
4. **One Part I exemplar: chapter 5, Ownership.** The full dual-audience
   treatment. Part I's listings are frozen — `book-sync` compares them verbatim
   and the pre-push hook enforces it — so this is purely prose, and it is the
   template for the other thirteen. Record in `HANDOFF.md` exactly what you did:
   how you rewrote the recap blockquote, how you judged each Python reference,
   what you cut — and **roughly how much of the original survived untouched**.
   That last number is the calibration a successor needs most: it is the
   difference between "make these edits" and "rewrite this chapter," and getting
   it wrong in either direction is expensive.
5. **The chapters whose argument is reversed**, in this order: **ch24, ch27,
   ch31, ch32, ch36, ch39**, then ch28, ch29, ch34. These cannot be edited
   sentence by sentence — they argue *for* what the framework now does the
   opposite of, so they are rewrites. This is the most conceptually expensive
   work in the book and the least suitable for a successor. Do it while you can.
6. **One Part II exemplar of the ordinary kind: chapter 26, Logging.** A chapter
   with a working example, no reversed argument, and nothing new to invent.
   Whatever routine you use — read the row, read the example crate, read its
   README, write, copy the transcript — write that routine down in
   `HANDOFF.md`. Steps 1–5 are the hard part; this is the assembly line, and
   Opus will run it.

## Phase 2 — the assembly line

Everything left, in `SUMMARY.md` order: Part I chapters 2–4 and 6–14, then the
Interlude, then the remaining Part II chapters, then ch40, ch41, and appendices
A–C. Follow your own recorded routine, working from `BOOK-MAP.md`'s punch list
for each chapter. Update `HANDOFF.md` after each one, and tick items off the map
as you clear them — a successor should be able to see what is left without
opening a chapter.

Two notes. **ch41** is the missing-pieces inventory: the framework changed under
it, so ask Ray for the current list rather than inferring one — an inventory that
names the wrong gaps is worse than none. **The Interlude and ch40** both present
the shipped testbench in `rustdv/tinyalu_tb/`, which was converted on 2026-07-29;
its transcript and counts are in the last entry of `STATUS.md`.

## How to be efficient

- One chapter at a time, and read exactly five things for it: its row in
  `chapter-notes.md`, its lines in `BOOK-MAP.md`, its current `book-pdf/src/`
  file, its example crate, and that crate's `README.md` for the transcript.
  Nothing else. The map is what lets that short list be enough — every fact that
  lives outside the chapter is already in it.
- Do not re-read `FABLE.md` per chapter. Its rules go in your head once; if you
  need a reminder, your own `HANDOFF.md` should have it.
- Do not summarise your work back to Ray at length. A finished chapter and a
  one-line `HANDOFF.md` update is the report.
- Do not hedge in the prose. Say the thing. The book's voice is warm,
  first-person, concrete, and honest about costs — and avoid "honestly",
  "genuinely", "straightforward".
- When you are unsure, ask. Do not invent an answer and write it as settled:
  that is exactly how the design this book teaches went wrong the first time.

Start with Phase 0. When `figure-plan.md` is ready, stop and tell Ray.
