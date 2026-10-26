---
feature: 2026-10-18-schematic-fidelity-polish
implementation_1: "2026-10-23T09:58:14-07:00"
---

# Review Implementation Log: Schematic fidelity and polish

## Implementation of Review Findings #1

> **started at:** 2026-10-23T09:58:14-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-18-schematic-fidelity-polish/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review frontmatter `implemented` confirmed `false` before repairs began
- the review's three observations (inline JSON request schema loss, slow runtime TLS tests, whole-work closure) are explicitly nonblocking and not part of this repair inventory

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | GraphQL Postman requests omit API-level fixed headers | fixed | all six children verified; shared `fixed_headers` in `gen/src/export/postman/request.rs` used by REST and GraphQL builders |
| R1-F1.1 | Single collection, REST request: API fixed header retained (control) | clean | control; `postman_runnable::every_rest_and_graphql_request_carries_the_api_fixed_headers` asserts `GetUser` carries `X-Protocol-Version: 2` |
| R1-F1.2 | Single collection, GraphQL without variables: retain API fixed header | fixed | same test: variable-free `Viewer` carries `2` (was absent) |
| R1-F1.3 | Single collection, GraphQL with typed variables: retain API fixed header | fixed | same test: typed-variable `Issues` carries `2`, body language still `json` |
| R1-F1.4 | Shared-module collection, REST requests: each member's header retained (control) | clean | control; `postman_runnable::every_shared_module_member_request_carries_its_own_fixed_headers`: `GetUser (Hub)` → `2`, `GetUser (Enterprise)` → `3` |
| R1-F1.5 | Shared-module collection, GraphQL without variables: each member's header retained | fixed | same test: `Viewer` at `{{graphqlUrl}}` → `2`, at `{{graphqlUrl2}}` → `3` (lowercase key on member 2) |
| R1-F1.6 | Shared-module collection, GraphQL with typed variables: each member's header retained | fixed | same test: `Issues` at `{{graphqlUrl}}` → `2`, at `{{graphqlUrl2}}` → `3` |
| R1-F2   | Imported model renames miss form parts and nested or API-level references | fixed | all children verified; one recursive `TypeRef::names_mut` applied over `RestApi::json_references_mut`, generated from the same macros as the read-only lists `validate_references` reads |
| R1-F2.1 | JSON multipart part, direct and array-contained renamed component → final name, imports | fixed | `renamed_component_in_json_part_is_followed`, `..._array_json_part_...`, `..._plain_multipart_part_...` in `define/tests/openapi_multipart_import.rs` (all red before) |
| R1-F2.2 | JSON success shape, direct / array-contained → both import with final reference | fixed | `renamed_component_in_json_success_is_followed`, `..._array_json_success_...` (array red before) |
| R1-F2.3 | SSE item shape restored from extension, direct / array-contained → both import | fixed | `renamed_component_in_sse_item_is_followed`, `..._array_sse_item_...` (array red before) |
| R1-F2.4 | NDJSON item shape restored from extension, direct / array-contained → both import | fixed | `renamed_component_in_ndjson_item_is_followed`, `..._array_ndjson_item_...` (array red before) |
| R1-F2.5 | Endpoint JSON error shape, direct / array-contained → both import | fixed | `renamed_component_in_endpoint_error_is_followed`, `..._array_endpoint_error_...` (array red before) |
| R1-F2.6 | API-level JSON error defaults, direct / array-contained → both import | fixed | `renamed_component_in_api_error_default_is_followed`, `..._array_api_error_default_...` (both red before) |
| R1-F2.7 | JSON request body, direct component → final name (clean); inline array loss is a nonblocking observation | clean | control; `renamed_component_in_json_request_is_followed` → `AddVoiceSampleRequest2` |
| R1-F2.8 | Model catalog and component-schema references keep final names (clean) | clean | control; `renamed_component_keeps_its_final_name_in_catalog_and_component_schemas`; catalog `remap_type_ref` now reuses `names_mut` |
| R1-F2.9 | Optional / map / composition wrappers around a renamed component → recursive rewrite | fixed | `renamed_component_in_optional_json_success_is_followed`, `..._map_api_error_default_...`, `..._nested_sse_item_...` (all red before); unit guard `components::tests::json_references_mut_reaches_every_reference_validation_reads` (12 refs incl. GraphQL) |
| R1-F2.10 | Sibling: AsyncAPI importer model naming | clean | never renames to avoid collisions; payload references and model names come from one transform (subagent sweep of `gen/src/asyncapi_import.rs`) |
| R1-F2.11 | Sibling: GraphQL operation references / pagination `item_type` / WebSocket definitions | clean | GraphQL covered by `json_references_mut` and the unit guard though OpenAPI import cannot produce it; import never sets `item_type`; WebSocket definitions hold no `TypeRef` |

