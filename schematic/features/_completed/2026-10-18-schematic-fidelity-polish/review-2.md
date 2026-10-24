---
$schema: feature-review.yaml
ready: true
findings: []
observations:
  - "Inline JSON request schemas remain outside the supported typed import"
  - "Existing runtime TLS tests exceed the Level-1 time budget"
  - "Whole-work closure remains separate from this feature"
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-23T10:22:15-07:00"
spec: 2026-10-18-schematic-fidelity-polish/spec.md
implemented: true
implemented_by: claude/opus
description: "A **feature** review of `2026-10-18-schematic-fidelity-polish/spec.md`"
feature: 2026-10-18-schematic-fidelity-polish/review-2.md
previous: 2026-10-18-schematic-fidelity-polish/review-1.md
---

# Review 2: Schematic fidelity and polish

The feature is **production ready within the accepted scope**. Both previous findings are verified fixed, including every repair-ledger child row. There are no new findings, partial repairs, regressions, or newly discovered siblings requiring repair.

Reviewed the specification, its design table and contract document, review 1, the original implementation log, and the repair log's **Finding Ledger** before assessing the implementation. Review 1's accepted decisions remain in force, including the limited AsyncAPI round trip, deferred payload schemas and generic component naming, and predecessor functionality outside this feature. No new design decision or human confirmation is required.

## Previous Finding Verification

The parent rows record each finding's outcome. Child rows enumerate every required instance from the repair ledger, including its additional wrapper and consumer checks. All cited tests ran in this review's passing `just test` run.

Header evidence is in `schematic-gen`'s [public collection-builder tests](../../gen/tests/l1/postman_runnable.rs:612), [shared fixed-header implementation](../../gen/src/export/postman/request.rs:192), and [GraphQL request projection](../../gen/src/export/postman/graphql.rs:131).

