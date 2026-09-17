---
id: 6
key: v0.2
title: Clear a repo in one pass
type: milestone
status: backlog
created: 2026-09-17
updated: 2026-09-17
due: 2026-10-03
---

## Ships

Mark many worktrees at once and clear a repository's dead ones in a single pass.

## Done when

- [ ] Worktrees can be marked, and every action applies to the whole marking
- [ ] One keystroke marks everything in a repo that is safe to remove
- [ ] The verdict distinguishes untracked scratch from modified tracked files
- [ ] Stashes are attributed to the worktree that made them
- [ ] A panic cannot leave the terminal unusable
- [ ] The rendered frame is covered by snapshot tests against a fixture repo

## Explicitly not in this milestone

- A configuration file
- Any non-interactive output
- Undo: removal is still final in v0.2
