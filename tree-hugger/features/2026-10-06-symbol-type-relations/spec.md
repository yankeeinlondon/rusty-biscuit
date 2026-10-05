---
kind: feature
name: symbol-type-relations
date: 2026-10-06
status: draft-spec
area: tree-hugger
packages:
  - tree-hugger
  - tree-hugger-cli
depends-on:
  - 2026-10-03-symbol-filters-and-reports
  - 2026-10-05-deferred-kind-precision
---

# Spec: Generics, Inheritance, Modifiers, Attributes and Line Counts in Symbol Records

## Problem

After `2026-10-03-symbol-filters-and-reports`, `SymbolRecord` carries scope,
qualified names, the `async` and `static` modifiers, `is_deprecated`,
`is_test`, and the `parameters` and `has_docs` metrics. Several v2 facets that
already exist in the schema are empty or nearly so:

- **`type_info.generic_params`** is always empty. The older `TypeMetadata`
  collects Rust type-parameter names as plain strings
  (`extract_rust_type_parameters` in `lib/src/file/extract/type_metadata.rs`),
  but nothing maps them into the v2 facet, and no other language is read.
- **`relations.extends`** is always empty.
- **`relations.implements`** is filled in one case only: each member of a
  Rust `impl Trait for Type` block records the trait, with the trait's ID
  when exactly one trait of that name is declared in the same file
  (`lib/src/file/extract/rust_impl.rs`, applied in
  `lib/src/file/tree_file.rs`). The implementing type itself carries no
  relation, and no other language's `implements` or Swift conformance is
  recorded.
- **The remaining modifiers and the raw attribute list** are empty.

No metric reports a callable's body size.

## What was proposed

Proposed in `2026-10-03-symbol-filters-and-reports` (items B3, B4 and B7) and
moved here by the author on 2026-10-04.

- **Generic parameters (`type_info.generic_params`):** name, kind (type,
  lifetime, const), bounds and default. This replaces the Rust-only string
  list as the source of generic-parameter data in v2 records.
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
  `2026-10-03-symbol-filters-and-reports` already parses Rust outer
  attributes, C++ `[[…]]`, Java and Scala annotations, C# attribute lists,
  Python decorators and Swift attributes far enough to derive `is_deprecated`
  and `is_test` (`lib/src/file/extract/attributes.rs`, which fills no facet).
  TS and JS decorators are not read yet and need a reader; for the others
  this item only populates the facet.
- **`lines` metric:** physical lines intersecting the half-open body span. A
  missing body gives `None`, an empty span gives `Some(0)`, and an end at
  column 1 excludes that line.
- **Outline relations:** show the container's declared inheritance relations
  on its `--tree` container line.

## Scheduling

Scheduled 2026-10-06, after `2026-10-05-deferred-kind-precision`, so that
constructors, enum variants and type aliases carry their precise kinds before
relations and modifiers are attached to them. `2026-10-07-cyclomatic-complexity`
follows and shares the `metrics` row this feature extends with `lines`.

It was first deferred because no filter or report consumed these facts. The
author ruled on 2026-10-05 that a missing consumer is not a reason to defer:
the facts have to be available before callers can start using them. Each item
is an additive facet or metric under the schema's minor-version rule.

## Prerequisites

- `2026-10-03-symbol-filters-and-reports` (the `metrics` facet and its absence
  semantics, the attribute parsing behind `is_deprecated` and `is_test`, and
  `--tree`).
- `2026-10-05-deferred-kind-precision` (precise kinds for the records these
  relations and modifiers attach to).

## Open questions

- **Member-level `implements` on Rust impl members.** Today the trait is
  recorded on each member of the impl block. Once the implementing type
  carries the relation, decide whether members keep their copy (it tells a
  reader which trait a method satisfies) or drop it so the relation has one
  home.

## Notes

- Keep god-files' declaration-span size/risk calculations unchanged. The
  `lines` metric counts body lines, which differ from declaration spans; reuse
  it there only where units and definitions are identical, with regression
  tests proving unchanged scores.
- Library matrices stay exhaustive over all 16 languages: a "generics and
  inheritance" row (generic parameters with bounds; same-file bases get
  correct IDs, external or ambiguous bases keep text without invented IDs), a
  modifier/attribute row, and a `lines` column in the metrics row.
