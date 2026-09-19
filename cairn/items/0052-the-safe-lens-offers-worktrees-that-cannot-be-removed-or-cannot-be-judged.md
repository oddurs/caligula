---
id: 52
title: The safe lens offers worktrees that cannot be removed or cannot be judged
type: bug
status: done
milestone: v0.2
created: 2026-09-17
updated: 2026-09-18
priority: p0
effort: s
area: verdict
---

## What happens

`Lens::Safe` is `wt.salvage() == Salvage::Nothing && !wt.is_main`, and two kinds
of worktree slip through it:

- **Locked.** `fix/locked` is listed as safe to remove, while `App::ask_remove`
  refuses it outright — "Locked: … — press L to unlock first". The header count
  agrees with the lens rather than with the action.
- **Broken.** A worktree git cannot read has never been through `parse_status`,
  so every counter is still zero and `salvage()` returns `Nothing` by default.
  The verdict is not derived, it is the absence of data wearing the same words.

The second is the more serious. The directory being gone happens to make it true
here, but a worktree broken for any other reason — a corrupted gitdir, a
permission problem, files still on disk — would be reported as safe to remove
while holding work.

## What should happen

The lens offers only what is both removable and knowable. A worktree that cannot
be read says so instead of claiming there is nothing in it.

## Why this blocks the sweep

0017 marks everything this lens shows and 0016 removes the marking in one
confirmation. Both inherit whatever the lens gets wrong, which is why this is a
release blocker rather than a tidy-up.

## Reproduction

`tests/snapshots/snapshots__the_safe_lens_hides_everything_with_something_in_it.snap`
records both, from the fixture built in item 0020.

## Acceptance criteria

- [x] A locked worktree never appears under the safe lens
- [x] A broken worktree never appears under the safe lens
- [x] `Worktree::verdict` for a broken worktree says git could not read it,
      rather than that there is nothing to salvage
- [x] `Salvage` distinguishes "nothing to salvage" from "unknown", so the
      difference cannot be lost again by a default
- [x] The snapshot diff shows both leaving the safe lens
