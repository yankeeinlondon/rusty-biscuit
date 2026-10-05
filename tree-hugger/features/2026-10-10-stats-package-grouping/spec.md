---
kind: feature
name: stats-package-grouping
date: 2026-10-10
status: draft-spec
area: tree-hugger
packages:
  - tree-hugger-cli
depends-on:
  - 2026-10-03-symbol-filters-and-reports
---

# Spec: Package Grouping in `hug --stats`

## Problem

`--stats` from `2026-10-03-symbol-filters-and-reports` counts selected
declarations by kind, visibility and language, with doc coverage. In a
monorepo scan the totals cannot be broken down by package.

## What was proposed

Proposed in the pre-split draft of `2026-10-03-symbol-improvements` (item D6),
and moved here by the author on 2026-10-03.

Group `--stats` output by package using manifest-backed identity from sniff
when available, otherwise an explicit unassigned bucket. Never infer package
names from directory names.

## Scheduling

Scheduled 2026-10-10, after `2026-10-09-changed-files-filter`, the other
feature that brings sniff onto `hug`'s path.

It was first deferred because it adds a sniff dependency on the reporting path
for a view nobody had asked for. The author ruled on 2026-10-05 that a missing
consumer is not a reason to defer: the grouping has to exist before anyone can
use it.

## Prerequisites

- `2026-10-03-symbol-filters-and-reports` (`--stats`).
- sniff's monorepo package discovery (the same data `sniff repo packages`
  reports).

## Notes

- Synthetic export records stay reported separately from declaration doc
  coverage, as in ungrouped stats.
- Zero matches in a group gives a `null` coverage ratio rather than 0%, as
  ungrouped stats do. (`doc_coverage.unavailable` is a different thing: the
  count of declarations in languages without doc-comment detection.)
