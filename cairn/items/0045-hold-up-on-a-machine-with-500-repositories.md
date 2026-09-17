---
id: 45
title: Hold up on a machine with 500 repositories
type: chore
status: backlog
milestone: v1.0
created: 2026-09-17
updated: 2026-09-17
priority: p1
effort: l
area: runtime
---

## Problem

92 repositories and 189 worktrees settle in under three seconds. That is one
machine, warm, on an SSD. Probing is several git invocations per worktree, and
`R` re-probes everything from scratch.

## Proposal

Measure first on a synthetic tree of 500 repositories, cold and warm. Only then
decide what to change — fewer invocations, a cache keyed on the git directory's
mtime, or a cheaper first pass that fills detail lazily.

## Acceptance criteria

- [ ] A fixture generator builds a tree of 500 repositories with worktrees
- [ ] Measurements recorded before any change: cold, warm, and re-probe
- [ ] First rows on screen in under 500ms regardless of tree size
- [ ] The interface stays responsive to keys throughout the scan
- [ ] A benchmark in CI that fails on a regression beyond a stated threshold
- [ ] Whatever is changed, `--plain` over 500 repositories stays correct
