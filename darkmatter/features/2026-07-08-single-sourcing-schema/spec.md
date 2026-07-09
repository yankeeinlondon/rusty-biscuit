---
status: ready for planning and implementation
reviewed: true
review_iterations: 4
depends_on:
  - ../2026-07-08-schema-plus/spec.md
inputs:
  - ../../docs/schemas/darkmatter.yaml
  - ../../lib/src/markdown/compose/context/catalog.rs
  - ../../lib/src/markdown/schemas/simplified/types.rs
related:
  - ../2026-07-08-modal-and-autocomplete/spec.md
---

# Single-Sourcing the Frontmatter Schema and Context-Variable Catalog

**Status:** Reviewed and ready for planning and implementation. Defines *what*
changes and why, including the two implementation choices that were still open
in the first draft: runtime projection from the authored YAML and
interpolation-scoped array rendering.

## Purpose

Today, Darkmatter describes its owned frontmatter — most visibly the `ctx.*`
context variables — in **two** places that must agree by hand:

1. `darkmatter/docs/schemas/darkmatter.yaml` — the authored base
   **SimplifiedSchema** (loaded via `include_str!`, transcluded verbatim into
   docs, and the validation authority for frontmatter).
2. `darkmatter/lib/src/markdown/compose/context/catalog.rs` —
   `CONTEXT_VARIABLE_DESCRIPTORS`, a hand-maintained Rust `const` carrying each
   variable's `name`, `display_type` (`ContextValueType`), `description`,
   `category`, `subsection`, `order`, and a verified `example`.

These two overlap on **name, type, description, and required/generated flags**
and have already drifted. Concretely, `ctx.now` is typed `string` in the YAML
but `DateTime` in the catalog; `ctx.today` is `date` in both. The YAML is
**wrong** — `now` is a datetime.

This spec makes the **YAML the single source of truth** for that overlapping
data and feeds the Rust catalog from it, so the two can never drift again. It
also removes the impedance that made unification hard: a set of presentation-only
"types" in the catalog that have no SimplifiedSchema equivalent. Those are
replaced by **array types plus a small family of list-formatting expression
functions**.

