# Project: rustdv & "Rust for RTL Verification"

## Context
- I am Ray Salemi, author of "Python for RTL Verification" (PDF in
  /reference — ground truth on the Python/cocotb/pyuvm side).
- This project produced two finished things (see TOUR.md for the tour):
  1. **rustdv** — a working Rust verification framework (cocotb + pyuvm
     analog) in /rustdv, with a TinyALU regression on Icarus Verilog.
  2. **"Rust for RTL Verification"** — a complete 41-chapter book in
     /book-pdf/src (mdBook), every figure verified running.

## Folder map
- /rustdv — the framework workspace (crates rustdv-gpi-sys … rustdv, plus
  tinyalu_tb). Runs via sim/run_rustdv.sh.
- /book-pdf/src — the manuscript. Rebuild rendered book: `mdbook build book-pdf`.
- /output/examples — every book figure, runnable (Part I per-figure files,
  Parts II–V as sim chapter crates); /output/.design-doc.md is the design record.
- /output/regression — regress.py, wired into the git pre-push hook.
- /skills — AI verification skills (rtl-spec-analysis, rustdv-testbench,
  rustdv-verify-cover).
- /reference — read-only sources. Do not modify.
- STATUS.md — authoritative implementation history and deviations log.
- TOUR.md — orientation for new readers and new threads (start there).

## Standing rules
- Never rmtree+recreate directories in this folder from the VM (sync
  engine races); build in /tmp and copy over.
- The pre-push hook runs the full regression; keep it green. Verify claims
  by running things — transcripts in the book/README files are real output
  and must stay in sync with reruns.
- Flag uncertainty openly (Open Questions / STATUS deviations) rather than
  presenting guesses as settled.
- Ask before overwriting existing deliverables; show a short plan before
  generating long documents.
