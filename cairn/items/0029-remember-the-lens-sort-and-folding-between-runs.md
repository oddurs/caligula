---
id: 29
title: Remember the lens, sort and folding between runs
type: feature
status: backlog
milestone: v0.3
depends_on:
- 27
created: 2026-09-17
updated: 2026-09-17
priority: p2
effort: s
area: config
---

## Problem

Anyone who uses this daily uses it in one shape — the salvageable lens, sorted
by risk — and sets it up again every time.

## Acceptance criteria

- [ ] Lens, sort and the set of folded repositories persist across runs
- [ ] State lives in the state directory, never in the config file
- [ ] `--lens` and `--sort` on the command line win, and do not overwrite state
- [ ] A corrupt or unreadable state file is ignored, not fatal
