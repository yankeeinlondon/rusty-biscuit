---
title: Phase 1 Baseline Inventory — Single-Sourcing the Frontmatter Schema
feature: 2026-07-08-single-sourcing-schema
phase: 1
created: 2026-07-09
status: complete
---

# Phase 1 — Preconditions and Baseline Inventory

This document is the Phase 1 deliverable: a frozen snapshot of the exact
catalog/schema drift and the exact caller/test/doc migration surface **before**
any implementation begins. Phase 1 changes no source code; it records what
Phases 2–7 must act on.

All line references are against the working tree at `created:` above.

## 1. Precondition — schema-plus readiness (confirmed)

`darkmatter/features/2026-07-08-schema-plus` is **fully implemented** and
supports every primitive this feature depends on:

- Plan is at `phase: 7` of `total_phases: 7` with **73/73** checkboxes checked
  and **0** unchecked.
- `example()` / `x-darkmatter-example` infrastructure is present in
  `schemas/resolve.rs`, `schemas/example.rs`, `schemas/simplified/grammar.rs`,
  `schemas/simplified/serialize.rs`, `schemas/simplified/types.rs`,
  `schemas/simplified/convert.rs` — so referenced `example(<file>)` artifacts
  (E3) are available.
- Typed function signatures / type domains (§ Type domains in schema-plus) are
  landed, satisfying D7 (typed `ExpressionFunctionDescriptor`).
- Pattern keys and cross-file type imports (`Name@file` / `@this`) are landed.

**Verdict:** the schema-plus dependency is satisfied; this feature can start.

## 2. Current `ctx.*` declarations — merged inventory (95 variables)

Two sources describe the same variables and must be reconciled:

- **YAML** — `darkmatter/docs/schemas/darkmatter.yaml`, the `ctx:` block
  (lines 29–125). Carries the SimplifiedSchema *type* + *flags*
  (`generated` = `G`, `required` = `R`; absence of `R` on a `generated` key
  means the property is optional/nullable).
- **Catalog** — `darkmatter/lib/src/markdown/compose/context/catalog.rs`,
  `CONTEXT_VARIABLE_DESCRIPTORS` (lines 80–1245). Carries `display_type`
  (`ContextValueType`), `description`, `category`, `subsection`, `order`,
  and a `TypeShapeOnly` `example`.

Catalog display order **already equals** YAML declaration order (the aliases
`utc`/`dow`/`dow_abbr` sit in the same position in both), so projecting order
from YAML via `IndexMap` (D5) preserves today's report order.

Legend for the **Action** column:
- `ok` — no shape change (type already agrees or only presentation enum collapses)
- `→datetime` / `→string[]` / `→object[]` — retype in YAML (Phase 2)
- `REMOVE` — `_list` twin dropped (Group 1)