### Progress

- starting the work on 'R1-F1 GraphQL Postman requests omit API-level fixed headers' at 09:58:52
        - defect class: one request projection drops API-level request metadata that sibling request projections preserve
        - subagent (rust-developer) report:
                - extracted `fixed_headers(api, overrides)` in `gen/src/export/postman/request.rs`; REST `build_headers` and GraphQL `operation_item` both start from it (REST output unchanged)
                - `GraphQlOperation` has no per-operation headers, so GraphQL passes no overrides; fixed headers are not filtered by credential header, same as REST
                - sibling sweep: REST Postman requests clean (already used `api.headers`); `RestApi` carries no other API-level request metadata besides `headers`/`auth` (auth already applied to GraphQL); Postman environment clean (variables only); OpenAPI export clean (GraphQL skipped, REST applies `api.headers`); AsyncAPI clean (`api.headers` → `bindings.ws.headers`)
                - tests added in `gen/tests/l1/postman_runnable.rs`: `every_rest_and_graphql_request_carries_the_api_fixed_headers`, `every_shared_module_member_request_carries_its_own_fixed_headers` (second member uses lowercase `x-protocol-version: 3`)
                - no committed Postman artifact changed (no shipped definition has a GraphQL surface); `artifact_drift::generated_postman_artifacts_are_up_to_date` passes
                - docs: `schematic/docs/io/export-postman.md` GraphQL section now states API fixed headers apply
                - `cargo nextest run -p schematic-gen --test l1 -E 'test(/^postman_runnable::/) | test(graphql) | test(postman)'` → 55 passed; `--lib` postman/graphql → 49 passed; `just lint` clean
        - orchestrator check: reviewed the diff; subagent flagged `docs/graphql.md:184` (`Accept: application/json`) — that line describes the generated Rust client, not Postman export, so no drift
