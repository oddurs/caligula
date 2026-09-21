---
id: 57
title: A repository row describes worktrees the lens is hiding
type: bug
status: done
milestone: v0.3
assignee: Oddur Sigurdsson
created: 2026-09-20
updated: 2026-09-20
priority: p0
effort: s
area: chrome
---

## What happens

`Repo::totals` sums every worktree in the repository, but a lens shows only
some of them. Under `safe to remove` the result is a summary that contradicts
the rows beneath it:

```
▾ unifont (18)                     58   8      1d
│ ● chore/tidy-the-board                        4d
│ ● feat/tui-related                            9d
│ ● docs/three…-from-a-real-library            13d
```

All eight changed files are in unifont's main checkout, which this lens hides.
Every worktree shown under that row is clean. The row reads as "these have
eight files at risk", on the one lens whose entire purpose is to say they have
nothing at risk.

This is the same fault as item 0052 — a summary computed over a different set
than the one on screen — one level up.

## What should happen

A repository row describes the rows underneath it. When a lens or a filter
hides worktrees, their numbers go with them.

## Acceptance criteria

- [x] The repository row totals only the worktrees visible beneath it
- [x] The worktree count in `(n)` counts the same set
- [x] A folded repository still totals what it would show unfolded, not the
      whole repository
- [x] The age on a repository row is the newest of the visible worktrees
- [x] A snapshot under the safe lens shows the row agreeing with its rows
