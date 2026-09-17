---
id: 39
title: Publish to crates.io
type: chore
status: backlog
milestone: v1.0
depends_on:
- 35
- 38
created: 2026-09-17
updated: 2026-09-17
priority: p0
effort: s
area: packaging
---

## Acceptance criteria

- [ ] `Cargo.toml` carries description, licence, repository, homepage, keywords
      and categories
- [ ] `cargo publish --dry-run` is clean, and the packaged crate contains no
      fixtures or scratch files
- [ ] `cargo install caligula` on a machine that has never seen the source
      produces a working binary
- [ ] The licence file is present and correct
