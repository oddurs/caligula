---
id: 56
title: A locked or main worktree is told it is safe to remove
type: bug
status: done
milestone: v0.2
created: 2026-09-18
updated: 2026-09-18
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
