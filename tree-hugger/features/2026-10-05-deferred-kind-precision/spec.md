---
kind: feature
name: deferred-kind-precision
date: 2026-10-05
status: draft-spec
area: tree-hugger
packages:
  - tree-hugger
  - tree-hugger-cli
depends-on:
  - 2026-10-03-visibility-and-kind
  - 2026-10-03-symbol-filters-and-reports
---

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
| C++ | Constructor; EnumVariant |
| Swift | Constructor; TypeAlias |
| Scala | EnumVariant; TypeAlias; Field |
| PHP | Constructor; EnumVariant; Namespace |

Constructors outside TypeScript and JavaScript are still reported as `Method`
or are absent.

Three cells the fix marked `FEAT` already extract precisely as a side effect
of other work, and `2026-10-03-symbol-filters-and-reports` asserts them as
landed: C++ `typedef`/`using` (`TypeAlias`, from the shared C typedef
capture), C++ `namespace` (`Namespace`) and Lua `M:m` (`Method`). They are no
longer part of this entry.

The fix's `LIM` cells (Go interface and struct reported as `Type`; Swift
class, enum and struct reported as `Type`) are documented extractor limits,
not grammar limits. They are candidates for the same work but were not
assigned to any feature.

## Scheduling

Scheduled 2026-10-05, first of the seven features split out of
`2026-10-03-symbol-improvements` and `2026-10-03-symbol-filters-and-reports`.
It goes first because it changes which records exist and what kind they
carry, and every later feature (type relations, complexity, references,
report formats) reports on those records.

Each cell needs its own extraction and grammar review, and grammar support
cannot be guaranteed in advance. A cell the grammar cannot support is
recorded as a grammar limit in the kind matrix, not dropped silently.

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
- The plan needs an explicit per-language acceptance table and may phase the
  work by language.
