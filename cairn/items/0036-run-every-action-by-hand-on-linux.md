---
id: 36
title: Run every action by hand on Linux
type: chore
status: backlog
milestone: v1.0
created: 2026-09-17
updated: 2026-09-17
priority: p0
effort: m
area: runtime
---

## Problem

The Linux paths — `xdg-open`, `wl-copy`, `xclip`, `xsel` — were written from
memory and have never been executed. Shipping them untested is worse than not
shipping them: it promises something that may not work.

## Acceptance criteria

- [ ] On a real Linux desktop: scan, remove, remove-with-branch, prune, lock,
      shell-out, copy path, open in file manager — each run by hand
- [ ] Clipboard verified under both Wayland and X11, and the failure message is
      useful when neither tool is installed
- [ ] Terminal restoration verified after a shell-out and after a panic
- [ ] Findings recorded on this item, and anything broken raised as its own bug
- [ ] If a path cannot be made to work, it is removed and documented, not left
      in as decoration
