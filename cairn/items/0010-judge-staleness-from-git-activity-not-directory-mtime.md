---
id: 10
title: Judge staleness from git activity, not directory mtime
type: feature
status: done
milestone: v0.1
created: 2026-09-17
updated: 2026-09-17
priority: p0
effort: m
area: verdict
---

## Problem

Directory mtimes are touched by editors, builds and backup tools. Measured that
way, every checkout on the disk reports as fresh — 92 repositories here all read
"2h" on the first build, which is worse than showing nothing.

## Proposal

Take the newest of: the last commit, the per-worktree reflog, and the index.
Fall back to the directory only for a repository with no commits at all.

## Acceptance criteria

- [x] Staleness comes from commit time, `logs/HEAD` and `index`
- [x] A worktree git cannot read reports no age rather than a fabricated one
- [x] Four bands, coloured: active, recent, stale, ancient
