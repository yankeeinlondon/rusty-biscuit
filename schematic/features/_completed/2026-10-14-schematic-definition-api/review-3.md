---
$schema: feature-review.yaml
ready: true
findings: []
observations: []
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-21T16:12:23-07:00"
spec: "2026-10-14-schematic-definition-api/spec.md"
implemented: false
description: "A **feature** review of `2026-10-14-schematic-definition-api/spec.md`"
feature: "2026-10-14-schematic-definition-api/review-3.md"
previous: "2026-10-14-schematic-definition-api/review-2.md"
---

# Review 3: Schematic definition API

The feature is **production ready within the specification's scope**. Both previous findings are verified fixed, with no new findings. The accepted decision to store component names and wrapper shape in REST references remains in force; this review does not reopen descriptor evidence or require a stronger transfer contract.

## Previous Finding Verification

I read review 2, the repair ledger in `log.md`, and the earlier normalization ledger in `implementation-log.md`, then checked the implementation and reran the tests below. All paths are relative to `schematic/`. Test names identify evidence executed during this review, rather than relying on completion flags.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-H1 — Nested schema conflicts bypass module-group collision checks | R1-H1.3a–R1-H1.3h below; earlier normalization obligations | verified fixed | Component merging compares competing wire schemas before discarding either definition. Public API and module-group tests reject conflicts; compatible controls and artifact comparisons pass. | None. |
| R1-H1.3a — Nested versus nested | Distinct parents with incompatible same-named children | verified fixed | `builders::conflicting_nested_and_explicit_definitions_of_one_name_fail_build`, nested/nested row, checks both `components()` and API build and names both sources. | None. |
| R1-H1.3b — Nested versus explicit | Incompatible explicit registration after nested extraction | verified fixed | The same test's nested/explicit row rejects replacement. `components::flatten` compares the shape before replacing the component. | None. |
| R1-H1.3c — Explicit versus nested | Incompatible extraction after explicit registration | verified fixed | The same test's explicit/nested row rejects extraction. `Merged::nested` compares an existing name rather than silently retaining it. | None. |
| R1-H1.3d — Direct registration control | Distinct explicit types claiming one name; collected errors | verified fixed | `typed_registration` collision tests and `api_build_reports_every_registration_collision` pass. `DefinitionContext::components` calls `finish()` first. | None. |
| R1-H1.3e — API construction and validation | All three conflicting merged sets | verified fixed | All three rows exercise public API build. `RestApiBuilder::build` propagates `ctx.components()?`; invalid competing definitions cannot become a successfully built API. | None. |
| R1-H1.3f — Module-group assembly | Conflict registered after a member was built | verified fixed | `registry::typed::tests::a_nested_component_conflict_registered_after_a_member_build_fails_assembly` passes; assembly obtains the finished context's components. | None. |
| R1-H1.3g — OpenAPI projection and grouped merge | Reject conflicting construction; reject conflicting independently built group members | verified fixed | Construction tests prevent the original three failures reaching export. `export::openapi::tests::grouped_members_with_different_same_named_components_fail_and_equal_ones_collapse` checks the additional grouped-export boundary and equal-schema control. `merge_documents` reports both API names. | None. |
| R1-H1.3h — Compatible extraction and ordering | Repeated child, explicit child in either order, recursive root references | verified fixed | `the_same_type_extracted_repeatedly_or_also_registered_merges_in_first_position`, `a_recursive_type_nested_elsewhere_matches_its_copy_under_its_own_root`, whole-context and registry-order tests pass. Comparison normalizes root titles, nested OpenAPI transforms, and root-relative references; stored ordering remains stable. | None. |
| R1-H1 — Earlier normalization obligations | R1-H1.1–.2 and .4–.13 from the original ledger, already closed by review 2 | verified fixed | REST requests, responses, and JSON form fields still use `TypeRef`; APIs own components; generator, exports, and pagination read that representation. Transfer matrices, registry invariants, all 18 baselines, fresh generated clients, artifact drift, authoring-guidance tests, and doctests pass. The departure remains recorded under “Builder implementation and lowering.” | None; the remaining collision obligation is closed above. |
| R2-F1 — Public OpenAPI imports return definitions with unresolved references | R2-F1.1–R2-F1.13 below | verified fixed | Public import attaches catalog components, remaps request and response names, then calls `validate_references()` before returning success. | None. |
| R2-F1.1 — YAML text source | Unedited Petstore | verified fixed | `openapi_tests::every_source_form_imports_petstore_with_resolvable_exportable_components`, YAML row, asserts four endpoints, exact component names/paths, successful validation and export. | None. |
| R2-F1.2 — JSON text source | Same fixture serialized as JSON | verified fixed | The same public-result test's JSON row makes the same assertions. | None. |
| R2-F1.3 — File source | Fixture written to a temporary YAML file | verified fixed | The same test's path row exercises file loading and the same assembly result. | None. |
| R2-F1.4 — Byte source | Fixture bytes | verified fixed | The same test's bytes row exercises byte loading and the same assembly result. | None. |
| R2-F1.5 — Parsed-document source | Parsed original document | verified fixed | The same test's document row proves bypassing text parsing still runs assembly and validation. | None. |
| R2-F1.6 — JSON response reference | Create response points to `MissingPet` | verified fixed | `a_dangling_create_reference_fails_the_import_naming_api_endpoint_and_type`, response row, asserts the public error identifies Petstore, CreatePet, and MissingPet. | None. |
| R2-F1.7 — JSON request reference | Create request points to `MissingPet` | verified fixed | The same test's request row asserts the same located public error. | None. |
| R2-F1.8 — Form-body control | Multipart and URL-encoded fields | verified fixed | Inspection of `openapi/import/mappings/responses.rs::map_form_field` confirms text/file/files construction only. No imported JSON form reference bypasses the shared check. Authored JSON form references remain covered by builder transfer tests. | None for this class. |
| R2-F1.9 — Standalone CLI generation | Petstore and module-qualified imports | verified fixed | `run_import_from_petstore_yaml` and `a_standalone_client_spells_module_qualified_components_by_bare_name` pass. `with_standalone_rust_paths` changes Rust spelling without replacing component schemas. | None. |
| R2-F1.10 — Emitted definitions | Compiled Petstore assembly and OpenAI re-import | verified fixed | `slow_imported_builders_compile_and_match_public_import` compiles emitted builders and a client; `reimporting_the_vendored_openai_document_reproduces_the_committed_definition` passes. | None. |
| R2-F1.11 — Validation and export consumers | All five imported source forms | verified fixed | The source-form test calls public reference validation and OpenAPI export for every row and checks a nested PetStatus reference in exported Pet. | None. |
| R2-F1.12 — Component-free control | Inline JSON string response | verified fixed | `a_component_free_import_still_validates_and_exports` accepts the import with no components, validates, and exports. This does not assert additional inline-schema fidelity. | None. |
| R2-F1.13 — Import/emitted comparison | Remove the missing-components exemption | verified fixed | `import_overlay.rs` compares both exact component orders and qualified name/path sets before comparing remaining API fields. It no longer clears components. Source OpenAPI schemas and schemars-generated schemas intentionally differ; that bounded difference is documented in the repair log and current import documentation. | None. |

