---
id: 11
title: Say what would be lost by deleting a worktree
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

The question is never "what worktrees exist". It is "which of these can I delete
without losing work".

## Acceptance criteria

- [x] Three verdicts: nothing to salvage, commits only here, uncommitted work
- [x] Commits that exist nowhere else are counted against the upstream, or
      against the base branch when there is no upstream
- [x] The detail pane leads with the verdict, coloured by severity
- [x] A clean worktree still shows its last commits, so you can tell what it was
