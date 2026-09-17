---
id: 46
title: Preview the diff in the detail pane
type: feature
status: backlog
milestone: later
created: 2026-09-17
updated: 2026-09-17
priority: p3
effort: m
area: chrome
---

## Problem

The detail pane lists which files changed. Deciding whether the change matters
still means leaving for another tool.

## Proposal

A diff of the selected file, or a `--stat` of the whole worktree, inside the
pane. Costs a git invocation per selection, so it has to be lazy.
