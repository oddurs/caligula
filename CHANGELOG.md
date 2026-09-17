# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic
Versioning](https://semver.org/spec/v2.0.0.html).

Day-to-day work is tracked as [cairn items](cairn/items) and rendered into
[ROADMAP.md](ROADMAP.md); this file records what landed in a release.

## [Unreleased]

### Added

- Every git worktree on the machine, found by walking a set of roots and asking
  git about each repository it finds, grouped under the repository that owns it.
- A verdict for each worktree saying what would be lost by deleting it: nothing,
  commits that exist only there, or uncommitted work.
- Staleness measured from git activity — the last commit, the per-worktree
  reflog and the index — rather than from directory mtimes, which every editor
  and backup tool touches.
- Remove, remove-with-branch, prune and lock, each behind a confirmation that
  states the cost first.
- Lenses (all, dirty, salvageable, stale, safe to remove), three sort orders,
  and a filter over repository, branch and path.
