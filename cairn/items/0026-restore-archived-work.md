---
id: 26
title: Restore archived work
type: feature
status: backlog
milestone: v0.3
depends_on:
- 25
created: 2026-09-17
updated: 2026-09-17
priority: p2
effort: m
area: actions
---

## Problem

An archive nobody can find is a disk leak, not a safety net.

## Acceptance criteria

- [ ] A view lists archives: repository, branch, when, how big
- [ ] Restoring into a fresh worktree of that branch reproduces the tree
- [ ] Restoring into an existing dirty tree refuses rather than merging
- [ ] An archive can be deleted from the interface
- [ ] Archives older than the expiry the spike settled on are reported
