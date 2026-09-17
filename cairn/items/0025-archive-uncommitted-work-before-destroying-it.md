---
id: 25
title: Archive uncommitted work before destroying it
type: feature
status: backlog
milestone: v0.3
depends_on:
- 24
created: 2026-09-17
updated: 2026-09-17
priority: p0
effort: l
area: actions
---

## Problem

`d` on a dirty worktree is irreversible. The confirmation dialog is the only
thing between a marked sweep and losing an afternoon, and a dialog is not a
safety net — it is a speed bump.

## Proposal

Before a forced removal, write everything the worktree holds that exists nowhere
else — modified tracked files, untracked files, the index — into the archive
decided by the spike. Removal then costs nothing but time.

## Acceptance criteria

- [ ] A forced removal writes an archive first, and aborts the removal if the
      archive fails
- [ ] The archive captures modified tracked files, staged changes and untracked
      files, excluding ignored ones
- [ ] It records where it came from: repository, branch, head sha, timestamp
- [ ] The removal dialog says where the archive will be written
- [ ] A clean removal writes nothing
- [ ] Round-trip test: dirty a fixture worktree, remove it, restore the archive,
      compare against the original tree byte for byte
