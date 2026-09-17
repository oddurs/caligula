# Security

## Supported versions

caligula is pre-1.0. Only the latest release receives fixes.

## Reporting a vulnerability

Report privately through GitHub Security Advisories:

<https://github.com/oddurs/caligula/security/advisories/new>

Include what you can: the version (`caligula --version`), your platform, the
roots it was pointed at, and the smallest reproduction you have. Please do not
open a public issue for anything that could destroy someone's work.

Expect an acknowledgement within a week and an assessment within two. If a fix
is warranted it ships in the next release, and the advisory is published with
credit unless you would rather it was not.

## What caligula does, so you can judge the risk

- **It walks the filesystem.** Every directory under the configured roots is
  read, to a bounded depth. It follows no symlinks and descends into no
  repository it has already identified.
- **It runs `git`.** Read-only inspection runs with `GIT_OPTIONAL_LOCKS=0` so it
  cannot disturb a build. Nothing is executed from a repository's configuration,
  and no hook, alias or `core.fsmonitor` command is invoked on caligula's behalf
  beyond what a plain `git status` would run.
- **It deletes things, on purpose, when you confirm.** `git worktree remove`,
  `git branch -D`, `git worktree prune`. Every one of them is behind a
  confirmation that states the cost first. This is the feature, and it is also
  the largest thing that can go wrong.
- **It runs two things you name.** Your `$SHELL`, in a worktree you selected,
  and the platform file manager. It passes them a path and nothing else.
- **It sends nothing anywhere.** No network calls, no telemetry, no crash
  reporting. It never contacts a git remote: every judgement is made from what
  is already on the disk.

## What would be a vulnerability

- Destroying work without the confirmation that names it — a path that reaches
  `--force` removal from a keystroke the dialog did not describe.
- A verdict that reports "nothing to salvage" for a worktree that holds
  something. Being wrong in that direction is a data-loss bug, and it is treated
  as one.
- Anything that lets content inside a scanned repository — a branch name, a
  path, a commit subject, a config value — cause a command to run, escape a
  terminal escape sequence into the display, or make caligula act outside the
  roots it was given.
- Reading or writing outside the configured roots and caligula's own state
  directory.

Crashing on a malformed repository is a bug, and worth reporting, but it is not
a vulnerability: caligula holds no privileges you do not already have over these
files.
