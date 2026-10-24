---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (high): GraphQL Postman requests omit API-level fixed headers"
  - "R1-F2 (medium): Imported model renames miss form parts and nested or API-level references"
observations:
  - "Inline JSON request schemas still lose their type during import"
  - "Existing runtime TLS tests exceed the Level-1 time budget"
  - "Whole-work closure remains open outside this feature"
human_in_the_loop: null
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-23T09:47:07-07:00"
spec: 2026-10-18-schematic-fidelity-polish/spec.md
implemented: true
description: "A **feature** review of `2026-10-18-schematic-fidelity-polish/spec.md`"
feature: 2026-10-18-schematic-fidelity-polish/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-18-schematic-fidelity-polish/log.md
implemented_by: claude/opus
next: 2026-10-18-schematic-fidelity-polish/review-2.md
---

# Review 1: Schematic fidelity and polish

The feature is **not ready** because generated GraphQL Postman requests discard API-level fixed headers. One additional, nonblocking finding covers incomplete reference updates when import renames a model. Neither repair needs an author decision.

Two separate reviewers examined the specification and implementation log before implementation review began. Both found recommendations or explicit scope rulings for every open choice. Course 2 applies: adopt those choices, record the authorized deferrals, and continue. There are no previous implementation-review findings to carry forward.

## Decisions adopted for author review

The following decisions are accepted for this review. Alternatives are recorded so the author can revisit them through an ordinary change; none requires stopping the unattended pipeline. Sources are the specification's descriptive requirement and planning headings and the implementation log's phase sections.

1. **Use AsyncAPI 3.0 and a separate document for each WebSocket API.** This follows the October 21 author ruling and “AsyncAPI version and supported subset.” A lone API uses its module name; APIs sharing a module add a stable API-name suffix. Grouping APIs into one document would require extra identity and security metadata; exporting 2.x would abandon the selected output version.

2. **Describe the client application's view for every WebSocket endpoint role.** Client messages become send operations, server messages receive operations, and bidirectional messages both. The role remains channel metadata. One operation per message and action preserves each message's reply declaration; one operation per channel would lose that distinction. Sources: “AsyncAPI version and supported subset” and implementation log, Phase 10.

3. **Keep the documented AsyncAPI subset.** Selectors, fallback markers, endpoint roles, handshake parameters, supported applied authentication, and heartbeat hints survive. Correlation, authentication-flow hints, lifecycle roles, broader runtime hints, and environment-variable metadata remain outside the subset. Lifecycle messages become ordinary messages. Expanding the subset is additive work. In particular, the Core, Dock, and Integration documents re-import but cannot generate working clients without their omitted correlation settings. This is a recorded subset limitation, not a promise of reversible client generation. Sources: the AsyncAPI planning default and Phase 10's per-API round-trip table.

4. **Defer typed WebSocket payload schemas under the explicit definition-edit restriction.** Exported payload components describe their Rust type but otherwise accept any JSON value. Adding schema derives to provider payload types exceeds the authorized API-constructor edits; the October 21 ruling says to leave such changes unmade, log them, and continue. Retain this as an authorized gap against the desired typed payload output. The alternative is a separately scoped model change followed by regeneration. Do not describe the current payload components as validating message fields. Sources: “Pre-authorizations,” Phase 1's adopted defaults, and Phase 10.

5. **Use small Schematic extensions for WebSocket details absent from standard fields.** The chosen names are `x-schematic-selectors`, `x-schematic-fallback`, `x-schematic-role`, `x-schematic-heartbeat`, `x-schematic-param`, `x-schematic-value-prefix`, and `x-schematic-bearer-header`. Parameter metadata preserves secret flags and Rust-side names/types. Heartbeats are imported as well as exported. Dropping these details would break the selected round trip. Sources: the selector, role, and heartbeat requirements and Phase 10's extension table.

6. **Preserve legacy AsyncAPI import while following applied security.** Both supported version families use the server's applied security rather than the first declared scheme. A missing API-key name errors; unsupported credential locations warn and remain anonymous. A 3.0 operation without a message list covers all channel messages with an informational diagnostic; missing descriptions become empty strings. These departures from the frozen planning matrix have defined outcomes and match the chosen format interpretation. Alternatives would retain unused credentials or manufacture descriptions. Source: Phase 10.