| # | name | catalog `display_type` | YAML type | flags | category | subsection | order | Action |
|---|------|------------------------|-----------|-------|----------|------------|-------|--------|
| 1 | now | DateTime | **string** | G,R | Date and Time | | 1 | **→datetime** |
| 2 | now_utc | DateTime | **string** | G,R | Date and Time | | 2 | **→datetime** |
| 3 | today | Date | date | G,R | Date and Time | | 3 | ok |
| 4 | today_utc | Date | date | G,R | Date and Time | | 4 | ok |
| 5 | yesterday | Date | date | G,R | Date and Time | | 5 | ok |
| 6 | yesterday_utc | Date | date | G,R | Date and Time | | 6 | ok |
| 7 | tomorrow | Date | date | G,R | Date and Time | | 7 | ok |
| 8 | tomorrow_utc | Date | date | G,R | Date and Time | | 8 | ok |
| 9 | day | String | string | G,R | Date and Time | | 9 | ok |
| 10 | day_utc | String | string | G,R | Date and Time | | 10 | ok |
| 11 | day_abbr | String | string | G,R | Date and Time | | 11 | ok |
| 12 | day_abbr_utc | String | string | G,R | Date and Time | | 12 | ok |
| 13 | year | String | string | G,R | Date and Time | | 13 | ok |
| 14 | year_utc | String | string | G,R | Date and Time | | 14 | ok |
| 15 | month | String | string | G,R | Date and Time | | 15 | ok |
| 16 | month_name | String | string | G,R | Date and Time | | 16 | ok |
| 17 | month_name_abbr | String | string | G,R | Date and Time | | 17 | ok |
| 18 | day_of_month | String | string | G,R | Date and Time | | 18 | ok |
| 19 | day_of_month_suffixed | String | string | G,R | Date and Time | | 19 | ok |
| 20 | time | Time | string | G,R | Date and Time | | 20 | Time→string (see §3) |
| 21 | time_military | Time | string | G,R | Date and Time | | 21 | Time→string (see §3) |
| 22 | time_utc | Time | string | G,R | Date and Time | | 22 | Time→string (see §3) |
| 23 | time_military_utc | Time | string | G,R | Date and Time | | 23 | Time→string (see §3) |
| 24 | timezone | Timezone | string | G | Date and Time | | 24 | Timezone→string |
| 25 | timezone_offset | String | string | G,R | Date and Time | | 25 | ok |
| 26 | timezone_iana | Timezone | string | G | Date and Time | | 26 | Timezone→string |
| 27 | start_of_week_sun | Date | date | G,R | Date and Time | | 27 | ok |
| 28 | end_of_week_sun | Date | date | G,R | Date and Time | | 28 | ok |
| 29 | start_of_week_mon | Date | date | G,R | Date and Time | | 29 | ok |
| 30 | end_of_week_mon | Date | date | G,R | Date and Time | | 30 | ok |
| 31 | start_of_week_sun_utc | Date | date | G,R | Date and Time | | 31 | ok |
| 32 | end_of_week_sun_utc | Date | date | G,R | Date and Time | | 32 | ok |
| 33 | start_of_week_mon_utc | Date | date | G,R | Date and Time | | 33 | ok |
| 34 | end_of_week_mon_utc | Date | date | G,R | Date and Time | | 34 | ok |
| 35 | season | String | string | G,R | Date and Time | | 35 | ok |
| 36 | timestamp | Integer | number(integer) | G,R | Date and Time | | 36 | ok |
| 37 | timestamp_ms | Integer | number(integer) | G,R | Date and Time | | 37 | ok |
| 38 | utc | DateTime | **string** | G,R | Date and Time | Aliases | 1 | **→datetime** |
| 39 | dow | String | string | G,R | Date and Time | Aliases | 2 | ok |
| 40 | dow_abbr | String | string | G,R | Date and Time | Aliases | 3 | ok |
| 41 | repo | String | string | G | Repository | | 1 | ok (flag drift, see §3) |
| 42 | repo_root | String | string | G | Repository | | 2 | ok (flag drift, see §3) |
| 43 | is_monorepo | Boolean | boolean | G,R | Repository | | 3 | ok |
| 44 | package_root | Nullable(String) | string | G | Repository | Packages | 1 | ok |
| 45 | package_area_root | Nullable(String) | string | G | Repository | Packages | 2 | ok |
| 46 | packages | Csv | string | G | Repository | Packages | 3 | **→string[]** (G1) |
| 47 | packages_list | MarkdownList | string | G | Repository | Packages | 4 | **REMOVE** (G1) |
| 48 | package_areas | Csv | string | G | Repository | Packages | 5 | **→string[]** (G1) |
| 49 | package_areas_list | MarkdownList | string | G | Repository | Packages | 6 | **REMOVE** (G1) |
| 50 | current_package | Nullable(String) | string | G | Repository | Packages | 7 | ok |
| 51 | current_package_area | Nullable(String) | string | G | Repository | Packages | 8 | ok |
| 52 | area | String | string | G,R | Repository | Scope | 1 | ok |
| 53 | area_description | String | string | G,R | Repository | Scope | 2 | ok |
| 54 | area_root | String | string | G,R | Repository | Scope | 3 | ok |
| 55 | current_packages | MarkdownList | string | G,R | Repository | Scope | 4 | **→string[]** (G2) |
| 56 | depends_on | NestedMarkdownList | string | G,R | Repository | Scope | 5 | **→object[]** (G3) |
| 57 | used_by | NestedMarkdownList | string | G,R | Repository | Scope | 6 | **→object[]** (G3) |
| 58 | dirty_files | Csv | string | G,R | File Changes | | 1 | **→string[]** (G1) |
| 59 | dirty_files_list | MarkdownList | string | G,R | File Changes | | 2 | **REMOVE** (G1) |
| 60 | dirty_source_code_files | Csv | string | G,R | File Changes | | 3 | **→string[]** (G1) |
| 61 | dirty_source_code_files_list | MarkdownList | string | G,R | File Changes | | 4 | **REMOVE** (G1) |
| 62 | staged_files | Csv | string | G,R | File Changes | | 5 | **→string[]** (G1) |
| 63 | staged_files_list | MarkdownList | string | G,R | File Changes | | 6 | **REMOVE** (G1) |
| 64 | untracked_files | Csv | string | G,R | File Changes | | 7 | **→string[]** (G1) |
| 65 | untracked_files_list | MarkdownList | string | G,R | File Changes | | 8 | **REMOVE** (G1) |
| 66 | dirty_packages | Csv | string | G,R | File Changes | Packages | 1 | **→string[]** (G1) |
| 67 | dirty_packages_list | MarkdownList | string | G,R | File Changes | Packages | 2 | **REMOVE** (G1) |
| 68 | dirty_package_areas | Csv | string | G,R | File Changes | Packages | 3 | **→string[]** (G1) |
| 69 | dirty_package_areas_list | MarkdownList | string | G,R | File Changes | Packages | 4 | **REMOVE** (G1) |
| 70 | staged_packages | Csv | string | G,R | File Changes | Packages | 5 | **→string[]** (G1) |
| 71 | staged_packages_list | MarkdownList | string | G,R | File Changes | Packages | 6 | **REMOVE** (G1) |
| 72 | staged_package_areas | Csv | string | G,R | File Changes | Packages | 7 | **→string[]** (G1) |
| 73 | staged_package_areas_list | MarkdownList | string | G,R | File Changes | Packages | 8 | **REMOVE** (G1) |
| 74 | current_package_has_staged_files | Boolean | boolean | G,R | File Changes | Flags | 1 | ok |
| 75 | current_package_area_has_staged_files | Boolean | boolean | G,R | File Changes | Flags | 2 | ok |
| 76 | current_package_has_dirty_files | Boolean | boolean | G,R | File Changes | Flags | 3 | ok |
| 77 | current_package_area_has_dirty_files | Boolean | boolean | G,R | File Changes | Flags | 4 | ok |
| 78 | programming_languages_in_repo | Nullable(Csv) | string | G | Languages | | 1 | **→string[]** optional (G2) |
| 79 | programming_language | Nullable(String) | string | G | Languages | | 2 | ok |
| 80 | package_manager | Nullable(String) | string | G | Languages | | 3 | ok |
| 81 | docs_readme | Csv | string | G,R | Documents | | 1 | **→string[]** (G2) |
| 82 | docs_blast_radius | Csv | string | G,R | Documents | | 2 | **→string[]** (G2) |
| 83 | docs_drift | Csv | string | G,R | Documents | | 3 | **→string[]** (G2) |
| 84 | docs_skill | Nullable(String) | string | G | Documents | | 4 | ok |
| 85 | os | Nullable(String) | string | G | Operating System | | 1 | ok |
| 86 | os_distro | String | string | G,R | Operating System | | 2 | ok |
| 87 | os_package_manager | Nullable(String) | string | G | Operating System | | 3 | ok |
| 88 | os_version | String | string | G,R | Operating System | | 4 | ok |
| 89 | memory_total | Nullable(String) | string | G | Hardware | | 1 | ok |
| 90 | memory_used | Nullable(String) | string | G | Hardware | | 2 | ok |
| 91 | memory_avail | Nullable(String) | string | G | Hardware | | 3 | ok |
| 92 | cpu_cores | Nullable(Integer) | number(integer) | G | Hardware | | 4 | ok |
| 93 | cpu_arch | Nullable(String) | string | G | Hardware | | 5 | ok |
| 94 | gpu | Nullable(String) | string | G | Hardware | | 6 | ok |
| 95 | agent | String | string | G,R | Agent | | 1 | ok (desc drift, see §3) |
| 96 | model | String | string | G,R | Agent | | 2 | ok (desc drift, see §3) |

