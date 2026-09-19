---
id: 54
title: Lay the list out as a real table
type: feature
status: done
milestone: v0.2
assignee: Oddur Sigurdsson
created: 2026-09-18
updated: 2026-09-18
priority: p0
effort: m
area: chrome
---

## Problem

The right-hand side of a row is three unrelated facts concatenated into one
string and right-aligned as a blob, so nothing lines up down the list:

```
 │ ● chore/verify-workflow~2   18h
 │ ● feat/0049-classify  ↓58 ~8  9d
 ▾ deepwork             18wt 1●
```

`~2`, `↓58 ~8` and `18wt 1●` each begin at a different column, so the eye has
no column to run down and it reads as noise rather than as data. There is no
header, so `~` and `↓` are unexplained. And a repository's worktree count is
painted red because the whole right-hand string takes the colour of the dirty
count next to it — a neutral number wearing an alarm.

## Proposal

One column per fact, each right-aligned in a fixed width, under a header row
that names them: ahead, behind, changed files, flags, age. Empty means zero, so
there is no glyph to decode. Colour goes on the value, not on the run it
happens to sit in.

## Acceptance criteria

- [x] A header row names the columns, and stays put while the list scrolls
- [x] Ahead, behind, changed files, flags and age each occupy a fixed column,
      right-aligned, identical on every row
- [x] A zero is blank rather than `0`
- [x] A repository's worktree count is never coloured by its dirty count
- [x] Columns drop as the pane narrows in order of what they are worth, not
      of where they sit: uncommitted files last, then the locked/broken flags,
      then commits that exist only here, and the behind-count first
- [x] Rows are still exactly as wide as the pane at 60, 80, 120 and 200 columns
- [x] Snapshots record the table at each of those widths