R1-H1 was a partial repair in review 2; the missed nested merge siblings are now repaired. R2-F1 is also repaired. No unchanged instance, repair regression, newly found sibling, or blocked prerequisite remains.

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No finding recurs in this iteration. Review 2's carried normalization obligation is closed by the full instance verification above. No accepted decision needs another human ruling.

## Specification and test coverage

The review checked the required builder facts, typed registration and aliases, container/primitive handling, reference transfer, shared assembly, authentication coherence, endpoint policy metadata, shipped-provider migration, pagination adoption, query-free route identity, repeatable imports, and migration safeguards. The following groups describe the strongest relevant verification, not just test counts.

| Requirements | Verification level and evidence | Assessment |
| --- | --- | --- |
| Typed registration, aliases, containers, and destination references | Level 1 public registration/build/transfer tests; compiler diagnostics for invalid paths and private construction; fresh generated providers compile in isolated crates | Appropriate for the Rust authoring contract. |
| Required builder facts, authentication, placeholders, and literal prohibition | Level 1 builder tests plus diagnostic-checked compiler misuse cases and compile-fail doctests | Appropriate; missing facts are checked by the compiler, conflicting declarations by public build results. |
| Endpoint origin replacement and definition metadata | Level 1 generated clients executing against local HTTP servers, including sibling rejection, redirect/page-link behavior, caller tightening, request IDs, and secret metadata | Appropriate; these are HTTP/runtime promises, with no terminal input or display requirement. |
| Provider migration, snapshots, and generated artifacts | Level 1 ordered catalog/baseline tests with a description-mutation control; fresh Rust/OpenAPI/Postman output compared with committed artifacts | Appropriate; current output has no permanent drift exemption. Historical equivalence remains separate from current artifact comparison. |
| Pagination migration and required managed inputs | Level 1 exhaustive provider resolution/knob coverage, marker/default validation, generated calls, and committed-client pager tests | Appropriate; checks both definition decisions and the emitted caller surface. |
| Query migration and input locations | Level 1 JSON/YAML normalization matrices and compiled generated clients recording requests for the migrated routes, omitted/true/false controls, and GitLab encoding | Appropriate; request behavior is observed at the HTTP boundary. |
| Import assembly and repeatability | Level 1 five-source public import/export test, missing request/response references, compiled Petstore assembly, and byte-for-byte OpenAI re-import | Appropriate; both public import results and emitted code are checked. |
| Guidance and obsolete-surface retirement | Level 1 source/layout guards, authoring-guidance corpus, compiling documentation, and retained/replacement behavior tests | Appropriate; no newly added test is stranded. |

