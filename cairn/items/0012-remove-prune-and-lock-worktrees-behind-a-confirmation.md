---
id: 12
title: Remove, prune and lock worktrees behind a confirmation
type: feature
status: done
milestone: v0.1
created: 2026-09-17
updated: 2026-09-17
priority: p0
effort: m
area: actions
---

## Acceptance criteria

- [x] `d` removes a worktree and keeps its branch; `D` deletes the branch too
- [x] A dirty worktree is removed only with `--force`, and the dialog says so
- [x] The dialog states the cost before it asks: path, branch, what is lost
- [x] `p` prunes stale administrative records, `L` locks and unlocks
- [x] The repository is re-probed after every action, so the list cannot lie
- [x] Verified against `git worktree list` on a throwaway repository
