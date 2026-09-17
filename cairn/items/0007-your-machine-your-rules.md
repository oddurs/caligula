---
id: 7
key: v0.3
title: Your machine, your rules
type: milestone
status: backlog
created: 2026-09-17
updated: 2026-09-17
due: 2026-10-24
---

## Ships

Configured for your disk, and no longer losing work when it removes something.

## Done when

- [ ] Roots, ignores and staleness thresholds come from a config file
- [ ] Orphaned worktree directories are found and can be cleared
- [ ] `--plain` and `--json` describe the machine without starting the TUI
- [ ] Uncommitted work is archived before it is destroyed, and can be restored

## Explicitly not in this milestone

- Creating worktrees
- Watching the filesystem for changes
- Packaging for anything but `cargo install --path .`
