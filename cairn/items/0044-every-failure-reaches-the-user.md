---
id: 44
title: Every failure reaches the user
type: chore
status: backlog
milestone: v1.0
created: 2026-09-17
updated: 2026-09-17
priority: p0
effort: m
area: runtime
---

## Problem

`git()` returns `Option` and a `None` is currently indistinguishable from a
clean, empty result. A repository that fails to probe for an interesting reason
shows up as a normal repository with nothing in it.

## Acceptance criteria

- [ ] Audit every `ok()`, `unwrap_or_default()` and `let ... else` that drops an
      error, and decide for each: act on it, surface it, or comment why it is
      genuinely nothing
- [ ] A repository that fails to probe is shown as failed, with the reason
      available, rather than as empty
- [ ] A scan-wide failure — an unreadable root, git missing — is reported at the
      top and does not look like an empty disk
- [ ] No `unwrap` or `expect` outside tests and `main` without an invariant
      written on the line
