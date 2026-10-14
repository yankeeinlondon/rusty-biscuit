---
spec: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-12-shared-type-tables/spec.md
plan: claudine/features/2026-10-12-shared-type-tables/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/schemas/resolve.rs
docs_updated_during_phase_2:
  - darkmatter/docs/topics/schemas/definition.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3: []
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_code:
  - darkmatter/lib/src/markdown/schemas/resolve.rs
documentation:
  - darkmatter/docs/topics/schemas/definition.md
completed_phase: 3
implemented: true
packages:
  - darkmatter
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


## Phase 2

Host: macOS (local). No other OS was run in this phase (see "OS risk" below).

### Wave 2 — implementation (`darkmatter/lib/src/markdown/schemas/resolve.rs`)

- Added `std::sync::Arc`; `Namespace.named_types` and
  `CachedNamespace.named_types` are now `Arc<IndexMap<String, PropertyDef>>`.
- First load: `Arc::new(load_named_types(&path)?)` once; the cache gets
  `Arc::clone(&named_types)` and the returned `Namespace` takes the original
  pointer, so both share one allocation (previously two deep copies).
- Cache-hit and `this` branches: `Arc::clone(...)` instead of deep
  `IndexMap::clone`.
- Root table in `expand_document_imports`: both arms (`shape.properties.clone()`
  for `Single`, `IndexMap::new()` for `Union`) wrapped in one `Arc::new`, still
  after the `schema_has_imports` early return, so no-import schemas allocate
  nothing.
