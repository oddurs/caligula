---
id: 20
title: Snapshot-test the rendered frame against a fixture repository
type: chore
status: done
milestone: v0.2
created: 2026-09-17
updated: 2026-09-17
priority: p0
effort: l
area: testing
---

## Problem

Every layout bug so far was found by rendering into a pseudo-terminal by hand
and reading the columns. That found two real ones — the age column falling off
the right edge, and a fold-all that did not hold — but it is not repeatable and
it does not run in CI.

## Proposal

A fixture builder that creates a throwaway repository with known worktrees
(clean, dirty, ahead, locked, prunable, orphaned), and snapshot tests that
render a frame through ratatui's `TestBackend` and compare with `insta`.

## Acceptance criteria

- [x] A fixture builder creates a repository with one worktree of each kind,
      deterministically, in a temporary directory that is cleaned up
- [x] Snapshots cover: the list, a repository detail, a worktree detail with
      changes, the confirmation dialog, the help overlay
- [x] Snapshots are stable across runs: no timestamps, paths or ids leak in
- [x] At least one snapshot at 80 columns and one at 200
- [x] `scripts/task test` runs them; they pass on a machine that has never run
      caligula before
