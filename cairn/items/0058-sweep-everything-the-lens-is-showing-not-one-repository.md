---
id: 58
title: Sweep everything the lens is showing, not one repository
type: feature
status: done
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

- [x] `A` marks every visible worktree that is safe to remove, across repositories
- [x] It respects the lens and the filter, reaching no further than what is shown
- [x] It never marks the main checkout, a locked worktree, or an unreadable one
- [x] Pressing it again clears that marking rather than doubling it
- [x] It says how many it marked and across how many repositories
- [x] The key is in the help overlay and the README

## 2026-09-20

Review found that adding one row to the help overlay pushed r, R and q off the bottom — popup clamps to the terminal height and Paragraph clips in silence, so on an 80x30 terminal the screen whose whole job is to say how to do things had stopped saying how to quit. It scrolls now, like the failure box, with a test that walks four heights and insists quit is reachable at each.

Also: the 'Marked n' message reported the size of the whole safe set rather than what the keystroke added, so marking two by hand and then pressing A said 'Marked 4' when it had added two. Both sweeps now say what they did; the footer already carries the total.
