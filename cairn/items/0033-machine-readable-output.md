---
id: 33
title: Machine-readable output
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

## Acceptance criteria

- [ ] `--json` emits one object per worktree with every field the detail pane
      shows, including the salvage verdict as a stable enum string
- [ ] The shape is documented in the README and treated as an interface
- [ ] `--json` and `--plain` together is an error, not a silent precedence
- [ ] Field names are snake_case and never null: absent is absent
