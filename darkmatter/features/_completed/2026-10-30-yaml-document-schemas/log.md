---
feature: 2026-10-30-yaml-document-schemas
review: review-1.md
implementation_1: "2026-10-31T22:23:58-07:00"
implementation_2: "2026-11-01T00:08:38-07:00"
implementation_3: "2026-11-01T01:04:36-07:00"
---

# Review Implementation Log

## Implementation of Review Findings #1

> **started at:** 2026-10-31T22:23:58-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-30-yaml-document-schemas/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was already `false`; it was left `false` before repairs began
- the earlier phase-by-phase implementation record is `implementation-log.md` in this folder

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | DMLS suppresses required fallback validation | fixed | all required children verified; siblings F1.6/F1.7 fixed |
| R1-F1.1 | CLI fallback report, inline schema with required owner, no data → missing-property failure | clean | `schema_validate_yaml::a_root_schema_without_data_fails_its_required_property`: exit 1, one problem naming `owner` |
| R1-F1.2 | DMLS overlay/diagnostics, same no-data buffer → publish missing-property error | fixed | `yaml_document_session::deleting_the_last_data_property_of_a_root_schema_document_reports_the_missing_property`: one `dm.schema.missing_required` naming `owner`; fails (`[]`) with the exception restored |
| R1-F1.3 | DMLS fallback with a data property → type error | clean | same test, first step: exactly `[dm.schema.type_mismatch]` |
| R1-F1.4 | DMLS assistance selection, no-data buffer → data assistance plus declaration assistance inside `$schema` | fixed | same test: data-key hover shows the description; root completion offers `owner`; completion inside the declaration offers `string`; declaration hover still works |
| R1-F1.5 | Claimed-kind route → claiming definition stays authoritative | clean | `yaml_document_editor::a_claimed_*` and `yaml_document_session::*` roster/shipped-kind tests unchanged and passing |
| R1-F1.6 | (discovered) declaration error of an informal schema published twice (envelope + route prepare error) | fixed | `yaml_document_session::an_invalid_declaration_of_a_root_schema_document_is_published_once`; fails with two diagnostics without the guard |
| R1-F1.7 | (discovered) named-type failure published twice; code actions on informal schemas | fixed | `yaml_document_editor::an_informal_standalone_schema_is_judged_as_data_and_each_problem_is_published_once` (offers "Add required key `owner`") |
| R1-F2   | Unsaved schema sources do not consistently refresh YAML validation | fixed | all required children verified; YAML-path siblings F2.8–F2.13, F2.15–F2.17 fixed |
| R1-F2.1 | Existing root definition buffer → refresh before save | clean | existing `an_unsaved_definition_edit_refreshes_an_open_document_until_the_definition_closes` passes |
| R1-F2.2 | Imported named types inside a schema root, unsaved → refresh | fixed | `an_unsaved_edit_of_a_type_library_in_a_schema_root_refreshes_the_definitions_dependents` (saved control clean; unsaved → `type_mismatch` + hover `number`; close → clean; save → mismatch); fails with the snapshot disabled |
| R1-F2.3 | Imported named types outside schema roots, unsaved → refresh | fixed | `an_unsaved_edit_of_a_type_library_outside_the_schema_roots_refreshes_the_definitions_dependents`; same steps; fails with the snapshot disabled |
| R1-F2.4 | Explicit fallback schema target, unsaved → refresh | fixed | `an_unsaved_edit_of_an_explicit_root_schema_target_refreshes_the_document_that_names_it`; fails with the snapshot disabled |
| R1-F2.5 | New root definition buffer not on disk → discovered | fixed | `an_unsaved_new_definition_in_a_schema_root_claims_its_kind_until_it_closes`; fails with the root-listing merge disabled |
| R1-F2.6 | Saved changes + watched events → same result | clean | last step of the F2.2–F2.4 tests; existing create/delete tests pass |
| R1-F2.7 | Close without save → saved text restored | fixed | close step of the F2.2–F2.5 tests (imports, targets, new files); definitions already covered |
| R1-F2.8 | (discovered) open buffer at a missing import location | fixed | `an_unsaved_file_at_a_missing_import_of_a_definition_is_read_until_it_closes` |
| R1-F2.9 | (discovered) open buffer at a missing explicit `$schema` target | fixed | `an_unsaved_file_at_a_missing_root_schema_target_is_selected_until_it_closes` |
| R1-F2.10 | (discovered) imports read by `kind: schema` kind checks | fixed | `an_unsaved_edit_of_an_imported_type_library_refreshes_a_schema_documents_kind_checks` and in-process `yaml_document_editor::a_schema_documents_kind_check_reads_an_imported_type_librarys_unsaved_text` (fails with dependency tracking reverted) |
| R1-F2.11 | (discovered) ruling-8 shipped-copy identity for an editor-only file | fixed | `yaml_kind_registry::an_unsaved_copy_at_the_shipped_path_replaces_the_built_in_definition` (distinguishes old code only on Unix, through a symlinked alias) |
| R1-F2.12 | (discovered) `$schema` value hover preview read disk | fixed | preview reads the open buffer (`overlay/references.rs`); covered by the hover steps of F2.4 |
| R1-F2.13 | (discovered) `example(file)` and error-span re-reads | fixed | the same library read seam (`buffers::read`); `buffers::` unit tests |
| R1-F2.14 | (discovered) Markdown frontmatter `$schema` targets read saved files only | clean | outside this spec, which governs open YAML documents; behavior unchanged from base and documented in `dmls-schema-support.md` |
| R1-F2.15 | (discovered) open schema document's own envelope/standalone imports | fixed | `an_unsaved_edit_of_an_imported_type_library_refreshes_an_open_schema_documents_authoring_state` (`kind: schema` and informal); fails with the fix disabled |
| R1-F2.16 | (discovered) `.json` schema targets | fixed | `an_unsaved_edit_of_a_json_schema_target_refreshes_the_document_that_names_it`; fails before the fix |
| R1-F2.17 | (discovered) definition in a schema-root folder that does not exist yet | fixed | `an_unsaved_definition_in_a_schema_root_folder_that_does_not_exist_claims_its_kind_until_it_closes`; fails before the fix |
| R1-F2.18 | (discovered) trigger registries | clean | accepted human-review item: existing saved-file trigger behavior is kept |
| R1-F3   | Standalone YAML loses bare-name schema lookup roots | disputed | cited authority ("existing policy") is the gated frontmatter policy; YAML matches it (see progress) |
| R1-F3.1 | Library, gate closed, bare SCHEMAS_DIR reference | disputed | `yaml_kind_registry::a_bare_root_schema_searches_the_schema_roots_only_inside_a_repository` pins parity: fails to resolve, as frontmatter does |
| R1-F3.2 | CLI, no repository | disputed | `schema_validate_yaml::outside_a_repository_a_bare_root_schema_resolves_as_frontmatter_does`: `.md` and `.yaml` give the same exit code (2) |
| R1-F3.3 | DMLS, no workspace folders | disputed | `yaml_document_session::a_bare_root_schema_searches_the_schema_roots_only_inside_a_workspace_folder`: YAML and Markdown both get `dm.schema.prepare` |
| R1-F3.4 | Library, gate open → valid | clean | same library and DMLS tests: valid inside the repository/folder |
| R1-F3.5 | Absolute path outside gates → valid | clean | all three new tests: library valid, CLI exit 0, DMLS no schema diagnostics |
| R1-F3.6 | Kind discovery outside gates stays disabled | clean | `yaml_kind_registry::a_registry_built_outside_a_repository_holds_the_shipped_kinds_only` |
| R1-F3.7 | (sibling) `semantic::kind_checks` import roots | clean | uses the registry roots, matching DMLS standalone named types gated on `trigger_boundary` |
| R1-F3.8 | (sibling) DMLS `$schema` reference assistance roots | clean | `SchemaReferenceFacts::select_in` receives the same roots as validation |
| R1-F4   | YAML schema preparation still runs on the editor request thread | fixed | all required children verified; siblings F4.7/F4.8 fixed |
| R1-F4.1 | Kind registry discovery/preparation → background | clean | existing `a_yaml_documents_kinds_are_prepared_off_the_dispatch_thread` passes |
| R1-F4.2 | Claimed-kind validator construction → background | fixed | `a_yaml_documents_contract_is_prepared_and_compiled_off_the_dispatch_thread` (`team.yaml`): `Work::YamlContractPreparation` threads exclude the dispatch thread; fails with preparation forced back onto the request path |
| R1-F4.3 | Inline fallback → background | fixed | same test, `inline.yaml` |
| R1-F4.4 | Referenced fallback → background | fixed | same test, `service.yaml` → `./contract.yaml` |
| R1-F4.5 | Data-only edit → reuse prepared contract | fixed | `a_data_only_edit_reuses_the_prepared_contract_and_reads_no_schema_file`: zero preparations/compiles/reads across data edits, diagnostics still update; `$schema` change re-prepares; library `a_prepared_contract_judges_later_data_as_validation_does_without_compiling` |
| R1-F4.6 | Built-in kind semantic imports → off dispatch | fixed | thread test `owners.yaml`: `Work::YamlKindCheck` threads exclude the dispatch thread |
| R1-F4.7 | (discovered) open standalone schema import resolution on dispatch | fixed | `Work::StandaloneImportCheck` threads exclude the dispatch thread (thread test) |
| R1-F4.8 | (discovered) `$schema` reference facts re-probed on every rebuild | fixed | prepared with the contract; `schema_target_inspections` 0 after data edits |
| R1-V1   | (phase gate) hashing guard: `declaration_key` used `DefaultHasher` | fixed | now `biscuit_hash::xx_hash` of the serialized declaration; `hashing_guard` passes |
| R1-V2   | (phase gate) `overlay::tests::local_references_record_no_external_dependency` read the moved standalone cache | fixed | test reads the YAML contract cache's dependency count (test-only accessor); passes |
| R1-V3   | (cross-OS) Windows `recursive_schema_golden::inline_root_entries_match_their_goldens`: 272-char fixture path kept the `\\?\` prefix | fixed | the test's `relative` strips a verbatim disk prefix; `just cross-check darkmatter --os windows` pass (9749) |
### Progress

- starting the work on 'R1-F1 DMLS suppresses required fallback validation' at 22:24:32
        - defect class: a secondary content classifier (standalone-schema envelope recognition) overrode the selected validation route and hid its result
        - removed the informal-schema exception in `dmls/src/overlay/mod.rs`; every envelope overlay now merges the YAML result through `with_yaml`, the same ownership rule `kind: schema` uses, so authoring state inside `$schema`/`types` is kept
        - discovered sibling: once the exception was removed, a declaration error was published twice (envelope + route prepare error); `dmls/src/diagnostics/yaml_document.rs` now lets the envelope own a root-`$schema` route failure when the envelope reported an error or a standalone resolution failure (R1-F1.6, R1-F1.7)
        - sibling sites checked with no change needed: completion/hover arbitration keyed on `SchemaAuthoringState::Standalone`, named-reference declarations, code actions (read the YAML tree), semantic tokens
        - decision (human review): an informal schema whose `$schema` names its own `types` now shows the same "unknown named type" error `md schema validate` reports (exit 2); the alternative of hiding route prepare errors would repeat the R1-F1 defect class; 2 repository files are affected (`claudine/docs/research/{model-config,reasoning-level}/_schema.yaml`)
        - consequence: informal schema files that do not satisfy their own required properties now show matching DMLS errors, as the CLI already reported
        - three tests that relied on the exception were updated (`lsp_session` x2, `named_type_references` x1); the earlier implementation log's recommended library exemption is superseded
        - docs: `docs/topics/schemas/dmls-schema-support.md`, darkmatter skill pages `dmls.md`, `schema.md`, `cli.md`
        - verification: `just test yaml_` 319 passed; dmls L1 (`cargo nextest run -p dmls`, L1 filter) 1694 passed; `just lint` clean; new tests fail with the exception restored
- work completed for 'R1-F1 DMLS suppresses required fallback validation' at 22:36:52
- starting the work on 'R1-F3 Standalone YAML loses bare-name schema lookup roots' at 22:36:52
        - defect class (as reviewed): a gate on automatic kind discovery also removes the roots that explicit schema-reference lookup needs
        - disputed: the finding's authority ("Schema-file lookup continues to use the shared selector and its existing policy") names the existing policy, and that policy is gated. Markdown frontmatter gets bare-name roots only from the trigger registry: `md schema validate` discovers only in a repository (`cli/src/commands/schema/validate.rs:291`), compose checks `repository_root()` (`lib/src/markdown/compose/schema_validation.rs:267`), DMLS only inside workspace folders, and `docs/topics/schemas/authoring-schemas.md` already says "`md` searches these folders only for a document in a repository"
        - reproduced: outside a repository with `SCHEMAS_DIR` set, `a.md` and `a.yaml` both exit 2 ("no file matched `contract.yaml`"); after `git init` both exit 0; YAML matches frontmatter
        - parity tests added to pin the shared behavior: `yaml_kind_registry::a_bare_root_schema_searches_the_schema_roots_only_inside_a_repository` (library, F3.1/F3.4/F3.5), `schema_validate_yaml::outside_a_repository_a_bare_root_schema_resolves_as_frontmatter_does` (CLI, F3.2/F3.5), `yaml_document_session::a_bare_root_schema_searches_the_schema_roots_only_inside_a_workspace_folder` (DMLS, F3.3/F3.4/F3.5)
        - sibling sites `semantic::kind_checks` imports (F3.7) and DMLS `$schema` reference assistance (F3.8) use the same roots as validation; clean
        - docs: `docs/topics/schemas/definition.md` now states the gate in both bare-name passages
        - author question (human review): should explicit bare-name lookup ignore the discovery gate? If so, change frontmatter and YAML together
        - verification: `just test yaml_` 321 passed; `schema_root` 43, `roots` 127 passed; `just lint` clean
- work completed for 'R1-F3 Standalone YAML loses bare-name schema lookup roots' at 22:42:03
- starting the work on 'R1-F2 Unsaved schema sources do not consistently refresh YAML validation' at 22:42:03
        - defect class: editor buffers were applied only to existing root candidates, while dependency resolution and root enumeration still read saved files
        - library seam: new `simplified-schema/lib/src/resolve/buffers.rs` (`SourceBuffers`, a `Send + Sync` path→text snapshot, and `buffers::with`, which installs it for one call). Resolver reads (`$schema` targets including JSON Schema, `Name@file`, `example(file)`, error-span re-reads) and `select_schema_file` honor it; an editor-only file is selected only when no file exists at that location
        - decision (human review): a thread-scoped snapshot rather than an explicit reader parameter, which would have changed about 30 resolver signatures that carry `Copy` `ShippedSources`. This follows the existing thread-local `observed::collect` pattern. Rejected: an explicit parameter, and a DMLS-only copy of resolution
        - DMLS: open YAML/JSON buffers form the snapshot; open YAML buffers inside a searched root, including a root folder that does not exist yet, are listed with disk candidates (new `SchemaRootSource::absent_root_files`); imports and targets are recorded with their hash and buffer origin; the YAML cache's change checks see buffers; opening a buffer at a searched path marks candidate registries stale
        - renamed `set_yaml_buffer`→`set_schema_buffer` and `note_yaml_buffer`→`note_schema_buffer` (they now carry JSON too)
        - first subagent left five rows open (F2.10, F2.11, F2.15, F2.16, F2.17); a second subagent closed all five with fixes and tests, each confirmed failing without its fix
        - R1-F2.14 (Markdown frontmatter `$schema` targets see unsaved edits only after save) is outside the spec, which governs open YAML documents; this behavior already existed and is documented. R1-F2.18 (triggers) is the accepted saved-file human-review item
        - docs/skills: `docs/lsp/indexing.md`, `docs/topics/schemas/{dmls-schema-support,definition}.md`, `simplified-schema/docs/topics/selecting-schema-files.md`, skills `darkmatter/{dmls,schema}.md`, `simplified-schema/SKILL.md`
        - verification: darkmatter `just test` filters `yaml_` 333, `session` 153, `registry` 244, `standalone` 71, `named_type` 93, `unsaved` 18, `shipped` 84, `discovery` 208, `trigger` 592, `guard` 266 all passed; simplified-schema `just test resolve`, `buffers::`, `guard` passed; `just lint` clean in both areas
- work completed for 'R1-F2 Unsaved schema sources do not consistently refresh YAML validation' at 23:14:07
- starting the work on 'R1-F4 YAML schema preparation still runs on the editor request thread' at 23:14:07
        - defect class: only registry discovery ran on the background worker; final contract preparation (fallback resolution, lowering, dependency checks) and validator compilation stayed synchronous in overlay construction
        - library: new public `prepare_yaml_contract` → `YamlContract` (passive `validate`, except the `schema`/`yaml-schema` kind checks, which read imports and run only in the worker) and `LoadedYamlDocument::declaration_key()` (hashes `kind` and `$schema`, never data); `validate_loaded_yaml_document` is now prepare-then-validate, so CLI and Messenger callers are unchanged
        - DMLS: new `overlay/yaml_contract.rs`; the existing registry job prepares the registry and then the contract (`RegistryNeed.yaml`), keyed on declaration, context identity/generation, and kind-registry version (plus full text only for schema-authoring documents); the request path only validates data
        - sibling R1-F4.7: an open YAML standalone schema's import resolution (`resolve_standalone`, `check_standalone_named_types`) also ran on the dispatch thread → moved to the worker job
        - sibling R1-F4.8: `SchemaReferenceFacts::select_in` (probing, target previews) ran on each rebuild → prepared with the contract
        - decision (human review): a new `YamlDocumentOutcome::Pending` while no contract exists for the current declaration, so an earlier declaration's contract is never used. The server waits before publishing, so `Pending` reaches only schema-independent requests. Rejected: reusing `Unvalidated`
        - decision (human review): contracts live in a sibling cache rather than the registry entry, so an inline `$schema` keystroke does not force a trigger/kind rescan; staleness is event-driven, and contract folders now join `trigger_watch_folders`, so DMLS may register extra watchers
        - thread evidence: test-only `work-counters` `Work::{YamlContractPreparation, YamlKindCheck, StandaloneImportCheck}` record their threads; `a_yaml_documents_contract_is_prepared_and_compiled_off_the_dispatch_thread` (claimed, inline, referenced, `kind: schema` with import) and `a_data_only_edit_reuses_the_prepared_contract_and_reads_no_schema_file` (zero preparations/compiles across data edits; `$schema` change re-prepares). Both failed with preparation forced back onto the request path, and passed once restored
        - docs/skills: `docs/lsp/indexing.md` (new "YAML document contracts" section), `docs/topics/schemas/{dmls-schema-support,definition}.md`, skills `darkmatter/{dmls,schema}.md` (removed the stale "dispatch-thread only" line)
        - verification (scoped): `yaml_` 337, `session` 155, `registry` 244, `unsaved` 18, `standalone` 71, `thread` 14, `schema_validate_yaml` 20, `kind_semantic_checks` 9 passed; Messenger `research` 44 passed; `just lint` clean
- work completed for 'R1-F4 YAML schema preparation still runs on the editor request thread' at 23:35:08
- phase verification (orchestrator, no subagents running) started at 23:13
        - first full darkmatter `just test` stopped on `hashing_guard::production_source_hashes_through_biscuit_hash`: the R1-F4 `declaration_key` used `DefaultHasher` → switched to `biscuit_hash::xx_hash` of the serialized declaration (R1-V1)
        - second run stopped on dmls `overlay::tests::local_references_record_no_external_dependency`: R1-F4 moved a YAML standalone schema's resolution into the contract cache, and the test still read the old cache → it now also reads the contract entry's dependency count through a `#[cfg(test)]` accessor and still asserts zero (R1-V2)
        - third full run: 13023 passed, 10 skipped
        - Windows cross-check: `simplified-schema` pass (1398), `dmls` pass (1708); `darkmatter` failed one test on a 272-character fixture path that `dunce` keeps in `\\?\` form (the cross-check consume directory is about 60 characters deeper than CI; the fixture is on `main`, and no change in this cycle touches dependency-path recording). First fix attempt: the test's `relative` strips a verbatim disk prefix → `darkmatter` pass (9749) (R1-V3); recorded in the `os` skill (`windows-paths.md` item 7)

### Completion Report

- **outcome: complete**, elapsed 1 h 34 min (22:23:58 → 23:58)
- parent findings: total 4 = fixed 3 (R1-F1, R1-F2, R1-F4) + already fixed 0 + blocked 0 + unresolved 0 + disputed 1 (R1-F3)
        - R1-F3 is disputed: its cited authority, "Schema-file lookup continues to use the shared selector and its existing policy," names a policy that gates bare-name root lookup on a repository (`md`) or workspace folder (DMLS) for frontmatter too, and the docs already say so. The YAML route matches that policy, and parity tests now pin it. A next review must cite authority that explicit lookup should ignore the gate, or drop the finding. An author question is in the review's `human_review_items`
- no finding or child row is open. R1-F2.14 (Markdown) and R1-F2.18 (triggers) are clean because they are outside this spec or covered by an accepted human-review item
- verified repairs:
        - R1-F1: informal standalone schemas publish their fallback validation, each problem once, and keep authoring assistance
        - R1-F2: a library `SourceBuffers` snapshot seam carries unsaved definitions, imports, explicit YAML/JSON targets, editor-only and missing-location files, and absent root folders into discovery and preparation, with refresh and restore on close
        - R1-F4: the registry worker prepares the YAML contract, compiled validators, file-reading kind checks, standalone import resolution, and reference facts; the request path only validates data, and a data-only edit compiles nothing
- human-review decisions recorded in the review: R1-F1 own-`types` informal schemas now show the CLI's error; R1-F2 thread-scoped snapshot seam; R1-F3 author question; R1-F4 `Pending` outcome and sibling contract cache with extra watch folders
- no performance measurement was deferred
- changed files:
        - simplified-schema: `lib/src/resolve.rs`, `lib/src/resolve/{buffers.rs (new),graph.rs,select.rs}`, `docs/topics/selecting-schema-files.md` (R1-F2)
        - darkmatter lib: `src/markdown/schemas/{mod.rs,resolve.rs}`, `src/markdown/schemas/yaml_documents/{mod.rs,registry.rs,semantic.rs}`, `src/markdown/schemas/triggers/discovery.rs`, `src/work_counters.rs` (R1-F2, R1-F4, R1-V1); tests `tests/l1/{yaml_document_selection,yaml_kind_registry,recursive_schema_golden}.rs` (R1-F2, R1-F3, R1-F4, R1-V3)
        - darkmatter cli: `tests/l1/schema_validate_yaml.rs` (R1-F1, R1-F3)
        - dmls: `src/overlay/{mod.rs,yaml_contract.rs (new),yaml_document.rs,registry.rs,named_types.rs,references.rs}`, `src/diagnostics/yaml_document.rs`, `src/{router.rs,registry_jobs.rs,work_counters.rs}` (R1-F1, R1-F2, R1-F4, R1-V2); tests `tests/l1/{yaml_document_session,yaml_document_editor,lsp_session,named_type_references}.rs`
        - docs: `darkmatter/docs/lsp/indexing.md`, `darkmatter/docs/topics/schemas/{definition,dmls-schema-support}.md`
        - skills: `.claude/skills/darkmatter/{cli,dmls,schema}.md`, `.claude/skills/simplified-schema/SKILL.md`, `.claude/skills/os/windows-paths.md`
- final verification:
        - `darkmatter`: `just test` 13023 passed, 10 skipped (all findings); `just lint` clean, including the zed-dmls wasm check; `just check-tier-coverage darkmatter`: 0 stranded
        - `simplified-schema`: `just test` 1398 + 1210 (minimal features) passed; `just lint` clean (R1-F2)
        - `messenger`: `just test` 783 passed, 2 skipped (consumer of `validate_yaml_document`, R1-F4)
        - Windows: `just cross-check {simplified-schema,dmls,darkmatter} --os windows` all pass (R1-F2 path keys, R1-V3)
- nothing committed; `cargo fmt` not run

## Implementation of Review Findings #2

> **started at:** 2026-11-01T00:08:38-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-30-yaml-document-schemas/review-2.md'
- this is iteration 2 of the review-to-implement cycle 
- the review's `implemented` flag was already `false`; it was left `false` before repairs began
- the review reports one parent finding (R2-F1, high) with 18 child rows, `recurrence: true`, no blocked findings, and no `human_in_the_loop` items

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R2-F1   | Secondary envelope checks override the selected YAML contract | fixed | all 18 required children verified, plus discovered R2-F1.19–F1.22; no open child |
| R2-F1.1 | DMLS informal-envelope admission, `description: null` under optional `string` → no error | fixed | `yaml_document_editor::metadata_named_data_the_selected_contract_accepts_is_clean` and protocol `yaml_document_session::metadata_named_fields_of_a_root_schema_document_are_judged_by_its_contract_alone`; both failed (`document_malformed`) with the fix disabled |
| R2-F1.2 | DMLS informal-envelope admission, `categories` under `unknown` (null, scalar, empty string/object) → no error | fixed | `yaml_document_editor::any_categories_shape_is_data_of_the_selected_contract` (absent + 8 shapes); failed before |
| R2-F1.3 | DMLS standalone category semantics, `categories: [ops, ops]` under `string[]` → no duplicate-category error | fixed | both accepts tests above (`[ops, ops]` under `string[]`); failed before |
| R2-F1.4 | DMLS informal-envelope admission, `types: [ops]` under `string[]` and the shape matrix → no envelope error | fixed | accepts tests plus `any_types_shape_is_data_of_the_selected_contract`; failed before |
| R2-F1.5 | DMLS standalone named-type resolution, `types: { mode: production }` under `unknown` → no unknown-type error | fixed | accepts tests (`types: { mode: production }` under `unknown`); failed before |
| R2-F1.6 | DMLS referenced fallback (`$schema: ./contract.yaml`), null description → no error | fixed | accepts tests, referenced `./contract.yaml` case; failed before |
| R2-F1.7 | DMLS root-union fallback, null description → no error | fixed | accepts tests, two-arm root-union case; failed before |
| R2-F1.8 | Library fallback validation, all matrix inputs → valid (clean control) | clean | `yaml_document_selection::metadata_named_fields_are_data_of_the_selected_contract` (whole matrix valid through the public library; passes before and after) |
| R2-F1.9 | CLI fallback publication, `description: ops` and null → exit 0 (clean control) | clean | `schema_validate_yaml::a_metadata_named_field_is_data_of_the_root_schema`: `ops` and `null` both exit 0 |
| R2-F1.10 | DMLS ordinary-data control: declared `owner` beside null description → no error | clean | accepts tests include declared `owner` beside null description; `metadata_named_data_the_selected_contract_rejects_is_its_problem_alone` gives exactly one `type_mismatch` for data the contract rejects |
| R2-F1.11 | Unmodified shipped `kind: schema` with null description → rejected (control) | clean | `kind_semantic_checks::the_shipped_schema_definition_rejects_a_null_description_and_an_unknown_key`: `TypeMismatch` and `UnknownKey`, no `KindCheck` |
| R2-F1.12 | Library `schema` kind semantic check under an opened source-copy definition, extra `note` → follows selected definition | fixed | `kind_semantic_checks::an_opened_schema_definition_admits_an_added_property_and_keeps_its_checks`: failed before on `note`; undeclared category still `KindCheck` |
| R2-F1.13 | Library `yaml-schema` kind semantic check, same edit → follows selected definition, export claims still verified | fixed | `kind_semantic_checks::an_opened_yaml_schema_definition_admits_an_added_property_and_keeps_its_checks`: failed before; unresolvable import still `KindCheck` |
| R2-F1.14 | Library `schema-trigger` kind semantic check, same edit → follows selected definition, match semantics kept | fixed | `kind_semantic_checks::an_opened_trigger_definition_admits_an_added_property_and_keeps_its_checks`: failed before (hard-coded key list); invalid `$path` glob still `KindCheck` |
| R2-F1.15 | CLI selected-definition report, three controls + added property → pass | fixed | `schema_validate_yaml::an_opened_checkout_{schema,yaml_schema,trigger}_definition_admits_an_added_property`: exit 0 both steps; all three failed with old calls restored |
| R2-F1.16 | DMLS selected-definition report, three controls + added property → no schema error | fixed | `yaml_document_editor::an_opened_checkout_definition_alone_judges_its_documents_shape` (schema, yaml-schema, trigger in `docs/` and `darkmatter/schemas/`) and protocol `yaml_document_session::an_opened_checkout_schema_definition_admits_an_added_key_in_the_editor`; `note` gave `document_malformed` before |
| R2-F1.17 | Declaration diagnostics and assistance in fallback `$schema` → preserved, each once | clean | `an_invalid_declaration_of_a_root_schema_document_is_published_once`, `an_informal_standalone_schema_is_judged_as_data_and_each_problem_is_published_once`, `deleting_the_last_data_property_of_a_root_schema_document_reports_the_missing_property` pass with assertions unchanged; new `a_fallback_schema_names_no_sibling_types` |
| R2-F1.18 | Direct library envelope admission (factories/imports) → compiled definitions stay authoritative | clean | `kind_semantic_checks::an_imported_file_is_still_admitted_against_the_compiled_definition`; `envelope_admission::a_selected_document_is_read_without_admission_and_keeps_its_semantic_checks` |
| R2-F1.19 | (discovered) registry candidate preparation admits discovered `yaml-schema` files against the compiled `yaml-schema.yaml`, even under an opened checkout copy | clean (decision) | ruling 2 step 4 names the shipped definition for candidates; a candidate load is a definition load, which ruling 8 keeps on the compiled copy; recorded for human review |
| R2-F1.20 | (discovered) untagged YAML: Markdown link hover answered inside the `$schema` contract | fixed | hover defers through `DocumentOverlay::yaml_contract_covers`; covered by the protocol tests' hover steps |
| R2-F1.21 | (discovered) route shape errors ranged on the whole `$schema` value | fixed | `located_inline_failure` places them on the offending arm/token (Markdown too); `an_invalid_declaration_of_a_root_schema_document_is_published_once` (1:18) |
| R2-F1.22 | (discovered) `standalone_payload_key` chose `types` for a `kind: schema` file exporting `$schema` | fixed | now prefers `$schema`; `lsp_session` standalone tests |

### Progress

- starting the work on 'R2-F1 Secondary envelope checks override the selected YAML contract' at 00:10:30
        - defect class: a secondary envelope validator or classifier applies a different structural or semantic contract after the document has selected its authoritative schema
        - the finding has two independent halves, repaired by two serial subagents: (A) built-in kind semantic checks in the library (R2-F1.11–F1.15, F1.18), then (B) the DMLS informal-envelope classifier and editor projection (R2-F1.1–F1.10, F1.16, F1.17), which builds on (A)
        - subagent A (library kind checks), completed 00:24:
                - R2-F1.12–F1.14 fixed: whole-file kind checks no longer re-admit against the compiled `schema.yaml`/`yaml-schema.yaml` or the trigger parser's hard-coded key list; they read the selected definition's accepted copy through the new `simplified_schema::parse_selected_standalone_schema_document`, `resolve::prepare_standalone_yaml_schema`, and Darkmatter's `check_trigger_semantics`
                - tests `kind_semantic_checks::an_opened_{schema,yaml_schema,trigger}_definition_admits_an_added_property_and_keeps_its_checks`: failed before (the added `note` became a `KindCheck`), pass after; under the open copy an undeclared category, an unresolvable `yaml-schema` import, and an invalid `$path` glob are still `KindCheck`
                - R2-F1.15 fixed: `schema_validate_yaml::an_opened_checkout_{schema,yaml_schema,trigger}_definition_admits_an_added_property` exit 0 with and without `note`; all three failed with the old calls restored
                - R2-F1.11 pinned: `kind_semantic_checks::the_shipped_schema_definition_rejects_a_null_description_and_an_unknown_key`
                - R2-F1.18 pinned: `kind_semantic_checks::an_imported_file_is_still_admitted_against_the_compiled_definition` and `envelope_admission::a_selected_document_is_read_without_admission_and_keeps_its_semantic_checks`
                - kind checks read the softly converted copy (new `EffectiveSchema::data_copy`, ruling 4); guarded by `the_schema_checks_read_the_softly_converted_categories`
                - sibling sites: CLI `md schema validate` and Messenger `validate_yaml` reach the fixed report through `validate_loaded_yaml_document` (clean); Markdown trigger discovery keeps the strict parser, unknown keys included (clean, public loading behavior); DMLS sites belong to half B
                - decision (human review): registry candidate preparation still admits a discovered `yaml-schema` against the compiled `yaml-schema.yaml` even when a checkout copy of it is selected (ruling 2 step 4 names the shipped definition; this is a definition load like a direct load, ruling 8); rejected: admitting candidates against the checkout copy, which would make one context's registry depend on another definition's edit (row R2-F1.19)
                - decisions (human review): metadata whose shape the library cannot read (non-mapping `types`, non-string `categories`) is still a `KindCheck`, because silently ignoring it would turn off the undeclared-category check; an absent trigger `match` gives no finding (the definition owns required-ness); trigger `match` values stay raw, as discovery reads them
                - docs/skills: `docs/topics/schemas/{definition,schema-triggers}.md`, `simplified-schema/docs/topics/language/schema-documents.md`, skills `darkmatter/schema.md`, `simplified-schema/SKILL.md`
                - verification (scoped): darkmatter `yaml_` 339, `kind_semantic` 16, `trigger` 594, `shipped_schema_corpus` 12, `schema_validate_yaml` 23, `standalone` 71, `guard` 266; simplified-schema `envelope` 41, `resolve` 165 (minimal leg 1212); `just lint` clean in both
        - subagent B (DMLS classifier and editor projection), completed 00:54:
                - R2-F1.1–F1.7 fixed: an open YAML file is a schema document only by its claimed `kind` (`schema`, `yaml-schema`, or a `schema-trigger` in a `schemas/` folder); an untagged root `$schema` is the contract, and its sibling `description`/`categories`/`types` are data, as in Markdown frontmatter (`overlay/mod.rs::envelope_overlay`, `overlay/yaml_contract.rs`, `diagnostics/{yaml_document,frontmatter}.rs`)
                - the review's matrix (description/categories/types × absent, null, `123`, `[ops, 123]`, `[123]`, `""`, `[]`, `{}`, `ops`) is covered by `yaml_document_editor::any_{description,categories,types}_shape_is_data_of_the_selected_contract`; all failed with the fix disabled
                - R2-F1.16 fixed: claimed schema documents are read with `parse_selected_standalone_schema_document` over the accepted copy (`EffectiveSchema::data_copy`, now `pub`), and a trigger's strict-parser error is no longer published once a definition has judged the file
                - R2-F1.17 preserved: declaration errors are the route's one located prepare error; assistance inside `$schema` kept (local names, `Name@file` hover/definition through a worker-built import catalog, `suggest(...)` lints)
                - discovered and fixed: R2-F1.20 (link hover inside the `$schema` contract), R2-F1.21 (route shape errors now located on the offending arm/token, Markdown too), R2-F1.22 (`standalone_payload_key` prefers `$schema`); a drifted doc comment at `EffectiveSchema::conversion` was moved back onto its function
                - sibling sites clean: `overlay/registry.rs::classify` and target previews load schema files (compiled admission, R2-F1.18); code actions read the YAML tree; semantic tokens do not use the classification; Markdown/non-YAML buffers keep the content-based classifier (`standalone_activation_is_content_based_not_path_based`)
                - decision (human review): a sibling `types` in an untagged YAML file is not a type library, matching the library/CLI report; completion/hover/goto inside `$schema` no longer reach into it; rejected: keeping `types` navigation without its diagnostics, which advertises types the validator rejects. Parity: `claudine/docs/research/model-config/_schema.yaml` exit 2 (unknown named type `model_config`) and `reasoning-level/_schema.yaml` exit 2 (non-descending cycle); DMLS shows exactly one matching error each
                - decision (human review): an opened checkout trigger with an extra key shows no discovery-skip warning while open (strict Markdown discovery still skips the saved file); untagged YAML that fails to load reports `dm.frontmatter.yaml_parse` and keeps no last-good declaration model
                - observation: a whitespace `$schema` reference now reports the route's `dm.schema.prepare`, matching the CLI; both surfaces reject it (clean)
                - tests whose subject is standalone authoring were moved to `kind: schema` fixtures with assertions kept (`lsp_session` meta-schema/standalone tests, `named_type_references` tests, three `overlay::tests`); two untagged duplicates removed; `meta_schema_phase6_shipped_schema_activation_and_current_error` now expects one `dm.frontmatter.yaml_parse`
                - docs/skills: `docs/topics/schemas/dmls-schema-support.md`, `docs/lsp/indexing.md`, skills `darkmatter/{dmls,schema}.md`
                - verification (scoped): `yaml_` 349, `session` 157, `standalone` 71, `named_type` 93, `lsp_session` 111, `informal` 5, `schema_validate_yaml` 24, `kind_semantic` 16, `guard` 266; `cargo nextest run -p dmls` 1717 passed; `just lint` clean
        - orchestrator reconciliation: reviewed the combined diff (`overlay/mod.rs` selection rule, `standalone.rs` `Admission::{Required,Selected,Skip}`); every ledger row checked against its named test
- work completed for 'R2-F1 Secondary envelope checks override the selected YAML contract' at 00:54:10
- phase verification (orchestrator, no subagents running) started at 00:55
        - darkmatter `just test`: 13043 passed, 10 skipped; `just lint` clean (including zed-dmls wasm); `just check-tier-coverage darkmatter`: 0 stranded
        - simplified-schema `just test`: 1400 + 1212 (minimal features) passed; `just lint` clean
        - messenger `just test`: 783 passed, 2 skipped (consumer of `validate_loaded_yaml_document`)
        - Windows cross-check (`just cross-check {darkmatter,dmls} --os windows`) could not start: `scripts/ci/affected_scope.py` refuses the plan because `biscuit-clipboard-cli`'s `[package.metadata.ci] expiry` lapsed on 2026-10-31; unrelated to this change, and moving it needs the author's fresh justification, so it was not edited. Cross-OS proof is left to CI's nightly. No path-handling production code changed in this cycle; the new fixtures use temp directories

### Completion Report

- **outcome: complete**, elapsed 50 min (00:08:38 → 00:59)
- parent findings: total 1 = fixed 1 (R2-F1) + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- child rows: 22 (18 from the review + 4 discovered); fixed 14 (F1.1–F1.7, F1.12–F1.16, F1.20–F1.22), clean 8 (F1.8–F1.11, F1.17–F1.19); none open
- verified repairs:
        - whole-file kind checks for `schema`, `yaml-schema`, and `schema-trigger` follow the selected (possibly checkout-copy) definition and keep only semantic checks; direct library loads keep compiled admission
        - DMLS treats an untagged root-`$schema` YAML file's other fields as data judged by that contract alone, and reads claimed schema documents from the selected definition's accepted copy
- human-review decisions recorded in the review: candidate admission stays compiled (F1.19); unreadable metadata shapes remain `KindCheck`; absent trigger `match` gives no finding; trigger values read raw; sibling `types` is not a type library in untagged YAML; no discovery-skip warning for an open checkout trigger; untagged unloadable YAML uses `yaml_parse`
- no performance measurement was deferred
- changed files:
        - simplified-schema: `lib/src/{lib.rs,resolve.rs,envelope.rs}`, `lib/src/simplified/{mod.rs,standalone.rs}`, `lib/tests/l1/envelope_admission.rs`, `docs/topics/language/schema-documents.md` (F1.12–F1.14, F1.18)
        - darkmatter lib: `src/markdown/schemas/{mod.rs,yaml_documents/mod.rs,yaml_documents/semantic.rs,triggers/envelope.rs}`, tests `tests/l1/{kind_semantic_checks,yaml_document_selection}.rs` (F1.8, F1.11–F1.14, F1.18)
        - darkmatter cli: `tests/l1/schema_validate_yaml.rs` (F1.9, F1.15)
        - dmls: `src/overlay/{mod.rs,yaml_contract.rs,yaml_document.rs,suggestions.rs,named_types.rs,schema.rs}`, `src/diagnostics/{yaml_document.rs,frontmatter.rs}`, `src/providers/mod.rs`, tests `tests/l1/{yaml_document_editor,yaml_document_session,lsp_session,named_type_references}.rs` (F1.1–F1.7, F1.10, F1.16, F1.17, F1.20–F1.22)
        - docs: `darkmatter/docs/topics/schemas/{definition,schema-triggers,dmls-schema-support}.md`, `darkmatter/docs/lsp/indexing.md`
        - skills: `.claude/skills/darkmatter/{schema,dmls}.md`, `.claude/skills/simplified-schema/SKILL.md`
- final verification: darkmatter `just test` 13043 passed / `just lint` clean / tier coverage 0 stranded (all rows); simplified-schema `just test` 1400 + 1212 / `just lint` clean (F1.12–F1.14, F1.18); messenger `just test` 783 passed (consumer); Windows cross-check not runnable (unrelated expired CI metadata), left to CI
- nothing committed; `cargo fmt` not run

## Implementation of Review Findings #3

> **started at:** 2026-11-01T01:04:36-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-30-yaml-document-schemas/review-3.md'
- this is iteration 3 of the review-to-implement cycle 
- the review's `implemented` flag was already `false`; it was left `false` before repairs began
- defect class (R3-F1): a secondary envelope classifier treats the absence of a successful whole-document validation result as permission to apply its own strict rules to YAML that never claimed its envelope

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R3-F1   | Unclaimed YAML in schema folders receives false trigger errors | fixed | every required child verified; no discovered defect open |
| R3-F1.1 | Darkmatter whole-YAML selector: null/non-string/empty/absent kind → NoSchema, selected contract, or fallback's own error | clean | `yaml_document_selection::a_kind_that_is_not_a_string_claims_nothing` widened to 11 shapes × inside/outside a schema root × no/valid/missing fallback; `an_unclaimed_kind_falls_back_to_the_root_schema_or_to_no_schema` covers the unclaimed string |
| R3-F1.2 | `md schema validate`: `kind:` in `schemas/` → exit 0/no schema; with missing fallback → exit 2, one reference error | clean | new `schema_validate_yaml::a_null_kind_in_a_schema_root_claims_nothing`: exit 0, `schema: null`, `problems: []`; then exit 2, one `schema` error naming `missing.yaml`, no `kind` mention |
| R3-F1.3 | DMLS classification/overlay, `NoSchema`: null (both spellings), number, boolean, four list shapes, two mapping shapes in `schemas/` → no trigger error | fixed | `yaml_document_editor::an_unclaimed_kind_without_a_root_schema_is_not_validated_inside_or_outside_a_schema_folder`: 13 shapes, outcome `no-schema`, zero diagnostics; fails (whole-document `dm.schema.prepare`) with the gate disabled |
| R3-F1.4 | DMLS classification/overlay, failed fallback: same shapes + `$schema: ./missing.yaml` → only the fallback's preparation error | fixed | `yaml_document_editor::an_unclaimed_kind_with_a_missing_root_schema_reports_only_the_reference_error`: outcome `failed`, exactly one `dm.schema.prepare` starting at `./missing.yaml`; fails with the gate disabled |
| R3-F1.5 | DMLS with selected fallback `{owner: string}`: all thirteen kind shapes → contract alone judges | clean | `yaml_document_editor::an_unclaimed_kind_with_an_inline_root_schema_is_judged_by_it_alone`: `validated`, no diagnostics (passes with or without the gate; kept as a guard) |
| R3-F1.6 | DMLS location/control: shapes outside `schemas/`; absent/empty/ordinary-string kinds inside → unchanged | clean | the three provider tests above run every shape in both `schemas/data.yaml` and `docs/data.yaml` with the same expectations |
| R3-F1.7 | DMLS diagnostic publication over normal protocol: no-schema → failed fallback → valid fallback → no false error | fixed | `yaml_document_session::an_unfinished_kind_in_a_schema_root_publishes_only_whole_document_results` (didOpen/didChange): clean → clean → one `dm.schema.prepare` at 1:9 → clean; fails with the gate disabled |
| R3-F1.8 | Genuine claimed-trigger checks and strict Markdown discovery parser preserved | clean | `just test trigger` 595/595 (invalid-glob, vacuous, count-bound, renamed-kind, open-trigger-stays-strict tests); `a_kind_check_is_published_once_on_the_node_it_judges`, `an_opened_checkout_definition_alone_judges_its_documents_shape`; strict parser tests unchanged |
| R3-F1.9 | Nearby comment promising ordinary schema-root YAML stays silent matches the code | fixed | comments in `overlay/mod.rs::envelope_overlay` and `diagnostics/frontmatter.rs`; `docs/topics/schemas/dmls-schema-support.md`; `.claude/skills/darkmatter/dmls.md` |
| R3-F1.10 | (discovered probe) unloadable YAML in `schemas/` lexically claiming the trigger: parse problem published once | clean | new `yaml_document_editor::an_unloadable_file_in_a_schema_folder_reports_its_parse_problem_once`: claimed → one `dm.schema.prepare`; unclaimed `kind:`/`kind: 42` → one `dm.frontmatter.yaml_parse` (existing `envelope_error()` dedupe) |
| R3-F1.11 | (sweep) other classifiers acting on a null/non-string `kind` in whole YAML | clean | `schema::standalone_envelope_claim` claims only string scalars; `trigger_boundary`/`lookup_kinds` select registries only; other `TriggerError`/`TriggerWarning` readers consume states only `envelope_overlay` creates; `registry.rs::classify` mirrors saved-file discovery, which the review requires unchanged |
- starting the work on 'R3-F1 Unclaimed YAML in schema folders receives false trigger errors' at 01:05:39
        - subagent (rust-developer), completed 01:13:
                - root cause: DMLS's `envelope_overlay` ran the strict trigger parser on every YAML file directly inside a `schemas/` folder lacking a validated report, so the parser's deliberate null/non-string `kind` error (meant for Markdown trigger discovery) was published beside `NoSchema` or the fallback's own error
                - repair: new public predicate `darkmatter::markdown::schemas::claims_trigger_envelope` (top-level `kind` is the string `schema-trigger` or `trigger-schema`, or a lexical declaration line for unloadable text); DMLS enters the trigger branch only when it holds. The strict parser and saved-file discovery are unchanged (BOM/CRLF normalization moved into a shared private helper)
                - tests: 13-shape provider matrix × inside/outside `schemas/` × no/missing/inline fallback; one normal-protocol regression; CLI test; widened library selection test; parse-once probe; unit test of the predicate. The R3-F1.3/.4/.7 tests fail with the gate disabled
                - discovered: a renamed-spelling literal trips `trigger_spelling_guard`; the renamed case is covered by the library unit test and the existing workspace test instead
                - decision (human review): a saved `schemas/data.yaml` with `kind:` is clean in the editor, but Markdown trigger discovery still treats it as a broken trigger (skip warning on matching Markdown documents, counted as failed at startup), as the review required ("keep saved-file Markdown discovery behavior unchanged"); alternative: make discovery treat a null/non-string `kind` as unclaimed too, which would hide a trigger whose `kind` value was dropped. The doc page states the difference
                - scoped verification: `yaml_document` 84, `trigger` 595, `schema_validate_yaml` 25, `lsp_session` 111, `envelope` 94 passed; `just lint` clean
        - orchestrator reconciliation: reviewed the gate (`overlay/mod.rs:673`) and the predicate in `triggers/envelope.rs`; each row checked against its named test
- work completed for 'R3-F1 Unclaimed YAML in schema folders receives false trigger errors' at 01:16:35
- phase verification (orchestrator, no subagents running)
        - darkmatter `just test`: 13050 passed, 10 skipped; `just lint` exit 0 (including zed-dmls wasm); `just check-tier-coverage darkmatter`: 0 stranded
        - messenger `just test`: 783 passed, 2 skipped (consumer of the whole-YAML API)
        - no Windows cross-check: the change adds no path handling (a YAML-value predicate and an editor gate); tests use temp-dir fixtures. Cross-OS proof is left to CI

### Completion Report

- **outcome: complete**, elapsed about 12 min (01:04:36 → 01:16)
- parent findings: total 1 = fixed 1 (R3-F1) + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- child rows: 11 (9 required + 2 discovered); fixed 4 (F1.3, F1.4, F1.7, F1.9), clean 7 (F1.1, F1.2, F1.5, F1.6, F1.8, F1.10, F1.11); none open
- verified repair: DMLS applies the strict trigger checks only to a YAML document whose `kind` string names the trigger envelope; a null or non-string `kind` keeps the whole-document result (`NoSchema`, the fallback's own error, or the selected contract) alone, matching `md schema validate`
- human-review decision: saved-file Markdown trigger discovery still reads a null/non-string `kind` in `schemas/` as a broken trigger (review-mandated); the editor no longer does
- no performance measurement was deferred
- changed files:
        - darkmatter lib: `src/markdown/schemas/triggers/{envelope.rs,mod.rs}`, `src/markdown/schemas/mod.rs`, `tests/l1/yaml_document_selection.rs` (F1.1, F1.8)
        - darkmatter cli: `tests/l1/schema_validate_yaml.rs` (F1.2)
        - dmls: `src/overlay/mod.rs`, `src/diagnostics/frontmatter.rs`, `tests/l1/{yaml_document_editor,yaml_document_session}.rs` (F1.3–F1.7, F1.9, F1.10)
        - docs/skills: `darkmatter/docs/topics/schemas/dmls-schema-support.md`, `.claude/skills/darkmatter/dmls.md` (F1.9)
- final verification: darkmatter `just test` 13050 passed / `just lint` clean / tier coverage 0 stranded (all rows); messenger `just test` 783 passed
- nothing committed; `cargo fmt` not run
