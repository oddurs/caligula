---
id: 15
title: Mark several worktrees at once
type: feature
status: backlog
milestone: v0.2
created: 2026-09-17
updated: 2026-09-17
priority: p0
effort: m
area: chrome
---

## Problem

Every action applies to exactly one row. A repository with forty-five dead
worktrees needs forty-five confirmations, which is enough friction that nobody
does it — and the mess this tool exists to find goes on sitting there.

## Proposal

A marking set, held by path so it survives re-sorting, re-filtering and the
re-probe after an action. Space marks and unmarks. Actions apply to the marking
when it is non-empty, and to the cursor row when it is not.

## Acceptance criteria

- [ ] `space` marks and unmarks the row under the cursor and moves down one
- [ ] Marked rows are unmistakable at a glance, not merely a different shade
- [ ] The footer shows how many are marked and what they add up to: files at
      risk, commits that exist only there
- [ ] The marking survives sort, lens, filter and a repository re-probe
- [ ] `esc` clears the marking before it does anything else
- [ ] A marked worktree that disappears from disk is dropped from the marking
