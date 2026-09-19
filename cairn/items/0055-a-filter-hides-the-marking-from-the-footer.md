---
id: 55
title: A filter hides the marking from the footer
type: bug
status: done
milestone: v0.2
assignee: Oddur Sigurdsson
created: 2026-09-18
updated: 2026-09-18
priority: p1
effort: s
area: chrome
---

## What happens

`footer` checks the filter before anything else, so whenever a filter is set the
marked-stakes line never appears. Filter to a repository, press `a` to sweep it,
and the footer still reads `filter deepwork · esc clear` — no count, no totals,
and no sign of whether the sweep marked anything at all.

Filtering and then sweeping is the normal way to clear one repository, so this
is not a corner: it is the main path.

## What should happen

The marking is what decides what `d` applies to, so it outranks a filter
indicator and a transient message alike — the same rule already applied to the
status line. The filter is still shown, alongside rather than instead.

While a filter is actively being typed the filter line wins, because you have to
see what you are typing.

## Acceptance criteria

- [x] With a marking held, the footer shows its count and totals even when a
      filter is set
- [x] The filter is still visible in that state
- [x] While the filter is being typed, the filter line wins
- [x] A snapshot records a marking held with a filter set

## 2026-09-18

Review caught the fix half-done. The filter badge was appended after the key hints, and the footer does not wrap, so it was the first thing clipped — below about 115 columns it never appeared and the criterion it was meant to satisfy still failed. It now sits beside the count. The regression test was worse: under filter 'feat' nothing in the fixture is safe, so the sweep marked nothing and the single mark came from a manual toggle — the filter-then-a path that produced the report would have regressed undetected. It now filters to something that has a safe worktree and asserts the sweep itself made the marking.
