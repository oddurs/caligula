---
id: 19
title: Attribute stashes to the worktree that made them
type: feature
status: backlog
milestone: v0.2
created: 2026-09-17
updated: 2026-09-17
priority: p1
effort: m
area: verdict
---

## Problem

A stash is salvageable work, and today it is reported once at the repository
level: "2 entries (shared across this repo)". Standing in front of a worktree
you are about to delete, that does not answer whether one of those stashes is
yours.

## Proposal

`git stash list` records the branch each entry was made on. Match entries to the
worktree checked out on that branch, and count the rest as repository-level.

## Acceptance criteria

- [ ] A stash made on a branch is shown against the worktree holding that branch
- [ ] Stashes that match no live worktree stay at repository level
- [ ] A worktree with a stash of its own is never "nothing to salvage"
- [ ] The detail pane shows each matched stash: its message and its age
- [ ] Parsing is tested against `stash list` output, including a detached HEAD
      entry and a branch name containing a colon
