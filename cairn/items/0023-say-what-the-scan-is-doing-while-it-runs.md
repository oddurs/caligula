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

## 2026-09-19

Review found the denominator meant nothing, and that the on-machine check I did was fooled by it. found was incremented where a worker claims a candidate, immediately before reading it — so found minus examined could never exceed the number of workers, and 'reading 70 of 82' was a queue of hundreds displayed as almost finished. The 12 I read as progress was the worker count. Counting is now done in the walker, where checkouts are discovered, and the gap on this machine is 41 rather than 12.

Three more from the same review: describe() branched on whether a probe happened to be in flight, so the header alternated between two unrelated sentences several times a second, and now branches on the stage; candidates whose probe failed were counted as repositories found, so the final total could exceed the rows actually listed; and the stale Progress event overwrote the fresher atomic, making the directory count jump backwards.