> Row count note: 96 rows because `utc`/`dow`/`dow_abbr` are counted; the spec's
> "95 variables" excludes one of the datetime aliases in its own count. The set
> of runtime-captured keys is the authority (parity test
> `descriptor_name_set_equals_captured_runtime_key_set`, catalog.rs:1267).

## 3. Drift catalog (what must be corrected)

### 3a. Temporal type drift (Task 3 — Phase 2 fixes)

The YAML is **wrong** for datetime-valued keys:

| key | YAML now | correct | source of truth |
|-----|----------|---------|-----------------|
| `now` | `string` | `datetime` | catalog `DateTime` + value `2024-06-15T10:30:00` |
| `now_utc` | `string` | `datetime` | catalog `DateTime` + value `...T17:30:00Z` |
| `utc` (alias of `now_utc`) | `string` | `datetime` | catalog `DateTime` |

`today`-family (today/today_utc/yesterday*/tomorrow*/start_of_week*/end_of_week*)
are **already** `date` in the YAML — no change (acceptance criterion 4).

**Time-family decision needed in Phase 2:** `time`, `time_military`, `time_utc`,
`time_military_utc` are `Time` in the catalog but `string` in the YAML. Their
captured values are *not* ISO times — e.g. `time` = `"10:30 AM"`, `time_utc` =
`"05:30 PM (UTC)"`, `time_military_utc` = `"17:30 (UTC)"`. SimplifiedSchema
`time` expects a strict `HH:MM:SS` form, so retyping these to `time` would
mislabel their real shape. **Recommendation:** keep them `string` and drop the
presentation-only `Time` display type (D3). This is a type-collapse, not a YAML
retype. Flag for explicit Phase 2 sign-off.

