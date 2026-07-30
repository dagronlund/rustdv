# Rights inventory — every place a licensing decision lands

*Written 2026-07-29. Nothing here has been changed; this is the checklist for
when Ray makes the decision. Not legal advice.*

## The decision that is pending

Ray's intent: publish rustdv publicly with rights reserved until a maintainer
emerges, ideally the [FOSSi Foundation](https://fossi-foundation.org/), without
letting a rights decision foreclose options.

**One correction worth carrying into that decision.** Handing a project to a
foundation does not require holding reserved rights — stewardship and licensing
are separate, and projects move to foundations while already permissively
licensed. What rights-reserved actually preserves is the ability to *relicense
unilaterally*, since Ray owns 100% and there are no contributors to negotiate
with. That is real, but it buys a freedom the plan may not need while blocking
the adoption that produces a maintainer: no one can evaluate the framework at
work without a licence, no one can take a `cargo` dependency, no one can
contribute, and no reader of the book may use the examples in their own
testbench.

The usual resolution is a permissive licence now plus contribution terms that
keep transfer flexible — Apache-2.0 is written for this, with an explicit patent
grant and explicit contribution language, which is why foundations prefer it. A
DCO keeps contributing lightweight; a CLA gives more relicensing latitude and
deters some contributors.

## The five places, and their state today

| # | Where | Says today | Needs |
|---|---|---|---|
| 1 | `LICENSE` (repo root) | "All rights reserved… not open source and is not released under any public licence at this time." | Replace, or keep and fix everything below to match. It is the only file asserting reserved rights. |
| 2 | Ten `Cargo.toml` files | `license = "MIT OR Apache-2.0"` | Either honour it or remove it. **This already contradicts #1.** |
| 3 | crates.io | `rustdv` 0.0.1 published as a name placeholder | **Check what licence metadata is live.** If 0.0.1 carried `MIT OR Apache-2.0`, a public grant already exists, and crates.io versions can be yanked but never deleted. Needs Ray — no network in the VM. |
| 4 | `rustdv/Cargo.toml` `repository` | `https://github.com/raysalemi/rustdv` | Git remote is `https://github.com/rustdv/rustdv`. Cosmetic, but it is the URL crates.io displays. |
| 5 | The book and the examples | No licence statement in `book-pdf/`; no `license` field in any `output/examples/*/Cargo.toml` | Undefined status for the code a reader is told to copy — the one place ambiguity directly undercuts the product. |

### The ten crates in #2

`rustdv`, `rustdv-gpi-sys`, `rustdv-gpi`, `rustdv-sim`, `rustdv-methodology`,
`rustdv-runner`, `rustdv-macros`, `rustdv-vpi-stubs`, `framework-tests`,
`tinyalu_tb` — all declare `license = "MIT OR Apache-2.0"`.

## If the answer is dual MIT/Apache-2.0

The Rust convention, and what a `MIT OR Apache-2.0` field points at:

- `LICENSE-MIT` and `LICENSE-APACHE` at the repo root, replacing `LICENSE`.
- A licence section in `README.md` stating the dual grant.
- A `license` field added to each `output/examples/*/Cargo.toml`, so the code
  readers copy is unambiguous.
- A licence line in the book's front matter, since the book ships in the same
  repo and a reader will look there rather than at the root.
- `CONTRIBUTING.md` with a DCO or CLA, whichever Ray picks — this is the piece
  that keeps a future transfer clean.

## Already handled correctly, for the record

- The two commercial books and the Accellera TLM spec live **outside** the repo
  in `../rustdv-reference/`, and nothing copies them in.
- SystemVerilog quotations in the book come from `uvmprimer-master`, which is
  Ray's own example code.

## One factual note on AI-drafted prose

Purely AI-generated text is not copyrightable in the US, so some of the
manuscript may sit outside copyright regardless of what the repo declares. This
does not break a licence grant — the grant covers what Ray owns, which includes
the selection, arrangement, direction and editing. No open-source project treats
this as a blocker; it earns at most a sentence in a `NOTICE`. Recorded so a
future thread does not rediscover it as a problem.
