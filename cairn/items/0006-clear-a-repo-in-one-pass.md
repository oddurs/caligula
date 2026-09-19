---
id: 6
key: v0.2
title: Clear a repo in one pass
type: milestone
status: done
created: 2026-09-17
updated: 2026-09-19
due: 2026-10-03
---

## Ships

Mark many worktrees at once and clear a repository's dead ones in a single pass.

## Done when

- [x] Worktrees can be marked, and every action applies to the whole marking
- [x] One keystroke marks everything in a repo that is safe to remove
- [x] The verdict distinguishes untracked scratch from modified tracked files
- [x] Stashes are attributed to the worktree that made them
- [x] A panic cannot leave the terminal unusable
- [x] The rendered frame is covered by snapshot tests against a fixture repo

## Explicitly not in this milestone

- A configuration file
- Any non-interactive output
- Undo: removal is still final in v0.2
