---
id: 8
title: Find every git worktree under a set of roots
type: feature
status: done
milestone: v0.1
created: 2026-09-17
updated: 2026-09-17
priority: p0
effort: l
area: scan
---

## Problem

`git worktree list` answers for one repository you are already standing in.
Nothing answers for the machine.

## Proposal

Walk the roots in a background thread, treat any directory holding a `.git`
entry as a repository, resolve it to its common git directory, and ask git for
the rest. Stream repositories to the interface as they are probed.

## Acceptance criteria

- [x] A directory containing `.git` is a repository and is not descended into
- [x] Linked worktrees are discovered through their repository, so they are
      found even inside hidden directories like `.claude/worktrees/`
- [x] Two worktrees of the same repository resolve to one repository entry
- [x] Hidden directories are skipped, except `.worktrees`
- [x] Results stream in: the list is usable before the scan finishes
