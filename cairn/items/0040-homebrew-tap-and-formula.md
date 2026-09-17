---
id: 40
title: Homebrew tap and formula
type: chore
status: backlog
milestone: v1.0
depends_on:
- 37
created: 2026-09-17
updated: 2026-09-17
priority: p1
effort: m
area: packaging
---

## Acceptance criteria

- [ ] A tap repository with a formula installing the released binary
- [ ] `brew install <tap>/caligula` works on arm64 and x86_64 macOS
- [ ] The release workflow opens the formula bump automatically
- [ ] `brew audit --strict` passes
