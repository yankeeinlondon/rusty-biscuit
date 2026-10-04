# Spec: Kind Precision Deferred by the Visibility-and-Kind Fix

## Problem

`2026-10-03-visibility-and-kind` corrected kind, visibility, `static` and
container data for an accepted extraction set across all 16 languages. Its
kind matrix marked some constructs `FEAT`: a precision upgrade that was not
asserted by the fix. Those declarations are absent from symbol reports or
reported with a less precise kind.

## What was proposed

The fix originally assigned its `FEAT` cells to
`2026-10-03-symbol-filters-and-reports`. On 2026-10-04 the author kept that
feature to the fix's accepted extraction set, with no new kinds, and moved the
cells here.

| Language | Deferred constructs (`FEAT` cells) |
|---|---|
| TypeScript | EnumVariant; Namespace |
| Go | TypeAlias (`type A = B` is not reported at all today); Field |
| Python | Constructor |
| Java | Constructor; EnumVariant |
| C# | Constructor; EnumVariant |
| C | EnumVariant |
| C++ | Constructor; EnumVariant; TypeAlias; Namespace |
| Swift | Constructor; TypeAlias |
| Scala | EnumVariant; TypeAlias; Field |
| PHP | Constructor; EnumVariant; Namespace |
| Lua | Method |

Constructors outside TypeScript and JavaScript are still reported as `Method`
or are absent. C++ `typedef`/`using` already report `TypeAlias` as a side
effect of the fix's shared C typedef capture, but the cell was not asserted.

The fix's `LIM` cells (Go interface and struct reported as `Type`; Swift
class, enum and struct reported as `Type`) are documented extractor limits,
not grammar limits. They are candidates for the same work but were not
assigned to any feature.

## Why deferred

Each cell needs its own extraction and grammar review, and grammar support
cannot be guaranteed in advance. Bundling them would have expanded
`2026-10-03-symbol-filters-and-reports` well beyond its filtering and
reporting purpose.

## Prerequisites

- `2026-10-03-visibility-and-kind` (the kind matrix and its cell states).
- `2026-10-03-symbol-filters-and-reports` (its tests distinguish
  "unsupported extraction" from "language has no construct"; landing a cell
  here flips the affected expectations from unsupported to supported).

## Notes

- Changing extraction bumps the extractor revision that keys the CLI cache.
- Each landed cell updates the kind matrix test (`KIND_MATRIX`), the
  per-language fixtures and `tree-hugger/docs/symbol-kinds-and-visibility.md`
  together.
- A named subset can be scheduled; it needs an explicit per-language
  acceptance table and leaves the remainder here.
