---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-H1 (high): Nested schema conflicts bypass module-group collision checks"
    - "R2-F1 (high): Public OpenAPI imports return definitions with unresolved references"
observations: []
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-21T15:35:56-07:00"
spec: "2026-10-14-schematic-definition-api/spec.md"
implemented: true
implemented_by: claude/opus
log: "/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-14-schematic-definition-api/log.md"
description: "A **feature** review of `2026-10-14-schematic-definition-api/spec.md`"
feature: "2026-10-14-schematic-definition-api/review-2.md"
previous: "2026-10-14-schematic-definition-api/review-1.md"
next: "2026-10-14-schematic-definition-api/review-3.md"
---

# Review 2: Schematic definition API

The feature is **not production ready**. The normal authoring and shipped-provider paths pass their tests, but two public boundaries do not meet the specification: conflicting nested components can produce an incorrect OpenAPI schema, and the public importer returns definitions that fail the new reference-validation contract.

The normalization decision in review 1 remains accepted. Neither finding asks to restore embedded descriptors, introduce context identity, or compare duplicated Rust paths.

## Previous Finding Verification

Review 1 contained no formal findings and performed no implementation review. Its author ruling was subsequently tracked as **R1-H1**, with 13 required instances in the repair's **Finding Ledger**. That actionable inventory is verified below; it is not treated as an empty previous review merely because the original frontmatter had `findings: []`. There are no earlier reviews in this directory.

Paths in this table are relative to `schematic/` unless stated otherwise. Test evidence refers to the current code and this review's runs, rather than the repair's completion claims.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-H1 — Normalize REST schema references | R1-H1.1–R1-H1.13 | partially fixed | The stored model, consumers, artifacts, and documentation use the normalized representation. The collision reproduction below demonstrates a remaining hole in R1-H1.3. | Reject conflicting nested component definitions; see the carried finding. |
| R1-H1.1 — Store identity and wrapper shape only | JSON response, JSON request, JSON multipart and URL-encoded fields | verified fixed | `define/src/{response,request}.rs` store `TypeRef`; `Endpoint::json_references` visits all four sites. Builder and form-transfer tests pass. | None. |
| R1-H1.2 — API owns its registry | Entire context, including later module-group registrations | verified fixed | `define/src/components.rs` defines `SchemaComponent`; API build assigns `ctx.components()`. `definitions/src/registry/typed.rs` reassigns the finished components to every member; `members_built_before_a_later_registration_carry_the_full_components` and registry invariants pass. | None. Conflict checking is assessed separately in R1-H1.3. |
| R1-H1.3 — Resolve references and reject registry conflicts | Destination resolution, shared-context conflicts, removal of descriptor-agreement checks | partially fixed | Missing response/request/form names and duplicate transferred components are rejected. Direct nominal collisions are collected. `components::flatten` replaces an existing nested definition and `nested` discards an existing name without checking its schema; the public build/export reproduction below succeeds with incompatible definitions. | The nested/nested, nested/explicit, and explicit/nested instances below. |
| R1-H1.4 — Extend the existing reference vocabulary | Integer map keys, primitive widths, free-form JSON | verified fixed | `define/src/models/type_ref.rs` adds `KeyedMap` and primitive variants; registration tests exercise supported keys and nested containers. The Phase 9 log records the extensions. | None. |
| R1-H1.5 — Remove descriptor evidence; retain WebSocket schema | REST removal and unchanged WebSocket fields | verified fixed | `define/src/schema.rs` retains `type_name` and `module_path`; live REST consumers use `TypeRef`. Source searches found no remaining `Schema::descriptor` reader. WebSocket tests pass. | None. |
| R1-H1.6 — Migrate definitions | Registry lookups removed, shared assembly, alias/generic registration | verified fixed | `definitions/src/registry/typed.rs` assembles from one context. Shipped source scans, registry invariants, aliases, and Bitbucket registration tests pass; removed registry APIs have no live consumers in the searched sources. | None beyond the shared collision defect in R1-H1.3. |
| R1-H1.7 — Migrate generator | Rust spelling, reference diagnostics, removal of registry arguments | verified fixed | Generator type spelling uses `api.rust_type`; dangling-name diagnostics cover wrapped references, and freshly generated clients compile. | The separate public-import boundary is a new finding, R2-F1. |
| R1-H1.8 — Migrate exports | API-owned components, strict completeness, Postman | verified fixed | OpenAPI components read `api.schemas`; strict completeness, module-document component checks, and all three artifact-drift tests pass. | No exporter repair independent of the upstream collision/import defects. |
| R1-H1.9 — Preserve typed list recognition | Root-array inference and component-derived item spelling | verified fixed | `define/src/pagination/resolve.rs::root_array_item_type` reads `TypeRef::Array`; typed list boundary tests and shipped pagination coverage pass. Optional, nominal, and non-GET controls remain non-lists. | None. |
| R1-H1.10 — Update snapshots and generated artifacts | 18 definition snapshots; Rust, OpenAPI, Postman outputs | verified fixed | All 18 current baselines, description-mutation control, and current artifact comparisons pass without an exception list. The recorded `x-schematic` change matches the stored `TypeRef` representation. Historical execution claims were not rerun in an old checkout. | None identified in the current artifacts. |
| R1-H1.11 — Update builder/transfer tests | References, components, malformed transfer data | verified fixed | `builders::{references_resolve_through_the_destination_context_and_survive_transfer,deserialized_definitions_are_checked_by_the_same_resolution,transferred_components_and_references_input_matrix}` pass and inspect the public result. | Missing nested-conflict cases belong to R1-H1.3. |
| R1-H1.12 — Record the departure | Amend “Builder implementation and lowering” | verified fixed | The departure note directly below that section records the normalized references, API-owned registry, and removal of descriptor checks. | None. |
| R1-H1.13 — Update current guidance | Docs, READMEs, skills, creation command | verified fixed | Current definition-builder/registration/IO pages and skills describe `TypeRef` plus `RestApi::schemas`; `authoring_guidance` passes. The obsolete representation remains in historical log prose with an explicit superseding note. | None. |

