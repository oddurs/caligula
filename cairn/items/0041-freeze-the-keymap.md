---
id: 41
title: Freeze the keymap
type: chore
status: backlog
milestone: v1.0
created: 2026-09-17
updated: 2026-09-17
priority: p0
effort: s
area: chrome
---

## Problem

Keys have accumulated one at a time. Before promising stability, the map should
be looked at as a whole: `d`/`D` sit next to each other and one deletes a branch;
`c` shells out while `y` copies; `z`/`Z` fold. Some of that is muscle memory from
other tools and some of it is an accident of the order things were built.

## Acceptance criteria

- [ ] Every binding reviewed together, against the tools it will sit beside
- [ ] Destructive keys are not adjacent to navigation keys on a QWERTY layout
- [ ] Any change made now, before anyone depends on it, with the reason recorded
- [ ] The map is documented in one place that `--help`, the help overlay and the
      man page all derive from
- [ ] A test asserts no key is bound twice
