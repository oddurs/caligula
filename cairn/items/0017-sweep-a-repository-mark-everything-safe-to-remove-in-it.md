---
id: 17
title: 'Sweep a repository: mark everything safe to remove in it'
type: feature
status: done
milestone: v0.2
depends_on:
- 15
- 52
created: 2026-09-17
updated: 2026-09-18
priority: p0
effort: s
area: actions
---

## Problem

The common case is not choosing between worktrees, it is clearing out everything
that has nothing left in it.

## Acceptance criteria

- [x] One key marks every worktree in the current repository whose verdict is
      "nothing to salvage", and nothing else
- [x] It never marks the main checkout or a locked worktree
- [x] With a lens active it marks only what the lens shows
- [x] Pressing it again clears that marking rather than doubling it
- [x] Says how many it marked, or why it marked none
