---
id: 1
key: v0.1
title: Find them and remove them
type: milestone
status: done
created: 2026-09-17
updated: 2026-09-17
due: 2026-09-16
---

## Ships

Every git worktree on the machine, each one saying what it would cost to delete it.

## Done when

- [x] A scan finds every worktree under the configured roots, including linked
      worktrees inside hidden directories
- [x] Worktrees are grouped under the repository that owns them
- [x] Staleness is measured from git activity, never directory mtime
- [x] Every worktree states what would be lost by removing it
- [x] Remove, remove-with-branch, prune and lock all work, each behind a
      confirmation that spells out the cost first
- [x] Installs with `cargo install --path .`

## Explicitly not in this milestone

- Acting on more than one worktree at a time
- Any configuration beyond command-line flags
- Anything that has been run on Linux
