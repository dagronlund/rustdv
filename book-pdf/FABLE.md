# Brief for the prose pass
# Notes for RustDV Book.

# Previous versions of the testbench are unimportant

The book occasionally talks about "a previous version of the testbench". This is a mistake. The reader doesn't care about our history or about the evolution of Rustdv.  Just explain what happens now.  Look through the book for this mistake and remove it.

# TLM Chapter

There is no need for story about the test being a component. This is the AI's revelation.  There should be no references to an earlier version of rustdv, the reader doesn't care.

## No Analysis in FIFO chapter

RustdvShared does not land as a name. I cannot remember what it is. 

Move tlm_fifo analysis port to analysis port chapter.  It does not belong in the chapter on blocking and trying with TLM_fifos because the reader does not understand Analysis ports yet.

# Analysis Chapter

The Analysis chapter needs to be completely rewritten it has many problem.

* The analysis chapter needs to explain this new concept of the `WriteSink` trait. There is no explanation, it just gets thrown at the reader. 

* The chapter needs to start by explaining the concept of a publisher and many subscribers.

* The chapter needs to warn the reader that the analysis layer in RustDV is a copy of the analysis layer in UVM, though it does require a write() function that takes no time.

* It needs to describe the publisher port and subscriber port first.  It needs to discuss the WriteSink trait and how this contains the write function as in UVM. It needs a complete explanation of what `on_write` is and how it relates to `connect_write`.

* The chapter needs a digression to discuss `RustdvShared` and how it works. 

* The first example is excellent, but you need to do a good job explaining what it does since it is not a TinyALU testbench. There is a counter and a collector that do different things with the same data explain that.

* The chapter needs to explain the AnalysisHub and how it is nothing like the analysis_fifo.  It has no storage, it simply connects function calls.  It is up to the subscriber to store information if it wants to. 

* Now that all this has been explained, you can show the reader the example in action and repeat how `RustdvShared` fits into this.

* "Now the habit this chapter exists to correct."  Whose habit?  Yours? The reader has no habit.  *an AnalysisBus is not a FIFO and stores no items* is just a fact. You are not correcting a misconception.



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

**Comparisons are foils, not nostalgia.** "In Python, a typo'd attribute is a
runtime `AttributeError`" is useful. "As you saw in the Python book" is not.
Prefer two sharp comparisons to none — a dual audience means both foils, not no
foil — and where the dialects diverge, one parenthetical, SystemVerilog first:
*(SV: `uvm_config_db#(int)::set`; pyuvm: `ConfigDB().set`)*.

**SystemVerilog quotations** come only from `../rustdv-reference/uvmprimer-master/`,
run ≤15 lines, carry a source label, never appear with simulation output, and
total a handful across the whole book.

**Say the point plainly.** "Honestly", "genuinely" and "straightforward" read as
persuasion rather than statement.


## When you are unsure

Ask Ray. Do not invent an answer and write it as settled — that is precisely how
the original design failed. This applies to anything in `chapter-notes.md`
marked **ASK RAY**, to the caption question above, and to any place a chapter
needs a fact the examples do not contain.

Reference material — cocotb, pyuvm, four releases of the SystemVerilog UVM, and
the example code from both earlier books — is outside the repo at
`../rustdv-reference`, read-only. Use it to check what the UVM actually does
rather than what a comment says it does. 