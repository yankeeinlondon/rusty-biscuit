---
implementation_1: "2026-10-26T23:38:44-07:00"
---

# Review/Implement Log for Shared Acyclic Definitions

## Implementation of Review Findings #1

> **started at:** 2026-10-26T23:38:44-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-pipeline-finalization/darkmatter/features/2026-10-25-shared-acyclic-definitions/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review `implemented` flag confirmed `false` before repairs began

### Finding Ledger

| ID       | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| -------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1    | Blanket exclusions leave compatible structured definitions unshared (high) | fixed | all child rows fixed and verified; full gates green (see Completion Report) |
| R1-F1.1  | Disk catalog declaration (`GlobalCatalog::from_file`), `group` with two `entry` uses → store `entry` once, catalog consumers preserved | fixed | `shared_acyclic_consumers::a_disk_catalog_declaration_stores_a_repeated_type_once_like_ordinary_resolution`: control 1, `from_file` 1; `description()`, `shape()`, `member()` → resolving `Ref`, lowered baseline with one `$defs` entry |
| R1-F1.2  | Embedded catalog declaration (`GlobalCatalog::from_documents`) → same selection as disk and ordinary resolution | fixed | `an_embedded_catalog_declaration_selects_what_the_disk_catalog_selects`: `from_documents` stores 1; lowered baseline byte-identical to disk |
| R1-F1.3  | Ordinary schema `{ path: FILE }` with `file(match(...))` → 1 definition, file suggestions/validation unchanged | fixed | `a_matched_file_in_a_repeated_object_is_shared` (count 1 = plain control; out-of-glob file valid at both uses, Launch/Completion; no `x-darkmatter-match`; suggestions equal the inlined use) |
| R1-F1.4  | Ordinary schema `{ inner: { path: FILE }[] }` → 1 definition | fixed | `a_matched_file_in_a_nested_list_is_shared` (same assertions) |
| R1-F1.5  | Ordinary schema `{ <pattern::^n>: FILE }` → 1 definition | fixed | `a_matched_file_in_a_dictionary_is_shared` (same assertions; suggestion compared at the property) |
| R1-F1.6  | Ordinary schema `tuple([FILE, number])` → 1 definition | fixed | `a_matched_file_in_a_tuple_is_shared` (same assertions) |
| R1-F1.7  | Ordinary schema `FILE[] \| { n: number }` → 1 definition | fixed | `a_matched_file_list_in_a_union_is_shared` (same assertions) |
| R1-F1.8  | Ordinary schema `{ path: FILE }[]` → 1 definition | fixed | `a_matched_file_in_a_list_of_objects_is_shared` (same assertions) |
| R1-F1.9  | Ordinary schema `tuple([string, ...FILE[]])` → 1 definition | fixed | `a_matched_file_in_a_tuple_spread_is_shared` (same assertions) |
| R1-F1.10 | `GlobalDeclaration::definitions` and `GlobalCatalog::baseline` → sharing retained through effective-baseline path, no dangling refs | fixed | `a_shared_catalog_baseline_merges_into_document_schemas_and_validates_as_inlined` (rekey `entry-2`, equal-body reuse, every `$ref` resolves, located failures at `/<use>/n` match inline control), `a_baseline_definition_reached_only_by_overridden_properties_is_dropped_by_the_merge`, `the_shipped_baseline_shares_its_lifecycle_types_and_validates_as_its_inlined_form`; claudine `claudine_schemas` 799/799, budget M unchanged |
| R1-F1.11 | Root-union file-match contexts (five original container shapes) → previous suggesting behavior preserved (compatibility guard) | fixed | `a_root_union_arm_keeps_a_suggestion_only_match_through_a_repeated_type` (five shapes stay valid, now stored), `a_root_union_arm_enforces_a_recursive_glob_reached_through_a_shared_type` (seven cases); both fail with the fix disabled |
| R1-F1.12 | Docs and skills describing the blanket exclusions as permanent rules → updated | fixed | `definition.md`, `.claude/skills/darkmatter/schema.md`, `claudine/docs/topics/schemas.md`, source docs (`is_shareable`, `MatchCopies`, `Sharing`, `catalog.rs`, `darkmatter_base_schema*`, `merge_baseline`); grep finds no stale claim |
| R1-F1.13 | Sibling found by sweep: `file_match::file_alternatives` read a reference to a shared acyclic definition as recursive (`file(eager)[] \| { n: number }` used twice read as a file property) → read as its inlined body | fixed | `a_shared_type_offers_the_file_alternatives_of_its_inlined_value` (fails with the fix disabled); claudine `supplied` 70/70 |
| R1-F1.14 | Downstream consumer found by sweep: catalog sharing left unreferenced baseline definitions (17,908 bytes) in merged effective schemas → no unused JSON definition after merge | fixed | `a_baseline_definition_reached_only_by_overridden_properties_is_dropped_by_the_merge`; Claudine `effective-trigger-*` goldens byte-identical |
| R1-F1.15 | Sibling check: remaining `is_shareable` exclusions (top-level null, `operands`/`deferred`, arm markers, `suggest`, optional `file` arm, arm referencing a cycle) | clean | they depend on the definition's own top level, so every use needs the same fallback; the spec requires these compatibility cases (review's compatibility table) |