- Unchanged: `load_named_types` signature and classification, the selected
  `PropertyDef` `.cloned()` in `expand_import` (works through `Arc` deref),
  `strip_top_level_presence`, `apply_import_postfix`, dependency insertion
  before cache lookup, canonical cache keys, base dirs, cycle stack, depth cap,
  and the literal missing-name error text ("top-level `$schema:` entries are
  its named types").
- Comments: the module section comment and `Namespace` doc now name both
  envelopes (`$schema:` and `kind: schema` / `types:`); the module comment
  states the per-expansion sharing lifetime; the `named_types` field doc says
  the table is shared and never mutated; the root copy carries a one-line WHY.

### Wave 2 — documentation (`darkmatter/docs/topics/schemas/definition.md`)

- "Cross-File Named-Type Imports": named types are now described as the
  top-level mapping entries of either envelope, linking to
  `#standalone-schema-documents`.
- Added an implementation note (before the "composition is the type system"
  paragraph) with a compact YAML example: tables are shared unexpanded within
  one resolution, each use gets its own definition copy, nothing persists
  across calls, no fixed speedup promised. No feature/fix references.

### Wave 3 — tests (new private module `resolve.rs::shared_type_tables`, L1)

| Requirement | Test |
| --- | --- |
| AC: first load and cache share one allocation; two later imports (distinct names `A`/`B`, reference spelled `./types.yaml` and `types.yaml`) reuse it; full `expand_schema` keeps it; one dependency edge — both envelopes | `first_load_cache_and_later_imports_share_one_table` |
| AC: `this` twice from inline root shares the root table; no cache entry or dependency | `this_from_inline_root_shares_the_root_table` |
| AC: `this` twice from a loaded file shares that file's table and is distinct from the root's — both envelopes | `this_from_loaded_file_shares_that_files_table` |
| AC: one cross-file type imported twice; constraints + description only on `primary`; exact JSON for both, `required == ["primary"]`, imported `required` still stripped on `backup`; one dependency — both envelopes | `cross_file_type_imported_twice_keeps_each_use_independent` |
| AC: inline-root `@this` imported twice, modifiers on one use only, exact JSON | `root_this_type_imported_twice_keeps_each_use_independent` |
| AC: `@this` inside an imported file selects that file's `name` (string) while root `@this` selects the root's `name` (number); use-site independence inside the imported object — both envelopes | `this_inside_imported_file_selects_that_files_type` |

- Identity tests call `ImportEngine::resolve_namespace` / `expand_schema`
  directly with `FileResolutionContext::new(tempdir)`; cache lookups use
  `canonical_path`. No public inspection API, counters, or env mutation.
- Envelope parameterization is a two-iteration in-process loop (no spawns;
  each test runs in ~30 ms).
- Existing tests reused rather than duplicated: single-use imports for both
  envelopes (`suggest_constraint_phase4::named_imports_share_pure_and_tagged_mapping_namespaces`),
  dependency dedup (`multiple_imports_from_same_file_resolve_and_dedup`),
  cycles, missing-name/file, raw-JSON and root-union rejection, shipped corpus.
  No existing expectation was edited.
- Mutation check: temporarily replacing the first-load and `this` sharing with
  deep copies (`Arc::new((*x).clone())`) made the three identity tests fail and
  left the three behavior tests green (behavior is meant to be identical);
  the source was then restored.
- Input robustness matrix: not applicable (no parser/loader change), as the
  plan states.

### Gates (local macOS)

- `darkmatter`: `just test` — 9160 run, 9160 passed, 11 skipped (baseline 9154
  + 6 new); `just lint` — exit 0.
- `claudine`: `just test` — 8673 run, 8673 passed, 9 skipped; `just lint` —
  exit 0 (claudine source unchanged; run as the downstream consumer).
- No pre-existing failures; nothing skipped beyond the suites' standard skips.

### OS risk

No cross-check run. The change is `std::sync::Arc` ownership only, and the new
tests build expected paths with the production `canonical_path`
(`canonicalize_simplified`) on hermetic temp dirs, so the macOS `/private`
symlink and Windows `\\?\` spellings match by construction. Existing CI cells
(Linux/macOS on PR, Windows on `main`) cover the rest.

### Departures from the spec

None.

## Phase 3

Host: macOS (local). No source, test, doc, or skill file changed in this
phase; it is validation, diff audit, and handoff only.

### Acceptance checks (local macOS)

| Command | Result |
| --- | --- |
| `just test` in `darkmatter/` | 9160 run, 9160 passed, 11 skipped (5 slow, all pre-existing `dmls` corpus/parity tests); exit 0 |
| `cargo nextest run -p darkmatter --lib -E 'test(/resolve::\|shared_type_tables/)'` | 102 passed, incl. all 6 `shared_type_tables::*` (each ≈ 30–40 ms) |
| `just lint` in `darkmatter/` | exit 0 (lib, cli, dmls, zed-dmls-cli, zed-dmls wasm32-wasip2) |
| `just test` in `claudine/` (downstream consumer) | 8673 run, 8673 passed, 9 skipped; exit 0 |
| `just lint` in `claudine/` | exit 0 |

- Evidence is local macOS only. No Linux, Windows, or WSL2 run was made in
  this phase; those come from the existing CI cells (PR: Linux + macOS; push to
  `main`: + Windows; nightly: + WSL2). No CI cell was added. The OS reasoning
  from Phase 2 stands: the change is `std::sync::Arc` ownership, and the new
  tests compute expected cache keys with production `canonical_path`.
- The claudine lint log printed `Checking darkmatter … (/Volumes/coding/personal/rusty-biscuit/…)`;
  `cargo metadata` confirms the workspace resolves darkmatter to this
  worktree, so that line is replayed cached compiler output, not a build of
  another checkout.
- Baseline comparison: Phase 1 baseline was 9154; the +6 are exactly the new
  `shared_type_tables` tests. Every resolver/corpus identity listed in the
  Phase 1 baseline passed, and no existing expectation was edited
  (`git diff` of `resolve.rs` shows no removed or changed lines inside any
  pre-existing test module).

### Final diff audit (`resolve.rs`, `definition.md`)

- Copy sites: `this` branch → `Arc::clone(&current.named_types)`; cache hit →
  `Arc::clone(&cached.named_types)`; first load → one
  `Arc::new(load_named_types(&path)?)` with `Arc::clone` into the cache and the
  original moved into the returned `Namespace`. No `named_types.clone()` deep
  copy remains.
- Root setup: one `Arc::new(match …)` holding the `Single` properties copy (or
  empty `Union` table), still after the `schema_has_imports` early return.
- Selected definition: `target.named_types.get(&name).cloned()` (L1109) still
  copies before recursive expansion, `strip_top_level_presence`, and
  `apply_import_postfix`. No `Arc::make_mut`, `Mutex`, `RefCell`, or global
  cache in the file; no expanded definition is shared.
- Phase 1 invariants 1–8 re-checked against the diff: dependency insertion
  before cache lookup, canonical keys, base dirs, request-scoped context and
  schema roots, `this` keeping `current.key`/`base_dir`, cycle stack and
  `MAX_IMPORT_DEPTH`, literal missing-name error text — all untouched lines.
- Comments: module comment, `Namespace` doc, and `named_types` field doc name
  both envelopes and describe the immutable per-expansion sharing; matches
  code. `definition.md` note matches implemented behavior, has no
  feature/fix references, and adds no planned behavior.
- Scope: no parser, loader signature, public API, `Cargo.toml`, CI, or
  unrelated file in the diff (`git diff --stat`: `resolve.rs`, `definition.md`,
  and this feature's own spec/plan/log).

### Acceptance criteria → evidence

| AC | Evidence |
| --- | --- |
| 1. No behavior change | darkmatter `just test` 9160/9160; Phase 1 resolver/corpus identities all pass; no expectation edits |
| 2. Allocation identity via `Arc::ptr_eq` | `shared_type_tables::first_load_cache_and_later_imports_share_one_table`, `this_from_inline_root_shares_the_root_table`, `this_from_loaded_file_shares_that_files_table` (Phase 2 mutation check made all three fail under deep copies) |
| 3. Use-site independence, both envelopes, cross-file / root `@this` / imported-file `@this` | `cross_file_type_imported_twice_keeps_each_use_independent`, `root_this_type_imported_twice_keeps_each_use_independent`, `this_inside_imported_file_selects_that_files_type`; single-use envelope coverage reused from `suggest_constraint_phase4::named_imports_share_pure_and_tagged_mapping_namespaces` |
| 4. Diff review | "Final diff audit" above |
| 5. Lint, portable L1, no new CI cells | darkmatter `just lint` exit 0; tests are hermetic temp-dir L1 in the lib test binary; no CI change |
| 6. Comments and docs | `resolve.rs` comment corrections; `definition.md` implementation note and two-envelope wording |

### Skipped / pre-existing

- No failures, pre-existing or new. No cross-OS run (see above). The
  `MAX_IMPORT_DEPTH` cap and whole-file-reference import targets still have no
  dedicated test; both were noted in Phase 1 as pre-existing gaps on code
  paths this work does not touch.
- Claudine skill: no update needed (no claudine code or workflow changed).

### Departures from the spec

None. Implementation complete, ready for review.
