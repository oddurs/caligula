---
id: 24
title: Where does archived work go, and when does it expire?
type: spike
status: backlog
milestone: v0.3
created: 2026-09-17
updated: 2026-09-17
priority: p0
effort: s
area: actions
---

## Question

When caligula destroys uncommitted work, where does the copy live, what format
is it in, and who deletes it?

## Why it has to be answered before the work

The archive is the feature that makes bulk removal defensible. Get the location
wrong and it is either inside the worktree being deleted, or somewhere nobody
finds again. Get expiry wrong and it silently eats disk forever.

## Options

- **Inside the repository** — `.git/caligula/archive/`. Travels with the repo,
  survives the worktree, dies with a `git clone --mirror`. But it is inside a
  directory git owns, and `git gc` has opinions.
- **A central store** — `~/.local/state/caligula/archive/<repo>/<branch>/`.
  One place to look, one place to prune, survives deleting the repository.
- **A stash entry** — `git stash create` plus a ref under `refs/caligula/`.
  Native, recoverable with git alone, invisible to `stash list`. But it cannot
  hold untracked files without `-u`, and it dies with the repository.

## What would settle it

Try each against the real cases: untracked files only, a submodule, a file with
a colon in the name, a worktree whose repository is deleted a week later. Then
decide expiry: never, N days, or bounded total size.

## Answer

<!-- Filled in when the spike closes. -->
