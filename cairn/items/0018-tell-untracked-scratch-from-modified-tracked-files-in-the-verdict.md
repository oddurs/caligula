---
id: 18
title: Tell untracked scratch from modified tracked files in the verdict
type: feature
status: done
milestone: v0.2
created: 2026-09-17
updated: 2026-09-19
priority: p0
effort: m
area: verdict
---

## Problem

"7 uncommitted files" is one number covering two very different risks. Seven
modified tracked files is work. Seven untracked files is often `.DS_Store`, an
editor swap file and a scratch note — and the verdict crying wolf over those is
exactly how a user learns to ignore it.

## Proposal

Keep the counts separate all the way to the verdict line, and let the wording
say which kind it is. Do not change what counts as dirty for the `safe` lens by
default — an untracked file is still something — but make the distinction
visible, and add a lens that treats untracked-only as safe.

## Acceptance criteria

- [x] The verdict distinguishes modified tracked files from untracked ones
- [ ] A worktree that is untracked-only says so, and in a lower-severity colour
      than one with modified tracked files
- [ ] The `safe` lens is unchanged; a new lens or flag treats untracked-only as
      safe, and is not the default
- [x] Ignored files are still excluded entirely
- [x] Unit tests over parsed status output covering: tracked-only,
      untracked-only, both, conflicts
- [x] The removal dialog pluralises properly: `App::ask_remove` says
      "2 uncommitted files", not "2 uncommitted file(s)"

## 2026-09-17

The removal dialog says '2 uncommitted file(s) will be destroyed' — the lazy '(s)' rather than the real pluralisation used everywhere else. Fix it here, where the verdict wording is already being worked on. It is in App::ask_remove.

## 2026-09-17

The dialog's 'file(s)' fell out with this: extracting one cost_line shared by the single and bulk dialogs meant there was one place to pluralise properly. Criterion ticked in 0016's branch; the rest of this item is untouched.

## 2026-09-19

Two criteria deliberately not done, both on evidence rather than effort.

Criterion 2 asked for untracked-only in a lower-severity colour. Declined: losing an untracked file loses the whole file, while losing a modified tracked file loses only the edits. Neither is recoverable, and this tool's premise is never to say something is safer than it is. The sentence now distinguishes them, which is what lets the reader judge; the colour does not claim one is the lesser.

Criterion 3 asked for a lens or flag treating untracked-only as safe. Dropped after measuring the case that motivated it: across deepwork's 45 worktrees the dirt is 75 tracked changes and 2 untracked files, and the tracked ones are a real one-line edit to docs/ROADMAP.md in every worktree. The flag would have made no difference to the repository it was imagined for, and it is a mode on a destructive tool, which has to earn more than that.

## 2026-09-19

Review found that the longer verdict turned a latent bug into a real one. The removal dialog sizes itself with wrapped_rows, which split on whitespace and rejoined with single spaces — so a line padded into columns measured up to 26 columns shorter than it renders. With the old '3 uncommitted files' these lines rarely crossed the width; with '3 modified files, 2 untracked files' they do routinely, and with eight listed worktrees the body could be clipped by eight rows — worktrees vanishing from the dialog asking you to destroy them. wrapped_rows now measures runs of spaces.

Also: tracked_changes summed staged and unstaged, so a file edited, staged and edited again counted twice, and the verdict said five files above a list of four. It now counts distinct tracked paths, and the verdict test drives the parser rather than setting counters so the sentence cannot disagree with what git said.
