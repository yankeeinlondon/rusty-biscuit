---
status: draft
review_iterations: 0
depends_on:
  - ../2026-07-08-single-sourcing-schema/spec.md
inputs:
  - ../../dmls/src/providers/dsl.rs
  - ../../dmls/src/providers/frontmatter.rs
  - ../../dmls/src/overlay/expressions.rs
related:
  - ../2026-07-08-single-sourcing-schema/spec.md
---

# DMLS Interpolation & Frontmatter Assistance (Modal + Autocomplete)

**Status:** Draft for review. This is the **editor-facing** consumer of the
library work in
[single-sourcing-schema](../2026-07-08-single-sourcing-schema/spec.md). It owns no
library or schema change — it consumes the single-sourced context-variable catalog
and the new array/list formatting functions to improve DMLS hover ("modal") and
completion ("autocomplete") inside `{{ … }}` interpolation and frontmatter.

## Dependency

This spec **must not land before** single-sourcing-schema. It relies on:

- `context_variable_descriptors()` returning YAML-sourced, corrected type +
  description (e.g. `ctx.now` = `datetime`, not `string`).
- The six list-formatting functions (`as_csv`, `as_tsv`, `as_space_separated`,
  `as_line_separated`, `as_unordered_list`, `as_ordered_list`) being registered in
  the expression-function catalog.
- List-typed `ctx.*` variables being real arrays (e.g. `ctx.packages: string[]`,
  the `*_list` twins removed).

If this work starts first, it would re-encode the same type data DMLS is trying to
stop duplicating — so ordering is a hard constraint, not a preference.

## Motivating defect

Today the `{{ ctx.today }}` hover from `dmls::providers::dsl` shows only:

```
Expression
ctx.today
Resolved from ctx.* at compose time (not evaluated here).
```

It omits the type and description that the schema already carries and that the
**frontmatter** provider's `ctx_hover` already renders. The two hover surfaces
should agree.

## Scope (v1)

### 1 — Enriched `{{ ctx.* }}` hover (modal)

In `dmls::providers::dsl::interpolation_hover`, the `ctx.*` branch shows the
variable's **type** and **description** from `context_variable_descriptors()`,
matching the frontmatter provider's `ctx_hover` output. The
"resolved-at-compose-time / not-evaluated-here" note is retained as a trailing
line, but is no longer the *only* content.

Target shape (parity with `ctx_hover`):

```
Expression
ctx.today  (date) — read-only, Darkmatter-owned
Local date in ISO-8601 format.
Resolved from ctx.* at compose time (not evaluated here).
```

### 2 — Completion (autocomplete)

- `{{ ctx.<partial> }}` completion items carry the variable's type in
  `detail` and description in `documentation` (sourced from the same catalog), so
  the completion list is self-describing.
- The six formatting functions are offered as completion inside `{{ }}` with
  signature-style `detail` (e.g. `as_csv(list) -> string`) and a description,
  reusing the existing `expressions::function_signatures()` surface once the new
  functions are registered upstream.

### 3 — Formatting-function signature help (modal, optional)

If it is cheap given the existing function-descriptor surface, hovering a
formatting-function call inside `{{ }}` shows its signature + description. Marked
optional for v1; it must not block items 1–2.

## Out of scope

- Any change to the catalog, schema, expression evaluator, or the formatting
  functions themselves — those belong to single-sourcing-schema.
- Frontmatter-block completion/hover beyond what the existing Layer-2 provider
  already does (this spec is about the `{{ }}` interpolation surface and the shared
  ctx catalog).
- Non-`ctx` interpolation hover changes beyond wiring the shared descriptor data.

## Acceptance criteria

1. `{{ ctx.<name> }}` hover shows type + description sourced from
   `context_variable_descriptors()`, matching the frontmatter `ctx_hover` for the
   same variable.
2. `{{ ctx.<partial> }}` completion items expose type (`detail`) and description
   (`documentation`).
3. The six formatting functions appear in `{{ }}` completion with signature detail
   and description.
4. No DMLS-side re-declaration of ctx types/descriptions — all values flow from the
   library catalog.
5. Builds and passes on macOS, Windows, and Linux; existing DMLS L1/L2 suites stay
   green.

## Open questions

- Whether item 3 (signature help) lands in v1 or defers.
- Whether array-typed `ctx.*` completion should hint the default line-separated
  rendering and suggest a formatting function inline.
