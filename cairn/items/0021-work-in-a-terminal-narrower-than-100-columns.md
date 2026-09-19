---
id: 21
title: Work in a terminal narrower than 100 columns
type: bug
status: done
milestone: v0.2
created: 2026-09-17
updated: 2026-09-19
priority: p1
effort: m
area: chrome
---

## What happens

The layout gives the list 42% of the width and the detail pane the rest. Below
about 100 columns the detail pane is too narrow for the branch line and the
working-tree list, and both panes become cramped rather than one being useful.

## What should happen

Below a threshold the interface shows one pane at a time — the list, or the
detail — and a key moves between them.

## Acceptance criteria

- [x] Below the threshold only one pane is drawn, and it uses the full width
- [x] A key toggles between list and detail, and the footer says so
- [x] Nothing is drawn outside its pane at 60, 80 and 100 columns
- [x] Resizing across the threshold in either direction keeps the selection
- [x] Snapshot tests at 60 and 80 columns

## 2026-09-18

At 80 columns the list pane is clamped to 34, so branch names truncate hard (fe…/ahead) even though the numeric columns now fit. That is this item's problem to solve — one pane at a time below the threshold, rather than two cramped ones.
