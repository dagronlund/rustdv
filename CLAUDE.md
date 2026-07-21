# Project: rustdv & "Rust for RTL Verification"

## Context
rustdv is a Rust hardware verification framework (a cocotb + pyuvm analog) by
Ray Salemi, companion to the book *Rust for RTL Verification*. The repo holds
two finished things (see TOUR.md for the tour):

1. **rustdv** — the framework (crates in /rustdv), with a TinyALU regression
   passing on Icarus Verilog.
2. **"Rust for RTL Verification"** — a complete 41-chapter book in
   /book-pdf/src (mdBook), every figure verified running.

Ground truth for the methodology — the cocotb, pyuvm and SystemVerilog UVM
sources, plus the example code from the earlier books — lives outside this
repository, so the repo carries only its own product.

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
