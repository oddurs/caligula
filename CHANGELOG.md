# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic
Versioning](https://semver.org/spec/v2.0.0.html).

Day-to-day work is tracked as [cairn items](cairn/items) and rendered into
[ROADMAP.md](ROADMAP.md); this file records what landed in a release.

## What semantic versioning covers here

Held stable, and broken only with a major version:

- the command-line interface — flags, their meanings, and the exit codes
- the keymap
- the configuration file, once there is one

Explicitly **not** an interface, and free to change in a patch release: the
arrangement of the screen. Which pane holds what, how a row is laid out, what
the detail pane shows and in what order — all of that is recorded in snapshots
so that it cannot change *by accident*, but it is expected to change on purpose
as the tool gets better at saying what it knows.

## [Unreleased]

## [0.2.0] — 2026-09-26

The first published release. Version 0.1.0 was the development version that
reached the milestone of the same name; it was never tagged and never released,
so everything from it is listed here.

### Added

- Every git worktree on the machine, found by walking a set of roots and asking
  git about each repository it finds, grouped under the repository that owns it.
- A verdict for each worktree saying what would be lost by deleting it: nothing,
  commits that exist only there, uncommitted work, or — when git cannot read the
  worktree at all — that nothing can be said about it either way.
- Staleness measured from git activity: the last commit, the per-worktree reflog
  and the index. Never directory mtimes, which editors and backup tools touch
  constantly and which report every checkout on a disk as fresh.
- Remove, remove-with-branch, prune and lock, each behind a confirmation that
  states the cost before it asks.
- A marking: `space` marks a worktree and steps down, `a` marks everything in a
  repository that is safe to remove, and `A` marks everything the view is
  showing. `d` removes the marking behind one confirmation that lists what will
  go worst-first, totals the files and commits that exist nowhere else, and
  names what it is keeping and why.
- A table: one column per fact — commits that exist only here, commits behind,
  files at risk, flags, age — under a header that names them. Columns are given
  up as the pane narrows in order of what they are worth, and a repository row
  carries its group in the same columns.
- Stashes attributed to the worktree whose branch they were made on, and the
  ones on no live branch reported against the repository, since nothing else
  will surface them.
- Lenses (all, dirty, salvageable, stale, safe to remove), three sort orders,
  and a filter over repository, branch and path.
- One pane at a time below a hundred columns, with `tab` between them.
- Failures shown in full, with the command that produced them, scrollable.

### Fixed

- `git()` and `git_run()` named their repository with `-C` and then inherited
  the ambient environment, where `GIT_DIR`, `GIT_WORK_TREE` and `GIT_INDEX_FILE`
  all outrank `-C`. Anything launched from a git hook would have probed every
  worktree on the machine against that one repository.
- The safe-to-remove lens offered locked worktrees, which git refuses to remove,
  and unreadable ones, whose counters are all zero because nothing was ever read
  — so the absence of data was being reported as the absence of work.
- Summaries that described a different set than the one on screen: the marking
  totals, the removal dialog, the repository row, its ordering and the detail
  pane each counted worktrees a lens was hiding.
- Rows wider than their pane, which silently lost their right-hand end — the age
  column in the list, and every row of the repository detail.

### Known limits

- macOS and Linux only. Nothing here has run on Windows.
- Creating worktrees is out of scope; caligula finds, judges and removes them.
- Nothing is read from a remote. Every judgement is made from what is on disk.
