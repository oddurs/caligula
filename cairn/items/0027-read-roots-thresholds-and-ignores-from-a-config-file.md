---
id: 27
title: Read roots, thresholds and ignores from a config file
type: feature
status: backlog
milestone: v0.3
created: 2026-09-17
updated: 2026-09-17
priority: p0
effort: l
area: config
---

## Problem

Roots come from `--root` or a guess at `~/Code`. The deny list is a constant in
the source. The staleness bands — 3 days, 14 days, 60 days — are hard-coded, and
they are a judgement about how somebody works, not a fact.

## Proposal

`~/.config/caligula/config.toml`, every key optional, command-line flags winning
over the file. Ship a documented example rather than writing a file on first run.

## Acceptance criteria

- [ ] `roots`, `depth`, `deny`, and the four staleness bands are configurable
- [ ] `--root` and `--depth` override the file
- [ ] A missing config file is normal: defaults, no warning
- [ ] A malformed config file names the file, the line and the key, and exits 2
      rather than falling back silently
- [ ] `--config <path>` for a different file, and `--no-config` to ignore it
- [ ] The example config is in the README and every key is commented
