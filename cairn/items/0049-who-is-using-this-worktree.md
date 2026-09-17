---
id: 49
title: Who is using this worktree?
type: spike
status: backlog
milestone: later
created: 2026-09-17
updated: 2026-09-17
priority: p3
effort: l
area: git
---

## Question

Can caligula say which worktrees are in use right now — an editor open in one,
a build running in another, an agent working in a third — and is that worth the
machinery it would take?

## Why it has to be answered before the work

`deepwork` has 45 worktrees under `.claude/worktrees/`. "Stale" is a decent
proxy for "nobody is using this", but an agent that has been running for an hour
without committing looks identical to a worktree abandoned in March. Getting
that wrong means the sweep deletes live work.

## Options

- Processes with a working directory inside the tree, via `lsof` or `/proc`
- Lock files and editor droppings: `.swp`, editor server sockets
- Agent session state, if any of it is on disk in a documented place

## What would settle it

Whether the signal is reliable enough to gate a destructive action on. If it is
only advisory, it belongs in the detail pane and nowhere near the sweep.

## Answer

<!-- Filled in when the spike closes. -->