### Work Log

- starting the work on 'R1-F1 Blanket exclusions leave compatible structured definitions unshared' at 23:39:21
        - defect class: a context-free exclusion disables sharing for a whole loader or a whole type, instead of limiting the fallback to the individual uses whose existing semantics require inlining
        - split into two serial subagent passes because both touch the resolver: (A) file-match predicate, rows R1-F1.3–R1-F1.9 and R1-F1.11; (B) catalog loader and baseline consumers, rows R1-F1.1, R1-F1.2, R1-F1.10; each updates the docs for its half (R1-F1.12)
        - starting subagent A (file-match context) at 23:39:21
        - subagent A (file-match context) completed at 23:56:38
                - discovered: the root-union difference lives in lowering (`MatchCopies` sent every glob-reaching reference to an enforcing copy), not in selection; a "root is a union" resolution-time switch is not exact, because file-reference arms resolve as ordinary schemas and per-arm layer merges move references into arms after resolution
                - discovered: inside a cyclic definition's enforcing copy, inlined acyclic values were enforced, so a shared type used both at an arm's top level and inside a recursive body must suggest at the first and enforce at the second
                - sibling defect fixed: `file_match::file_alternatives` / `collect_reference_alternatives` read a reference to a shared acyclic definition like a recursive one (`file(eager)[] | { n: number }` used twice was read as a file property, a single inlined use was not); it now reads an acyclic reference as its inlined body
                - completed: removed `holds_file_match` from `is_shareable`; `MatchCopies` lowers each reference as its inlined value would at that place (cyclic or inside an enforcing copy → `<key>@match:<prop>`; shared acyclic → shared body; shared acyclic reaching a cyclic glob → redirecting `<key>@arm:<prop>` copy); added `DefinitionTable::is_cyclic`
                - design decision: fix in `MatchCopies` lowering, the only place that sees every route into a root-union arm (inline arms, file arms, merges); alternatives rejected: root-is-union predicate (inexact), per-use pass-2 fallback (cannot see merge-moved uses), enforcing globs in shared bodies (forbidden behavior change); downside: an extra `@arm` copy per arm property where a shared type reaches a recursive glob — **for the author to confirm**
                - sweep: other `is_shareable` exclusions (top-level null, `operands`/`deferred`, arm markers, `suggest`, optional `file` arm, arm referencing a cycle) depend on the definition's own top level, not on the use context, so they are not the same defect class
                - tests added in `darkmatter/lib/tests/l1/shared_acyclic_consumers.rs`: `matched_file_types_are_shared!` (seven generated tests), `a_root_union_arm_enforces_a_recursive_glob_reached_through_a_shared_type`, `a_shared_type_offers_the_file_alternatives_of_its_inlined_value`; updated `a_root_union_arm_keeps_a_suggestion_only_match_through_a_repeated_type`; each new guard confirmed failing with its fix disabled
                - focused checks: `just test shared_acyclic` 52/52, `recursive_schema_golden` 17/17, `root_union` 116/116, `file_match` 25/25, `match` 467/467, `completion` 159/159, `schema` 2846/2846, `just lint` clean; claudine `supplied` 70/70, `claudine_schemas` 799/799
        - starting subagent B (catalog loader and baseline consumers) at 23:56:38
        - subagent B (catalog loader and baseline consumers) completed at 00:25:21
                - discovered: with catalog sharing on, the shipped `fm` stores 9 definitions (`md-stack` is used by `composed`, `failed`, `cancelled`, `finalize`); `ctx`, `current`, `env`, `err`, `timing` store none
                - discovered: the effective-baseline path is JSON-only; `merge_baseline` / `fold_definitions` already move, rekey, and reuse a baseline's `$defs`
                - discovered and fixed: catalog sharing left 9 unreferenced baseline definitions (17,908 bytes) in Claudine's trigger effective schemas, because the trigger overrides the properties that used them; `move_baseline_definitions` now drops lower-layer definitions that no copied property reaches (`scope::retain_reached_definitions`); Claudine's effective outputs are byte-identical to before
                - completed: both catalog routes resolve with `Sharing::Automatic`; `GlobalCatalog::baseline()` returns `(SimplifiedSchema, Arc<DefinitionTable>)`; the shipped baseline JSON is lowered with its table; `ComposeOptions` carries `baseline_definitions` so the cached-baseline fingerprint stays equal to the stored baseline's
                - design decision: the definition-free public API `darkmatter_base_schema()` gets its inline form at the API boundary by resolving the shipped `fm` a second time with `Sharing::Off` (`catalog::darkmatter_inline_baseline`, once per process); every other consumer carries the table; alternatives rejected: rebuilding inline atoms from references (would reimplement the resolver's postfix merge rules and drift), returning the shared form (breaks callers lowering with `to_json_schema`), adding the baseline table to `layered_definitions` (silent `DefKey` shadowing); downsides: one extra `fm` resolution on first call, `GlobalCatalog::baseline()` signature change — **for the author to confirm**
                - consumer sweep: `shipped_baseline`, `darkmatter_base_json_schema*`, `with_baseline_from_file`, `ComposeOptions::with_darkmatter_baseline_schema` / `with_baseline_schema`, `prepare_schemas`, `baseline_canonical_json`, `merge_baseline` adapted; `frontmatter_shape` layered shapes keep the inline baseline (read with tables that hold no baseline definitions); `GlobalDeclaration::shape/member`, `compose/context/catalog.rs`, `expression/scope/projection.rs`, `shell_expansion/template/typing.rs`, `call_check.rs`, DMLS host-global hover/completion, Claudine `globals.rs` and `supplied.rs` already follow references through the table (clean)
                - byte budget: M unchanged at 768,262 (`both`); constant stays 1,000,000
                - goldens re-blessed after behavior tests passed (frozen inputs and manifest unchanged): `embedded-darkmatter-base` 383,792 → 86,921 bytes (now under 128 KiB, so stored as `.json`), `effective-baseline-inline` (now `.json`), `effective-baseline-schema-file` 774,698 → 162,908, `effective-baseline-root-union` 767,550 → 155,760; the three `effective-trigger-*` entries are byte-identical
                - tests added in `shared_acyclic_consumers.rs`: `a_disk_catalog_declaration_stores_a_repeated_type_once_like_ordinary_resolution`, `an_embedded_catalog_declaration_selects_what_the_disk_catalog_selects`, `a_shared_catalog_baseline_merges_into_document_schemas_and_validates_as_inlined`, `a_baseline_definition_reached_only_by_overridden_properties_is_dropped_by_the_merge`, `the_shipped_baseline_shares_its_lifecycle_types_and_validates_as_its_inlined_form`
                - focused checks: darkmatter `catalog` 239, `baseline` 234, `shared_acyclic` 57, `recursive_schema_golden` 17, `effective` 63, `schema` 2847; claudine `claudine_schemas` 799, `supplied` 70, `host_catalog` 55, `globals` 7; `just lint` clean in both
        - orchestrator reconciliation started at 00:25:21: no stale "catalog unshared" or "file-match never shared" claims remain in docs, skills, or source comments (grep); the remaining "never shared" text describes the per-definition `is_shareable` exclusions that still apply
        - full gates (run by the orchestrator after both subagents finished, with no subagent running):
                - `darkmatter`: `just test` 12,735 passed / 0 failed (10 skipped); `just test-l2` 18 + 69 + 3 passed; `just lint` clean; `just check-tier-coverage darkmatter` → stranded (0)
                - `claudine`: `just test` 11,663 passed / 0 failed (10 skipped); `just test-l2` 289 + 3 passed; `just lint` clean
                - DMLS allocation bench `cargo bench -p dmls --bench pipeline_work_a3` re-run (the shipped baseline JSON changed representation): hover verb 66,277 and hover expression 69,943 are unchanged in every case; diagnostics unchanged warm (a3x10-warm 73,175,112) and slightly lower cold (a3x1-cold 318,554,816 → 318,552,184); the cold overlay rose about 0.94 MB (a3x1-cold 25,085,641 → 26,027,170, +3.7%), consistent with the one extra `fm` resolution for the inline baseline projection; warm totals are within 8 KB
                - cross-OS check skipped: the repair changes resolver and lowering logic only, with no path, process, or OS-specific code
- work completed for 'R1-F1 Blanket exclusions leave compatible structured definitions unshared' at 00:34:00

### Completion Report

- **outcome:** complete
- **elapsed:** 55 minutes (2026-10-26T23:38:44-07:00 → 2026-10-27T00:34:00-07:00)
- **parent findings:** total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- **instance rows:** 15 (R1-F1.1–R1-F1.12 from the review; R1-F1.13–R1-F1.15 from the sweep): 14 fixed, 1 clean
- **verified repairs:**
        - file-match half: a repeated type holding `file(match(...))` is now stored once, like the plain `file` control; root-union arms keep the suggesting behavior of the inlined copy and still enforce globs inside recursive types, because `MatchCopies` now lowers each reference as its inlined value would at that place
        - catalog half: global catalog declarations (disk and embedded) and the built-in baseline share repeated acyclic types; every consumer carries the definition table, except `darkmatter_base_schema()`, which projects an inline form at its API boundary; the baseline merge drops lower-layer definitions that no copied property reaches
        - sibling: `file_match::file_alternatives` reads a shared acyclic reference as its inlined body
- **decisions for the author to confirm:**
        - the root-union file-match fix lives in `MatchCopies` lowering, adding a redirecting `<key>@arm:<prop>` copy where a shared type reaches a recursive glob
        - `darkmatter_base_schema()` stays definition-free by re-resolving `fm` once per process with `Sharing::Off` (its only remaining caller); `GlobalCatalog::baseline()` now returns `(SimplifiedSchema, Arc<DefinitionTable>)`
        - goldens re-blessed: `embedded-darkmatter-base` (383,792 → 86,921 bytes; now `.json` instead of `.digest`), `effective-baseline-inline` (now `.json`), `effective-baseline-schema-file` (774,698 → 162,908), `effective-baseline-root-union` (767,550 → 155,760); frozen inputs and the manifest are unchanged
- **open findings:** none
- **byte budget:** M unchanged at 768,262; `PAYLOAD_LIMIT_BYTES` stays 1,000,000
- **changed files (this cycle):**
        - R1-F1.3–R1-F1.9, R1-F1.11, R1-F1.13: `darkmatter/lib/src/markdown/schemas/resolve/recursive.rs`, `simplified/convert.rs`, `simplified/definitions.rs`, `simplified/query.rs`, `file_match.rs`
        - R1-F1.1, R1-F1.2, R1-F1.10, R1-F1.14: `darkmatter/lib/src/markdown/schemas/resolve.rs`, `catalog.rs`, `mod.rs`, `scope.rs`, `frontmatter_shape.rs` (comment), `compose/context/options.rs`, `compose/schema_validation.rs`, `schemas/tests/mod.rs`, `schemas/tests/embedded_sources.rs`
        - tests: `darkmatter/lib/tests/l1/shared_acyclic_consumers.rs`, `global_catalog.rs`, `base_schema_end_to_end.rs`; goldens under `darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/`
        - R1-F1.12 docs: `darkmatter/docs/topics/schemas/definition.md`, `.claude/skills/darkmatter/schema.md`, `claudine/docs/topics/schemas.md`
- **final verification:** darkmatter `just test` (12,735 pass), `just test-l2` (pass), `just lint` (clean); claudine `just test` (11,663 pass), `just test-l2` (pass), `just lint` (clean); tier coverage stranded 0; DMLS bench recorded above
