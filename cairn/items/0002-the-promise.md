---
id: 2
key: v1.0
title: The promise
type: milestone
status: backlog
depends_on:
- 1
created: 2026-09-17
updated: 2026-09-17
due: 2026-11-21
---

## Ships

Installable in one command on macOS and Linux, and documented for a stranger.

> See every git worktree on this machine and safely remove the ones with
> nothing left in them.

That sentence, kept: a frozen keymap, an output format worth being held to, and
every failure path reaching the person in front of it.

## Done when

- [ ] `brew install` or `cargo install caligula` works on a machine that has
      never seen the source
- [ ] Every action has been run by hand on Linux, not merely compiled for it
- [ ] fmt, clippy and the full suite are green on macOS and Linux in CI
- [ ] A stranger can install it, understand a row, and clear a repo from the
      README alone
- [ ] The keymap is frozen and documented in both `--help` and the man page
- [ ] Every failure path reaches the user; nothing is swallowed
- [ ] 500 repositories do not stall the interface

## Explicitly not in this milestone

- **Creating worktrees.** Branch and worktree creation stays with `scripts/agent`.
- **Anything that talks to a remote.** No fetch, no push, no PR status; every
  judgement is made from what is already on disk.
- **Windows.** macOS and Linux only, said plainly in the README.

Nothing in v1.0 is a new feature. Anything exciting belongs in `later`.