### 3b. Presentation-type collapse (Task 3 / D3)

`ContextValueType::{Timezone, Csv, MarkdownList, NestedMarkdownList}` are the
four presentation-only kinds with no SimplifiedSchema equivalent:

- `Timezone` (2 keys: `timezone`, `timezone_iana`) → `string`. YAML is already
  `string`, so this is display-enum removal only.
- `Csv` / `MarkdownList` / `NestedMarkdownList` → array types (see §4).

### 3c. Flag / nullability drift (Phase 2 must decide)

The catalog's non-`Nullable` `String`/`Csv` type disagrees with the YAML's
optional (`generated`, no `required`) flag on several keys. Capture *can* emit
`null` for these, so the YAML's "optional" is the accurate side:

- `repo`, `repo_root` — catalog `String` (non-null) vs YAML `string` optional.
- `packages`, `package_areas` — catalog `Csv` (non-null) vs YAML `string`
  optional. When these become `string[]` (G1), Phase 2 must decide whether the
  survivor is **optional `string[]`** (matches capture emitting `null`) or a
  required non-null array. **Recommendation:** keep optional to match capture.

### 3d. Description wording drift (D5 — YAML wins)

The two sources carry independently-worded descriptions; under D5 the YAML
description is authoritative and the catalog's is discarded. Two classes:

1. **Cosmetic** (majority): e.g. `day` — catalog "Full day of week name
   (local)." vs YAML "Full day of week name, local time." No action beyond
   letting YAML win.
2. **Catalog carries information the YAML lacks** — Phase 2 must **enrich the
   YAML** so no documentation is lost:
   - `time` → catalog "12-hour format with AM/PM"; YAML only "Local time."
   - `time_utc` → catalog "UTC time in 12-hour format with AM/PM"; YAML "UTC time."
   - `agent` → catalog "trimmed from the AGENT env var; defaults to \"unknown\"";
     YAML "Executing agentic CLI name."
   - `model` → catalog "trimmed from the MODEL env var; defaults to \"default\"";
     YAML "Active model identifier."
   - `repo` → catalog "from preferred remote URL"; YAML plain "Repository name…".

## 4. Variable collapse / retype lists (Groups 1–3)

Exactly as enumerated in the spec, confirmed against the sources above.

### Group 1 — CSV/MarkdownList twins → one `string[]`, drop the `_list` twin (10 pairs)

`packages`, `package_areas`, `dirty_files`, `dirty_source_code_files`,
`staged_files`, `untracked_files`, `dirty_packages`, `dirty_package_areas`,
`staged_packages`, `staged_package_areas` — each survives as `string[]`; the
`*_list` twin is **removed** (10 keys removed).

Old rendering to reproduce with D4 functions:
- bare survivor (was `Csv`) → `{{ as_csv(ctx.foo) }}`
- `*_list` (was `MarkdownList`) → `{{ as_unordered_list(ctx.foo) }}`

### Group 2 — single list vars → `string[]` (5 keys, name unchanged)

