# Project: rustdv & "Rust for RTL Verification"

## Context
rustdv is a Rust hardware verification framework (a cocotb + pyuvm analog) by
Ray Salemi, companion to the book *Rust for RTL Verification*. The repo holds
two products (see TOUR.md for the tour):

1. **rustdv** — the framework (crates in /rustdv), with a TinyALU regression
   passing on Icarus Verilog.
2. **"Rust for RTL Verification"** — a 41-chapter book in /book-pdf/src
   (mdBook), every figure verified running.

**Active work: the UVM restoration.** A prior pass wrongly stripped the UVM's
dynamic build/connect process and its TLM FIFOs; branch `ch23_onwards` is
restoring them (build/connect phases, the ConfigDb, the factory, TLM), one
TinyALU testbench version at a time. This is reworking the framework and, from
the working code, the Part II+ prose — so treat the later chapters as under
revision, not settled. `output/.design-decisions.md` is the authoritative
decision log (its §0 is the mission and method); read it and CLAUDE.local.md
before proposing anything architectural. Do **not** follow the older
`output/.design-doc.md` — it is the pre-restoration spec whose closed-world
design caused the problems now being fixed.

Ground truth for the methodology — the cocotb, pyuvm and SystemVerilog UVM
sources, plus the example code from the earlier books — lives outside this
repository at ../rustdv-reference, so the repo carries only its own product.

## Folder map
- /rustdv — the framework workspace (crates rustdv-gpi-sys … rustdv, plus
  tinyalu_tb). Runs via sim/run_rustdv.sh.
- /book-pdf/src — the manuscript. Rebuild rendered book: `mdbook build book-pdf`.
- /output/examples — every book figure, runnable (Part I per-figure files,
  Parts II–V as sim chapter crates).
- /output/regression — regress.py, wired into the git pre-push hook.
- /skills — AI verification skills (rtl-spec-analysis, rustdv-testbench,
  rustdv-verify-cover).
- STATUS.md — authoritative implementation history and deviations log.
- TOUR.md — orientation for new readers and new threads (start there).

## Standing rules
- The pre-push hook runs the full regression; keep it green. Verify claims
  by running things — transcripts in the book/README files are real output
  and must stay in sync with reruns.
- Flag uncertainty openly (Open Questions / STATUS deviations) rather than
  presenting guesses as settled.