7. **Persist reusable AsyncAPI definition, model catalog, and diagnostics files through staged publication.** Dry runs write nothing. A vendored, attributed AsyncAPI 3.0 JSON Schema validates documents offline. Count-only import summaries or structural checks without schema validation were weaker alternatives; the implementation supplies both reusable output and validation. Sources: “The importer persists its returned definition and catalog,” “A generate recipe and committed artifacts,” and Phase 10.

8. **Use document-level integer version 1 for OpenAPI extensions.** Absence means version 1; unsupported or malformed versions disable all Schematic metadata with a warning. Known members decode separately. Standard OpenAPI declarations remain authoritative; unknown members are informational, malformed known values warn, and strict import fails on warnings/errors. This adopts the extension-version and strict-mode defaults instead of adding another option or discarding unrelated valid metadata. Sources: the version, conflict, diagnostic, and strict-mode requirements and Phase 4.

9. **Keep only additional authentication details in extensions.** Bearer header overrides and API-key prefixes survive; ordinary authentication method, names, locations, and scopes use standard security declarations. Environment-variable names remain configuration metadata, never credential values. Secret flags, Rust parameter names, and reserved-expansion path metadata also round-trip. The unused schema/auth extension types were deleted, while active document/operation writers have a separate reader. Keeping a second complete authentication representation would create conflicting sources. Sources: “Auth details OpenAPI cannot express round-trip,” the supported-subset default, and Phase 4.

10. **Keep pagination outside OpenAPI extension version 1.** This is the logged planning default; a later extension can add it. GraphQL operations remain outside OpenAPI and receive named skip diagnostics, while Postman exports them. The alternative would broaden this extension version beyond the adopted subset. Sources: Phase 1's subset decisions and the GraphQL export requirement.

11. **Retain the chosen conflict and error-attribution rules.** Conflicting document error metadata is withdrawn with a warning; malformed modern success metadata cannot fall back to its legacy copy. An extension reference needed to fill missing information must resolve; standard information can take precedence over an unusable extension with a warning. Semantic structural problems can return located error diagnostics in permissive import, while malformed document shapes fail construction. Missing servers now warn and therefore fail strict import. These rules preserve standard-field precedence rather than silently merging contradictory metadata. Source: Phase 4.

12. **Accept the narrow null-default annotation loss.** The OpenAPI model library reads an explicit JSON null default as absent. Keeping that annotation through a separate representation would break the existing document-equals-its-own-parse invariant. Non-null defaults remain, and nullable/optional value behavior is tested. This is annotation loss, not evidence that every default survives. Unannotated references remain direct references; references needing sibling constraints or annotations receive a composition wrapper. Sources: Phase 2's recorded departure and bridge decisions.

13. **Keep closure validation as a generation/export backstop.** Transitive schema-reference checks stay separate from endpoint-reference validation so existing imports can report their unresolved nested references without a new unconditional construction failure. Generation and export still reject incomplete closure. Alternative placement would change permissive import behavior. Source: Phase 2.

14. **Defer the nine Bitbucket generic component names containing angle brackets.** They are concrete generic structs, not container registrations, but their names violate the intended OpenAPI component-name restriction. Renaming provider model registrations exceeds the explicit definition-edit scope. The exact known set is pinned; accepting the authorized deferral does not make those names portable OpenAPI component keys. A follow-up can rename registrations and regenerate references together. Sources: “Pre-authorizations” and Phase 2's component-name departure.

15. **Export constant headers inline on operations.** Endpoint values override API values without case sensitivity; effective authentication identifies credential headers. Compatible ordinary parameter declarations merge with constants; reserved names and incompatible declarations produce diagnostics while extension metadata retains the original facts. Shared parameter components would add machinery without improving this contract. Seventeen recorded header warnings remain where standard fields cannot faithfully express the declaration. Source: Phase 3.

