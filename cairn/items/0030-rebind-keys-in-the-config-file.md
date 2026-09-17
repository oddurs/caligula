---
id: 30
title: Rebind keys in the config file
type: feature
status: backlog
milestone: v0.3
depends_on:
- 27
created: 2026-09-17
updated: 2026-09-17
priority: p2
effort: m
area: config
---

## Problem

`d` is remove, and on a machine where the same finger means something else in
every other tool, that is a hazard rather than a preference.

## Acceptance criteria

- [ ] A `[keys]` table maps actions to keys, merged over the defaults
- [ ] An unknown action name, or two actions bound to one key, is an error that
      names both
- [ ] The help overlay and the footer show the bindings in force, not the
      defaults
- [ ] Destructive actions can be bound, but never to a bare `enter` or `space`
