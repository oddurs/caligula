---
id: 22
title: Show the whole error when an action fails
type: bug
status: done
milestone: v0.2
created: 2026-09-17
updated: 2026-09-19
priority: p1
effort: s
area: actions
---

## What happens

A failed git command is reported on the single-line footer, which truncates it.
`git worktree remove` failures are often several lines and the useful part is
the last one.

## Acceptance criteria

- [x] A failure opens a dismissible box carrying git's full stderr
- [x] The command that was run is shown, so it can be repeated by hand
- [x] `esc` or any key dismisses it; the list underneath is untouched
- [x] Successes stay on the footer as they are now

## 2026-09-19

Review found the box reproduced the defect it was written to remove: it clipped, with no scroll and no indicator, and what it clipped was the tail — where git puts the sentence that matters, and where a sweep's later failures land. It now scrolls and says how many lines are left. Three more: the removal path showed the literal string 'git worktree remove' with no -C, no path and no --force, so the one command in the box that mattered most was the one that could not be repeated; branch-deletion failures still went only to the one-line footer, which the box is drawn over; and the box swallowed ctrl-c, so the first press no longer quit.
