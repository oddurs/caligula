# Contributing

## Setup

```sh
git clone git@github.com:oddurs/caligula.git
cd caligula
scripts/setup          # wires core.hooksPath to .githooks; run once
scripts/agent doctor   # verifies git, gh, the toolchain and the hooks
```

## The workflow

One unit of work is one branch, in one worktree, with one pull request. `main`
advances only through a merged pull request — the `pre-push` hook and a
server-side ruleset both refuse a direct push.

```sh
scripts/agent start fix/0021-narrow-terminal   # prints a worktree path
cd ../.worktrees/caligula/fix/0021-narrow-terminal

# ... work ...

scripts/agent check                            # fmt, lint, test, build
scripts/agent commit "fix(chrome): fold to one pane below 100 columns"
scripts/agent pr
```

Once it is merged, from the primary checkout:

```sh
scripts/agent done fix/0021-narrow-terminal
```

This is a solo repository, so pull requests need no approving review — but they
do need a green `required` check, and they are still the only way into `main`.

## Commits

[Conventional Commits](https://www.conventionalcommits.org): imperative,
subject under 72 characters, no trailing period. The body explains *why*; the
diff already says what.

```
feat(actions): remove every marked worktree behind one confirmation

Refs: 0016
```

The `commit-msg` hook enforces the format.

## Checks

Everything goes through one seam, so CI and your machine cannot disagree:

```sh
scripts/task check     # fmt:check, lint, test, build — exactly what CI runs
```

Never `--no-verify`. If a check is wrong, fix the check in its own pull request.

## The backlog

Issues and the roadmap are [cairn](https://oddurs.github.io/cairn) items under
`cairn/items/`, versioned with the code. `ROADMAP.md` is generated from them —
never edit it by hand.

```sh
cairn next             # what is ready to start
cairn show 15          # one item in full
```

For anything larger than a fix, write the item before the code: the problem, the
proposal, and how you will know it is done.

## Tests

A bug fix arrives with the test that would have caught it. The interface is held
to recorded screens; if your change moves the layout on purpose, re-record and
read the diff — that diff is the review of your change to the interface.
