---
id: 16
title: Remove every marked worktree behind one confirmation
type: feature
status: backlog
milestone: v0.2
depends_on:
- 15
created: 2026-09-17
updated: 2026-09-17
priority: p0
effort: m
area: actions
---

## Problem

Marking is worthless if acting on the marking is still one dialog per worktree.

## Proposal

One dialog for the whole marking, itemised, ordered with the most costly first,
and totalled. Removal runs in order and does not stop at the first failure.

## Acceptance criteria

- [ ] The dialog lists every marked worktree with its own verdict, worst first
- [ ] It totals what is at stake: N worktrees, N files, N commits only here
- [ ] Marked worktrees needing `--force` are called out separately and in red
- [ ] The main checkout and locked worktrees are refused with a reason, not
      silently skipped
- [ ] A failure part-way through completes the rest and reports each failure
- [ ] Every affected repository is re-probed once, not once per worktree
