---
id: 47
title: Refresh when the disk changes
type: feature
status: backlog
milestone: later
created: 2026-09-17
updated: 2026-09-17
priority: p3
effort: l
area: runtime
---

## Problem

The view is a snapshot. Leave it open while an agent commits in a worktree and
it quietly goes stale.

## Proposal

Watch the git directories rather than the working trees — an index or reflog
write is the signal, and there are far fewer of them. Needs a dependency, which
has to earn its place.
