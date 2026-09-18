---
id: 15
title: Mark several worktrees at once
type: feature
status: done
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

- [x] `space` marks and unmarks the row under the cursor and moves down one
- [x] Marked rows are unmistakable at a glance, not merely a different shade
- [x] The footer shows how many are marked and what they add up to: files at
      risk, commits that exist only there
- [x] The marking survives sort, lens, filter and a repository re-probe
- [x] `esc` clears the marking before it does anything else
- [x] A marked worktree that disappears from disk is dropped from the marking

## 2026-09-17

Review found five things worth fixing before this landed: a rescan left the marking populated so esc no longer quit; holding space stopped dead on a repo header; a status message hid the stakes for six seconds; the help overlay and README never learned the key; and a dialog opened while a marking was held read as though it applied to the marking. The last is recorded as a screen — the dialog now says which it means, until 0016 makes actions apply to the marking.
