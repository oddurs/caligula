---
id: 19
title: Attribute stashes to the worktree that made them
type: feature
status: done
milestone: v0.2
created: 2026-09-17
updated: 2026-09-19
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

- [x] A stash made on a branch is shown against the worktree holding that branch
- [x] Stashes that match no live worktree stay at repository level
- [ ] A worktree with a stash of its own is never "nothing to salvage"
- [x] The detail pane shows each matched stash: its message and its age
- [x] Parsing is tested against `stash list` output, including a detached HEAD
      entry and a branch name containing a colon

## 2026-09-19

Criterion 3 — 'a worktree with a stash of its own is never nothing to salvage' — deliberately not done, on a fact I checked rather than assumed. refs/stash lives in the common dir, so a stash is a repository-level ref: removing the worktree leaves it untouched. Verified in a throwaway repo — stashed on feat/x, removed the worktree with --force, and the stash was still listed afterwards. Counting it as work that would be destroyed would be false, and this tool's whole claim is that its verdict can be trusted. The stash is surfaced against the worktree instead, which is what makes it findable.

Criterion 5 asked for a test covering a branch name containing a colon. git forbids colons in ref names — 'git branch bad:name' is refused by check-ref-format — so there is no such case. The test covers what actually occurs instead: a detached-head stash, which git records as '(no branch)', and a branch with a slash.
