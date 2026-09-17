# Working in this repository

Instructions for an agent. Follow them exactly; they override any default
workflow you would otherwise apply.

## Attribution

Never attribute work in this repository to an assistant, a model, or a tool.
Not in commit messages, trailers, pull request bodies, issue comments, code
comments, documentation, changelogs, or release notes. No `Co-Authored-By:`
naming a model, no "generated with" footer, no robot emoji.

The work is published under the owner's name. The `commit-msg` hook rejects a
message that breaks this, and `scripts/agent pr` strips it from a pull request
body — but do not rely on either. Write it correctly the first time.

## Setup

```sh
scripts/setup          # once per clone: wires core.hooksPath to .githooks
scripts/agent doctor   # verify before starting anything
```

## The loop

One unit of work is one branch, in one worktree, with one pull request. Two
agents must never share a checkout.

```sh
scripts/agent start feat/0015-mark-worktrees       # prints the worktree path
cd ../.worktrees/caligula/feat/0015-mark-worktrees # move there yourself

# ... work ...

scripts/agent check                     # before every commit
scripts/agent commit "feat(chrome): mark several worktrees at once"
scripts/agent pr                        # checks, pushes, opens the PR
```

After the pull request is merged, from the primary checkout:

```sh
scripts/agent done feat/0015-mark-worktrees
```

Rules that are not negotiable:

- **Never commit to `main`.** It advances only through a merged pull request.
  The `pre-push` hook and the server-side ruleset both refuse.
- **Never use `--no-verify`**, `continue-on-error`, or `|| true` to make a check
  pass. If a check is wrong, fix the check in its own pull request.
- **Never edit `ROADMAP.md`.** It is generated from `cairn/items/`.

## The seam

All automation goes through `scripts/task`. Do not put a `cargo` invocation in
CI, a hook, or a script — put it here, once:

```sh
scripts/task fmt        # format in place
scripts/task fmt:check  # verify formatting
scripts/task lint       # clippy, warnings denied
scripts/task test       # the full suite
scripts/task build      # compile everything
scripts/task check      # all of the above; what CI runs
```

## Commits

Conventional Commits, imperative, subject under 72 characters, no trailing
period:

```
fix(chrome): keep the age column inside the list pane

The row was built one column wider than the pane, so the age was
truncated off the right-hand edge on every terminal width.

Refs: 0021
```

Types: `feat` `fix` `chore` `docs` `perf` `refactor` `test` `build` `ci`
`style` `revert`. The branch and the `Refs:` trailer both carry the cairn id.

## The backlog

The roadmap and the issues are [cairn](https://oddurs.github.io/cairn) items in
`cairn/items/` — Markdown with YAML frontmatter, versioned with the code.

```sh
cairn next                     # what is ready to start
cairn show 15                  # one item in full
cairn new "Title" -t feature --set area=chrome
cairn set 15 status=doing
```

Before work larger than a fix, write the item first, and write the *reasoning*:
the problem, the proposal, the cost you weighed, how you will know it is done.
An item is worth more after it closes than before, because it is then the answer
to *why is it like this*.

## Architecture, and what must stay true

- **Staleness is git activity, never directory mtime.** Editors, builds and
  backup tools touch directories constantly; measured that way every checkout on
  the disk reports as fresh. Take the newest of the last commit, the
  per-worktree reflog and the index, and fall back to the directory only for a
  repository with no commits at all. This is not a detail — it is the difference
  between the tool being right and being confidently wrong.

- **Rendering is a pure function of `App` and the `now` it is given.** No clock,
  no environment, no filesystem while drawing, or the snapshots stop being
  reproducible. `App::now` is sampled at scan and refresh time and passed down.

- **A row is laid out from its fixed columns inwards.** The marker, badges and
  age have known widths; the label takes what is left. Never build a row and
  hope it fits — an overflowing row loses its right-hand end silently, which is
  how the age column disappeared once already. The width tests in `ui.rs` hold
  it.

- **Every destructive action states the cost before it asks.** The confirmation
  dialog names the path, the branch, and exactly what would be lost. A dialog
  that just says "are you sure" is worse than none, because it trains the answer.

- **Browsing never takes a lock.** Read-only git runs through `git()` with
  `GIT_OPTIONAL_LOCKS=0`, so pointing caligula at a repository cannot disturb a
  build running in it. Only a confirmed action uses `git_run()`.

- **caligula never writes to a repository except through git.** No editing files
  in a worktree, no writing refs by hand. If git cannot do it, ask whether it
  should be done at all — the exception is deleting an orphaned directory no
  repository claims any more, which git by definition cannot help with.

## Tests

```sh
scripts/task test                      # everything
INSTA_UPDATE=always scripts/task test  # accept a deliberate screen change
```

The interface is held to recorded screens. If a change moves the layout on
purpose, re-record and **read the diff** — it is the review of your change to
the interface. Never re-record to make a failure go away without reading what
moved.

Name a test after the behaviour it protects, not after the function it calls.

## Comments

Explain **why**, not what. If a line needs a comment to say what it does, the
line is the problem. Match the density and voice of the surrounding code.
