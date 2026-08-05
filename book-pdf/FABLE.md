# Brief for the prose pass
# Notes for RustDV Book.

# Chapter 32 — finish it after the rename thread lands

The chapter was restructured on 2026-08-05: the publisher/many-subscribers
concept first, then the two ports, the `write` trait, a `RustdvShared`
digression, the enrollment-versus-connection split, and only then the
counter/collector example, the storing-nothing bus, and the slow-subscriber
section. The bones are right. What remains is blocked on a **code thread**
(the work order is the ch32 row and long-form items in
`book-pdf/chapter-notes.md`), and the prose must follow it:

* `WriteSink` becomes `Subscriber`, and `on_write()` becomes `subscribe()`.
  Re-paste every affected listing, then rewrite the subscriber sections in the
  new vocabulary: **the plain struct with `write()` is the subscriber; a
  component hosts it.** The parent `connect`s (which stream); the component
  `subscribe`s (which receiver). Stop calling the hosting component "the
  subscriber", and say in one sentence that `uvm_subscriber` is a component
  while rustdv's `Subscriber` is plain data — the same lesson the chapter
  already teaches about where storage lives.
* The identifier `analysis_fifo` appears nowhere. The crate's bus fields are
  renamed; re-paste Figures 4, 6 and 9.
* The `TlmFifo` tap demonstration (removed from ch31 on 2026-08-05) lands at
  the end of ch32 once the code thread moves the example into the ch32 crate.
  Teach it as the port of `uvm_tlm_fifo`'s built-in analysis ports,
  `TlmFifo::put_ap()` / `TlmFifo::get_ap()`: the data path is still a queue —
  one consumer takes each item, the producer blocks when full — while the taps
  are observation alongside; every subscriber sees every item, nothing is
  consumed, nobody is delayed. The move also changes every ch31 transcript's
  test count, so ch31's transcripts get re-pasted from the regenerated README.

Do not start until `python3 output/regression/verify-book-listings.py` and
`bash output/regression/verify-transcripts.sh` are green against the renamed
crates.

# Buy me a coffee

I want a Buy Me a Coffee link at the top of every page of the HTML version of the book. I want to put https://buymeacoffee.com/raysalemi in the header of every page.

# Final step

The book is ready to go to market.  Make one final complete sweep of the book to make final edits for its publication.


---

## The absolute rule: you change no code

You edit **`book-pdf/src/*.md`**, `book-pdf/src/SUMMARY.md`, and the planning
files this brief asks you to create. Nothing else.

Off limits, no exceptions: `rustdv/`, `output/`, `sim/`, `skills/`,
`toolchain-drop/`, any `.rs`, `.toml`, `.json`, `.sh`, `.py`, or HDL file.

**When the code and the chapter disagree, the code wins.** The examples are
verified running; the manuscript is known stale. If an example looks wrong, or
a chapter needs a figure that does not exist — **stop and tell Ray.** Do not
edit the example to match your prose. That is the failure this whole project
exists to undo: prose driving design.

**Transcripts are copied, never composed.** Take them verbatim from the
chapter's `output/examples/*/README.md`. Do not retype, reformat, tidy, invent,
or regenerate them, and do not run a simulator. A transcript that looks wrong
is something you report.


---

## Voice

The model is *Python for RTL Verification*, in `../rustdv-reference/salemi_books/`.
Read enough of it to absorb how it teaches, and write like that. What follows is
not a style guide — you have the source — it is the two or three places this book
must differ from it.

**The reader.** That book was written for Python engineers. This one addresses a
verification engineer who knows the UVM from *either* SystemVerilog or Python,
with reading ability in both and no prior book. So nothing may assume the reader
has run cocotb or pyuvm, or has read the earlier books. Naming them as sources is
fine; leaning on shared memory of them is not.

**The reader has no history with rustdv, and rustdv has no history worth
telling.** Never explain the present design by contrast with an earlier design,
an earlier draft, an earlier name, or "a previous version of the testbench"
(the reader's own TB 2.0–8.0 climb is fine — that is their history). Explain
what happens now, plainly, as if it were always so.

**Comparisons are foils, not nostalgia.** "In Python, a typo'd attribute is a
runtime `AttributeError`" is useful. "As you saw in the Python book" is not.
Prefer two sharp comparisons to none — a dual audience means both foils, not no
foil — and where the dialects diverge, one parenthetical, SystemVerilog first:
*(SV: `uvm_config_db#(int)::set`; pyuvm: `ConfigDB().set`)*.

**SystemVerilog quotations** come only from `../rustdv-reference/uvmprimer-master/`,
run ≤15 lines, carry a source label, never appear with simulation output, and
total a handful across the whole book.

**Do not celebrate the compiler.** No "promise kept", no "the seam at work", no
selling types. State what a mechanism does and move on; where a compile error
is real, show it and let it speak.

**Say the point plainly.** "Honestly", "genuinely" and "straightforward" read as
persuasion rather than statement.

**Answer, then stop.** When Ray asks a question, give the verdict and the one
or two reasons that decide it. No essays, no surveys of alternatives he did not
ask for, no narration between steps of a task.


## When you are unsure

Ask Ray. Do not invent an answer and write it as settled — that is precisely how
the original design failed. This applies to anything in `chapter-notes.md`
marked **ASK RAY**, to the caption question above, and to any place a chapter
needs a fact the examples do not contain.

Reference material — cocotb, pyuvm, four releases of the SystemVerilog UVM, and
the example code from both earlier books — is outside the repo at
`../rustdv-reference`, read-only. Use it to check what the UVM actually does
rather than what a comment says it does.