The author decision has been resolved; there is no remaining human prerequisite. R1-H1 is a **partial repair**, not a regression of the working direct-registration checks. The nested instances are siblings that the repair's conflict sweep missed. R2-F1 is a newly identified implementation gap, not an earlier finding relabeled as new.

## Unblocked Findings

### R1-H1 (high): Nested schema conflicts bypass module-group collision checks

**Authority — spec and accepted ruling:** the spec's “Module-group assembly and request coverage” section says that “module-group collisions fail validation instead of relying on last-write-wins merging.” Review 1's accepted normalization ruling further requires that “merging module-group registries rejects conflicting definitions of the same name.” This carries the still-incomplete R1-H1.3 obligation from the repair ledger.

**Defect class:** component-name conflict detection checks explicit registrations but loses incompatible definitions introduced by nested schema extraction.

In schematic-define, [flatten](../../define/src/components.rs:31) replaces a nested entry when an explicit registration arrives, while [nested](../../define/src/components.rs:44) silently keeps the first entry when another nested schema has that name. [DefinitionContext::leaf](../../define/src/registration.rs:151) only compares explicitly registered Rust types. Consequently, unique parent models from different modules can contain same-named child models with different wire types, and API build and export both succeed with one parent's schema applied to both.

Reproduction uses the existing two-module collision fixture pattern from `define/tests/typed_registration.rs` and the parent/child pattern from `define/tests/builders.rs`. Define `left::Status { value: String }`, `right::Status { value: bool }`, and distinct `Left { status: left::Status }` / `Right { status: right::Status }` models deriving `JsonSchema`. Register the parent types in one context and build a normal JSON endpoint. Both parents reference `#/components/schemas/Status`, but only one incompatible `Status` is retained. Exporting the built API succeeds and emits the incorrect shared component. This is a plausible model composition, not forged serialized input.