The work was prompted by a DMLS need (a `{{ ctx.* }}` hover that should show the
schema's type and description). That editor-facing functionality now lives in its
own **dependent** spec,
[modal-and-autocomplete](../2026-07-08-modal-and-autocomplete/spec.md); this spec
is the library foundation it consumes and owns **no** DMLS deliverable. The value
here stands on its own regardless — it removes a standing drift hazard in a core,
widely-consumed catalog.

## Background: why the two diverge

`ContextValueType` (catalog) carries kinds that SimplifiedSchema (YAML) does not:

| `ContextValueType` | SimplifiedSchema equivalent? | Notes |
|--------------------|------------------------------|-------|
| `Date`, `DateTime`, `Time` | ✅ `date`, `datetime`, `time` | Already in the grammar (`types.rs`, `grammar.rs`). |
| `Integer`, `Number`, `Boolean`, `String`, `Object` | ✅ `number(integer)`, `number`, `boolean`, `string`, `object` | Direct. |
| `Nullable(inner)` | ✅ optional/nullable property semantics | Optional keys are already nullable in SimplifiedSchema. |
| `Timezone` | ❌ | Not a real type — see below. |
| `Csv` | ❌ | A *rendering* of a list, not a type. |
| `MarkdownList` | ❌ | A *rendering* of a list, not a type. |
| `NestedMarkdownList` | ❌ | A *rendering* of a (possibly nested) list. |

The last four are the whole reason a separate display-type enum exists. Remove
them and `ContextValueType` collapses onto the SimplifiedSchema type set, at
which point the catalog's type field is a pure projection of the YAML.

## Decisions

### D1 — The YAML is the single source of truth

`darkmatter/docs/schemas/darkmatter.yaml` is authoritative for every
Darkmatter-owned frontmatter property, including the full `ctx.*` catalog:
**name, type, description, and flags** (`generated`, `required`, `default`).
The Rust catalog is *derived* from it; it never re-declares that data.

### D2 — Fix the YAML's temporal types

SimplifiedSchema already supports `date`, `datetime`, and `time`. Correct every
mistyped temporal `ctx.*` entry (starting with `ctx.now` / `ctx.now_utc` →
`datetime`) so the schema reflects reality. (If any needed temporal keyword were
*missing* from SimplifiedSchema, adding it would be in scope — but they are all
present, so this is a data fix only.)

### D3 — Eliminate the presentation-only "types"

`ContextValueType::{Csv, MarkdownList, NestedMarkdownList, Timezone}` are removed
as types:

- **`Timezone` → `string`.** The three timezone variables are already distinct
  strings and stay so: `ctx.timezone` (abbreviation), `ctx.timezone_offset` (UTC
  offset), `ctx.timezone_iana` (IANA name). Meaning lives in the name +
  description, not a type.
- **`Csv`, `MarkdownList`, `NestedMarkdownList` → array types** (`string[]`,
  `date[]`, …). A list is a list; how it is rendered is the caller's choice via
  the formatting functions in D4.

After D3, `ContextValueType` is isomorphic to SimplifiedSchema's type set and
should be **retired** in favor of the SimplifiedSchema type directly (the catalog
descriptor stores a SimplifiedSchema type, not a parallel enum).

### D4 — Add array/list formatting expression functions

New read-side expression functions (usable in `{{ }}` interpolation, same
framework as `length`, `relative`, …), each taking a list and returning a string:

| Function | Output |
|----------|--------|
| `as_line_separated(list)` | newline (`\n`) delimited — **the default rendering** |
| `as_csv(list)` | comma delimited |
| `as_tsv(list)` | tab delimited |
| `as_space_separated(list)` | space delimited |
| `as_unordered_list(list)` | Markdown unordered list |
| `as_ordered_list(list)` | Markdown ordered list |

Rules:

- **Bare array interpolation renders line-separated by default**, so
  `{{ ctx.some_list }}` ≡ `{{ as_line_separated(ctx.some_list) }}`.
  `as_line_separated` is therefore a no-op kept for completeness and explicitness.
- **`as_unordered_list` / `as_ordered_list` auto-nest.** When an element is itself
  a list, it renders as an indented sublist, recursively. This makes nesting a
  property of the *data*, so no `NestedMarkdownList` type and no dedicated
  `as_nested_*` function are needed. **Auto-nesting is the only rendering mode in
  v1** — there is no force-flatten variant (O3): the list functions never flatten
  nested data, and no `as_flat_*` function or `flatten:` option is added until a
  concrete caller can define the expected ordering and lossiness.

### D5 — Feed the catalog from the YAML

`CONTEXT_VARIABLE_DESCRIPTORS` stops hand-declaring name/type/description/flags.
Those come from parsing the base schema at runtime through a private `LazyLock`
projection of `darkmatter_base_schema()`. Keep the public accessor
`context_variable_descriptors() -> &'static [ContextVariableDescriptor]`, backed
by the projected vector, so DMLS, `md schema about`, and existing library callers
do not need an API change.

Presentation-only metadata that is not schema data is handled explicitly:

- `order` derives from YAML declaration order (`IndexMap` preserves it).
- `example` moves to schema-plus `example(<file>)` artifacts.
- `category` and `subsection` remain in a deliberately small Rust grouping map
  keyed by variable name. This is presentation taxonomy, not schema semantics;
  putting it into the validation schema would force fake grouping fields into
  SimplifiedSchema and make ordinary validation data carry documentation layout.
  The grouping map must be total for every projected `ctx.*` key, and a test must
  fail if the YAML adds or removes a key without updating grouping.

### D6 — Collapse redundant list/CSV variable pairs

Many `ctx.*` variables exist today only as pre-rendered twins — a comma-separated
`X` and a Markdown-list `X_list` of the same data (e.g. `packages` /
`packages_list`, `dirty_files` / `dirty_files_list`, `staged_packages` /
`staged_packages_list`, plus the nested Markdown variables `depends_on` and
`used_by`). Under D3/D4 both twins become the *same* array, so each pair collapses
to a single array-typed variable; callers choose rendering with D4 functions.
This is a **breaking change** to the `ctx.*` surface, accepted because the
monorepo has no external users (see Migration). The exact keep/rename list is
enumerated below.

### D7 — Type the expression-function catalog

`ExpressionFunctionDescriptor` currently carries only an untyped `signature` string
(`as_csv(list)`) — no parameter types, no return type. Add typed signatures:
per-parameter **data types** and a **return type** (data types, plus `error` as a
union member for fallible functions — e.g. `as_csv(list: any[]) -> string | error`),
using the type domains defined in
[schema-plus](../2026-07-08-schema-plus/spec.md) (§ Type domains). This makes the
catalog the single source of function types, so `example()` files inherit them
(schema-plus O-A1, Solution 2) and DMLS / `md schema about` get real signatures.
Functions remain **catalog-only** — a frontmatter property can never be typed as a
function (schema-plus, structural exclusion). Prerequisite for the E3 function
examples below.

## Consequences

### DMLS (dependent spec)

- Because both DMLS providers already read `context_variable_descriptors()`, they
  inherit the corrected type/description automatically once the catalog is
  single-sourced — no change is required *here* to benefit them.
- The editor-facing deliverables (enriched `{{ ctx.* }}` hover, completion for
  ctx vars and the new formatting functions) are specified in
  [modal-and-autocomplete](../2026-07-08-modal-and-autocomplete/spec.md), which
  **depends on** this spec. No DMLS deliverable is in scope here.

### `md schema about` / docs

- `context-variables.md` and `md schema about` render from the same derived
  catalog. Ordering follows YAML declaration order; grouping comes from the
  total Rust grouping map described in D5.

### Compose / evaluator

- The interpolation evaluator must treat list-typed `ctx.*` values as **first-class
  arrays** and support the D4 functions over them (today several are pre-rendered
  strings). This is the largest implementation surface, but the evaluator already
  carries `serde_json::Value::Array`, so the risk is contained to capture,
  formatting functions, and interpolation output rendering.

## Evaluator array support

**Verdict: feasible, low-to-medium risk. No value-model surgery required.** The
interpolation/expression value model is already `serde_json::Value`, which has
`Value::Array` as a first-class variant that flows through the whole evaluator.
Existing functions already pattern-match it (`is_array`, `first`, `last`,
`contains`, emptiness). The "evaluator is String-typed" hazard from the stalled
*real-errors* work is a different axis (error typing), not value typing — values
are already JSON-typed.

The work reduces to three well-bounded change sites:

1. **New functions** — `expression/functions.rs` dispatches by `match name` with
   `fn(&[Value]) -> Result<Value, String>`; `first_fn`/`last_fn` already consume
   arrays. The six D4 functions are new arms following that exact pattern;
   `as_unordered_list`/`as_ordered_list` recurse on nested `Value::Array` for
   auto-nesting. Small, mechanical.

2. **Default array rendering at the output boundary.** Today a bare array renders
   as JSON via `scalar_string` (`expression/mod.rs:317`,
   `Value::Array(_) => value.to_string()` → `["a","b"]`). D4 wants line-separated.
   `scalar_string` is *also* used for equality comparison (`mod.rs:443`) and
   frontmatter shell expansion (`frontmatter_shell_expansion.rs:1760`). Two
   options:
   - (a) change the `scalar_string` array arm globally (one spot; comparison
     impact is cosmetic-only, shell impact is an edge case);
   - (b) **recommended** — render arrays line-separated only on the interpolation
     *output* path (`interpolation/evaluator.rs:250`, `eval()` →
     `EvalResult::Value`), leaving `scalar_string` (and thus equality + shell)
     byte-identical.
   No existing test pins whole-array → JSON interpolation output (array tests only
   index elements, e.g. `items[0]`), so the output change is safe.

3. **ctx list capture becomes arrays.** List variables are currently pre-rendered
   to strings in `context/capture.rs` via `context/format.rs` helpers
   (`format_csv`, `format_md_list`) and stored as `Value::String`. Capture instead
   emits `Value::Array`; the CSV/markdown twins collapse (D6) and the
   `format_csv`/`format_md_list` join helpers retire (their job moves to the D4
   functions). This is the largest surface but purely mechanical.

Net: no new primitives, no error-typing dependency. The render-boundary choice is
settled here: arrays render line-separated only on the interpolation output path.

## Exact `ctx.*` array conversion and collapse list

Extracted from `context/catalog.rs` (95 variables). Only the list-typed variables
change shape; every scalar keeps its name (D3 retypes `timezone`/`timezone_iana`
→ `string`, `timestamp*` → `number(integer)`, and `Nullable(inner)` → optional
`inner`, but names are untouched).

### Group 1 — CSV/MarkdownList twins → one `string[]` (drop the `_list` twin)

Ten pairs collapse; the **bare name survives**, the `_list` variant is **removed**.

| Survivor (`string[]`) | Removed twin |
|-----------------------|--------------|
| `packages` | `packages_list` |
| `package_areas` | `package_areas_list` |
| `dirty_files` | `dirty_files_list` |
| `dirty_source_code_files` | `dirty_source_code_files_list` |
| `staged_files` | `staged_files_list` |
| `untracked_files` | `untracked_files_list` |
| `dirty_packages` | `dirty_packages_list` |
| `dirty_package_areas` | `dirty_package_areas_list` |
| `staged_packages` | `staged_packages_list` |
| `staged_package_areas` | `staged_package_areas_list` |

→ **10 variables removed.** Each survivor was the CSV form (`Csv`); it becomes
`string[]` and renders line-separated by default. `{{ as_csv(...) }}` reproduces
the old bare-name behavior; `{{ as_unordered_list(...) }}` reproduces the old
`_list` behavior.

### Group 2 — single list vars (no twin) → `string[]`

Rename type only; names unchanged.

| Variable | Was | Becomes |
|----------|-----|---------|
| `current_packages` | `MarkdownList` | `string[]` |
| `docs_readme` | `Csv` | `string[]` |
| `docs_blast_radius` | `Csv` | `string[]` |
| `docs_drift` | `Csv` | `string[]` |
| `programming_languages_in_repo` | `Nullable`(CSV) | `string[]` (optional) |

### Group 3 — genuinely nested vars → nested array

`depends_on` and `used_by` (`NestedMarkdownList`) are the *only* tree-shaped vars:
`render_dependency_list` emits `- 'pkg' depends on:` with each dependency as a
sub-bullet. Flattening would lose the grouping.

**Decision:** model each as an **object array**:

```yaml
depends_on:
  - package: darkmatter
    dependencies:
      - biscuit-terminal
      - renderable
```

Use `object[]` in the YAML and spell out the object shape in the description
until schema-plus grows precise nested object-array typing. Prefer objects over
tuple arrays because the JSON shape is self-describing for DMLS hover,
`md schema about`, examples, and any future serialized diagnostics; a tuple form
like `[package, [dependencies...]]` is shorter but brittle and hard to read. The
composed verb wording (`depends on:` / the "has no dependencies" line) is dropped
as presentation; the variable name plus object fields convey it. (Alternative,
rejected: keep them as opaque pre-rendered `string` — it perpetuates a
MarkdownList special case and defeats the cleanup.)

### Net effect

- **10 `_list` variables removed** (Group 1).
- **15 variables retyped** from a presentation type to `string[]` (Groups 1 base
  names + Group 2), rendered line-separated by default.
- **2 variables** (`depends_on`, `used_by`) become loosely-typed nested arrays.
- All in-repo docs referencing a removed `*_list` var or expecting CSV/bullet
  output from a bare name are updated in the same change (Migration).

## Runtime projection mechanism

**Verdict: runtime projection of the parsed base schema is feasible and is the
recommendation.** No build-time codegen needed.

- `darkmatter_base_schema() -> SimplifiedSchema` already parses the YAML. Its
  `SchemaShape.properties` is an `IndexMap<String, PropertyDef>`; `ctx` is a
  `PropertyAtom` whose `ty` is `TypeExpr::InlineObject(SchemaShape)`, and each
  child (`now`, `today`, …) is a `PropertyAtom` exposing everything the catalog
  needs: type (`TypeExpr::Primitive(SimplifiedType)` → `as_keyword()`),
  `description: Option<String>`, and constraints (`Required` / `Generated` /
  `Default(_)` via `keyword()`).
- **Ordering is free:** `IndexMap` preserves YAML declaration order, so the
  projected catalog order = YAML document order.
- **`const` → `LazyLock` blast radius is contained.** Keep the public accessor
  `context_variable_descriptors() -> &'static [ContextVariableDescriptor]`, backed
  by a private `LazyLock<Vec<…>>` projected from the base schema on first use.
  External consumers (DMLS, `md schema about`) see no signature change. Internal
  `.iter()` uses deref transparently; only the `suggest(CONTEXT_VARIABLE_DESCRIPTORS,
  …)` call (`interpolation/evaluator.rs:307`) needs a `&`/`&*`.
- Because `ContextValueType` is retired (D3), the projected type is just the
  SimplifiedSchema type — no enum mapping table to maintain.

**Mechanism decision:** runtime `LazyLock` projection. Committed codegen is the
fallback only if a `const`/compile-time guarantee is later required.

## Examples and grouping fidelity

Examples are the one piece of catalog data the YAML genuinely cannot hold at full
fidelity. `Example` is structured (`invocation`, `result`, `verification`) and
some variants are **test-executed** (`Executable`), whereas SimplifiedSchema's
`->` grammar carries only a description string. Three ways to close the gap:

- **E1 — Extend the property grammar to carry examples inline.** Rejected. It
  mixes validation grammar with documentation/test fixtures and has no clean place
  for structured example data.
- **E2 — Thin Rust presentation sidecar.** The catalog merges YAML-derived core
  with a Rust map keyed by name holding `example`. Kept as the **fallback** if E3
  slips, but it re-introduces a Rust-side authored surface.
- **E3 — Referenced example files via `example()` (chosen).** Examples become
  their own schema-validated YAML artifacts, attached to a property with an
  `example(<file>, …)` constraint. This is the cleanest: examples are first-class
  data, validated by SimplifiedSchema itself, with nothing authored in Rust and
  nothing bloating the base schema. It requires the composition primitives in
  **[schema-plus](../2026-07-08-schema-plus/spec.md)** (`example()` constraint,
  `@` cross-file type import, pattern keys), so **single-sourcing depends on
  schema-plus** and starts after it.

**Resolution:** `example` moves to referenced YAML files via schema-plus's
`example()` constraint (E3). `order` derives from YAML document order. `category`
and `subsection` stay in a minimal Rust grouping map keyed by variable name, with
a totality test against the projected YAML keys. This keeps the validation schema
semantic and avoids inventing YAML comments or section markers as parser-visible
metadata.

## Migration

The monorepo has no external users, so breaking the `ctx.*` surface (D6, D3) is
acceptable and preferred over carrying compatibility aliases. In-repo documents
that reference removed twins (`*_list`, comma forms) are updated in the same
change. A short deprecation-alias window is explicitly **not** required unless
implementation surfaces a concrete in-repo consumer that cannot migrate in the
same change.

Migration rules:

- Replace `{{ ctx.foo_list }}` with `{{ as_unordered_list(ctx.foo) }}`.
- Replace uses that depended on old comma-separated bare output with
  `{{ as_csv(ctx.foo) }}`.
- Leave bare `{{ ctx.foo }}` only when line-separated output is intended.
- Update examples, snapshots, docs, and any generated schema-about output in the
  same change that changes the capture values.

## Acceptance criteria

1. `darkmatter.yaml` is the only hand-authored declaration of `ctx.*`
   name/type/description/flags; the Rust catalog derives them.
2. A test fails if the derived catalog and the YAML disagree on any
   name/type/description/flag (drift guard).
3. A test fails if any projected `ctx.*` key lacks grouping metadata, or if the
   grouping map references a key no longer present in the YAML.
4. `ctx.now` / `ctx.now_utc` (and any other mistyped temporal keys) are
   `datetime`; `ctx.today`-family remain `date`.
5. `ContextValueType`'s presentation-only variants are gone; the catalog stores a
   SimplifiedSchema type (or the enum is retired entirely).
6. The catalog accessor keeps the existing public shape
   `context_variable_descriptors() -> &'static [ContextVariableDescriptor]`.
7. The six D4 functions exist, are documented in the expression-function catalog,
   and have verified examples; `{{ ctx.list }}` with no function renders
   line-separated only on the interpolation output path.
8. `scalar_string` behavior used for equality comparison and frontmatter shell
   expansion remains byte-identical unless a separate spec changes those surfaces.
9. `as_unordered_list` / `as_ordered_list` render nested arrays and the
   `depends_on` / `used_by` object-array shape as nested sublists.
10. Removed `*_list` variables are absent from the generated catalog and the YAML;
    in-repo callers are migrated to formatting functions.
11. `md schema about`, `context-variables.md`, `md schema validate`, and compose
    output remain correct for the migrated variables.
12. Builds and passes on macOS, Windows, and Linux.

## Open questions

- **O3 — Force-flatten list function.** ✅ Resolved: **no force-flatten in v1**
  (see D4). `as_unordered_list` / `as_ordered_list` always auto-nest; structure is
  a property of the data. Rejected alternatives: adding
  `as_flat_unordered_list` / `as_flat_ordered_list` now (doubles the list API
  before any caller needs flattening) and a `flatten:` named option (expands the
  function-call convention — no expression function takes named options today).
  Both wait for a concrete caller that can pin the expected ordering and lossiness.

## Out of scope

- Non-`ctx` frontmatter properties keep their current YAML declarations; this
  spec only single-sources what currently duplicates into the catalog.
- Style frontmatter (`style.*`) remains authoritative in `darkmatter::style`
  (Non-Goal carried from the base-schema work).
- General SimplifiedSchema grammar expansion beyond confirming
  `date`/`datetime`/`time` (already present).
