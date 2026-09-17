---
id: 35
title: fmt, clippy and the suite green on macOS and Linux in CI
type: chore
status: backlog
milestone: v1.0
created: 2026-09-17
updated: 2026-09-17
priority: p0
effort: m
area: packaging
---

## Problem

Everything has been checked on one machine, by hand, on macOS.

## Acceptance criteria

- [ ] A workflow runs `scripts/task check` on macOS and Linux for every push
      and pull request
- [ ] Clippy runs with `-D warnings`; formatting is checked, not applied
- [ ] The toolchain is the one pinned in `rust-toolchain.toml`
- [ ] Caches make a warm run take under two minutes
- [ ] A red build blocks the merge

## 2026-09-17

CI landed with the repository bootstrap: .github/workflows/ci.yml runs scripts/task check on ubuntu-latest and macos-latest, with a 'required' aggregate job that branch protection requires. What is left of this item is confirming it stays green and that the matrix is the minimum meaningful set.