The bounded sweep covered both insertion directions, both name-introduction paths, and the later decisions that consume the merged value:

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-H1.3a | `components::nested`, nested versus nested | Register `Left`, then `Right`; their child `Status.value` schemas are string and Boolean | Build succeeds; string definition silently retained | Reject the incompatible shared name. |
| R1-H1.3b | `components::flatten`, nested versus explicit | Register both parents, then explicitly register `right::Status` | Build succeeds; Boolean definition overwrites the extracted string definition | Reject the incompatible replacement. |
| R1-H1.3c | `components::nested`, explicit versus nested | Explicitly register `left::Status`, then both parents | Build succeeds; incompatible nested Boolean definition silently discarded | Reject the incompatible extraction. |
| R1-H1.3d | `DefinitionContext::leaf` and `finish` — clean | Explicitly register both distinct `Status` types | Second registration fails; components also fail | Keep this rejection and collected error. |
| R1-H1.3e | `RestApiBuilder::build` / `RestApi::validate_references` | Each of the three conflicting merged component sets above | Accepts: names are now unique and endpoint references resolve | Conflict must already have failed before the competing definitions are discarded. |
| R1-H1.3f | `assemble_module_group` | Shared context's finished components assigned to each API | Code inspection: calls the same `components()` function, then validates already-merged data | Propagate the same conflict error to group assembly. |
| R1-H1.3g | OpenAPI component projection and reference-closure validation | Export all three accepted conflicting APIs | Export succeeds; both parents point to one `Status`; retained value type is string / Boolean / string respectively | Conflicting group must not reach successful export. |
| R1-H1.3h | Existing same-type/nested extraction controls — clean | Existing `api_carries_the_whole_context_with_nested_definitions_flattened`, registry-order tests, and shipped API invariants | Pass; repeated compatible extraction preserves the registry | Preserve compatible reuse and ordering. |

Repair the common component merge so conflicting wire definitions fail before either is lost, while preserving compatible repeated extraction and existing ordering. Normalize root-only schema metadata when comparing an explicit component with its extracted copy. This needs a bounded merge check, not a new descriptor-evidence or protection framework. Add public build/group tests for the three failing insertion shapes and keep the clean controls.

### R2-F1 (high): Public OpenAPI imports return definitions with unresolved references

**Authority — spec:** requirement R7, “Every endpoint reference is validated against its destination context,” explicitly says: “Imported in-memory definitions receive the same check.” The unchanged, shipped Petstore import fixture already fails this requirement; no malformed input is needed.

**Defect class:** a public construction path returns a successful REST definition without assembling or validating the components its references require.

In schematic-define, [OpenApiImport::map_to_rest_api](../../define/src/openapi/import/builder.rs:373) always sets `schemas: vec![]`. [OpenApiImport::build](../../define/src/openapi/import/builder.rs:200) returns that API after constructing the model catalog, without attaching components or calling reference validation. The response/request [reference reader](../../define/src/openapi/import/mappings/responses.rs:387) creates a named reference without using its resolver. In schematic-gen, [with_model_components](../../gen/src/import_pipeline.rs:198) compensates only for standalone CLI generation; it does not repair the public library result.

Reproduce with `schematic/gen/tests/fixtures/petstore.yaml`:

```rust
use schematic_define::openapi::{OpenApiImport, OpenApiSource};

let imported = OpenApiImport::new(OpenApiSource::path(fixture_path))
    .build().unwrap();
assert_eq!(imported.api.endpoints.len(), 4);
assert!(imported.api.schemas.is_empty());
// Err: Petstore endpoint CreatePet: unresolved schema reference "Pet"
let result = imported.api.validate_references();
```

