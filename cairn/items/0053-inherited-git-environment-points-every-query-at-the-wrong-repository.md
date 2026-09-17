---
id: 53
title: Inherited git environment points every query at the wrong repository
type: bug
status: doing
milestone: v0.2
assignee: Oddur Sigurdsson
claimed: 2026-09-17
created: 2026-09-17
updated: 2026-09-17
priority: p0
effort: s
area: git
---

## What happens

`git()` and `git_run()` address a repository by `-C <path>` and then inherit the
ambient environment. `GIT_DIR`, `GIT_WORK_TREE` and `GIT_INDEX_FILE` all take
precedence over `-C`, so when any of them is set every query is answered by
whichever repository the environment names, not the one caligula asked about.

Run caligula from inside a git hook — a `pre-push` that calls a script, say —
and it inherits exactly those variables. Every worktree on the machine is then
probed against one repository: wrong branches, wrong verdicts, and a removal
dialog naming a path that has nothing to do with what it is about to read.

Found while running the new snapshot suite from a `pre-push` hook: eight of nine
probe tests fail with `GIT_DIR` set, and the suite is green without it.

## What should happen

caligula names the repository it means and nothing in the environment can
redirect it.

## Reproduction

```sh
GIT_DIR=/some/other/repo/.git cargo test --test probe   # 8 of 9 fail
GIT_INDEX_FILE=/tmp/x.idx     cargo test --test probe   # 8 of 9 fail
```

## Acceptance criteria

- [x] `git()` and `git_run()` remove every repository-selecting variable from
      the child environment: GIT_DIR, GIT_WORK_TREE, GIT_INDEX_FILE,
      GIT_COMMON_DIR, GIT_OBJECT_DIRECTORY, GIT_NAMESPACE
- [x] A test asserts the removals are registered on the command, without
      mutating the environment of the test process
- [ ] `GIT_DIR=<another repo> scripts/task test` is green
- [ ] The test fixture is hermetic for the same reason, so it cannot pass only
      because the ambient environment happened to be empty

## 2026-09-17

Criteria 1 and 2 land in this branch. Criteria 3 and 4 need the integration suite and its fixture, which are item 0020's deliverable and not yet on main — they get proven and ticked when 0020 rebases onto this and merges.

## 2026-09-17

Criterion 4 is a safety requirement, not tidiness. Proving this fix by running 'GIT_DIR=<the real repo>/.git cargo test' let the fixture's own 'git init' and 'git worktree add' execute against the caligula repository: it set core.bare=true, created five fixture branches, and left six worktree records pointing into a temp directory. Nothing was pushed and it was fully reversible — unset core.bare, unlock, prune, delete the branches — but a contributor who runs the suite from inside a git hook would do the same to their own repository without knowing why. git_in must remove the same variables git() now does.