16. **Retain multipart compatibility choices and the two OpenAI re-import differences.** Explicit JSON part encoding creates structured parts; object parts without it stay text with a warning. The vendored OpenAI version and realtime session JSON-part difference remain pinned instead of rewriting generated provider definitions outside the allowed scope. Inferring JSON or rerunning the import would change existing generated client types. This accepted choice does not excuse the independently reproducible rename failure in R1-F2. Source: Phase 4.

17. **Use one runtime registration inventory in catalog order and one naming resolver.** The resolver lives in `schematic-definitions` and preserves the Rust assembler's suffix behavior across writers. Existing selection names are preserved; no unused alias mechanism was added and no artifact needed renaming. The existing batch-generation interface remains usable by fixture callers. Split per-module test declarations retain completeness guards to keep tests small; they still need maintenance when modules are added. Separate runtime lists or changing public module spellings would reintroduce drift. Sources: the single-list/naming defaults and Phases 5–6.

18. **Use one Cargo feature per generated module, with every module enabled by default.** WebSocket-only dependencies become optional; shared dependencies stay unconditional. Every regeneration, including a partial run, derives the manifest from the full inventory. Per-API features cannot separate APIs sharing a module; an extra all-APIs feature is unnecessary. Source: the feature-granularity default and Phase 7.

19. **Keep the endpoint methods delivered by the predecessor feature.** Generated examples show those existing methods and the typed send alternative. This feature adds no new wrapper-method layer. Reverting working predecessor methods merely to preserve the older accepted-gap wording would undo separately implemented functionality. The ledger correctly identifies that the old gap is superseded in practice. Sources: Phase 1, Phase 8, and the final accepted-gap table.

20. **Compile documentation through the existing recipes.** Executable generated examples use compiling fences; default and selected module configurations are checked. The definition README is included through a doctest-only item, an equivalent way to compile its examples without expanding crate-root documentation. Sources: the documentation requirements and Phases 8 and 11.

21. **Generate deterministic Postman examples and validate before emission.** Use the existing `jsonschema` library offline against registered JSON schemas, selecting explicit examples/defaults before deterministic placeholders and checking complete composition constraints. Its production dependency is justified because validation protects emitted examples, rather than only tests. No general pattern solver is promised. Schema-valid placeholders may still be unhelpful: Ollama can select an error alternative and a permissive OpenAI request schema can yield an empty object. Improving authored models/examples is separate work. Source: the example-source default and Phase 9.

22. **Keep Postman variable and artifact conventions.** Parameter variables receive location prefixes; shared-module members receive separate server and credential variables. Optional cookies are described rather than automatically enabled; a wholly optional Cookie header is disabled. Cookie values must already use the documented encoding. Environment export is optional, environments sit beside collections, and responses stay inside collections. These choices avoid multiple cookie headers and accidental credential sharing. GraphQL without variables exports an empty variables object as requested by the spec. Sources: the environment/header-cookie requirements, artifact-placement default, and Phase 9.

23. **Record predecessor and whole-work gaps without expanding this feature.** Optional Ollama authentication, broader authentication-source cleanup, source-unverified Hugging Face shapes, and orphan registrations remain accepted gaps. The old wrapper-method entry is superseded as described above. The sniff migration, predecessor verification review, and earlier base-URL composition check remain open. No dependent source, CI workflow, live provider, lifecycle directory, or commit was changed. These are scope boundaries, not requests to halt this review. Sources: “Pre-authorizations,” the ledger/checklist requirements, and Phase 13.

24. **Record the remaining import corner cases without inventing a larger protection layer.** The log notes dropped null members in string-parameter enums, file-count bounds above the model's 32-bit limit, and a missing location on the error for a second YAML document. The latter document is still rejected. These unusual-input or preexisting limitations do not justify a blocking fix cycle here. The separately reproduced model-renaming issue is included in the ordinary findings rather than hidden in this list. Source: Phase 4's review agenda.

25. **Leave unrelated utility cleanup separate.** The log identifies a stale install recipe, old stream wording in API-creation guidance, an unused generated stream dependency, and a vendor-named GraphQL address helper outside the pagination-constructor prohibition. These do not change the artifact contracts reviewed here. Broadening the implementation into utility/dependency cleanup would exceed its surgical scope. Sources: Phases 1, 7, and 12 and the final open-item list.

