---
id: 51
title: The repository detail's worktree rows do not fit their pane
type: bug
status: backlog
milestone: v0.2
depends_on:
- 20
created: 2026-09-17
updated: 2026-09-17
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

- [ ] The repository detail's worktree rows are exactly as wide as their pane
- [ ] The verdict is clipped from the end, keeping the beginning
- [ ] No row in the detail pane wraps at 80, 120 or 200 columns
- [ ] The snapshot diff for this change shows the wrapping disappearing
