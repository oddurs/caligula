---
id: 14
title: Restore the terminal when the program panics
type: bug
status: backlog
milestone: v0.2
created: 2026-09-17
updated: 2026-09-17
priority: p0
effort: s
area: runtime
---

## What happens

A panic anywhere in the draw or event loop unwinds past `ratatui::restore()`.
The terminal is left in raw mode on the alternate screen: no echo, no prompt,
and the panic message is painted onto a screen that is about to be discarded.
The user has to type `reset` blind.

## What should happen

The terminal comes back first, then the panic message prints on the normal
screen where it can be read and reported.

## Reproduction

1. Introduce a panic in `ui::draw` (an out-of-range index will do)
2. Run `caligula`
3. The shell is unusable afterwards

## Acceptance criteria

- [ ] A panic hook restores the terminal before the default hook runs
- [ ] The panic message is legible on the normal screen after the program exits
- [ ] `stty -g` before and after a forced panic reports identical settings
- [ ] Restoration is idempotent: a normal quit after a shell-out is unaffected
