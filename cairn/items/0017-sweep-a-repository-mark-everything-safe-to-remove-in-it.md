---
id: 17
title: 'Sweep a repository: mark everything safe to remove in it'
type: feature
status: backlog
milestone: v0.2
depends_on:
- 15
- 52
created: 2026-09-17
updated: 2026-09-17
priority: p0
effort: s
area: actions
---

## Problem

The common case is not choosing between worktrees, it is clearing out everything
that has nothing left in it.

## Acceptance criteria

- [ ] One key marks every worktree in the current repository whose verdict is
      "nothing to salvage", and nothing else
- [ ] It never marks the main checkout or a locked worktree
- [ ] With a lens active it marks only what the lens shows
- [ ] Pressing it again clears that marking rather than doubling it
- [ ] Says how many it marked, or why it marked none