## Findings

### R1-F1 (high): GraphQL Postman requests omit API-level fixed headers

**Authority — spec:** “GraphQL operations export to Postman and are skipped by OpenAPI” requires each operation to be a **“runnable `POST`”**, and the feature goal requires external artifacts to remain faithful to definitions. An API with a required fixed version/protocol header is an ordinary supported definition. Its exported GraphQL request must include that header to be runnable against the service.

**Defect class:** one request projection drops API-level request metadata that the other request projections preserve.

In `schematic-gen`, the [GraphQL request builder](../../gen/src/export/postman/graphql.rs:123) explicitly starts with no headers. It never applies the API's fixed headers. The [REST header builder](../../gen/src/export/postman/request.rs:132) copies them correctly. Consequently, an API declaring `X-Protocol-Version: 2` produces REST requests with that header and GraphQL requests without it. Collection-level authentication does not supply ordinary fixed headers.

**Reproduction:** copy the existing `graphql_api` fixture in `schematic-gen`'s `postman_runnable` tests, add the single API header `X-Protocol-Version: 2`, and call the public single-collection builder. Repeat through the grouped builder with a second member using the same header. Inspect the serialized request headers for the fixture's REST request, variable-free GraphQL operation, and typed-variable GraphQL operation. All GraphQL requests lose the header; every REST control keeps it.

| Instance | Site in `schematic-gen` | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F1.1 | Single collection, REST request | API fixed header | Header retained; clean | Retain header |
| R1-F1.2 | Single collection, GraphQL without variables | Same API/header | Header absent | Retain header |
| R1-F1.3 | Single collection, GraphQL with typed variables | Same API/header | Header absent | Retain header |
| R1-F1.4 | Shared-module collection, REST requests | Both members declare the header | Both retain it; clean | Each retains its member's header |
| R1-F1.5 | Shared-module collection, GraphQL without variables | Both members declare the header | Both omit it | Each retains its member's header |
| R1-F1.6 | Shared-module collection, GraphQL with typed variables | Both members declare the header | Both omit it | Each retains its member's header |

This covers both public collection-building paths and every request projection that reads API headers. Environment-file generation consumes the built collection and cannot restore omitted request headers. The existing tests check GraphQL method, URL, JSON fields, responses, and OpenAPI skip diagnostics, but their GraphQL fixture has no API headers.

**Repair:** apply the API's effective fixed headers when building GraphQL requests, preserving GraphQL-specific media-type handling. Extend the existing fixture test with the required header and check standalone and shared collections. No new provider call or terminal test is needed.

### R1-F2 (medium): Imported model renames miss form parts and nested or API-level references

**Authority — spec:** “Multipart request bodies are imported” requires the imported multipart body to export equivalently. “The importer reads `x-schematic` for the supported subset” requires semantic export/import/export equality, including the declared response/error metadata. Renaming a component to avoid a generated request-type collision is allowed normalization; leaving references pointing to its old name is not.

**Defect class:** model identity is renamed in the catalog without updating every definition reference that later closure validation reads.

In `schematic-define`, the [import build step](../../define/src/openapi/import/builder.rs:197) updates endpoint references before validating them. Its [reference-remapping helper](../../define/src/openapi/import/builder.rs:233) handles only direct named JSON request/success/stream/error references. It does not descend through containers, visit JSON form parts, or visit API-level error defaults. The newly imported form and extension metadata can therefore fail reference validation after an otherwise valid import.

**Reproduction:** copy the ElevenLabs-shaped multipart fixture in `schematic-define`'s `openapi_multipart_import` tests. A JSON-encoded `metadata` part referencing component `Metadata` imports successfully. Change the component name and its reference to `AddVoiceSampleRequest`, which collides with the request type reserved for operation `AddVoiceSample`. The component becomes `AddVoiceSampleRequest2`; the form part still names `AddVoiceSampleRequest`. The public import returns `Definition(UnresolvedReference { api: "ElevenLabs", endpoint: "AddVoiceSample", name: "AddVoiceSampleRequest" })`.

