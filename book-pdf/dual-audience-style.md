# Dual-Audience Style Sheet

*Internal editorial rules for the dual-audience edit. Not part of the rendered
book (not in SUMMARY.md). Governs every prose change; figures never change.*

## The reader

A verification engineer who knows the UVM — from SystemVerilog at work, from
pyuvm, or from one of the two prior books. Assume fluency in verification and
UVM concepts (driver, monitor, scoreboard, sequence, factory, config database,
analysis port). Assume **no** Python beyond reading ability, **no**
SystemVerilog beyond reading ability, and **no** prior book. The two prior
books are recommendations for learning the UVM, never prerequisites.

## The recap device

Old: `> **In Python we...**` (37 instances, Parts II–V).
New: `> **In the UVM...**`

Written in UVM API terms, which pyuvm and SV-UVM share (`start_item`,
`get_next_item`, `raise_objection`, analysis ports). Where dialects differ,
one parenthetical, SV first (larger audience), pyuvm second:
*(SV: `uvm_config_db#(int)::set`; pyuvm: `ConfigDB().set`)*. Never two
parentheticals in one sentence — if the dialects diverge that much, pick the
concept-level description instead.

## Three treatments for existing Python references

Judge each of the ~204 "the Python book" / "In Python" passages individually:

1. **Keep** — the Python fact is shared context any engineer can read, and it
   illuminates Rust. Rewrite so it never assumes the reader has *lived* it:
   "In Python, a typo'd attribute is a runtime AttributeError" (fine) vs. "as
   you remember from the Python book" (banned).
2. **Pair** — add the SV analog when it's as sharp or sharper. Prefer pairing
   in Part I (see table in the edit plan §2 Move 3). Order: SV first or
   Python first, whichever contrast teaches Rust faster — not mechanically.
3. **Neutralize** — say it in Rust's own terms when the comparison was only
   scaffolding. Prefer this when a pass reads as list-making.

Rule against blandness: prefer two sharp comparisons over zero. Dual-audience
means both foils, not no foil.

## The typing theme

Stated once, in Chapter 1, as a dual pitch:

- To the Python-fluent reader: your runtime errors move to compile time.
- To the SV-fluent reader: your typed language still defers its worst checks
  to runtime — `$cast`, `uvm_config_db#(T)::get`, null virtual interfaces,
  string-keyed factory overrides, TLM connection errors at `connect_phase`.
  Rust's compiler stops all of these before elaboration.

Everywhere else: **show, don't sermonize.** Compile-fail figures stay; the
"this is the book's deeper reason" reprises go. A compile error speaks for
itself once Chapter 1 has framed it. Delete sentences whose only job is to
re-argue that compile-time checking is good.

## Banned framings (grep-lint list)

- "the Python book taught you" / "as you learned in the Python book"
- "you remember" + any Python/pyuvm/cocotb referent
- "the last book" / "last time" meaning the Python book
- "your Python testbench" / "the testbench you wrote" (they may not have)
- "In Python we" (the old recap label; also as prose opener)
- Any sentence requiring the reader to have run cocotb/pyuvm to parse it
- Second-person Python nostalgia ("you have created millions of objects in
  Python")

Allowed: naming the books as *sources* ("pyuvm, which *Python for RTL
Verification* teaches"), historical attribution, Appendix B's existence.

## SystemVerilog quotations

- Only from `/reference/uvmprimer/` (vendored from the uvmprimer repo `claude`
  branch, source commit recorded in `/reference/uvmprimer/PROVENANCE.md`).
- Short (≤ ~15 lines), labeled: `// From The UVM Primer examples,
  23_UVM_Sequences/tb_classes/add_sequence.svh`.
- Never accompanied by simulation output. Quotations, not claimed-run figures.
- Budget: 5–8 across the whole book. Each must beat the prose alternative.
- regress.py sync-checks quotes against the vendored files.

## Voice

Unchanged. Warm, first-person, concrete, honest about costs. Jokes stay.
"In Python we..." chapter openers are no longer a convention (supersedes
TOUR.md's book-voice note — update TOUR.md in the final pass). Figure
conventions unchanged: `// Figure N:` captions, output after `--`.

## Mechanical invariants

- No figure, transcript, code block, or figure number changes. If an edit
  seems to require one, stop and flag it.
- Chapter files keep their names; SUMMARY.md order unchanged (Appendix C adds
  one line).
- book.toml description/subtitle: "A complete course in Rust and rustdv."
- After each part: run regress.py (all 107 figure-sync checks must pass) and
  the banned-framings grep.
