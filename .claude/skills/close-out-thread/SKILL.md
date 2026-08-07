---
name: close-out-thread
description: Use when a rustdv session is ending, or when the user says to close out, hand off, or wrap up. Verifies the tree is green, hunts the specific kinds of staleness this repo keeps producing (branch names in files, orientation documents describing finished work as active, notes that outlived the code they describe, a changed example whose README or chapter was not regenerated), and points the next thread at start-thread.
---

# Close out a thread

**Do every step below, then report once.** No narration between steps.

The job is to leave TOUR.md, CLAUDE.md and CLAUDE.local.md true, because the
next thread runs `start-thread`, reads them, and acts on whatever they say.

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

Run it in the **foreground** with a generous per-call timeout rather than
backgrounding it with `nohup`. A backgrounded process does not survive between
tool calls in this sandbox — each call is a fresh environment, so a `nohup …
&` job started in one call is gone by the next, and polling its log in a
later call finds nothing. A single foreground call does not have that
problem; it only needs its timeout set long enough (verified 2026-08-06: a
foreground `examples`-suite build ran several minutes to completion under a
600s call timeout, no chunking needed). If one call still isn't enough,
chunk by suite instead of backgrounding:

```
python3 output/regression/regress.py --suite unit
python3 output/regression/regress.py --suite book-sync
python3 output/regression/regress.py --suite examples
python3 output/regression/regress.py --suite custom
```

Anything red is not handed to the next thread — either fix it or say plainly,
at the top of your report, that you are leaving it red and why.

**A green sandbox run is evidence about the sandbox.** Say "verified on Linux".
macOS/arm64 ships too, and only Ray can confirm it.

## 2. Do not mention branch names — in any file, or in what you tell Ray

This is about *text*, not about git. Delete no branches; delete the places a
document names one.

Ask git which names are branches and look for those. Do not hard-code a list of
names in this file: they change, and a stale list here would be exactly the kind
of rot this step exists to catch.

```
git for-each-ref --format='%(refname:short)' refs/heads refs/remotes \
  | sed 's|^origin/||' | sort -u | grep -Ev '^(main|master|HEAD)$' \
  | while read -r b; do git grep -n -F -- "$b" -- . ':!*.lock'; done
```

Must come back empty. `main`/`master` are skipped because they appear
legitimately in URLs like `blob/master/...`; if you want to check those, do it
by eye.

This finds mentions of branches that still exist, which is the case that
matters — a document naming the branch you are on, or one you just merged. A
name whose branch was deleted long ago is only a word, and no grep can
distinguish it from prose. Branches are ephemeral; every one ever written into a
file in this repo was wrong within days and misdirected a thread. If one turns
up, rewrite the sentence to say what is true without it — a thread that needs to
know where it is runs `git branch --show-current`.

## 3. Orientation documents must match reality

Read them and check each claim, do not skim:

| File | What goes stale in it |
|---|---|
| `TOUR.md` | "Where the work stands" describing finished work as pending; hard-coded test counts; a promise ("the one thing still outstanding") whose subject was fixed and removed |
| `CLAUDE.md` | the Context block — what is active, what remains, chapter counts |
| `CLAUDE.local.md` | what the current work is |
| `output/.design-decisions.md` §0.5 | the status paragraph |

**Never write a count you did not just measure**, and prefer not to write one at
all. `regress.py --list` prints the test ids; the filesystem knows how many
chapters there are. Prose numbers here have gone stale more than once and were
then quoted back as fact. Note that `--list` and a full run do not report the
same total — some entries expand at run time — so a number is ambiguous even
when it is fresh. Point at the command instead.

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

## 9. Hand off

No prompt to write. The next thread starts with `/start-thread`, which reads
the orientation files, sets up the toolchain, and proves the tree is green on
its own — the same steps this file used to type out as a paste-able template.
Tell Ray, in one line, that the next thread should run it. If there's a
specific next job, say that in one more line — that is the one thing
`start-thread` cannot know on its own. Do not restate TOUR.md and do not draft
a block for Ray to paste: that was the prompt template this section used to
carry, and `start-thread` is now that template, run instead of retyped. A
second copy of it here is exactly the kind of duplicate this project keeps
striking.

## 10. Clean up session artifacts

Run the cleanup-tmp skill before handing off:

```
/cleanup-tmp
```

This removes all `/tmp/rustdv-*/` directories and reports space freed.
## Report

One message. What changed, what is verified and on what platform, what is left
red and why, then the one-line handoff to `start-thread`. No recap of steps —
Ray watched them go by.
