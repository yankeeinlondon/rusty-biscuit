---
spec: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-12-shared-type-tables/spec.md
plan: claudine/features/2026-10-12-shared-type-tables/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
---

# Implementation Log for 2026-10-12-shared-type-tables (3 phases)

## Phase 1

Phase 1 is scope and evidence only: no source, test, doc, or skill file was
changed. All line numbers refer to
`darkmatter/lib/src/markdown/schemas/resolve.rs` as of this phase (before any
Phase 2 edit).

### Ownership map

- `Namespace` (≈L892) and `CachedNamespace` (≈L919) each hold
  `named_types: IndexMap<String, PropertyDef>` by value.
- **Copy site 1 — `this` branch** (`resolve_namespace`, ≈L1147–1153):
  `current.named_types.clone()` deep-copies the current namespace's table on
  every `Name@this`.
- **Copy site 2 — cache hit** (≈L1208–1214): `cached.named_types.clone()`
  deep-copies the cached file table on every repeat import of a file.
- **Copy site 3 — first-load insertion** (≈L1215–1230): `load_named_types`
  returns the table, the cache receives `named_types.clone()`, and the
  original is moved into the returned `Namespace`. So the first load
  produces two allocations; the cache and first return do not share.
- **Root properties copy** (`expand_document_imports`, ≈L943–946):
  `shape.properties.clone()` for `SimplifiedSchema::Single`, an empty
  `IndexMap` for `SimplifiedSchema::Union`. It runs *after* the
  `schema_has_imports` early return (≈L939–941), so schemas with no imports
  allocate nothing. It is the intentional one-time copy: `schema` is then
  consumed by `expand_schema`, so `@this` needs its own unexpanded table.
  Phase 2 wraps it (and the empty union table) in an `Arc`.
- **Selected-definition clone** (`expand_import`, ≈L1100):
  `target.named_types.get(&name).cloned()` — the single `PropertyDef` copy
  that recursive expansion, `strip_top_level_presence`, and
  `apply_import_postfix` consume. Must stay.
- `load_named_types` (≈L1258) returns `shape.properties` for a mapping
  envelope (pure `$schema` or `kind: schema` / `types`, via
  `parse_standalone_schema_document`) and rejects raw JSON Schema
  (`AmbiguousReferenced`), root-union, and whole-file-reference documents
  (`SchemaDocument`). Phase 2 leaves its signature unchanged and wraps the
  result at the call site.

### Engine boundaries (unchanged by this work)

`expand_document_imports` builds a fresh `ImportEngine` (and so a fresh
`namespace_cache`) per call. Callers: inline mapping (≈L167, `Root`), inline
root-union mapping arms (≈L219, `Root`, one engine per arm), file-referenced
single schema (≈L726, `File`), and file-referenced root-union inline arms
(≈L778, `File`, one engine per arm). Sharing will therefore last exactly one
expansion; no cross-call state is introduced.

### Invariants to re-check after Phase 2 edits

1. `self.dependencies.insert(canonical.clone())` precedes the cache lookup, so
   cache hits still record the dependency edge.
2. Cache key is `canonical_path(&path)` (`canonicalize_simplified` with
   fallback); `NamespaceKey::File(canonical)` on both hit and miss paths.
3. Imported-file `base_dir` is `path.parent()` (fallback `.`), stored in the
   cache and returned on hits.
4. Request-scoped `file_resolution_context` and `schema_roots` are used only
   for path resolution before the cache; untouched.
5. `this` keeps `current.key` and `current.base_dir`, so `@this` inside an
   imported file still selects that file's table.
6. Cycle stack frame `(target.key, name)` and `MAX_IMPORT_DEPTH` (32) check in
   `expand_import`; untouched.
7. Missing-name error text "… (a schema file's top-level `$schema:` entries
   are its named types)" (≈L1104) is literal error text and must be preserved
   even though it omits `kind: schema` / `types`.
