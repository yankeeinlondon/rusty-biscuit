---
feature: 2026-10-14-schematic-definition-api
implementation_2: "2026-10-21T15:41:29-07:00"
---

# Schematic Definition API — Review Implementation Log

## Implementation of Review Findings #2

> **started at:** 2026-10-21T15:41:29-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-14-schematic-definition-api/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- iteration 1's repair was logged in `implementation-log.md` under `## Implementation of Review Findings #1`; this iteration logs here, as instructed
- the review's `implemented` flag was already `false` at start; confirmed before any repair
- the review is `recurrence: true`; its `## Recurrence` list (R1-H1.3a–R1-H1.3h) is the minimum scope for R1-H1
- inventory: 2 parent findings (R1-H1, R2-F1), no blocked findings, no observations, no nonblocking follow-ups

### Finding Ledger

| ID         | Finding or required instance/consumer and expected result                                                                       | Disposition | Verification evidence or blocker |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------- | ----------- | -------------------------------- |
| R1-H1 | Nested schema conflicts bypass module-group collision checks                                                                    | fixed | all child rows fixed or clean; `flatten` returns `Result` and rejects differing wire shapes for one name (`ComponentConflict` in `Collisions`) |
| R1-H1.3a | `components::nested`, nested vs nested: incompatible same-named child schemas from two parents must be rejected                 | fixed | `builders::conflicting_nested_and_explicit_definitions_of_one_name_fail_build` (nested/nested via `components()` and `RestApi` build) |
| R1-H1.3b | `components::flatten`, nested vs explicit: explicit registration must not silently overwrite an incompatible extracted schema    | fixed | same test, nested then explicit `right::Status`; error names `Status` and both sources |
| R1-H1.3c | `components::nested`, explicit vs nested: incompatible extracted schema must not be silently discarded                          | fixed | same test, explicit `left::Status` then nested |
| R1-H1.3d | `DefinitionContext::leaf`/`finish` clean control: two explicit same-named types still rejected and collected                    | clean | existing `typed_registration` collision tests and `api_build_reports_every_registration_collision` pass unchanged; `leaf`/`finish` untouched |
| R1-H1.3e | `RestApiBuilder::build` / `validate_references`: conflict fails before either definition is lost                                | fixed | `RestApiBuilder::build` propagates `ctx.components()?`; 3a–3c test asserts the build error |
| R1-H1.3f | `assemble_module_group`: same conflict error propagates to group assembly                                                       | fixed | `registry::typed::a_nested_component_conflict_registered_after_a_member_build_fails_assembly` |
| R1-H1.3g | OpenAPI export: conflicting group never reaches a successful export                                                             | fixed | export needs a built API, which 3e prevents; sibling `gen/src/export/openapi.rs::merge_documents` (grouped export) also compared now: `grouped_members_with_different_same_named_components_fail_and_equal_ones_collapse` |
| R1-H1.3h | Compatible repeated extraction and registry ordering preserved (clean control)                                                  | clean | `the_same_type_extracted_repeatedly_or_also_registered_merges_in_first_position`, `a_recursive_type_nested_elsewhere_matches_its_copy_under_its_own_root`, `api_carries_the_whole_context_with_nested_definitions_flattened`, registry order tests, artifact drift and 18 definition baselines pass; mutation checks show both normalizations are load-bearing |
| R2-F1 | Public OpenAPI imports return definitions with unresolved references                                                            | fixed | all child rows fixed or clean; `OpenApiImport::build` attaches one component per catalog model and runs `validate_references` |
| R2-F1.1 | YAML text source → public import result carries resolvable components; `validate_references` passes                          | fixed | `openapi_tests::every_source_form_imports_petstore_with_resolvable_exportable_components` (YAML row) |
| R2-F1.2 | JSON text source → same                                                                                                         | fixed | same table-driven test (JSON row) |
| R2-F1.3 | File path source → same                                                                                                         | fixed | same test (path row) |
| R2-F1.4 | Byte source → same                                                                                                              | fixed | same test (bytes row) |
| R2-F1.5 | Parsed-document source → same                                                                                                   | fixed | same test (parsed-document row) |
| R2-F1.6 | JSON response `$ref` to a missing component → import rejected naming API, endpoint, and type                                   | fixed | `openapi_tests::a_dangling_create_reference_fails_the_import_naming_api_endpoint_and_type` (response) expects `OpenApiError::Definition(UnresolvedReference { api: "Petstore", endpoint: "CreatePet", name: "MissingPet" })` |
| R2-F1.7 | JSON request `$ref` to a missing component → same rejection                                                                     | fixed | same test (request) |
| R2-F1.8 | Form-body (multipart/URL-encoded) reference sites — clean for this class                                                        | clean | `map_form_field` in `mappings/responses.rs` builds only text/file/files fields; the importer never emits `FormFieldKind::Json` |
| R2-F1.9 | Standalone CLI generation (`run_import_from_petstore_yaml`) keeps working                                                       | fixed | `run_import_from_petstore_yaml` passes; compensation narrowed to `with_standalone_rust_paths`, covered by `a_standalone_client_spells_module_qualified_components_by_bare_name` |
| R2-F1.10 | Emitted definition assembly / compiled Petstore builder and OpenAI re-import keep working                                       | clean | `slow_imported_builders_compile_and_match_public_import` and the OpenAI re-import byte-for-byte comparison pass |
| R2-F1.11 | Public `validate_references` and OpenAPI export accept the import from all five source forms                                    | fixed | source-form test asserts `validate_references` and `openapi::export` for all five forms |
| R2-F1.12 | Component-free import (inline JSON string response) still succeeds through validation and export                              | clean | `openapi_tests::a_component_free_import_still_validates_and_exports` |
| R2-F1.13 | `gen/tests/l1/import_overlay.rs` equivalence test no longer clears the emitted API's schemas before comparison                  | fixed | `.clear()` removed from `import_overlay.rs`; compares both exact component orders, the (name, Rust path) sets, and the rest of the `RestApi`; schema bodies differ by design (source OpenAPI vs schemars output), recorded as a departure |

