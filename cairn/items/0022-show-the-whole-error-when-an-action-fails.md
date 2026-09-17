---
id: 22
title: Show the whole error when an action fails
type: bug
status: backlog
milestone: v0.2
created: 2026-09-17
updated: 2026-09-17
priority: p1
effort: s
area: actions
---

## What happens

A failed git command is reported on the single-line footer, which truncates it.
`git worktree remove` failures are often several lines and the useful part is
the last one.

## Acceptance criteria

- [ ] A failure opens a dismissible box carrying git's full stderr
- [ ] The command that was run is shown, so it can be repeated by hand
- [ ] `esc` or any key dismisses it; the list underneath is untouched
- [ ] Successes stay on the footer as they are now