The same component and collision were then exercised through public export/import in every definition-reference location reached by the form/extension changes. Each cell used a direct reference and an array containing that reference. Direct references without the collision provided the positive form control; direct references in existing remapped locations provided collision controls.

| Instance | Site in `schematic-define` | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F2.1 | JSON multipart part | Direct and array-contained renamed component | Both fail with unresolved old name | Both point to final component and import |
| R1-F2.2 | JSON success shape | Direct / array-contained component | Direct imports; array fails | Both import with final reference |
| R1-F2.3 | SSE item shape restored from extension | Direct / array-contained component | Direct imports; array fails | Both import with final reference |
| R1-F2.4 | NDJSON item shape restored from extension | Direct / array-contained component | Direct imports; array fails | Both import with final reference |
| R1-F2.5 | Endpoint JSON error shape | Direct / array-contained component | Direct imports; array fails | Both import with final reference |
| R1-F2.6 | API-level JSON error defaults | Direct and array-contained component | Both fail, identifying endpoint `<api>` | Both import with final reference |
| R1-F2.7 | JSON request body | Direct component / inline array | Direct correctly becomes `AddVoiceSampleRequest2`; inline array becomes untyped JSON | Direct is clean; separate preexisting array loss is an observation below |
| R1-F2.8 | Model catalog and component-schema references | Same colliding name | Component and its schema references receive final names; clean | Keep final names consistent |

The later accepting/rejecting decision was included: all failed cases reach the public reference-validation error, not merely a private helper discrepancy. The helper's top-level named-only branch also rules out recursive renaming for optional/map/composition wrappers; fixing only arrays would leave the same class behind. Text, binary, empty responses, byte streams, and non-JSON form fields carry no nominal model reference and cannot exhibit this defect. URL-encoded import currently maps parts to text, so it does not introduce another nominal-part site. GraphQL is excluded from OpenAPI import by the spec.

**Repair:** reuse a recursive reference rewrite across the supported definition locations, including JSON form parts and API error defaults, then keep public import tests for the instance rows. This is medium because it requires a component/request-name collision combined with these less common reference locations; shipped artifact round trips pass. It does not block readiness by itself.

## Verification and requirement coverage

Reviewed the specification, implementation log, relevant implementation commits, changed exporters/importers, generated packaging/docs, committed artifacts, and the tests that exercise those boundaries. The tree was clean before review. No source fix was made.

| User-facing requirement group | Strongest relevant verification inspected/run | Level appropriate? |
| --- | --- | --- |
| Schema types, nullable values, compositions, constraints, maps, reference closure, inline containers | Public export tests, accepted/rejected value checks, and all-registry fidelity/closure scans | Level 1; yes, subject to the explicitly authorized naming gap |
| Constant headers and precedence | Fixture exports and committed Anthropic artifact assertions | Level 1; yes |
| OpenAPI extension equality, versions, standard-field precedence, strict diagnostics | Committed-artifact and focused public round trips; JSON/YAML field/document matrices | Level 1; yes; R1-F2 adds a missing collision combination |
| Inherited parameters and multipart imports | Public import/export fixture tests, including file arrays and encoding diagnostics | Level 1; yes; R1-F2 identifies a missing reference-rewrite case |
| Postman example bodies, saved responses, stream framing, auth, parameters, environments | Original-schema validators, golden/fixture tests, whole shipped-corpus checks, drift tests | Level 1; yes |
| GraphQL Postman requests and OpenAPI omission | Public fixture projections and named skip diagnostics | Level 1 is sufficient, but required API-header forwarding is missing: R1-F1 |
| AsyncAPI documents, selectors, roles, heartbeat hints, supported round trips | Five-API JSON/YAML round trips, offline document-schema validation, closure/drift checks | Level 1; yes within the documented subset and payload-schema deferral |
| AsyncAPI persistence and dry-run/publication behavior | Temporary-directory public API tests and shipped CLI wiring tests | Level 1; yes |
| Module features, partial regeneration, unified inventory/naming | Fixture propagation, fresh generated-crate compilation, feature-selection compilation, artifact drift | Level 1; yes |
| Generated Rust examples and definition README | Compiling generated examples with default/selected features; README doctest inclusion and logged doctest run | Level 1; yes |
| Ledger, dependent evidence, hygiene, whole-work checklist | Recorded disposition/checklist tables and executable hygiene checks | Reporting obligations met; open whole-work items remain explicitly open |

