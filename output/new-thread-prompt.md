# Prompt for a new rustdv thread

*Paste everything below the line into the new thread. Written 2026-07-28, at the
end of the TLM work (ch23–ch34 converted and green).*

---

You have two folders: `rustdv` (the repository) and `rustdv-reference`
(read-only upstream sources — cocotb, pyuvm, the SystemVerilog UVM, and my two
earlier books).

**This turn is orientation only. Do not change any file, do not run any build,
do not run the regression, do not propose a plan.** Read, then tell me what you
understand. You may run `git status`, `git log`, and `ls` to establish facts
rather than assume them.

## Read these, in this order

1. `TOUR.md` — start here. Its **"Where the work stands"** section, at the
   bottom under "Notes for AI sessions", was written for exactly this moment.
2. `CLAUDE.md` — the standing rules and the folder map.
3. `CLAUDE.local.md` — my local notes.
4. `output/.design-decisions.md` — the authoritative decision log. Read **§0
   through §0.5** carefully (premise, mission, method, the typing thesis, and
   the live status). Then read the sections covering the most recent work:
   **§22 (concurrency), §23 (TLM connection), §24 (analysis), §25 (non-blocking
   TLM), §26 (the analysis hub)**, plus **§16 (open questions)**. Skim the rest;
   the index table at the top maps sections to decision numbers.
5. `STATUS.md` — implementation history and the deviations log. It reads
   bottom-up: the newest entry is last and is the one that matters most.
6. The three chapters that were just finished, code first and then the README
   beside each:
   - `output/examples/ch31-component-communications/`
   - `output/examples/ch32-analysis-ports/`
   - `output/examples/ch34-connections-testbench-6.0/`
7. The framework code those chapters exercise:
   `rustdv/rustdv-methodology/src/` — `port.rs`, `fifo.rs`, `analysis.rs`,
   `shared.rs`, `component.rs`, `factory.rs` — and the derive in
   `rustdv/rustdv-macros/src/rustdv_macros.rs`.
8. `book-pdf/fable-brief.md` — the brief for the prose pass. You are not Fable,
   but you need to know what has been promised to the book.

Do **not** follow `output/.design-doc.md`. It is the pre-restoration
specification whose closed-world design caused the problems now being fixed; it
is kept only as the record of what went wrong.

## Then tell me what you understand

Answer in your own words — not by quoting the documents back at me. I am
checking whether you have the model, so tell me the *why* behind each answer,
not just the what.

1. **The mission.** What is being restored, what was destroyed, and what is the
   one instinct you are supposed to resist? What is D3, and why does it settle
   arguments?
2. **The method.** In what order do code, prose, and design decisions get
   written, and why that order?
3. **Where the work stands.** Which chapters and which TinyALU testbench
   versions are done and out of quarantine, which are still quarantined, and
   what is the next piece of work?
4. **The TLM layer as built.** How does a parent connect a port on a child it
   cannot name the type of? Why does that mechanism also work for the parent's
   own port, and what did it replace? What does `AnalysisFifo` store?
5. **Concurrency.** How can a parent's `run` be concurrent with its children's,
   and where does the objection race happen? What went wrong when it happened in
   the other place?
6. **What is decided versus what is open.** List the questions currently parked
   for me, and say which of them you must not settle on your own.
7. **The book's constraints.** Who writes the prose, what may that pass touch,
   and what does that imply about figure numbers and transcripts?
8. **Anything stale or contradictory.** If two documents disagree, or a document
   claims something the code does not do, say so plainly — including if it
   contradicts something above. Do not smooth it over. Older entries in the
   decision log and STATUS.md are historical records and can be superseded by
   later ones; say which you think wins and why.

Finish with the two or three things you are least sure about. I would rather
hear an honest gap now than a confident wrong answer later.

## Working notes for when we do start

Not for this turn, but so you are not surprised:

- The environment has no network. The toolchain comes from
  `toolchain-drop/install.sh`; the VM wipes `/tmp` between sessions, so it must
  be re-run, then `export PATH="/tmp/rust/bin:/tmp/oss-cad-suite/bin:$PATH"`.
  Use `CARGO_TARGET_DIR=/tmp/rustdv-target` for the framework and
  `/tmp/rustdv-examples-target` for the examples.
- Shell calls have a ~45 second budget and background processes are killed
  between calls. The full regression takes a few minutes from cold — pre-build
  the example workspace first, then run `python3 output/regression/regress.py`.
- The VM's disk is ~9.6 GB and the two target directories reach ~1.5 GB. A full
  disk surfaces as regression failures reading `No space left on device`, which
  looks exactly like a real break and is not.
- Never bulk-delete-and-recreate directories inside the mounted folder — the
  desktop sync engine races and forks `dir 2/` duplicates. Build in `/tmp` and
  copy over.
- I make the commits. Never claim the branch state without checking
  `git status`.
