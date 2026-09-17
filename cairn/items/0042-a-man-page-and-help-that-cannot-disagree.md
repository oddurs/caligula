---
id: 42
title: A man page and --help that cannot disagree
type: docs
status: backlog
milestone: v1.0
depends_on:
- 41
created: 2026-09-17
updated: 2026-09-17
priority: p1
effort: s
area: docs
---

## Acceptance criteria

- [ ] `caligula.1` generated from the same source as `--help`
- [ ] Every flag, every key and every config key appears in it
- [ ] It is installed by the release artefacts and the Homebrew formula
- [ ] CI fails if the generated page is stale
