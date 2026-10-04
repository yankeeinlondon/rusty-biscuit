# Spec: Generics, Inheritance, Modifiers, Attributes and Line Counts in Symbol Records

## Problem

After `2026-10-03-symbol-filters-and-reports`, `SymbolRecord` carries scope,
qualified names, the `async` and `static` modifiers, `is_deprecated`,
`is_test`, and the `parameters` and `has_docs` metrics. Several v2 facets that
already exist in the schema stay empty: generic parameters, `extends` and
`implements` relations, the remaining modifiers and the raw attribute list. No
metric reports a callable's body size.

## What was proposed

Proposed in `2026-10-03-symbol-filters-and-reports` (items B3, B4 and B7) and
moved here by the author on 2026-10-04.

- **Generic parameters (`type_info.generic_params`):** name, kind (type,
  lifetime, const), bounds and default.
- **Inheritance (`relations.extends`, `relations.implements`):** for example
  `class A extends B`, `impl Trait for Type`, `implements I`, Swift
  conformance. Preserve written base names and generic arguments even when
  the base is external. Fill a relation target ID only for an unambiguous
  declaration in the same file; otherwise leave it absent. Rust trait
  implementations attach the trait relation and same-file members to their
  target type, and several impl blocks must not overwrite each other. A target
  defined elsewhere keeps a textual container reference. No cross-file type
  resolution.
- **Remaining modifiers:** `unsafe`, `const fn`, `abstract`, `final`,
  `override`, `virtual`, `readonly` and `mut`, using existing `Modifier`
  variants and `Custom` for `abstract`, `final`, `override` and `virtual`
  rather than widening the enum without need.
- **Raw attribute facet:** Rust attributes, Java and C# annotations, TS, JS
  and Python decorators, and Swift attributes, each recorded as
  `{name, value}` in source order using the full syntactic name and raw
  argument text, without evaluating arguments or expanding macros.
  `2026-10-03-symbol-filters-and-reports` already parses these far enough to
  derive `is_deprecated` and `is_test`; this item only populates the facet.
- **`lines` metric:** physical lines intersecting the half-open body span. A
  missing body gives `None`, an empty span gives `Some(0)`, and an end at
  column 1 excludes that line.
- **Outline relations:** show the container's declared inheritance relations
  on its `--tree` container line.

## Why deferred

No filter or report in `2026-10-03-symbol-filters-and-reports` consumes these
facts. The author limited that feature to metadata a flag uses, so every
extracted fact is justified and tested by a consumer. Each item here is an
additive facet or metric under the schema's minor-version rule and can land
later without breaking reports.

## Prerequisites

- `2026-10-03-symbol-filters-and-reports` (the `metrics` facet and its absence
  semantics, the attribute parsing behind `is_deprecated` and `is_test`, and
  `--tree`).

## Notes

- Keep god-files' declaration-span size/risk calculations unchanged. The
  `lines` metric counts body lines, which differ from declaration spans; reuse
  it there only where units and definitions are identical, with regression
  tests proving unchanged scores.
- Library matrices stay exhaustive over all 16 languages: a "generics and
  inheritance" row (generic parameters with bounds; same-file bases get
  correct IDs, external or ambiguous bases keep text without invented IDs), a
  modifier/attribute row, and a `lines` column in the metrics row.
- Each new facet needs a filter or report that uses it, or an explicit
  library-only justification, before it is scheduled.
