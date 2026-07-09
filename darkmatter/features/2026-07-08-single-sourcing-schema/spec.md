---
status: draft
review_iterations: 0
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

**Status:** Draft for review. Defines *what* changes and why. The *how*
(codegen vs. runtime projection, evaluator array support) is deferred to a
design/plan phase.

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
  `as_nested_*` function are needed. (Open item O3 revisits this if a
  force-flatten mode is ever wanted.)

### D5 — Feed the catalog from the YAML

`CONTEXT_VARIABLE_DESCRIPTORS` stops hand-declaring name/type/description/flags.
Those come from parsing the base schema. Presentation-only metadata that is not
schema data (`category`, `subsection`, `order`, `example`) is handled per O2.
Mechanism (build-time codegen vs. runtime projection of the already-parsed base
schema) is a design decision — see O1.

### D6 — Collapse redundant list/CSV variable pairs

Many `ctx.*` variables exist today only as pre-rendered twins — a comma-separated
`X` and a Markdown-list `X_list` of the same data (e.g. `packages` /
`packages_list`, `dirty_files` / `dirty_files_list`, `staged_packages` /
`staged_packages_list`, plus the `NestedMarkdownList` trio `depends_on`,
`used_by`, `current_packages`). Under D3/D4 both twins become the *same* array,
so each pair collapses to a single array-typed variable; callers choose rendering
with D4 functions. This is a **breaking change** to the `ctx.*` surface, accepted
because the monorepo has no external users (see Migration). The exact keep/rename
list is enumerated during design.

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
  catalog. Ordering/grouping depends on O2's resolution.

### Compose / evaluator

- The interpolation evaluator must treat list-typed `ctx.*` values as **first-class
  arrays** and support the D4 functions over them (today several are pre-rendered
  strings). This is the largest implementation surface and a risk (see O4).

## Spike findings — evaluator array support (O4)

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

Net: no new primitives, no error-typing dependency; the only design choice is the
render-boundary scoping (recommend interpolation-only). O4 is considered
resolved — folded into scope above.

## O5 — exact `ctx.*` array conversion and collapse list

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

**Recommendation:** model each as a **nested array** — a list of
`[package, [dependencies…]]` (or `{ package, dependencies }`) elements — rendered
by `as_unordered_list`'s auto-nesting. The composed verb wording (`depends on:` /
the "has no dependencies" line) is dropped as presentation; the variable name plus
nesting convey it. SimplifiedSchema cannot express a precise tree type, so type
these two loosely in the YAML (`any`, or `object[]`) with the shape spelled out in
the description. (Alternative, rejected: keep them as opaque pre-rendered `string`
— it perpetuates a MarkdownList special case and defeats the cleanup.)

### Net effect

- **10 `_list` variables removed** (Group 1).
- **15 variables retyped** from a presentation type to `string[]` (Groups 1 base
  names + Group 2), rendered line-separated by default.
- **2 variables** (`depends_on`, `used_by`) become loosely-typed nested arrays.
- All in-repo docs referencing a removed `*_list` var or expecting CSV/bullet
  output from a bare name are updated in the same change (Migration).

## Spike findings — derivation mechanism (O1)

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
  projected catalog order = YAML document order (resolves the `order` half of O2).
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

## Examples fidelity (resolves O2)

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

**O2 resolution:** `example` moves to referenced YAML files via schema-plus's
`example()` constraint (E3). `order` derives from YAML document order (O1). The
only remaining sidecar candidates are `category`/`subsection` for docs grouping —
open sub-point: keep a minimal Rust grouping map, or derive grouping from the
YAML's section structure. Deferred to design.

## Migration

The monorepo has no external users, so breaking the `ctx.*` surface (D6, D3) is
acceptable and preferred over carrying compatibility aliases. In-repo documents
that reference removed twins (`*_list`, comma forms) are updated in the same
change. A short deprecation-alias window is explicitly **not** required unless
design surfaces a concrete need.

## Acceptance criteria

1. `darkmatter.yaml` is the only hand-authored declaration of `ctx.*`
   name/type/description/flags; the Rust catalog derives them.
2. A test fails if the derived catalog and the YAML disagree on any
   name/type/description/flag (drift guard).
3. `ctx.now` / `ctx.now_utc` (and any other mistyped temporal keys) are
   `datetime`; `ctx.today`-family remain `date`.
4. `ContextValueType`'s presentation-only variants are gone; the catalog stores a
   SimplifiedSchema type (or the enum is retired entirely).
5. The six D4 functions exist, are documented in the expression-function catalog,
   and have verified examples; `{{ ctx.list }}` with no function renders
   line-separated.
6. `as_unordered_list` / `as_ordered_list` render nested arrays as nested
   sublists.
7. `md schema about`, `context-variables.md`, `md schema validate`, and compose
   output remain correct for the migrated variables.
8. Builds and passes on macOS, Windows, and Linux.

## Open questions

- **O1 — Derivation mechanism.** ✅ Resolved by spike (see *Spike findings —
  derivation mechanism*): runtime `LazyLock` projection of the parsed base schema;
  ordering free from `IndexMap`; accessor keeps `&'static [...]` so external
  consumers are unaffected.
- **O2 — Home for presentation-only metadata.** ✅ Resolved (see *Examples
  fidelity*): `example` moves to referenced YAML files via
  [schema-plus](../2026-07-08-schema-plus/spec.md)'s `example()` constraint (E3;
  E2 sidecar is the fallback). `order` derives from YAML document order. Residual
  sub-point: whether `category`/`subsection` survive as a minimal Rust grouping
  map or derive from YAML section structure.
- **O3 — Force-flatten list function.** Do we ever want `as_unordered_list` to
  *not* nest? Default is auto-nest; add `as_flat_*` only on demand.
- **O4 — Evaluator array support.** ✅ Resolved by spike (see *Spike findings*
  above): feasible, no value-model surgery; scoped to three change sites. Only
  residual choice is render-boundary scoping (recommend interpolation-only).
- **O5 — Exact D6 keep/rename list.** ✅ Resolved — see *O5 — exact `ctx.*` array
  conversion and collapse list* above (10 `_list` twins removed, 15 retyped to
  `string[]`, `depends_on`/`used_by` → nested arrays). Residual: confirm the
  nested-array element shape for Group 3 (`[name, [deps]]` vs `{name, deps}`).

## Out of scope

- Non-`ctx` frontmatter properties keep their current YAML declarations; this
  spec only single-sources what currently duplicates into the catalog.
- Style frontmatter (`style.*`) remains authoritative in `darkmatter::style`
  (Non-Goal carried from the base-schema work).
- General SimplifiedSchema grammar expansion beyond confirming
  `date`/`datetime`/`time` (already present).