Reference evidence is in `schematic-define`'s [public import tests](../../define/tests/openapi_multipart_import.rs:397), [import normalization](../../define/src/openapi/import/builder.rs:197), [recursive traversal](../../define/src/models/type_ref.rs:248), and [reference-location guard](../../define/src/components.rs:426). Test names below refer to these files unless another source is linked.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: GraphQL Postman requests omit API-level fixed headers | R1-F1.1–R1-F1.6 | verified fixed | Both collection builders retain the owning API's header on REST and both GraphQL operation forms. REST endpoint override precedence is preserved. | Nothing |
| R1-F1.1: Standalone REST control | API fixed header | verified fixed | `every_rest_and_graphql_request_carries_the_api_fixed_headers`: `GetUser` retains version `2`. | Nothing |
| R1-F1.2: Standalone GraphQL without variables | API fixed header | verified fixed | Same test: `Viewer` retains version `2`. | Nothing |
| R1-F1.3: Standalone GraphQL with typed variables | API fixed header and JSON body | verified fixed | Same test: `Issues` retains version `2` and JSON body mode. | Nothing |
| R1-F1.4: Grouped REST controls | Each member's header | verified fixed | `every_shared_module_member_request_carries_its_own_fixed_headers`: REST requests retain versions `2` and `3` for their respective servers. | Nothing |
| R1-F1.5: Grouped GraphQL without variables | Each member's header | verified fixed | Same test: each `Viewer` retains its member's version, including the lowercase header name. | Nothing |
| R1-F1.6: Grouped GraphQL with typed variables | Each member's header | verified fixed | Same test: each `Issues` retains its member's version and address variable. | Nothing |
| R1-F2: Imported model renames miss form parts and nested or API-level references | R1-F2.1–R1-F2.11 | verified fixed | Public imports assert final reference identities. Recursive traversal covers every wrapper variant and the same reference locations validation reads. | Nothing in this finding |
| R1-F2.1: JSON multipart parts | Direct and array references; plain-document control | verified fixed | `renamed_component_in_json_part_is_followed`, its array counterpart, and `renamed_component_in_plain_multipart_part_is_followed` return the renamed component. | Nothing |
| R1-F2.2: JSON success bodies | Direct and array references | verified fixed | `renamed_component_in_json_success_is_followed` and its array counterpart assert the returned reference. | Nothing |
| R1-F2.3: SSE item bodies | Direct and array references | verified fixed | `renamed_component_in_sse_item_is_followed` and its array counterpart assert the returned item reference. | Nothing |
| R1-F2.4: NDJSON item bodies | Direct and array references | verified fixed | `renamed_component_in_ndjson_item_is_followed` and its array counterpart assert the returned item reference. | Nothing |
| R1-F2.5: Endpoint error bodies | Direct and array references | verified fixed | `renamed_component_in_endpoint_error_is_followed` and its array counterpart assert the returned error reference. | Nothing |
| R1-F2.6: API error defaults | Direct and array references | verified fixed | `renamed_component_in_api_error_default_is_followed` and its array counterpart assert the returned API default reference. | Nothing |
| R1-F2.7: JSON request control | Direct named request; distinguish preexisting inline-array loss | verified fixed | `renamed_component_in_json_request_is_followed` retains the final name. Review 1 explicitly excluded inline-array type recovery from this finding. | Nothing in this finding |
| R1-F2.8: Catalog and component controls | Final model names and nested schema references | verified fixed | `renamed_component_keeps_its_final_name_in_catalog_and_component_schemas` checks catalog names and the nested component `$ref`. Catalog rewriting retains its sanitization fallback. | Nothing |
| R1-F2.9: Nested wrappers | Optional, map, array nesting, keyed maps, compositions | verified fixed | Public optional-success, map-default, and nested-SSE tests pass. The reference-location guard checks 12 references including nested `AnyOf`. Exhaustive traversal handles keyed maps and all compositions; public reference validation still rejects unsupported composition types. | Nothing |
| R1-F2.10: AsyncAPI naming sibling | Payload references and catalog names | verified fixed | In `schematic-gen`, [payload naming](../../gen/src/asyncapi_import.rs:729) and [catalog naming](../../gen/src/asyncapi_import.rs:956) use the same version-specific transform, with no request-name reservation or subsequent collision rename. Shipped JSON/YAML round-trip controls pass. | Nothing in this rename class |
| R1-F2.11: Other reference consumers | GraphQL, pagination item types, WebSocket definitions | verified fixed | The 12-reference guard includes GraphQL variables, data and errors. OpenAPI import constructs no GraphQL surface or pagination item reference. WebSocket payloads use `Schema`, not the `TypeRef` identities this repair rewrites. | Nothing |

The repairs update the affected comments and the OpenAPI import and Postman export topic pages. The mutable traversal helpers are crate-private; they add no public API.

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No recurring finding. Both earlier defect classes are fully repaired; this review adds no instance to either class.

## Verification and requirement coverage

Reviewed the existing uncommitted repair diff and the feature's implementation, tests, generated artifacts, documentation, and recorded scope decisions. No implementation or test source was changed during this review.

| Requirement group | Verification inspected and exercised | Appropriate level |
| --- | --- | --- |
| Schema conversion, constraint preservation, closure, inline containers | Bridge tests, `schema_fidelity` registry scan and negative controls, component-reference tests | Level 1 |
| Fixed headers, inherited parameters, multipart bodies | Public export/import fixtures, precedence tests, committed Anthropic assertions and collision cases above | Level 1 |
| OpenAPI extension subset, auth details, standard-field precedence and strict mode | `extension_round_trip`, extension/standard input matrices and JSON/YAML loader checks | Level 1 |
| Postman example validity, saved responses, streams, environments, auth and parameters | `postman_runnable`, golden tests, schema validation, variable checks and artifact drift tests | Level 1 |
| GraphQL requests and named OpenAPI skips | Single/grouped builders check headers, addresses, JSON bodies and skip diagnostics | Level 1 |
| AsyncAPI selectors, fallback, roles, heartbeat, documents and persistence | Five-API JSON/YAML round trips, vendored schema validation, closure/drift checks, persistence and CLI wiring tests | Level 1 within the accepted subset |
| Generated feature selection and documentation | Fresh crates compile with REST, WebSocket, shared, paired and empty feature sets; disabled-item checks and documentation compilation | Level 1 with the Rust compiler as consumer |
| Inventory, naming and hygiene | Fixture registration through consumers, completeness guards, naming/collision tests and hygiene checks | Level 1 |
| Ledger and whole-work checklist | Original implementation log's disposition and open-item tables | Document review; separate work remains explicitly open |

