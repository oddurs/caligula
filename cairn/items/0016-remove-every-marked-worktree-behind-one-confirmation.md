---
id: 16
title: Remove every marked worktree behind one confirmation
type: feature
status: done
milestone: v0.2
depends_on:
- 15
created: 2026-09-17
updated: 2026-09-18
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

- [x] The dialog lists every marked worktree with its own verdict, worst first
- [x] It totals what is at stake: N worktrees, N files, N commits only here
- [x] Marked worktrees needing `--force` are called out separately and in red
- [x] The main checkout and locked worktrees are refused with a reason, not
      silently skipped
- [x] A failure part-way through completes the rest and reports each failure
- [x] Every affected repository is re-probed once, not once per worktree

## 2026-09-18

Review found four things before this landed. The one that mattered: repositories were re-probed by index, but refresh_repo re-sorts, so after a sweep across two repositories the second refresh named whichever repo had sorted into that slot and the first was never re-probed — leaving removed worktrees on screen. Now looked up by root. Also: the branch to delete is carried rather than recovered from the label, since git allows a branch literally named (wip); branch-delete failures are counted apart from removal failures, because a kept branch is not a surviving worktree; and the dialog height counts wrapped rows by words at the real popup width.
