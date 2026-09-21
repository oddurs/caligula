---
id: 58
title: Sweep everything the lens is showing, not one repository
type: feature
status: backlog
milestone: v0.3
created: 2026-09-20
updated: 2026-09-20
priority: p1
effort: s
area: actions
---

## Problem

The safe lens gathers every removable worktree on the machine into one screen —
27 of them here, across ten repositories. The gesture it invites is "take all of
these". There is no key for that: `a` sweeps the repository under the cursor, so
clearing what the lens found means visiting ten repositories and pressing the
same two keys in each.

## Proposal

`A` marks everything the current view shows that is safe to remove, whatever
repository it is in. The same guard as `a` — never the main checkout, never a
locked worktree, never one git could not read — and the same confirmation, which
already lists worst-first and totals across repositories.

It reaches no further than the view: with no lens and no filter that is the whole
machine, which is exactly what the confirmation dialog is for.

## Acceptance criteria

- [ ] `A` marks every visible worktree that is safe to remove, across repositories
- [ ] It respects the lens and the filter, reaching no further than what is shown
- [ ] It never marks the main checkout, a locked worktree, or an unreadable one
- [ ] Pressing it again clears that marking rather than doubling it
- [ ] It says how many it marked and across how many repositories
- [ ] The key is in the help overlay and the README