The same API cannot be exported through the public OpenAPI exporter: reference-closure validation fails because its components are absent. Changing just the create operation's JSON request or JSON response `$ref` to `#/components/schemas/MissingPet` also returns a successful import containing that dangling name. These are separate instances of the same missing assembly check.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F1.1 | YAML text loader → public import result | Unedited Petstore fixture | Import succeeds, 4 endpoints, empty components; reference validation/export fail | A successful in-memory API carries resolvable components. |
| R2-F1.2 | JSON text loader → public import result | JSON serialization of that same OpenAPI document | Same failure | Same valid API result. |
| R2-F1.3 | File loader → public import result | Original fixture path | Same failure | Same valid API result. |
| R2-F1.4 | Byte loader → public import result | Original fixture bytes | Same failure | Same valid API result. |
| R2-F1.5 | Parsed-document loader → public import result | Parsed original document | Same failure | Same valid API result; bypassing text parsing does not bypass assembly. |
| R2-F1.6 | JSON response reference reader | Replace create response `$ref` with `MissingPet` | Successful import contains `Named("MissingPet")` | Reject unresolved reference, naming API, endpoint, and type. |
| R2-F1.7 | JSON request reference reader | Replace create request `$ref` with `MissingPet` | Successful import contains `Named("MissingPet")` | Same rejection. |
| R2-F1.8 | Form-body reference sites — clean for this class | Inspect multipart and URL-encoded mappings | Importer emits text/file fields, not JSON `TypeRef` form fields | No additional named endpoint reference to validate at these sites. Their broader fidelity is outside this finding. |
| R2-F1.9 | Standalone CLI generation — compensated control | Existing Petstore `run_import_from_petstore_yaml` test | Passes; `with_model_components` supplies model-name bindings before generation | Preserve working client generation while fixing the public result. |
| R2-F1.10 | Emitted definition assembly — compensated control | Existing compiled Petstore builder fixture and OpenAI re-import | Passes; emitted registration/build reconstructs the registry | Preserve working definitions generation. |
| R2-F1.11 | Public reference validation and OpenAPI export | Original imported API from all five source forms | Both reject the successful import's missing components | The import boundary must produce a usable API rather than defer a guaranteed failure. |
| R2-F1.12 | Component-free endpoint — clean | Replace fixture paths with one inline JSON string response | Import, reference validation, and export succeed | Preserve legitimate component-free imports. This control tests reference assembly, not inline-schema fidelity. |

