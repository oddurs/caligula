---
id: 31
title: Find worktree directories no repository claims
type: feature
status: backlog
milestone: v0.3
created: 2026-09-17
updated: 2026-09-17
priority: p0
effort: m
area: scan
---

## Problem

Delete a repository and its linked worktrees are orphaned: the directories
survive, their `.git` files point at a git directory that is gone, and nothing
lists them. `git worktree list` cannot help — there is no repository left to ask.
These are pure dead weight and the current scan walks straight past them.

## Proposal

During the walk, a directory whose `.git` is a file pointing at a git directory
that does not exist is an orphan. Group them under a synthetic "orphaned" entry.

## Acceptance criteria

- [ ] An orphan is detected from its dangling `gitdir:` pointer
- [ ] Orphans are grouped separately, with the repository path they once had
- [ ] The verdict is honest: the working tree can be read, but nothing about
      commits or upstreams can be, and it says so rather than guessing
- [ ] Removing an orphan deletes the directory, since git cannot
- [ ] That removal is guarded by its own confirmation naming the directory
- [ ] Fixture covers: dangling pointer, unreadable pointer, `.git` file that is
      not a pointer at all
