# caligula

[![CI](https://github.com/oddurs/caligula/actions/workflows/ci.yml/badge.svg)](https://github.com/oddurs/caligula/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A terminal browser for git worktrees. It finds every checkout on the machine,
groups them by repository, and answers the question that actually matters before
you delete one: **is there anything in here worth saving?**

```
 caligula  94 repos · 171 worktrees (77 linked) · 83 dirty · 101 stale · 28 safe to remove
╭ sort risk · lens all ────────────────────────────────────╮╭──────────────────────────────────────────────╮
│   BRANCH                          ↑    ↓    ±       AGE  ││ worktree-agent-a849ea46  ancient · 173d      │
│ ▾ deepwork (45)                      171  105      155d  ││ ~/Code/deepwork/.claude/worktrees/agent-a84  │
│ │ ◆ main                                   28      155d  ││                                              │
│ │ ● worktree-agent-a849ea46          124   24      173d  ││ ┃ 24 uncommitted files                        │
│ │ ● worktree-agent-ad672f29          124    6      173d  ││                                              │
│ │ ● worktree-agent-ab0d5799           99    3      171d  ││ branch    worktree-agent-a849ea46             │
│ ▾ quarry (21)                     4         3        1h  ││ head      281245d0  wip                      │
│ │ ◆ main                                              1h  ││ repo      deepwork  ~/Code/deepwork          │
╰──────────────────────────────────────────────────────────╯╰──────────────────────────────────────────────╯
 j/k move  space mark  ←/→ fold  d remove  D +branch  p prune  c shell  f lens  s sort  / find  ? help
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

The marker says what kind of checkout it is:

```
 ● a linked worktree     ◆ the main checkout     ✗ git cannot read it
```

Its colour is staleness, measured from git activity — the reflog, the index, the
last commit — never from the directory's mtime, which editors and backup tools
touch constantly and which would report every checkout on the disk as fresh:
green under three days, cyan under two weeks, yellow under two months, red
beyond that.

Everything else has its own column, so you can run your eye down one:

| Column | Means |
| --- | --- |
| `↑` | commits that exist only here — the branch survives a removal, the checkout does not |
| `↓` | commits behind the upstream. Context, not risk: being behind costs nothing |
| `±` | files changed, staged or untracked. **This is the column that says work would be destroyed** |
| | `L` locked, `!` git has marked it prunable or cannot read it |
| `AGE` | time since the last git activity |

Blank means zero, so there is no glyph to decode. On a repository row the
columns carry the whole group: `±` and `↑` are its totals, `↓` is its worst,
and `(45)` is how many linked worktrees it has.

Columns are given up as the pane narrows, least useful first — the behind-count
goes before the commits, and both go before the flags, which say a worktree
cannot be removed at all.


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
| `space` | mark a worktree, and step down |
| `J` `K` | jump to the next / previous repo |
| `←` `→` `enter` | fold or unfold a repo |
| `z` `Z` | fold all / unfold all |
| `d` | remove everything marked, or the row under the cursor — asks first, always |
| `D` | the same, and delete the branches too |
| `p` | prune the repo's stale worktree records |
| `L` | lock or unlock the worktree |
| `c` | drop into a shell inside it; exit to come back |
| `o` `y` | open in the file manager / copy the path |
| `f` | lens: all → dirty → salvageable → stale → safe to remove |
| `s` | sort: activity → name → risk |
| `/` | filter by repo, branch or path |
| `r` `R` | re-probe the selected repo / rescan the disk |
| `esc` | clear the marking, then the filter |
| `?` `q` | help / quit |

Nothing destructive happens without a confirmation dialog that first spells out
what is at stake. Marking is how a repository gets cleared in one pass: `space`
down the list, then `d` once. The dialog lists every worktree it is about to
remove, worst first, totals the files and commits that exist nowhere else, and
names the ones it is keeping and why.

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