8. Comments to correct in Phase 2: the `Namespace` doc ("the set of
   top-level `$schema:` entries") and the `named_types` field doc
   ("(`$schema:` entries)") omit the `kind: schema` / `types` envelope;
   `CachedNamespace` has no doc.

### Coverage map

Existing tests (resolver private modules unless noted):

| Behavior | Existing test(s) | Adequate? |
| --- | --- | --- |
| Cross-file import | `schema_plus_phase1::expands_named_type_import` | yes (single use) |
| Cross-file reuse of one file, distinct names, dependency dedup | `schema_plus_phase1::multiple_imports_from_same_file_resolve_and_dedup` | yes for dedup and cache-hit lookup; not independence |
| Root `@this` | `schema_plus_phase1::resolves_this_self_import`, `constrained_import_applies_use_site_constraint` | yes (single use) |
| Use-site constraint and stripping of imported `required` | `constrained_import_applies_use_site_constraint`, `multiple_imports_from_same_file_resolve_and_dedup` (A is `required` in types, optional at use) | yes |
| Array import | `expands_array_named_type_import` | yes |
| Dependency edge | `import_records_dependency_edge`; `tests/mod.rs::dependencies_surface_import_and_example_edges` | yes |
| Cycle | `import_cycle_is_detected`; `lib/tests/l1/suggest_constraint_phase4.rs::import_cycles_remain_bounded_across_envelope_forms` | yes |
| Missing file / missing name / raw JSON target | `import_from_missing_file_errors`, `import_of_missing_type_name_errors`, `import_from_non_simplified_file_errors`; `suggest_constraint_phase4::raw_json_schema_remains_distinct_and_cannot_supply_named_imports` | yes |
| Root-union and whole-file-reference targets rejected | `suggest_constraint_phase4::pure_sequence_is_a_whole_file_union_but_not_a_namespace` (root union) | root union yes; whole-file-reference target has no import test (unchanged code path, not an acceptance claim) |
| Both mapping envelopes as named-import targets | `suggest_constraint_phase4::named_imports_share_pure_and_tagged_mapping_namespaces`, `tagged_schema_resolves_nested_imports_and_examples_from_its_directory` | yes (single use) |
| Many imports of one `kind: schema` file (shipped artifact) | `lib/tests/l1/content_policy_editor_schema.rs` (`policy@./content-policy.yaml` per entry) | exercises cache hits; asserts validation only |
| Shipped-schema corpus | `lib/tests/l1/meta_schema_repo_schemas.rs` | yes |
| Ordering | `IndexMap` ordering asserted implicitly by JSON equality tests above | yes |
| Depth cap (`MAX_IMPORT_DEPTH`) | none found | pre-existing gap; code path untouched by this work, not an acceptance claim |

**Missing acceptance coverage for Phase 2** (nothing else needs adding):

1. Allocation identity (AC 2) — none exists. Needs private tests calling
   `ImportEngine::resolve_namespace` directly: first file load vs
   `namespace_cache` entry (`Arc::ptr_eq`), two later lookups for distinct
   names vs the original; `this` twice from the inline root vs root table;
   `this` twice from a loaded file namespace vs its loaded table; loaded table
   not `ptr_eq` root table. Build the engine with
   `FileResolutionContext::new(tempdir)` and look up the cache with
   `canonical_path(...)`.
2. Use-site independence (AC 3) — no test imports one type twice with a
   modifier on only one use. Needed for: cross-file reuse, inline-root
   `@this`, and `@this` authored inside an imported file (root and imported
   file must define the same name differently so output proves which table
   was selected). Parameterize over pure `$schema` and `kind: schema` /
   `types` target envelopes.

Fixture conventions: existing resolver tests use `tempfile::tempdir()`, a
local `write` helper, and `biscuit_file::FileResolutionContext::new(dir)`;
no environment mutation. Follow the same in Phase 2.

### Baseline (`just test` in `darkmatter/`, before any source change)

- Host: macOS (local). No other OS was run in Phase 1; no source changed.
- `just test` (darkmatter area): **9154 run, 9154 passed, 11 skipped, 0
  failed** in 51 s; exit 0. No pre-existing failures.
- `just lint` (darkmatter area): exit 0, clean (lib, cli, dmls, zed-dmls-cli,
  and the zed-dmls wasm32-wasip2 check).
- Caveat: the worktree already carried uncommitted changes from
  `2026-10-11-schema-loader-yaml-spellings` (e.g. `simplified/cursor.rs`,
  `simplified/source.rs`, dmls frontmatter diagnostics). The baseline
  includes them; none touch `resolve.rs`.
- Resolver/corpus test identities that passed and must still pass unchanged
  in Phase 3:
  - `markdown::schemas::resolve::tests::*`,
    `markdown::schemas::resolve::schema_plus_phase1::*` (incl.
    `expands_named_type_import`, `resolves_this_self_import`,
    `expands_array_named_type_import`, `constrained_import_applies_use_site_constraint`,
    `import_cycle_is_detected`, `import_from_missing_file_errors`,
    `import_of_missing_type_name_errors`, `import_from_non_simplified_file_errors`,
    `import_records_dependency_edge`, `unresolved_import_reaching_conversion_is_a_convert_error`,
    `multiple_imports_from_same_file_resolve_and_dedup`, all `example_*`),
    `markdown::schemas::resolve::bare_name_phase3::*` (72 resolver tests total)
  - `markdown::schemas::tests::dependencies_surface_import_and_example_edges`
    and `markdown::schemas::claudine_compat_tests::dependencies_surface_import_and_example_edges`
  - `darkmatter::l1 suggest_constraint_phase4::*` (10 tests, both envelopes,
    cycles across envelopes, raw-JSON and root-union rejection)
  - `darkmatter::l1 content_policy_editor_schema::*` (7 tests; shipped
    `kind: schema` artifact imported once per entry, exercising cache hits)
  - `darkmatter::l1 meta_schema_repo_schemas::*` (3 tests; shipped corpus)

### Phase 1 checkpoint

All three copy sites, the intentional root copy, and the selected-definition
clone are located; coverage gaps are limited to allocation identity and
use-site independence (including imported-file `@this` and both envelopes).
No unresolved ruling, parser change, or spike is needed; Phase 2 can proceed
as planned.