The specification introduces no terminal-rendering or keyboard behavior requiring Level 2 or Level 3 evidence. Existing generator status-output terminal tests are outside these changes. Cross-OS evidence is left to CI and is not a readiness condition for this review.

## Input Robustness Matrix

Default outcome: malformed required definition data fails deserialization, or semantic component/reference validation fails before generation. Malformed import declarations fail before a successful import result. The repair adds component assembly to the existing loaders; it introduces no new file syntax or loader.

Exceptions remain explicit: optional metadata accepts omission/null; absent/null component Rust bindings denote nested-only schemas; empty parameter collections are valid; schema objects and Boolean schemas are valid component data. Imported absent/empty parameter lists are valid, absent `required` means false, absent request body means none, and an empty foundation import may contain no endpoints. GET bodies are rejected. These exceptions do not authorize treating malformed collection members as absent.

The rerun public-result matrices cover component names, schemas, bindings, registry collections and wrapper references (`builders::transferred_components_and_references_input_matrix`); parameter collections and required/optional fields (`definition_serde` and `definition_baseline`); new policy, name, secret and pager fields (`definition_forms`); and JSON/YAML query-path normalization (`import_normalization`). They exercise absent, null, wrong whole type, applicable malformed-element shapes, and empty values. Document checks cover duplicate keys and trailing JSON/YAML content, including a second YAML document. The positive controls establish that edited fields affect the public result.

The source sweep checked Serde defaults, optional fields, `filter_map`, `unwrap_or_default`, and `.ok()` at the changed boundaries. Optional/default outcomes agree with the stated exceptions. Query normalization's collection defaults operate after typed document parsing and raw-text declaration checks; existing malformed-field tests exercise that boundary. JSON form filtering selects field variants rather than discarding failed parses. No new permissive load-bearing parse was found that warrants a finding under this specification.

## Verification performed

All commands ran locally during this review:

- `just test`: 2,287 selected tests passed, 13 skipped by the configured selection; all seven separate fresh-output guards and the separate runtime compile-fail guard passed.
- `just lint`, `just lint-schema`, and `just doctest`: passed.
- `just test-schema`: 55 passed, none skipped.
- `just check-tier-coverage schematic`: zero stranded tests. The generator's consolidated target declares `import_overlay`; the definition crate's OpenAPI feature is enabled by its canonical test configuration.

No production source, generated artifact, baseline, or repair claim was changed by this review.

## Observations

None requiring a separate author follow-up from this review.