`current_packages` (was `MarkdownList`), `docs_readme`, `docs_blast_radius`,
`docs_drift` (were `Csv`), `programming_languages_in_repo` (was
`Nullable(Csv)` → optional `string[]`).

### Group 3 — genuinely nested → `object[]` (2 keys)

`depends_on`, `used_by` (were `NestedMarkdownList`). Modeled as
`object[]` of `{ package, dependencies|users }`. Current runtime shape is a
pre-rendered string produced by `render_dependency_list`
(capture.rs:1045–1064): `- 'pkg' depends on:` with each edge as a `    - dep`
sub-bullet, or an "empty" line when no edges. That verb wording is dropped as
presentation.

### Net effect (matches spec)

- **10** `_list` keys removed.
- **15** keys retyped to `string[]` (10 Group 1 survivors + 5 Group 2).
- **2** keys become `object[]` nested arrays.
- **4** presentation display types retired (`Timezone`, `Csv`, `MarkdownList`,
  `NestedMarkdownList`); `Time` retired too (§3a).

## 5. In-repo caller migration list (Task 4)

### 5a. Live callers referencing removed `_list` keys

| location | reference | migration |
|----------|-----------|-----------|
| `prompts/context.md:34` | `{{ctx.package_areas_list}}` | `{{ as_unordered_list(ctx.package_areas) }}` |
| `.claude/skills/darkmatter/compose.md:184` | doc row `ctx.dirty_files_list` | update doc for removed twin |
| `.claude/skills/darkmatter/compose.md:185` | doc row `ctx.staged_files_list` | update doc for removed twin |

(The remaining `_list` hits are inside `catalog.rs` example strings, which are
deleted when the catalog is rebuilt.)

### 5b. Live callers of bare survivors that relied on CSV output → `as_csv`

Bare `{{ ctx.foo }}` output changes from comma-joined to line-separated
(Phase 5). Callers that wanted CSV must move to `{{ as_csv(ctx.foo) }}`:

| location | reference(s) | notes |
|----------|--------------|-------|
| `prompts/context.md:36` | `{{ctx.programming_languages_in_repo}}` | inline prose "language(s) found …" — wants CSV → `as_csv` |
| `prompts/performance-review.md:4` | `packages: "{{ctx.packages \|\| '' }}"` | frontmatter value — verify shell/equality path; likely `as_csv` |
| `prompts/code-comment-quality.md:27` | `{{ctx.current_packages}}` | list context — likely `as_unordered_list` |
| `prompts/faster-builds-and-tests.md:17` | `{{ctx.current_packages}}` | list context — likely `as_unordered_list` |
| `claudine/docs/getting-started/index.md:299` | `{{ctx.dirty_files}}` | verify intended rendering |
| `darkmatter/example-docs/ctx-and-eval/test.md:21,23,24,31,32,38,39` | `docs_readme`, `docs_drift`, `docs_blast_radius`, `packages`, `package_areas`, `dirty_files`, `dirty_source_code_files` | example doc; update expected output/snapshots |

`claudine/docs/topics/context/expression-engine.md:13` uses
`length(ctx.packages)` — this **benefits** from arrays (length of an array vs a
string) and needs review but not necessarily a formatter.

### 5c. Documentation that lists ctx types/rendering (Phase 6 doc update)

- `.claude/skills/darkmatter/compose.md:176–195` — full ctx table with
  "Comma-separated …" / "Markdown bullet list …" descriptions.
- `darkmatter/docs/topics/context-variables.md` — catalog-derived topic doc.

## 6. Tests / snapshots / docs / generated artifacts asserting the affected surfaces (Task 5)

### 6a. Catalog + capture (largest blast radius)

- `darkmatter/lib/src/markdown/compose/context/catalog.rs`
  - `mod tests`: `descriptor_name_set_equals_captured_runtime_key_set`
    (removed `_list` keys break this until capture + catalog agree),
    `descriptor_traversal_order_is_deterministic`, `descriptor_names_are_unique`,
    `catalog_access_performs_no_capture`,
    `every_context_example_is_type_shape_only`.
  - `mod phase2_tests`: `capture_value_shape_matches_display_type`,
    `context_example_results_are_type_consistent` — both consume
    `ContextValueType` and `display_type`; must be rewritten when the enum is
    retired and examples move to `example()` files.