The spec explicitly says this work introduces no Level-2 requirement. It adds no keyboard, mouse, scrolling, or terminal-rendering promise requiring emulator or OS-input verification. Existing terminal-output tests are separate from these artifact contracts. Cross-OS proof is outside this readiness decision.

Commands run during this review:

- `just test` in `schematic`: **passed** — 2,972 regular tests, 16 fresh-output/compilation tests, and one compile-fail guard. The regular suite reports 22 skipped tests. The separate compilation recipes run their selected tests successfully.
- `just check-tier-coverage schematic`: **passed**, zero stranded tests. New test modules are declared in the consolidated generator target; the reviewed feature tests use live recipe tiers.
- Scoped temporary probes through the canonical `_test_all`/`_test` recipes: reproduced R1-F1 and R1-F2. Three probe tests intentionally failed their expected-behavior assertions; the extended rename probe also exposed the preexisting inline-request limitation below. All temporary test edits were removed byte-for-byte after collecting results.

The existing suite passed before the probes. Their failures demonstrate cases absent from that suite; they are not new permanent failing tests. Lint, full doctests, dependent suites, and other operating systems were not independently rerun in this review. The implementation log records their results, and the fresh-output documentation/feature checks did run as part of `just test`.

## Input robustness matrix

This applies to the OpenAPI and AsyncAPI readers, not to schema/example writers or Rust documentation rendering. The review inspected the public JSON/YAML matrix tests and their loading/field-validation code. Those tests ran in the passing Level-1 suite.

The established default is a located error for malformed standard shapes, with no publication; malformed optional Schematic metadata instead warns and is ignored, and strict OpenAPI import rejects those warnings. Exceptions include documented absent-server defaults, anonymous absent security, explicit empty selector lists, unsupported metadata warnings, and valid empty selector values. The tests distinguish absent, null, whole-field wrong types, malformed collection elements, and empty values where the field permits them. Loader tests reject duplicate keys, trailing garbage, and extra YAML documents, and exercise YAML bare-key null syntax.

The requested smell search covered defaults, optional values, string-filtering, parse `.ok()`, and empty fallbacks in the changed readers. Reviewed fallbacks generally follow an emitted diagnostic or an explicitly documented absence rule. R1-F2 concerns valid reference identities after normalization, rather than an invalid field shape; its table therefore follows those identities through catalog construction and the public acceptance/rejection decision instead of creating a redundant malformed-input matrix.

## Observations

### Inline JSON request schemas still lose their type during import

The rename sweep's array-shaped JSON request returned success but became untyped JSON. In `schematic-define`, [the standard request-schema reader](../../define/src/openapi/import/mappings/responses.rs:488) recognizes named references and otherwise falls back to an untyped value. That JSON mapping predates this feature; the multipart/extension work does not newly expose JSON request arrays. Treat this as information for a future importer change, not part of the current repair inventory. It also means a successful import alone is insufficient proof of semantic equality for arbitrary inline request schemas.

### Existing runtime TLS tests exceed the Level-1 time budget

Four existing certificate tests in `schematic-runtime`'s [client builder tests](../../runtime/tests/l1/builder.rs) took approximately 5.5–7.1 seconds in this review's regular suite. The implementation log recorded the same slow tests. This feature did not change them, so they are outside its findings; their timing still exceeds the repository's five-second Level-1 budget.

### Whole-work closure remains open outside this feature

The log correctly leaves the sniff handwritten-client migration open, along with the predecessor definition-vocabulary implementation review and the earlier base-URL composition check. The author also retains the recorded predecessor accepted gaps. Passing this feature's eventual repair review will not prove those separate tasks complete. No changes to sniff or lifecycle directories are requested here.
