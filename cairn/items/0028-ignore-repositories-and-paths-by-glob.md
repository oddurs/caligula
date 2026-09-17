---
id: 28
title: Ignore repositories and paths by glob
type: feature
status: backlog
milestone: v0.3
depends_on:
- 27
created: 2026-09-17
updated: 2026-09-17
priority: p1
effort: s
area: config
---

## Problem

Some repositories are never yours to clean: vendored checkouts, a colleague's
clone, anything under a mount you would rather not walk.

## Acceptance criteria

- [ ] `ignore = ["~/vendor/**", "**/node_modules/**"]` in the config file
- [ ] Ignored paths are not walked at all, not merely hidden afterwards
- [ ] `--all` shows what is being ignored and why, for debugging a pattern
- [ ] Tested against patterns with `~`, `**`, and a trailing slash
