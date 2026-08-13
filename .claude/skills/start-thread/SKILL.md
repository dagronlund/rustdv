---
name: start-thread
description: Use when a rustdv session is starting, or when the user says to come up to speed, orient, or start a thread. Reads the orientation files in order, sets up the toolchain, proves the tree is green, and reports findings — without editing anything. Replaces the handoff prompt close-out-thread used to emit.
---

# Start a rustdv thread

**Do every step below, then report once.** No narration between steps.

**Edit nothing.** This skill orients and verifies; it does not repair. Anything
red gets reported, not fixed.

## 1. Read the orientation files, in order

1. `TOUR.md` — whole file. "Where the work stands" at the bottom is the live
   status; "Notes for AI sessions" has the sandbox hazards.
2. `CLAUDE.md` and `CLAUDE.local.md` — standing rules.
3. `output/.design-decisions.md` §0–§0.5 — the mission and method.

## 2. Two rules that matter more than anything in those files

- **Repo prose is not evidence.** Never repeat what a document says about the
  code. Check the code, then speak.
- **Don't ask Ray about branches, and don't write one into a file.**

## 3. Toolchain

The VM wipes `/tmp` between sessions, so this may need rerunning even if it
worked an hour ago:

```
bash toolchain-drop/install.sh
export PATH="/tmp/rust/bin:/tmp/oss-cad-suite/bin:$PATH"
export CARGO_TARGET_DIR=/tmp/rustdv-$(id -u)/examples-target
export CARGO_TERM_COLOR=never
export CARGO_PROFILE_DEV_DEBUG=0
```

`cargo: command not found` from inside `regress.py` means exactly this and
nothing more sinister.

## 4. Prove the tree is green

```
python3 output/regression/regress.py
```

Run it in the **foreground** with a generous per-call timeout rather than
backgrounding it with `nohup`. A backgrounded process does not survive between
tool calls in this sandbox — each call is a fresh environment, and a `nohup …
&` job started in one call is gone by the next. A single foreground call does
not have that problem; it only needs a timeout set long enough (verified
2026-08-06: a foreground `examples`-suite build ran several minutes to
completion under a 600s call timeout, no chunking needed). If one call still
isn't enough, chunk by suite rather than backgrounding:

```
python3 output/regression/regress.py --suite unit
python3 output/regression/regress.py --suite book-sync
python3 output/regression/regress.py --suite examples
python3 output/regression/regress.py --suite custom
```

Anything red gets reported, not fixed — this skill orients, it does not
repair. If the tree is red, say so plainly at the top of the report.

**A green sandbox run is evidence about the sandbox.** Say "verified on
Linux"; macOS/arm64 ships too, and only Ray can confirm it.

## 5. Report once

**Read CLAUDE.md's "How to report to Ray" first — it governs this step.** Every
item in the report is something he must do or decide; if that is nothing, the
first line says so.

One message: what the orientation files claim about where the project stands,
what the regression actually showed (pass/fail counts per suite as run, never
a count copied from prose — `regress.py --list` or the run's own tally, not a
number written in a document), and anything that disagrees with what the files
said. Then stop.

Do not edit anything. Do not propose or start the next job — that is Ray's
call, made after he reads the report. If Ray already told you the next job
when he invoked this skill, hold it until the report is delivered, then begin.
