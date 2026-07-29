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

**Where it stands (2026-07-29):** the restoration's code is done. ch23–ch39
are converted and out of quarantine — phases, ConfigDb, factory, the whole TLM
layer, transactions, and all four sequence testbenches (TB 7.0–8.0). The test
suite is built on top: 110 no-simulator tests, 38 targeted simulator tests in
`rustdv/framework-tests/`, 5 compile-fail cases, and `sim-mutation`; the
regression is 236 and the pre-push hook runs all of it
(`output/regression/TESTING.md`). Next is the **TinyALU refactor**, the last
known technical debt. TOUR.md's "Where the work stands" section is the
orientation for a new thread; the decisions that most shape new code are D83b
(connection is a trait method, not a registry), D82b/D82c (children move out for
the run phase; each component races the objection event, never the whole tree),
and D90 (the analysis hub stores nothing — the subscriber owns its storage).

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