- work completed for 'R1-F1 GraphQL Postman requests omit API-level fixed headers' at 10:01:57
- starting the work on 'R1-F2 Imported model renames miss form parts and nested or API-level references' at 10:01:57
        - defect class: a model's identity is renamed in the catalog without rewriting every definition reference (at any nesting depth and at every definition location) that later reference validation reads
        - subagent (rust-developer) report:
                - added `TypeRef::names_mut` (mutable twin of `names`, every wrapper variant) and `RestApi::json_references_mut` / `Endpoint::json_references_mut` / `GraphQlOperation::json_references_mut`
                - read-only and mutable reference lists for endpoints, error tables, and GraphQL operations are generated by shared macros (`endpoint_json_references!`, `error_references!`, `operation_json_references!`) so the rename set cannot drift from what `validate_references` reads
                - `OpenApiImport::build` renames through that set; the old `remap_reference_names` is deleted; the catalog's `remap_type_ref` now reuses `names_mut` (keeps its sanitize fallback)
                - 18 public-import tests added to `define/tests/openapi_multipart_import.rs`: 12 failed and 6 passed with the old `builder.rs` swapped back in; all 18 pass now
                - sibling sweep: AsyncAPI importer clean (no collision renames); GraphQL references covered; pagination `item_type` never set by import; WebSocket definitions hold no `TypeRef`
                - docs: `schematic/docs/io/import-openapi.md` "Renamed models" now lists every rewritten location and states it holds at any nesting depth
                - `cargo nextest run -p schematic-define --all-features` → 869 passed; `cargo nextest run -p schematic-gen -E 'test(round_trip) | test(openapi)'` → 103 passed; `just lint` clean
                - new nonblocking observations (not repaired; preexisting, outside this defect class): AsyncAPI v2 import merges two schemas whose names collide after PascalCase conversion; in a plain multipart document without `x-schematic`, a JSON-encoded part that is an array of a component imports as untyped JSON (multipart counterpart of the review's inline-request observation)
        - orchestrator check: reviewed the combined diff; confirmed lookups are no-ops for unmapped names, so the always-run rename loop changes nothing when `name_mapping` is empty; confirmed `schematic-define` still compiles without default features (`cargo clippy -p schematic-define --no-default-features -- -D warnings` → clean)
- work completed for 'R1-F2 Imported model renames miss form parts and nested or API-level references' at 10:21:57
- phase gates run by the orchestrator after both repairs, with no subagent running:
        - `just test` in `schematic/` → exit 0: 2,993 passed (22 skipped; 4 slow, all the preexisting runtime TLS tests named in the review's observation), 16 fresh-output/compile tests passed, 1 compile-fail guard passed
        - `just lint` in `schematic/` → exit 0
        - `just check-tier-coverage schematic` → 0 stranded
        - cross-OS check not run: the changes touch no paths, processes, or `#[cfg]` OS code; public APIs are unchanged (new helpers are `pub(crate)`)

### Completion Report

- **outcome:** complete
- **elapsed:** 2026-10-23T09:58:14-07:00 → 2026-10-23T10:21:57-07:00 (about 23 minutes)
- **parent findings:** total 2 = fixed 2 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0 + disputed 0
- **instance rows:** 17 children, 11 fixed, 6 clean (R1-F1.1, R1-F1.4, R1-F2.7, R1-F2.8 controls; R1-F2.10, R1-F2.11 sibling sweeps); no row open
- **verified repairs:**
        - R1-F1: GraphQL Postman requests (single and shared-module collections, with and without variables) now carry the API's fixed headers through the same `fixed_headers` helper REST uses
        - R1-F2: an import-time component rename is followed at every location `validate_references` reads, at any nesting depth, including JSON multipart parts and API-level error defaults
- **open findings:** none
- **nonblocking observations carried forward (not findings of this review):**
        - review observations: inline JSON request schemas lose their type on import; runtime TLS tests exceed the L1 budget; whole-work closure items remain open outside this feature
        - discovered this cycle: AsyncAPI v2 PascalCase name merge; untyped array-of-component JSON multipart part in plain documents
- **changed files:**
        - R1-F1: `schematic/gen/src/export/postman/request.rs`, `schematic/gen/src/export/postman/graphql.rs`, `schematic/gen/tests/l1/postman_runnable.rs`, `schematic/docs/io/export-postman.md`
        - R1-F2: `schematic/define/src/models/type_ref.rs`, `schematic/define/src/components.rs`, `schematic/define/src/graphql.rs`, `schematic/define/src/openapi/import/builder.rs`, `schematic/define/src/openapi/import/mappings/schema.rs`, `schematic/define/tests/openapi_multipart_import.rs`, `schematic/docs/io/import-openapi.md`
- **final verification:** `just test` (schematic) exit 0 (verifies R1-F1 and R1-F2 tests plus the whole area); `just lint` exit 0; `just check-tier-coverage schematic` 0 stranded; `cargo clippy -p schematic-define --no-default-features -- -D warnings` clean (R1-F2 feature gating)

## Implementation of Review Findings #2

> **started at:** 2026-10-23T10:28:00-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-18-schematic-fidelity-polish/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- review 2 lists no findings (`findings: []`) and three observations; there is no code to repair in this pass

## Follow-up Pass for Review #2

- the review found in '/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-18-schematic-fidelity-polish/review-2.md' indicated that the specification is **production ready**
- the specification can be found at '/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-18-schematic-fidelity-polish/spec.md'
- the review's `findings` list is empty, so nothing qualifies for the fix-now default; each observation is carried to the backlog, as the pass requires

### Finding Ledger

| ID | Finding | Disposition | Evidence or reason |
| --- | --- | --- | --- |
| R2-O1 | Inline JSON request schemas remain outside the supported typed import | backlog (observation) | Observations are not repaired in this pass; `reference_from_schema` in `define/src/openapi/import/mappings/responses.rs` maps any inline schema to untyped JSON, and that behavior predates this feature |
| R2-O2 | Existing runtime TLS tests exceed the Level-1 time budget | backlog (observation) | Observations are not repaired in this pass; the four certificate tests in `runtime/tests/l1/builder.rs` are unchanged by this feature |
| R2-O3 | Whole-work closure remains separate from this feature | backlog (observation) | Observations are not repaired in this pass; the open items (sniff `FocusedProviderClient`, spec 4b's verifying review, the base-URL composition check) are outside this spec's code |

- verification: no source, test, or doc file changed in this pass, so no package area was affected and `just test` / `just lint` were not rerun; review 2 records both passing in `schematic/` on the current tree
- cross-OS check not run: no code changed
- backlog entry written: `schematic/fixes/_unscheduled/schematic-fidelity-polish-follow-ups/spec.md` (R2-O1, R2-O2, R2-O3, plus RL-1 and RL-2, the two defects found during repair 1 that only the log recorded)
- review 2 frontmatter set: `implemented: true`, `implemented_by: claude/opus`
- completed at 2026-10-23
