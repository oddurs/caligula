---
id: 56
title: A locked or main worktree is told it is safe to remove
type: bug
status: done
milestone: v0.2
created: 2026-09-18
updated: 2026-09-19
priority: p1
effort: s
area: verdict
---

## What happens

`Worktree::verdict` ends "Nothing to salvage — safe to remove" whenever nothing
would be lost. But nothing being lost is a statement about content, and "safe to
remove" is a statement about whether git will do it — and for the main checkout
and for a locked worktree it will not.

The repository detail pane shows it plainly:

```
 ● fix/locked        Nothing to salvage — safe to remove     1y
```

while `d` on that row refuses with "locked — in use by a build". This is the
same contradiction item 0052 fixed in the safe lens, left behind in the sentence.

## Acceptance criteria

- [x] A locked worktree's verdict says it is locked, not that it is safe to remove
- [x] The main checkout's verdict does not claim it is removable either
- [x] A worktree that really is safe still says so
- [x] `is_safe_to_remove` is what decides, so the sentence and the lens cannot
      disagree again

## 2026-09-19

Review caught that the fix was half of a signal: the words said locked while the colour still said go, because the verdict was painted by salvage() and a locked worktree has nothing to salvage. It also caught a dead fallback arm and, more usefully, that the new sentence put the shared half first — so 'Nothing to salvage — but this is the…' clipped away exactly the words that distinguished the row. Rather than fix the phrasing a third time, the reason a worktree cannot be removed is now one value, Unremovable, which the lens, the sweep, the removal guard, the colour and the sentence all read. Adding a fourth reason forces every one of them to account for it.
