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

## Why deferred

It adds a sniff dependency on the reporting path for a view nobody currently
needs; the ungrouped statistics answer the immediate question.

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