The spec adds no terminal rendering or keyboard behavior requiring Level 2 or Level 3 verification. Mock-server and document checks establish the specified artifact contracts, without establishing live-provider compatibility. Previously accepted scope limitations are not reopened as missing tests.

Commands independently run for this review:

- `just test` in `schematic`: **passed** — 2,993 regular tests, 16 fresh-output/compilation tests, and one compile-fail guard. The regular suite reports 22 skips; compilation recipes run their selected checks separately.
- `just lint` in `schematic`: **passed**.
- `just check-tier-coverage schematic`: **passed**, zero stranded tests. Generator tests are declared by the consolidated `l1` target; definition tests are auto-discovered and run with `openapi` enabled.
- `just doctest` in `schematic`: **passed**, including generated-crate default features and the selected `emqx` module.

Other operating systems, dependent suites, `just test-schema`, and `just lint-schema` were not independently rerun. Their earlier evidence remains in the implementation log; the fresh-output compiler checks above were rerun. Cross-OS proof is outside this review's readiness decision.

## Input Robustness Matrix

The OpenAPI and AsyncAPI readers were checked at their public import boundaries. Malformed standard fields produce errors; malformed optional Schematic metadata produces located warnings and is ignored, with strict OpenAPI import rejecting warnings. Exceptions retain their accepted contracts, including absent-server fallback, anonymous absent security, permitted empty selectors and writer-emitted null metadata.

The passing OpenAPI extension and standard-field matrices exercise absent, null, wrong whole-field and element types, and empty values with positive controls. Loader checks cover duplicate keys, trailing content, second YAML documents and bare-key YAML nulls. The AsyncAPI matrix exercises shared field readers and version-specific rules. Persistence tests verify failed imports and dry runs publish nothing; CLI tests verify command wiring.

The requested smell search covered `serde(default)`, optional-value handling, string filtering, `.ok()` and `unwrap_or_default()` in the extension reader, reconciliation, shared AsyncAPI reader and consuming import paths. Reviewed fallbacks follow diagnostics or documented absence rules. The repair changes reference normalization after decoding, rather than loading or field validation; its public collision tests verify returned identities and later reference validation. No new malformed-input finding was identified.

## Observations

### Inline JSON request schemas remain outside the supported typed import

Review 1's observation remains applicable: `schematic-define`'s [request-schema reader](../../define/src/openapi/import/mappings/responses.rs:488) recognizes a named component and otherwise falls back to untyped JSON. This repair fixes references that survive decoding; it does not add inline-array type recovery. That behavior predates the feature and remains outside the repair inventory.

### Existing runtime TLS tests exceed the Level-1 time budget

Four unchanged certificate tests in `schematic-runtime`'s [builder tests](../../runtime/tests/l1/builder.rs) took 5.834–7.500 seconds in this run. They passed but exceed the repository's five-second Level-1 budget. This feature did not change them.

### Whole-work closure remains separate from this feature

The original implementation log distinguishes this feature from the sniff handwritten-client migration, the predecessor definition-vocabulary implementation review, and the earlier base-URL composition check. Marking this feature ready does not close those tasks or expand the accepted AsyncAPI subset.