Populate and validate the imported API using the source schemas/model naming information already available during import. Do not require compiling generated model source to make this check. Update the compiled importer equivalence test: it currently [clears the emitted API's schemas](../../gen/tests/l1/import_overlay.rs:185) before comparison, masking precisely this difference. Test the public result's references and components as well as preserving the existing generated-client checks.

## Blocked Findings

None. Both repairs are authorized by the existing specification and settled normalization decision.

## Recurrence

Review 1 had no formal defect findings. Its accepted ruling nevertheless created the conflict-rejection obligation tracked by the repair as **R1-H1.3**. That obligation is carried here because the repair covered explicit nominal collisions and duplicate transferred component rows but missed the sibling nested/explicit merge paths. This is a missed sibling in the partial repair, not a regression caused by restoring an earlier representation. The complete remaining list is R1-H1.3a–R1-H1.3h above; the parent keeps its original ledger ID. `recurrence: true` records that repeated repair obligation. R2-F1 is new.

## Input Robustness Matrix

The changed configuration readers are the transferred-definition deserializers and the OpenAPI query-path normalization/import path. Writers and generated text are excluded from this matrix.

The default for malformed required transfer data is deserialization failure, or a semantic reference/component error before use. Exceptions are explicit: optional fields accept absence/null, an absent/null component `rust_path` denotes a nested-only schema, `{}` and Boolean schemas are valid schema values, and empty parameter collections are valid. A nested-only component cannot be named directly by an endpoint. For import normalization, absent/empty parameter lists are allowed, absent `required` means false, and an absent request body means no body; GET bodies are rejected.

Existing passing public-result matrices cover:

- `builders::transferred_components_and_references_input_matrix`: required component name/schema and registry list; optional Rust binding; missing/null, wrong whole types, wrong elements, empty values, duplicate fields, trailing content, and reference resolution.
- `definition_serde::parameter_collections_reject_malformed_json_matrix` and `definition_baseline::shipped_json_field_matrix_preserves_required_and_optional_shapes`: required collections versus optional metadata, round-trip preservation, malformed collection members, and duplicate/trailing JSON.
- `import_normalization::{json_import_normalization_matrix,yaml_import_normalization_matrix}` and neighboring tests: paths, parameter declarations, names, locations, required flags, schemas/types, request bodies, duplicate declarations, invalid operations, schema compatibility, and unresolved query-parameter references. File/byte sources share the YAML parsing branch; document sources share normalization after parsing.

The source scan checked `#[serde(default)]`, `Option<T>`, `filter_map`, `unwrap_or_default`, and `.ok()` in the changed registration/component/import code. R2-F1 is an assembly failure after valid parsing, not another null-versus-absence rule. Its valid fixture, missing response reference, missing request reference, and component-free control were exercised at the public API. No additional invalid-shape finding is claimed.

## Requirement Coverage and Verification Level

| Requirements | Strongest relevant verification inspected/run | Assessment |
| --- | --- | --- |
| Typed registration, aliases, wrappers, free-form JSON, and destination validation | L1 public API tests; compiler misuse diagnostics and isolated generated-crate compilation | Appropriate level. Nested conflicts and public import assembly remain the two findings. |
| Required builder facts, no public Default/literals, authentication coherence, response placeholders | L1 builder tests, public-default audit, compile-fail doctests and generated compiler cases | Appropriate level; passing. |
| Endpoint origin override, redirect/followed-link behavior, and caller tightening | L1 mock HTTP servers plus freshly generated clients sending actual loopback requests | Appropriate level for HTTP behavior; passing. No terminal input behavior is promised. |
| Request-id/secret emission, retained authentication policy, serde, every provider conversion | L1 emitted-source/runtime checks, public round trips, authoring scans, and 18 definition baselines | Appropriate level; passing. |
| Pagination relocation/removal, list recognition, managed query inputs | L1 provider coverage and generated-client pager HTTP/compiler checks | Appropriate level; passing. |
| Import emission and overlay, query-path merges, GitLab encoding, body locations and GET rejection | L1 import matrices, exact re-import, isolated compiled builders, mock-server request capture | Appropriate level. The compiled test does not prove the unmodified public import result is valid; R2-F1 covers that gap. |
| Generated artifacts, dependent-facing client compatibility, guidance | L1 exact artifact comparisons, fresh real-provider compilation, generated-schema suite, authoring corpus, doctests | Appropriate level. This review does not claim a fresh run of every downstream package or cross-OS checks. |

This feature introduces definition-authoring and HTTP-generation behavior, so no new Level 2 terminal-rendering or Level 3 keyboard verification is needed for these requirements. Existing terminal styling tests are not used as proof of schema correctness.

Verification performed on the current macOS checkout:

- `just test` in `schematic/`: **2,277 passed, 13 skipped**; fresh-output/compiler guards **7/7**; runtime compile-fail guard **1/1**. The main suite includes the definition baselines, all three current artifact-drift checks, public transfer matrices, and importer normalization tests.
- `just lint`: passed.
- `just test-schema`: **55/55 passed**, including the generated pager compile-fail guard. That existing compiler guard took 79.6 seconds; it is not an ordinary in-process behavior test.
- `just doctest`: passed for all five participating crates; two existing examples remain ignored.
- `just check-tier-coverage schematic` from the repository root: **0 stranded tests**. The new and rewritten tests inspected in this review are compiled by their declared targets and selected by live recipes; the fresh-output compiler tests are explicitly run by `just test`.
- Additional public-API reproductions: **2/2 passed**, asserting the defective outcomes across all import source forms, missing JSON request/response references, the valid component-free control, all three nested conflict shapes, and the direct-registration rejection control. These temporary probe tests were removed after execution; no production code or permanent tests were changed by the review.

The probe's initial export assertion expected a direct property `$ref`; OpenAPI conversion wraps it in `allOf`. The assertion was corrected to inspect the stored parent references and the actual exported retained child type. That intermediate probe failure was a reproduction assertion error, not an additional product finding.

## Observations

None.
