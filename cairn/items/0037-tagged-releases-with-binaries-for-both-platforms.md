---
id: 37
title: Tagged releases with binaries for both platforms
type: chore
status: backlog
milestone: v1.0
depends_on:
- 35
created: 2026-09-17
updated: 2026-09-17
priority: p1
effort: m
area: packaging
---

## Acceptance criteria

- [ ] Pushing a `v*` tag builds macOS (arm64, x86_64) and Linux (x86_64) binaries
- [ ] Artefacts are attached to a GitHub release with checksums
- [ ] The release notes come from the CHANGELOG entry for that version
- [ ] The workflow refuses to release if the tag and `Cargo.toml` disagree
- [ ] Dry-run documented, so a release can be rehearsed
