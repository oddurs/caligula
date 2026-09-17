---
id: 14
title: Restore the terminal when the program panics
type: bug
status: dropped
milestone: v0.2
assignee: Oddur Sigurdsson
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

## 2026-09-17

Not a bug. ratatui::init() already installs a panic hook that restores the terminal before the message prints — documented in ratatui 0.29's terminal/init.rs and verified against a real panic: with a deliberate panic!() in ui::draw, the message printed legibly on the normal screen, the process exited 101, and 'stty -g' was byte-identical before and after. Adding a second hook changed nothing, so the change was reverted and only a comment at the init() call site kept, so the next person does not repeat the investigation.

One real but minor thing found on the way: the shell-out path calls ratatui::init() again on return, and each init chains another hook onto the existing one. A long session with many shell-outs accumulates hooks, each calling restore(). Idempotent and bounded by the number of shell-outs, so it is noted rather than filed.