- `darkmatter/lib/src/markdown/compose/context/capture.rs`
  - `_list` insert sites at lines ~119–147 (name match), 898–941, 1078–1246
    (all `format_csv`/`format_md_list` calls), 1024–1064
    (`render_dependency_list` for `depends_on`/`used_by`).
  - Tests: `current_packages_lists_packages_under_cwd_as_markdown` (~1907),
    `depends_on_renders_nested_list_scoped_to_area` (~1926).
- `darkmatter/lib/src/markdown/compose/context/format.rs`
  - `format_csv`, `format_md_list` helpers + `test_format_csv`,
    `test_format_md_list` — retired (their job moves to D4 functions).

### 6b. Descriptor consumers (must keep compiling via the retained accessor)

- `darkmatter/lib/src/markdown/compose/context/mod.rs:18–19` — re-exports
  `ContextValueType`, `ContextVariableDescriptor`, `context_variable_descriptors`,
  `CONTEXT_VARIABLE_DESCRIPTORS`.
- `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs:30,141,150`
  — iterates `CONTEXT_VARIABLE_DESCRIPTORS` by name.
- `darkmatter/lib/src/markdown/compose/context/effective_state.rs:9,316,325` —
  iterates by name.
- `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:75,307` —
  `suggest(CONTEXT_VARIABLE_DESCRIPTORS, …)`; needs a `&`/`&*` when the const
  becomes a `LazyLock`-backed accessor (spec Runtime-projection note).
- `darkmatter/lib/src/catalog/mod.rs:444` — references the const.
- `darkmatter/dmls/src/providers/frontmatter.rs:15,288` and
  `darkmatter/dmls/src/overlay/expressions.rs:11,101,115` — DMLS reads
  `context_variable_descriptors()`; inherits corrected type/description for free
  (no change required here per spec).

### 6c. `md schema about` / schema-about output

- `darkmatter/lib/src/markdown/schemas/about.rs` — schema-about rendering.
- `darkmatter/cli/tests/schema_about.rs`,
  `darkmatter/cli/tests/level2_schema_about.rs` — CLI schema-about assertions.
- `darkmatter/docs/schemas/darkmatter-schema.md`,
  `darkmatter/docs/topics/schema-definition.md` — transclude/reference
  `darkmatter.yaml`; verify still correct after YAML edits.

### 6d. Expression-function catalog (D4/D7)

- `darkmatter/lib/src/markdown/compose/expression/catalog.rs` —
  `ExpressionFunctionDescriptor` (add typed signatures), the 6 new D4 functions,
  and tests: `descriptor_signature_set_equals_dispatchable_signature_set`,
  `every_descriptor_overload_is_dispatchable_at_its_declared_arity`,
  `feature_functions_are_present_in_exported_expression_catalog` (add the 6),
  `narrative_doc_function_table_matches_catalog`,
  `every_example_evaluates_to_its_declared_result`.
- `darkmatter/lib/src/markdown/compose/expression/functions.rs` — new dispatch
  arms; `darkmatter/lib/src/markdown/compose/expression/mod.rs:317,443` —
  `scalar_string` array arm (must stay byte-identical for equality/shell).
- `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:250` —
  interpolation output boundary (line-separated array rendering, Phase 5).
- `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:1760` —
  `scalar_string` in shell expansion (must stay byte-identical).
- `darkmatter/docs/topics/darkmatter-expressions.md` — generated function table
  between `<!-- BEGIN/END GENERATED FUNCTION TABLE -->` markers.

### 6e. Example artifacts (E3 — new `example()` YAML files, Phases 2 & 4)

- Referenced `ctx.*` example files (replacing the catalog's inline
  `TypeShapeOnly` examples), attached via `example(<file>)` in `darkmatter.yaml`.
- 6 new list-formatting-function example files (Phase 4). Prior-art examples
  already staged in the schema-plus feature dir
  (`as_unordered_list-example*.yaml`, `today-example.yaml`).

## 7. Validation checkpoint (Task 6) — status

This document **is** the baseline diff/inventory required before implementation:

- ✅ Exact catalog/schema drift enumerated (§2 merged table, §3 drift catalog).
- ✅ Exact caller migration list (§5) and test/doc/artifact surface (§6).
- ✅ Group 1/2/3 collapse & retype lists reconciled against both sources (§4).
- ✅ schema-plus precondition confirmed (§1).

No source code was changed in Phase 1. Phases 2–7 act on the lists above.
