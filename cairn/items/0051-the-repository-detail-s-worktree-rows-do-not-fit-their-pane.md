---
id: 51
title: The repository detail's worktree rows do not fit their pane
type: bug
status: done
milestone: v0.2
depends_on:
- 20
created: 2026-09-17
updated: 2026-09-18
priority: p1
effort: s
area: chrome
---

## What happens

The worktree table in a repository's detail pane is built from fixed column
widths — 26 for the branch, 34 for the verdict, then the age — which comes to
about 69 columns. The pane is narrower than that on a 120-column terminal, so
every row wraps and the age lands alone on the following line:

```
│   ◆ main                        Nothing to …lvage — safe to remove │
│  2h                                                                │
```

The verdict is also run through `truncate`, which keeps the tail and elides the
middle — right for a branch name, wrong for prose: "Nothing to …lvage — safe to
remove" instead of "Nothing to salvage…". The head-keeping helper it wants
already exists next to it as `clip`.

## What should happen

The row is laid out from the width it is given, the way list rows already are,
and prose is clipped from the end rather than the middle.

## Reproduction

`cargo test --test snapshots`, once item 0020 has landed the suite — the
baseline recorded in `tests/snapshots/snapshots__a_repository_detail.snap`
shows the wrapping.

## Acceptance criteria

- [x] The repository detail's worktree rows are exactly as wide as their pane
- [x] The verdict is clipped from the end, keeping the beginning
- [x] No row in the detail pane wraps at 80, 120 or 200 columns
- [x] The snapshot diff for this change shows the wrapping disappearing

## 2026-09-18

Review found the fix incomplete in three ways and the test vacuous. The two-column gap was subtracted from the budget but never drawn, so a label that filled its column ran into the verdict and the rows came out two columns narrower than the pane. The name and verdict floors were not checked against the width, so below about 56 columns every row wrapped again — the original defect. And clip(s, 0) returned an ellipsis rather than nothing, which widened a row whose verdict column had been dropped. The test that was supposed to catch all of this split the screen on the pane borders, landed on the empty string between them, and passed unconditionally; it now looks for each worktree's age on the same line as its label, and caught two of the three failures the moment it was written.
