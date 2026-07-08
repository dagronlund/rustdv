# Regression testing for Rust for RTL Verification

One command guards the whole project:

```
output/regression/regress.py
```

Exit 0 means nothing you care about has changed behavior. Three suites run:

| Suite | Question it answers | Needs Rust? |
|---|---|---|
| `book-sync` | Does every book figure still have a matching, verbatim example file? | no |
| `examples` | Does every example still build, run, print, panic, or fail-to-compile exactly as blessed? | yes |
| `custom` | Do your own drop-in tests (future rustdv crate, tools, scripts) still pass? | per test |

## One-time setup

```
output/regression/regress.py --bless          # freeze today's known-good outputs
output/regression/regress.py --install-hook   # refuse to `git push` a broken state
```

`--bless` runs every figure binary and records its stdout under
`goldens/`. Commit the goldens — they are the baseline every future run is
compared against. The hook can be bypassed in an emergency with
`git push --no-verify`.

## Daily use

```
output/regression/regress.py                       # everything
output/regression/regress.py --suite book-sync     # fast, no cargo needed
output/regression/regress.py --filter ch09         # just chapter 9 tests
output/regression/regress.py --list                # see all test ids
```

## The workflow that keeps bugs out

**Changed a figure in the book?** `book-sync` fails for that figure, telling
you the example is now stale. Update the example file (keep the figure code
verbatim; context blocks and the header comment are yours to maintain), then
re-run. If the program's output changed too, re-bless just that figure:

```
output/regression/regress.py --bless --filter ch09_fig03
```

**Added a new figure?** Add the example file following the naming convention
(`chNN_figMM_slug`), add an entry to `output/examples/manifest.json`, add a
row to the chapter README, then `--bless --filter chNN_figMM`. The book-sync
coverage test fails until book and examples agree.

**Renumbered figures?** `book-sync/coverage` lists exactly which figure
numbers no longer line up on each side.

**Changed example code (or upgraded Rust)?** Run the full suite. Output
diffs, exit-code changes, and changed compiler-error codes all surface as
failures. Bless only what you intended to change — an unintended diff IS the
regression.

**Adding functionality (e.g. the rustdv crate)?** Two options:

- Rust unit tests: put `#[test]` functions in the crate, then add the package
  name to `cargo_test_packages` in `regress.json`.
- Anything else: copy `tests/example-template/`, rename it, edit `test.json`
  (see the comment field for all options), remove `"disabled": true`. Each test
  declares a command plus expectations: exit code, required output substrings,
  and/or a golden stdout file. A `skip_if_missing` field names a required
  executable — the test skips (rather than fails) on machines without it.
  The simulator smoke tests (`tests/sim-*`) use this: they run where
  Icarus/Verilator are installed (including CI) and skip elsewhere.

## Continuous integration

`.github/workflows/ci.yml` runs two jobs on every push and PR: the full
regression suite (Rust toolchain, no simulators — sim tests skip), and the
simulator smoke tests on Icarus + Verilator. Commercial simulators can't run
in public CI; license-holders run `sim/run_smoke.sh <sim>` or the same
regress command locally.

The rule of thumb: **every new piece of functionality lands together with the
test that would catch its removal.** The pre-push hook then makes it
structurally hard to break something silently.

## Special cases the suite knows about

- **Intentional compile errors** (20 figures): the test asserts compilation
  *fails* with the book's error code (e.g. E0382). If a Rust upgrade changes
  an error code, the test names both codes.
- **Intentional panics** (3 figures): the test asserts a nonzero exit.
- **HashMap-order figures**: listed in `regress.json` under `compare_modes`.
  `unordered_lines` ignores line order (map iterated one entry per line);
  `normalized_maps` sorts the entries inside `{...}` on each line (map
  printed inline with `{:?}`).
- **Marked deviations from the book**: if an example must intentionally
  differ from the manuscript, list it under `deviations` in `regress.json`
  and put a `deviation from the book` comment at the changed line — book-sync
  then checks for the marker instead of demanding verbatim match. (Currently
  none; the original two were resolved by fixing the manuscript — see
  `output/examples/ERRATA.md`.)

## Relationship to check.sh

`output/examples/check.sh` answers "do the examples match **the book's
transcripts**?" — useful when editing the manuscript. `regress.py` answers
"did anything change since the **last blessed state**?" — that's the
regression guard. check.sh compares against prose that may contain errata;
regress.py compares against reality you approved.
