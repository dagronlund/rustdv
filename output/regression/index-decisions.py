#!/usr/bin/env python3
"""Check (or regenerate) the decision index in output/.design-decisions.md.

Decisions in that log are grouped by *topic*, not by number — D51 is in
§6b, D29 in §7 — so the index at the top is the only reliable way to find
one. This script keeps the index honest.

    index-decisions.py            # check: exit 1 if the index is stale
    index-decisions.py --print    # print a freshly generated index table

Run it after adding a decision. It is deliberately not wired into the
regression: the log is internal (never cited in reader-facing output), so
a stale index should nag, not block a push.
"""
import re
import sys
from pathlib import Path

LOG = Path(__file__).resolve().parents[2] / "output" / ".design-decisions.md"

# Matches "**D12. Title...", "- **D54. Title...", "~~**D8. Title..."
DECISION = re.compile(r"^(?:- )?(?:~~)?\*\*(D(\d+))\.\s*(.*)")


def parse(text):
    """Yield (number, id, title, section, struck) for each decision."""
    section = ""
    seen = set()
    for line in text.split("\n"):
        if line.startswith("## "):
            section = line[3:].strip()
            continue
        m = DECISION.match(line)
        if not m:
            continue
        ident, num, rest = m.group(1), int(m.group(2)), m.group(3)
        if ident in seen:          # a later reference, not the definition
            continue
        seen.add(ident)
        struck = line.lstrip("- ").startswith("~~")
        title = re.split(r"(?<=[a-z\)`])\.\s|\*\*", rest)[0].strip().rstrip(".,")
        yield num, ident, title, section, struck


def main():
    if not LOG.exists():
        sys.exit(f"not found: {LOG}")
    decisions = sorted(parse(LOG.read_text()))

    if "--print" in sys.argv:
        print("| # | Decision | Section |")
        print("|---|---|---|")
        for _, ident, title, section, struck in decisions:
            shown = f"~~{ident}~~" if struck else ident
            print(f"| {shown} | {title} | {section} |")
        return

    # Check mode: every decision must appear in the index table, and the
    # numbering must have no gaps (a gap means one was dropped or misnumbered).
    text = LOG.read_text()
    head = text.split("## Index of decisions", 1)
    if len(head) < 2:
        sys.exit("no '## Index of decisions' section — run with --print and paste it in")
    index_block = head[1].split("\n---", 1)[0]

    missing = [i for _, i, _, _, _ in decisions if f"| {i} " not in index_block
               and f"| ~~{i}~~ " not in index_block]
    numbers = [n for n, _, _, _, _ in decisions]
    gaps = [n for n in range(1, max(numbers) + 1) if n not in numbers] if numbers else []

    if missing:
        print(f"index is stale — not listed: {', '.join(missing)}")
    if gaps:
        print(f"gap in decision numbers: {', '.join('D%d' % n for n in gaps)}")
    if missing or gaps:
        print("regenerate with: index-decisions.py --print")
        sys.exit(1)

    print(f"decision index OK — {len(decisions)} decisions, D1..D{max(numbers)}, no gaps")


if __name__ == "__main__":
    main()
