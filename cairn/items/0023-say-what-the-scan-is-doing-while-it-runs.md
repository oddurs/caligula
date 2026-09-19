---
id: 23
title: Say what the scan is doing while it runs
type: chore
status: done
milestone: v0.2
created: 2026-09-17
updated: 2026-09-19
priority: p2
effort: s
area: runtime
---

## Problem

The header counts directories walked. The walk finishes in well under a second;
what takes the remaining time is probing repositories, and during that the
counter sits still and the tool looks hung.

## Acceptance criteria

- [x] Progress reports repositories probed against repositories found, not just
      directories walked
- [x] The indicator disappears the moment the last worker finishes
- [x] No measurable cost to the scan itself
