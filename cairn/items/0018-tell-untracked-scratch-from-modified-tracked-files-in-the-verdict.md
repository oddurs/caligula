---
id: 18
title: Tell untracked scratch from modified tracked files in the verdict
type: feature
status: backlog
milestone: v0.2
created: 2026-09-17
updated: 2026-09-17
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

- [ ] The verdict distinguishes modified tracked files from untracked ones
- [ ] A worktree that is untracked-only says so, and in a lower-severity colour
      than one with modified tracked files
- [ ] The `safe` lens is unchanged; a new lens or flag treats untracked-only as
      safe, and is not the default
- [ ] Ignored files are still excluded entirely
- [x] Unit tests over parsed status output covering: tracked-only,
      untracked-only, both, conflicts
- [ ] The removal dialog pluralises properly: `App::ask_remove` says
      "2 uncommitted files", not "2 uncommitted file(s)"

## 2026-09-17

The removal dialog says '2 uncommitted file(s) will be destroyed' — the lazy '(s)' rather than the real pluralisation used everywhere else. Fix it here, where the verdict wording is already being worked on. It is in App::ask_remove.

## 2026-09-17

The dialog's 'file(s)' fell out with this: extracting one cost_line shared by the single and bulk dialogs meant there was one place to pluralise properly. Criterion ticked in 0016's branch; the rest of this item is untouched.
