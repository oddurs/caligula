# caligula

[![CI](https://github.com/oddurs/caligula/actions/workflows/ci.yml/badge.svg)](https://github.com/oddurs/caligula/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A terminal browser for git worktrees. It finds every checkout on the machine,
groups them by repository, and answers the question that actually matters before
you delete one: **is there anything in here worth saving?**

```
 caligula  92 repos · 189 worktrees (97 linked) · 82 dirty · 100 stale · 48 safe to remove
╭ sort activity · lens all ────────────────────────────────╮╭──────────────────────────────────────────────╮
│ ▾ quarry                                          21wt   ││ refactor/site-tailwind  recent · 4d           │
│ │ ◆ main                                             1h  ││ ~/Code/.worktrees/quarry/refactor/site       │
│ │ ● feat/responsive-layout                          47m  ││                                              │
│ │ ● fix/0049-classify-containers                     1h  ││ ┃ 3 uncommitted files, 2 unpushed commits     │
│ │ ● refactor/site-tailwind           ↑2 ~3           4d  ││                                              │
│ ▾ deepwork                                     45wt 46●  ││ branch    refactor/site-tailwind  → origin…   │
│ │ ● worktree-agent-a849ea46      ↓124 ~24         170d  ││ head      281245d0  refactor(site): replace…  │
╰──────────────────────────────────────────────────────────╯╰──────────────────────────────────────────────╯
 j/k move  ←/→ fold  d remove  D +branch  p prune  c shell  f lens  s sort  / find  ? help
```

## Why

Worktrees accumulate. A branch gets checked out somewhere under `.worktrees/`,
the work lands, and the directory stays — along with a stash, three untracked
files, and a commit that was never pushed anywhere. `git worktree list` tells you
the paths. It does not tell you which ones you can delete without losing work.

## Install

```sh
cargo install --path .
```

## Use

```sh
caligula                      # scan ~/Code, ~/src, ~/Projects… or $HOME
caligula --root ~/work        # scan somewhere specific (repeatable)
caligula --depth 4            # how deep to walk below each root (default 8)
```

The scan runs in the background: repositories appear as they are probed, and a
few hundred of them settle in a couple of seconds. Directories starting with `.`
are skipped — except `.worktrees` — as are `node_modules`, `target` and friends.

## Reading a row

```
 │ ● feat/0049-classify        ↑2 ↓1 ~7    3d
   │ │                         │  │  │      └ time since the last git activity
   │ │                         │  │  └ files changed, staged or untracked
   │ │                         │  └ commits behind the upstream
   │ │                         └ commits that exist only here
   │ └ branch, or the commit when detached
   └ ● linked worktree   ◆ main checkout   ✗ git cannot read it
```

Colour is staleness: green under three days, cyan under two weeks, yellow under
two months, red beyond that. **Staleness is measured from git activity** — the
reflog, the index, the last commit — never from the directory's mtime, which
editors and backup tools touch constantly and which would report every checkout
on the disk as fresh.

Badges also carry `L` for a locked worktree and `!` for one git has marked
prunable.

## What you would lose

The detail pane leads with a verdict:

| Verdict | Meaning |
| --- | --- |
| `Nothing to salvage — safe to remove` | Clean, and every commit lives somewhere else. |
| `n unpushed commits` | The commits exist only here. Removing the worktree keeps the branch; `D` deletes both. |
| `n uncommitted files` | Real work on disk. Removal needs `--force`, and the dialog says so in red. |

Below it: tracking state, the head commit, the repo's shared stash count, every
changed file, and the commits that are not on the base branch. A clean worktree
shows its last five commits instead, so you can still tell what it was for.

## Keys

| | |
| --- | --- |
| `j` `k` `↓` `↑` | move |
| `J` `K` | jump to the next / previous repo |
| `←` `→` `enter` | fold or unfold a repo |
| `z` `Z` | fold all / unfold all |
| `d` | remove the worktree — asks first, always |
| `D` | remove the worktree and delete its branch |
| `p` | prune the repo's stale worktree records |
| `L` | lock or unlock the worktree |
| `c` | drop into a shell inside it; exit to come back |
| `o` `y` | open in the file manager / copy the path |
| `f` | lens: all → dirty → salvageable → stale → safe to remove |
| `s` | sort: activity → name → risk |
| `/` | filter by repo, branch or path |
| `r` `R` | re-probe the selected repo / rescan the disk |
| `?` `q` | help / quit |

Nothing destructive happens without a confirmation dialog that first spells out
what is at stake.

## Development

Everything automation does goes through one seam, so CI and your machine cannot
disagree:

```sh
scripts/setup          # once per clone: wires the git hooks
scripts/task check     # fmt:check, lint, test, build — exactly what CI runs
```

Work happens one branch at a time, each in its own worktree, each arriving
through a pull request:

```sh
scripts/agent start feat/short-slug    # prints the worktree path
scripts/agent check
scripts/agent commit "feat(chrome): one imperative line"
scripts/agent pr
```

The roadmap and the issue tracker are [cairn](https://oddurs.github.io/cairn)
items under `cairn/items/`, versioned with the code and rendered into
[ROADMAP.md](ROADMAP.md). `cairn next` says what is ready to start.
[CONTRIBUTING.md](CONTRIBUTING.md) has the rest.

One dependency: `ratatui`. Everything else is the standard library driving the
`git` binary, which is read with `GIT_OPTIONAL_LOCKS=0` so that browsing never
takes a lock out from under a build.
