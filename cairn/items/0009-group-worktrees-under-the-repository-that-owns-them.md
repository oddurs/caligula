---
id: 9
title: Group worktrees under the repository that owns them
type: feature
status: done
milestone: v0.1
created: 2026-09-17
updated: 2026-09-17
priority: p0
effort: m
area: chrome
---

## Problem

A flat list of ninety worktrees is a list of paths, not an inventory.

## Acceptance criteria

- [x] Every worktree appears under its repository, main checkout first
- [x] A repository row carries its own totals: linked worktrees, dirty count
- [x] Groups fold and unfold, and fold-all holds for repositories the scan has
      not reached yet
- [x] A group scrolled past the top of the viewport keeps a sticky header