### Progress

- starting the work on 'R1-H1: nested schema conflicts bypass module-group collision checks' at 15:42:06
        - defect class: when two schema definitions are merged under one component name, the merge keeps one and drops the other without comparing them, so incompatible wire shapes silently share a name
        - delegated to one foreground subagent (serial orchestration); its log items are recorded below
        - root vs nested schema for the same Rust type (scratch probe, removed): the explicit root carries `$schema`, an auto-added top-level `title`, and the schemars OpenAPI 3.0 transforms (e.g. `nullable: true` instead of `type: [X, "null"]`); schemars applies the transforms only to the root, not to the definitions it places under `components/schemas`
        - a recursive type's nested copies under its own root reach it as `"$ref": "#"` (shipped case: Anthropic `ToolResultContent` nested in `CreateMessageBody` vs nested in `ContentBlock`); first naive compare reported a false conflict there, so `#` is resolved to `#/components/schemas/{root}` in the comparison key
        - fix `define/src/components.rs`: `flatten` returns `Result`; every same-name arrival (nested/nested, nested/explicit, explicit/nested) compares a normalized wire shape (top-level `title` dropped both sides; openapi3 transforms applied to nested copies; root `#` refs resolved) and records `DefinitionError::ComponentConflict { schema_name, first, second }`, returned as `Collisions`; compatible arrivals keep the old ordering (explicit takes over a nested entry in place, first position wins)
        - new variant `DefinitionError::ComponentConflict` (owned source strings) in `define/src/registration.rs`; `DefinitionContext::components` propagates it; docs on `flatten`, `components()` updated
        - mutation checks: disabling the `#` resolution fails `a_recursive_type_nested_elsewhere_matches_its_copy_under_its_own_root`; disabling the nested transform fails `the_same_type_extracted_repeatedly_or_also_registered_merges_in_first_position`
        - sweep: `gen/src/export/openapi.rs::merge_documents` overwrote same-named `components.schemas` across grouped members without comparing (same defect class, public `build_openapi_grouped`) — fixed: different schemas fail with `invalid-schema-reference` naming both APIs; equal collapse
        - sweep clean: `definitions/src/registry/typed.rs` (uses `components()`), `registry/mod.rs` (read only), `define/src/openapi/import/mappings/schema.rs` (deconflicts names, never merges), `gen/src/import_pipeline.rs::with_model_components` (one per model), `define/src/openapi/export/components.rs` (projects one API's unique schemas), Postman (reads no schemas), import query twin merges (already schema-gated)
        - tests added: `define/tests/builders.rs` `conflicting_nested_and_explicit_definitions_of_one_name_fail_build` (3a/3b/3c via `components()` and `RestApi` build), `the_same_type_extracted_repeatedly_or_also_registered_merges_in_first_position`, `a_recursive_type_nested_elsewhere_matches_its_copy_under_its_own_root`; `definitions/src/registry/typed.rs` `a_nested_component_conflict_registered_after_a_member_build_fails_assembly`; `gen/src/export/openapi.rs` `grouped_members_with_different_same_named_components_fail_and_equal_ones_collapse`
        - docs: `docs/typed-registration.md`, `docs/io/export-openapi.md`, `docs/generation/validation.md`, `.claude/skills/schematic-define/SKILL.md`
        - observation (not fixed, out of scope): stored nested-only components keep untransformed schemars output (`type: [X, "null"]`, and a bare `"$ref": "#"` would be stored if a recursive type's copy under its own root were seen first); no shipped artifact currently contains a bare `#` ref
        - `cargo nextest run -p schematic-define --all-features -E 'binary(typed_registration) | binary(builders) | binary(authoring_guidance)'` 30/30 pass; full `-p schematic-define --all-features` 653/653
        - `cargo nextest run -p schematic-definitions -E 'test(registry)'` 18/18; full `-p schematic-definitions` 567/567
        - `cargo nextest run -p schematic-gen -E 'test(artifact_drift) | test(definition_baseline) | test(export::openapi)'` 64/64; `-E 'test(openapi_strict_completeness) | test(registry) | test(shipped)'` 12/12
        - `just lint` (schematic/) exit 0 after replacing a clippy `type_complexity` tuple type with a `fn` pointer alias
        - orchestrator check: reviewed the `components.rs`/`registration.rs` diff; the comparison stays bounded to wire shape, with no descriptor or Rust-path evidence; shipped artifacts under `schematic/openapi/` contain no `[X, "null"]` type arrays, so the untransformed-nested-storage observation is latent and does not affect either finding
- work completed for 'R1-H1: nested schema conflicts bypass module-group collision checks' at 15:53:48
- starting the work on 'R2-F1: public OpenAPI imports return definitions with unresolved references' at 15:53:48
        - defect class: a public construction path returns a successful REST definition without assembling or validating the components its references need
        - delegated to one foreground subagent (serial orchestration); its log items are recorded below
        - reproduced with new `define/tests/openapi_tests.rs` tests before any fix: Petstore import from YAML yields 4 endpoints and `schemas: []`; dangling `MissingPet` request/response `$ref`s import successfully (3 new tests fail on the unfixed importer)
        - source forms (`define/src/openapi/source.rs`): `Path`, `Json`, `Yaml`, `Bytes`, `Document`; `parse_source` has four text branches (Path/Yaml/Bytes via YAML loader, Json via JSON loader) and a parsed-document branch
        - R2-F1.8 clean: `map_form_field` (`define/src/openapi/import/mappings/responses.rs`) only builds `FormField::text`/`file`/`files`; no import path emits `FormFieldKind::Json`, so form bodies add no named reference
        - endpoint references are only `TypeRef::Named(sanitize_rust_ident(..))` or `Primitive(Json)` (`reference_from_schema`); only request names were remapped through `name_mapping`, so a response naming a renamed schema would have dangled
        - fix `define/src/openapi/import/mappings/schema.rs`: `map_all_schemas` now also returns `components`, one `SchemaComponent` per catalog model in source order: final model name, Rust path `{module_path}::{Name}` (bare name without a module path, matching the catalog's `module_path` contract), and the source schema with `#/components/schemas/{original}` refs renamed to final model names (so `CreatePetRequest` → `CreatePetRequest2` stays consistent inside other components)
        - fix `define/src/openapi/import/builder.rs`: `build` renames JSON response and request references through `name_mapping` (previously request only, so a response naming a renamed schema dangled), attaches the components, and calls `RestApi::validate_references()`; failure returns new `OpenApiError::Definition(#[from] DefinitionError)` (`define/src/openapi/error.rs`), e.g. `Invalid imported definition: Petstore endpoint CreatePet: unresolved schema reference "MissingPet"`; `DefinitionError` now also derives `Eq` (needed because `OpenApiError` is `Eq`)
        - `with_model_components` replaced by `with_standalone_rust_paths` (`gen/src/import_pipeline.rs`): the import now supplies components and real source schemas, so only the Rust-path spelling is overridden (bare names, because the standalone API module glob-imports `crate::types::*`); needed whenever `--module-path` is given; pinned by new unit test `a_standalone_client_spells_module_qualified_components_by_bare_name`
        - decision on R2-F1.13 ordering: the public import keeps source order (`Pet`, `CreatePetRequest2`, `PetStatus`); assembly flattens nested definitions right after their parent (`Pet`, `PetStatus`, `CreatePetRequest2`). Assembly's nested order comes from schemars' `$defs` map order (alphabetical without serde_json `preserve_order`, insertion order with it, a feature of the consuming build), so the importer does not imitate it. `slow_imported_builders_compile_and_match_public_import` now asserts both exact lists, equal (name, path) sets with assembled path = `::schematic_definitions::` + imported path, then the whole remaining `RestApi` equal; schemas differ by source (OpenAPI source vs schemars output) and are not compared. The `api.schemas.clear()` is gone
        - test fixture fix: `define/tests/import_normalization.rs` matrix imported one OpenAI path without the `BetaResponseItemList` component its response names; it now includes that one component (the import correctly rejected the dangling reference)
        - sweep (non-test `RestApi` construction / transfer): `define/src/builders.rs:176` builder seed — clean, `RestApiBuilder::build` runs `validate_references`; `definitions/src/registry/typed.rs::assemble_module_group` — clean, validates every member; `define/src/openapi/import/builder.rs::map_to_rest_api` — fixed (validated in `build`); other `schemas: vec![]`/`RestApi {` literals in `define/src/{types,request}.rs`, `define/src/openapi/export/{mod,info}.rs` are inside `#[cfg(test)]`; `gen/src/test_utils.rs` is `#[cfg(test)]`; no non-test code deserializes a `RestApi` for callers (Serde derive only; docs require `validate_references` on deserialized APIs); `gen/src/asyncapi_import.rs` builds WebSocket definitions, not `RestApi`; `gen` definitions route never reads the import's components (`definition_gen` re-registers models)
        - tests added: `define/tests/openapi_tests.rs` `every_source_form_imports_petstore_with_resolvable_exportable_components` (YAML/JSON/path/bytes/document; components, `validate_references`, `openapi::export`), `an_import_module_path_qualifies_component_rust_paths`, `a_dangling_create_reference_fails_the_import_naming_api_endpoint_and_type` (response and request), `a_component_free_import_still_validates_and_exports`
        - `cargo nextest run -p schematic-define --all-features -E 'binary(openapi_tests) | binary(import_normalization) | binary(builders)'` 59/59; `-E 'test(import) | test(openapi)'` 238/238; full `-p schematic-define --all-features` 657/657
        - `cargo nextest run -p schematic-gen -E 'test(import) | test(petstore) | test(openai)'` 32/32 (incl. `reimporting_the_vendored_openai_document_reproduces_the_committed_definition`, so the OpenAI import validates and its emitted module is unchanged); `--test l1 -E 'test(slow_imported_builders_compile_and_match_public_import)'` 1/1 (first run failed on a test bug of mine, moved instead of copied schemas; fixed); `-E 'test(artifact_drift) | test(definition_baseline)'` 33/33; `-E '!test(/(^|::)slow_/)'` 652/652
        - `just lint` (schematic/) exit 0, no warnings
        - docs: `docs/io/import-openapi.md` (import result components, renames, dangling-reference error, component-free imports, assembly order/schema difference, standalone Rust paths); `.claude/skills/schematic-define/SKILL.md`; `.claude/skills/schematic-define/schematic/openapi.md`
        - orchestrator check: reviewed the `builder.rs`, `error.rs`, `import_pipeline.rs` diff; `build` now attaches catalog components and fails on `validate_references`; the CLI compensation is narrowed to Rust-path spelling only
- work completed for 'R2-F1: public OpenAPI imports return definitions with unresolved references' at 16:05:52
- starting phase verification (orchestrator, no subagents running) at 16:05:52
        - `just test` (schematic/): 2287 passed, 13 skipped; fresh-output guards 7/7 (including `slow_imported_builders_compile_and_match_public_import`); runtime compile-fail guard 1/1; exit 0
        - `just lint` (schematic/): exit 0
        - `just doctest` (schematic/): exit 0 for every participating crate (2 existing ignored examples)
        - `just test-schema` (schematic/): 55/55 passed
        - `just check-tier-coverage schematic` (repository root): 0 stranded tests
        - every `schematic-define` dependent lives inside `schematic/`, so the new `OpenApiError::Definition` and `DefinitionError::ComponentConflict` variants are covered by the runs above
        - cross-OS check not run: the changes are JSON-value comparison and import assembly with no path, process, or `#[cfg]` code, so they carry no notable OS risk
- work completed for phase verification at 16:11:52

### Observations (not findings; not repaired here)

- nested-only components are stored in untransformed schemars form, so OpenAPI 3.0 export could carry `type: [X, "null"]`, and a bare `"$ref": "#"` could be stored if a recursive type's copy under its own root arrived first; no shipped artifact under `schematic/openapi/` contains either today
- a plain alias keeps its own component in the public import (endpoint references name the alias), while definition assembly resolves the alias to its target; Petstore has no aliases, so the equivalence test does not exercise this
- the example code in `.claude/skills/schematic-define/schematic/openapi.md` already used stale APIs (`OpenApiSource::from_file`, `result.models.models`) before this cycle

### Departures

- R2-F1.13: the public import keeps the source document's component order (`Pet`, `CreatePetRequest2`, `PetStatus`), while assembly places nested definitions right after their parent (`Pet`, `PetStatus`, `CreatePetRequest2`). The importer cannot reproduce that nested order reliably, because it depends on schemars output under a `serde_json` `preserve_order` feature chosen by whichever crate builds it. The test therefore asserts each exact order, set equality of (name, Rust path), and equality of every other `RestApi` field. Schema bodies differ by design: source OpenAPI schema vs schemars output.

### Completion Report

- **outcome:** complete
- **elapsed:** 2026-10-21T15:41:29-07:00 → 16:11:52 (about 30 minutes)
- **parent findings:** total 2 = fixed 2 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0 + disputed 0
- **instance rows:** 21 child rows; 16 fixed, 5 clean (R1-H1.3d, R1-H1.3h, R2-F1.8, R2-F1.10, R2-F1.12); none open
- verified repairs:
        - **R1-H1:** `components::flatten` now compares each same-named arrival (nested/nested, nested/explicit, explicit/nested) by normalized wire shape and fails with `DefinitionError::ComponentConflict` collected in `Collisions`. The failure propagates through `DefinitionContext::components`, `RestApiBuilder::build`, and `assemble_module_group`. A sibling site in the same class was fixed too: grouped OpenAPI export (`merge_documents`) used to overwrite same-named components across APIs. Compatible reuse and ordering are preserved
        - **R2-F1:** `OpenApiImport::build` attaches one component per catalog model (final name, module-qualified Rust path, source schema with renamed internal refs), renames response and request references through the name mapping, and runs `validate_references`. A dangling reference now fails with `OpenApiError::Definition`. The standalone CLI compensation is narrowed to bare Rust-path spelling. The equivalence test no longer clears schemas
- open finding/child IDs: none
- changed files:
        - R1-H1: `schematic/define/src/components.rs`, `schematic/define/src/registration.rs`, `schematic/gen/src/export/openapi.rs`, `schematic/define/tests/builders.rs`, `schematic/definitions/src/registry/typed.rs`, `schematic/docs/typed-registration.md`, `schematic/docs/io/export-openapi.md`, `schematic/docs/generation/validation.md`, `.claude/skills/schematic-define/SKILL.md`
        - R2-F1: `schematic/define/src/openapi/import/builder.rs`, `schematic/define/src/openapi/import/mappings/schema.rs`, `schematic/define/src/openapi/error.rs`, `schematic/define/src/registration.rs` (`Eq` derive), `schematic/gen/src/import_pipeline.rs`, `schematic/define/tests/openapi_tests.rs`, `schematic/define/tests/import_normalization.rs` (fixture now defines the schema its response names), `schematic/gen/tests/l1/import_overlay.rs`, `schematic/docs/io/import-openapi.md`, `.claude/skills/schematic-define/SKILL.md`, `.claude/skills/schematic-define/schematic/openapi.md`
- final verification (all on macOS, from `schematic/` unless noted):
        - `just test`: 2287 passed / 13 skipped, plus 7/7 fresh-output and 1/1 compile-fail guards (R1-H1.3a–h, R2-F1.1–13)
        - `just lint`: pass
        - `just doctest`: pass
        - `just test-schema`: 55/55 (generated-client compatibility for both repairs)
        - `just check-tier-coverage schematic` (root): 0 stranded
