---
id: 34
title: Filter the non-interactive output the way the lenses do
type: feature
status: backlog
milestone: v0.3
depends_on:
- 32
created: 2026-09-17
updated: 2026-09-17
priority: p1
effort: s
area: cli
---

## Problem

`--plain | grep safe` works until a branch is called `safe-mode`.

## Acceptance criteria

- [ ] `--lens <all|dirty|salvageable|stale|safe>` applies to `--plain` and
      `--json` and means exactly what the interactive lens means
- [ ] `--repo <glob>` and `--older-than <duration>` narrow the set
- [ ] Shared with the interactive code path, so the two cannot drift
- [ ] `caligula --plain --lens safe` is the documented answer to "what can I
      delete", and is tested against a fixture
