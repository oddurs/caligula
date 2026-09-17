---
id: 32
title: Describe the machine without starting the interface
type: feature
status: backlog
milestone: v0.3
created: 2026-09-17
updated: 2026-09-17
priority: p0
effort: m
area: cli
---

## Problem

Everything the scan learns is trapped behind a full-screen interface. The
obvious uses — a weekly report of what has gone stale, a pre-commit guard, a
prompt segment — all want a line of text.

## Acceptance criteria

- [ ] `--plain` prints one line per worktree and exits: repo, branch, verdict,
      age, path, tab-separated and stable
- [ ] Output is sorted deterministically; two runs over an unchanged disk match
- [ ] No colour, no spinner, no alternate screen when stdout is not a terminal
- [ ] Exit code 0 with results, 1 on a scan error, and 0 with no output when
      nothing matches
- [ ] Documented in `--help` with an example pipeline
