---
kind: feature
name: cyclomatic-complexity
date: 2026-10-07
status: draft-spec
area: tree-hugger
packages:
  - tree-hugger
  - tree-hugger-cli
depends-on:
  - 2026-10-03-symbol-filters-and-reports
  - 2026-10-06-symbol-type-relations
---

# Spec: Per-Symbol Cyclomatic Complexity

## Problem

Symbol reports carry basic metrics (`parameters` and `has_docs` after
`2026-10-03-symbol-filters-and-reports`; `lines` from
`2026-10-06-symbol-type-relations`), but no measure of control-flow
complexity.

## What was proposed

Proposed in the pre-split draft of `2026-10-03-symbol-improvements` (item B7),
and moved here by the author on 2026-10-03.

Add `cyclomatic` to the `metrics` facet: decision points + 1 for a callable
body, excluding nested callable/type bodies. Unavailable rules or a missing
body give `None`; a computed value is never invented.

## Scheduling

Scheduled 2026-10-07, after `2026-10-06-symbol-type-relations`, which adds the
`lines` metric to the same `metrics` facet and matrix row.

It was first deferred because a correct syntax-based definition needs a
reviewed decision-point list per language, a review effort of its own. That
review is the core of this feature; the open question below decides how it is
staged.

## Prerequisites

- `2026-10-03-symbol-filters-and-reports` (the `metrics` facet and its
  absence semantics: unrequested is absent, unsupported is `None`, measured
  zero is `Some(0)`).
- `2026-10-06-symbol-type-relations` (the `lines` metric and its matrix
  column).

## Open question carried over: complexity rules per language

A syntax-based decision-point list must define short-circuit expressions,
pattern guards and nested bodies, not simply count all branch-shaped nodes.

- **Ship reviewed language lists:** useful incrementally and no invented
  values; users see `None` for unreviewed languages.
- **Wait for all supported languages:** uniform coverage; delays useful
  metrics and expands the initial review.
- **Recommendation (pre-split draft):** reviewed lists first, with `None`
  elsewhere and a per-language value/unavailable assertion. Do not change
  god-files scoring to fill a missing metric with zero.

## Notes

- Keep god-files' declaration-span size/risk calculations unchanged; reuse a
  metric there only where units and definitions are identical, with
  regression tests proving unchanged scores.
- The per-language metrics matrix gains a `cyclomatic` column: a reviewed
  value or `None` for each of the 16 languages.
