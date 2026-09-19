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

## 2026-09-19

Review found the pane label was a replacement rather than a prefix: the footer returned as soon as the terminal was narrow, so below a hundred columns the filter never rendered while being typed, every status message was swallowed, and the marking stakes — the line that says what a removal is about to destroy — disappeared at exactly the width where the panes cannot say it themselves.

Worse, I re-accepted the snapshot that recorded it. During the rebase I ran INSTA_UPDATE=always across the conflicts without reading the diffs, and a_marking_under_a_filter_on_a_narrow_terminal — written two items earlier to guard precisely this — was updated to assert the regression. The rule in CLAUDE.md about never re-recording without reading what moved exists for this, and I broke it.

Also: a_repository_detail_at_eighty_columns stopped rendering a repository detail at all once the layout changed, so it would have passed through any regression in repo_detail; the header budget counted a separator before the first part and dropped a fact that fitted; tab was in no help overlay; and two of the three new snapshot tests were byte-identical to existing ones.
