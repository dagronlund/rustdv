---
name: close-out-thread
description: Use when a rustdv session is ending, when the user says to hand off, wrap up, prepare for the next thread, or asks for a prompt for a new thread. Verifies the tree is green, hunts the specific kinds of staleness this repo keeps producing (branch names in files, orientation documents describing finished work as active, notes that outlived the code they describe, a changed example whose README or chapter was not regenerated), and emits the next-thread prompt.
---

# Close out a thread

A new thread starts by reading TOUR.md, CLAUDE.md and CLAUDE.local.md and
acting on them. If those are stale it will act on stale information, and the
cost lands on Ray, who has to notice and correct each wrong conclusion. That
has happened often enough to be worth a checklist rather than good intentions.

**Do the whole list. Report once at the end** — no narration between steps.

## 0. Toolchain

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

## 1. Green tree

```
python3 output/regression/regress.py
```

Takes a few minutes; the sandbox kills a foreground command at ~45 s, so run it
with `nohup ... &` into a log under `/tmp/rustdv-$(id -u)/scratch/` and poll the
tail. Anything red is not handed to the next thread — either fix it or say
plainly, at the top of your report, that you are leaving it red and why.

**A green sandbox run is evidence about the sandbox.** Say "verified on Linux".
macOS/arm64 ships too, and only Ray can confirm it.

## 2. Do not mention branch names — in any file, or in the prompt you hand over

This is about *text*, not about git. Delete no branches; delete the places a
document names one.

```
git grep -n "$(git branch --show-current)" -- . ':!*.lock'
git grep -nE "ch23_onwards|rewrite-book|put_uvm_back_in_rustdv|fable-prep" -- . ':!*.lock'
```

Both must come back empty. Branches are ephemeral; every one ever written into
a file in this repo was wrong within days and misdirected a thread. If a branch
name turns up, rewrite the sentence to say what is true without it — a thread
that needs to know where it is runs `git branch --show-current`.

## 3. Orientation documents must match reality

Read them and check each claim, do not skim:

| File | What goes stale in it |
|---|---|
| `TOUR.md` | "Where the work stands" describing finished work as pending; hard-coded test counts; a promise ("the one thing still outstanding") whose subject was fixed and removed |
| `CLAUDE.md` | the Context block — what is active, what remains, chapter counts |
| `CLAUDE.local.md` | what the current work is |
| `output/.design-decisions.md` §0.5 | the status paragraph |

**Never write a count you did not just measure.** `regress.py --list` prints the
entry count; the filesystem knows how many chapters there are. Prose numbers in
this repo have gone stale more than once and were then quoted back as fact.

If your session finished something these files describe as outstanding, update
them now. That is not tidying — it is the deliverable.

## 4. Notes your session made false

When you learn that a written note no longer describes the code, strike it
where it lives rather than only mentioning it in chat. Both of these were
quoted as current fact months after the code was deleted:

- `STATUS.md` — an aside about `#[component(no_factory)]` being live logic
- `output/.design-decisions.md` — the same about `struct_attr_contains`

Strike through with `~~ ~~`, add the date and what actually happened. Never
silently delete: the record of a reversal is worth more than a clean page.

## 5. If you changed an example

```
python3 output/regression/verify-book-listings.py     # every ch15-40 listing vs its crate
bash output/regression/verify-transcripts.sh          # every README and book transcript
```

Both green, or the work is not finished. A changed example owes:

- its crate `README.md` — figure map **and** transcript, regenerated from a real run
- the chapter in `book-pdf/src` that quotes it
- a row in `book-pdf/chapter-notes.md` saying what changed and why

Record any constraint a future edit could break without failing a test — a
timing relationship the lesson depends on, a number that must stay in a range.
A test cannot catch a demonstration that silently stops demonstrating.

## 6. If you renamed anything

```
git status --short          # renames should read R, not D + new file
git grep -n "<the old name>"
```

Check the places a grep of the source alone will miss: `regress.json`,
`output/regression/tests/*/test.json`, `verify-transcripts.sh`, the examples
workspace `Cargo.toml` members, `book-pdf/src/SUMMARY.md`, the appendices, and
cross-references from neighbouring chapters.

## 7. Decision log

Anything architectural settled this session gets a numbered decision appended
to `output/.design-decisions.md`, in the last section or a new one at the end.

```
python3 output/regression/index-decisions.py
```

Decision numbers are permanent and never renumbered. A reversal is struck with
its reason, not deleted.

## 8. Memory

Update the memory files for anything durable — a rule Ray stated, a constraint
that is not derivable from the code. Not this session's narrative. No branch
names there either.

## 9. Emit the prompt

Short. Reading order, the two standing rules, the setup block, and the next
job. It must not name a branch and must not restate what TOUR.md already says —
a prompt that duplicates the orientation files becomes a third thing that goes
stale. CLAUDE.md's rule against per-thread prompt *files* stands: hand Ray the
prompt in chat, do not commit it.

Template:

```
Come up to speed on rustdv. Read, in order:

1. TOUR.md — whole file. "Where the work stands" at the bottom is the live
   status; "Notes for AI sessions" has the sandbox hazards.
2. CLAUDE.md and CLAUDE.local.md — standing rules.
3. output/.design-decisions.md §0–§0.5 — the mission and method.

Two rules that matter more than anything in those files:

- Repo prose is not evidence. Never repeat what a document says about the
  code. Check the code, then speak.
- Don't ask me about branches, and don't write one into a file.

Then set up the toolchain and prove the tree is green before touching
anything:

    bash toolchain-drop/install.sh
    export PATH="/tmp/rust/bin:/tmp/oss-cad-suite/bin:$PATH"
    export CARGO_TARGET_DIR=/tmp/rustdv-$(id -u)/examples-target
    export CARGO_TERM_COLOR=never
    python3 output/regression/regress.py

Then stop and tell me what you found. Don't edit anything.

<the next job, in a sentence or two>
```

## Report

One message. What changed, what is verified and on what platform, what is left
red and why, then the prompt. No recap of steps — Ray watched them go by.
