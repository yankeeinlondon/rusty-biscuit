---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-12T09:43:15-07:00"
spec: "2026-10-11-schematic-generation-integrity/spec.md"
feature: "2026-10-11-schematic-generation-integrity/review-3.md"
previous: "2026-10-11-schematic-generation-integrity/review-2.md"
implemented: false
description: "A **feature** review of `2026-10-11-schematic-generation-integrity/spec.md`"
ready: true
findings: []
observations:
  - "Postman cookie execution has a documented verification limit"
human_review: false
has_blocked_findings: false
blocked: false
refresh: refreshed
refresh_changes:
  - "Replaced the next spec and plan's prerequisite assumptions with verified request parts, authentication helpers, compatibility checks, and fresh-output test interfaces."
  - "Corrected the plan's assumed serialized inventory: current inventory reads generated Rust, and the runtime feature must introduce the persisted ABI marker."
  - "Updated current endpoint counts to 547, recorded feature counts and stale-tree deletion, and carried forward sniff's migrated GitHub and Bitbucket behavior."
  - "Carried forward the author's withdrawal of REST selectors without changing runtime scope or pipeline order."
  - "Added dated refresh logs and the next implementer's handoff note, and cleared the next spec's refresh_required flag."
---

# Review 3

The feature is **production ready** under its specification and the author's rulings. Both findings from review 2 are implemented, including its nonblocking nested-array finding. This review found no new high or medium defect within the feature's scope. No new human decision is required.

## Previous findings and rulings

### Postman optional inputs are preserved

**Authority checked: spec R10**, “A feature one output lacks fails validation for that output only; the others still generate.” Review 2 explicitly allowed supporting the missing inputs instead of rejecting the output. The repair chose support.

In `schematic-gen`, [build_headers](../../gen/src/export/postman/request.rs) now preserves declared headers and cookies, their descriptions, and their requiredness: optional entries are disabled until the user enables them. The warning-only unsupported-input detector is removed. The seven shipped OpenAI `openai-beta` header inputs appear in the regenerated collection. The cookie API-key projection also uses Postman's header placement rather than an unsupported cookie placement.

The defect class was an input projection silently omitting declared data depending on optionality. The public-result test [header_and_cookie_parameters_reach_every_postman_writer](../../gen/tests/l1/output_validation.rs:374) exercises all writer entry points; the batch test also checks independent Rust and OpenAPI success. The live OpenAI CLI validation succeeds for all outputs and retains only the embedded-query warnings permitted by R33.

| Sibling site | Shape checked | Result |
| --- | --- | --- |
| Batch Postman output, single-API writer, grouped writer | Required and optional header inputs | Present with correct enabled/disabled state; schema validation passes |
| Same three writers | Required and optional cookie inputs | Present as Cookie header entries with correct enabled/disabled state; schema validation passes |
| Postman URL projection | Declared query inputs | Existing query entries retained; query/golden tests pass |
| Postman body projection | JSON, multipart, URL-encoded, text, binary | Existing conversion tests pass; no optionality-dependent omission |
| Postman authentication projection | Header, query, cookie API keys; bearer and Basic | Mapping tests pass; cookie keys now use a Cookie header |
| Rust/OpenAPI batch projections | The same header/cookie fixture | Both succeed; OpenAPI preserves parameter locations and requiredness |

The docs and Schematic skill now describe the supported Postman inputs rather than warning-only omissions. Collection execution remains bounded by the observation below; document-shape tests are not claimed as Postman execution tests.

### Nested arrays are rejected at every Rust parameter location

**Authority checked: spec R26**, “reject unsupported nested array shapes during validation rather than dropping them.” In `schematic-gen`, [check_sent_params](../../gen/src/validation.rs:546), which checks the parameters a generated client sends, now includes query parameters alongside headers and cookies. The new `query-parameter` diagnostic identifies the API, endpoint, feature, and Rust output.

The defect class was inconsistent validation of the same unsupported shape across parameter locations. The public validators and batch-result tests in [output_validation.rs](../../gen/tests/l1/output_validation.rs:499) pass the complete location sweep:

| Sibling site | Shape checked | Result |
| --- | --- | --- |
| Rust query validation/projection | `Array(Boolean)` / `Array(Array(Boolean))` | Flat accepted; nested rejected before publication |
| Rust header validation/projection | Same pair | Flat accepted; nested rejected |
| Rust cookie validation/projection | Same pair | Flat accepted; nested rejected |
| OpenAPI parameter projection | Nested arrays at all three locations | Accepted as recursive schema data; Rust restriction does not block this output |
| Postman parameter projection | Nested arrays at all three locations | No generated Rust type to compile; entries remain exportable |
| Public batch publication | Nested query input with an existing Rust output | Rust fails without modifying prior files; OpenAPI and Postman succeed |

There are no nested arrays in shipped definitions. The validation and request-side docs explain the restriction and per-output behavior. This completes the medium finding without adding nested-array serialization.

### Earlier author rulings remain implemented

Review 2 contains no human rulings. The rulings carried forward from review 1 are also satisfied: duplicate same-API method/path routes are errors in all outputs; GitHub has one `GetGitTree` with optional `recursive`; Bitbucket has one binary `GetSource`; the obsolete five-file tree is deleted and its completion is recorded. The definition, response-model, and fidelity follow-ups carry the required endpoint and response-model changes.

The author's **review-1.md, item 1** ruling overrides the spec's proposed REST-selector extension: endpoint identity remains method plus normalized path permanently. Response distinctions and parameter-location rules assigned to later features are not new implementation requirements here. The narrowly related `sniff` migration remains the departure from the original generator-only scope already evaluated in review 2; this review does not request another confirmation.

## Findings

None.

## Observations

### Postman cookie execution has a documented verification limit

The new Postman mapping writes separate Cookie header entries for multiple cookie parameters and can add another through cookie-key authentication. The docs explicitly distinguish this from the generated Rust client's single merged Cookie header. The tests validate exported entries against the vendored Postman schema but do not execute a collection through Postman to establish how it combines these entries or resolves collisions with static headers. No shipped definition uses cookie parameters or cookie-key authentication, and R23–R26's request-execution guarantees apply to compiled Rust clients, where local-server tests prove merging and precedence. Keep this limitation documented; a focused consumer-execution check is useful when a real collection needs these combinations. This review does not require a new merge mechanism or a synthetic payload matrix.

## Verification and fidelity

Executed on the current macOS host:

- `just test` in `schematic/`: **1,780 main-suite tests passed**, 9 excluded/ignored; all **four fresh-output checks passed**. Main-suite execution took 2.641 seconds; the separate fresh-output run took 17.808 seconds.
- `just lint` in `schematic/`: passed.
- `just check-tier-coverage schematic`: zero stranded tests.
- Shipped `schematic-gen validate --api openai`: exit 0; Rust, OpenAPI, and Postman pass, with only the expected embedded-query warnings.

The new test modules are declared by the consolidated L1 target, and the four `slow_` checks are selected by the live `test-fresh-output` recipe that `just test` invokes. No application windows were opened. No implementation or generated artifact was edited for this review.

| Requirements | Boundary and strongest evidence | Assessment |
| --- | --- | --- |
| R1–R7, R34: partial groups, ownership, retained exports, refusal, publication reporting, dry run | Actual temporary filesystem and CLI observations; retained-plus-regenerated crate compiled by rustc (F2) | Appropriate; includes refusal without mutation and publication failure/retry |
| R8–R13, R33: scoped validation, independent outputs, names, route identity | Manufactured definitions and diagnostic assertions (F1); public batch/CLI exit status and actual artifacts (F2 for execution/filesystem) | Appropriate for generator-owned validation rules; earlier severity defect repaired |
| R14–R15, R20: unique operation IDs, reference updates, deterministic artifacts | Document/link assertions and fresh-to-committed comparisons (F1); Postman documents accepted by independent jsonschema code using the vendored upstream schema (F2 for schema conformance only) | Appropriate for specified document invariants; no provider or Postman execution claim |
| R16–R19: isolated offline compilation and coverage | Cargo/rustc consume real fresh output; child nextest runs compiled fixture clients; injected type error fails with compiler evidence; checkout witness remains unchanged (F2) | Appropriate; normal recipe selects all four guards |
| R21–R26: body bytes, content types, headers, cookies, keys, policies | Fresh compiled clients send to dynamically bound loopback servers; received bodies/headers/query values and zero-send failures asserted (F2) | Appropriate for generator-defined request mechanics; nested array refusals checked at all locations |
| R27: credential Debug suppression | Sentinel assertions in the compiled fixture (F1 for formatting logic, exercised in real compiled output) | Appropriate for this narrow request-input contract; broader redaction deferred |
| R28–R29: query separation and byte-equivalent URLs | Compiled clients send to a raw loopback recorder against pre-split request-line goldens (F2) | Appropriate for equivalence to the old generator, not evidence of provider-originated input |
| R30–R32: cleanup, regeneration, consumer compatibility, counts | Deleted paths inspected; current fresh-output/artifact checks pass; prior scoped dependent results and migrations recorded in the logs | No new blocking gap; 547 current endpoint impls after the two author-required merges |
| R35: build-owner compile checks and archive exclusion | Workflow guard precedes archive production; consumer execution and expected listings share slow-test exclusion; configuration contract test passes (F1) | Placement verified; hosted and additional OS results remain CI evidence, not review blockers |

No requirement here needs live-provider F3 evidence: the spec deliberately uses local servers for generated request mechanics. Provider-specific behavior remains an assumption, including GitHub's recursive query and Bitbucket's directory/file response distinction. The review does not claim those live-provider interactions were tested.

The configuration robustness matrix is **not applicable**. This feature parses generated Rust inventory, writes manifests, and delegates lockfile interpretation to Cargo; it adds no configuration-field deserializer. Rust inventory corruption and missing evidence have separate refusal tests.

## Downstream refresh and closure

Evaluated the next runtime spec and plan against the implementation logs, all previous reviews and rulings, deferred work, the parent decisions, and the next spec's refresh notes. Updated factual handoff details: the five-part request tuple, shared authentication resolver, existing compile helper, byte-based compatibility function, absence of a persisted ABI marker, stale-tree deletion, measured feature counts, and the migrated `sniff` baseline.

The next plan had assumed serialized inventory fields existed. Its verification section now targets the generated Rust compatibility marker that the runtime feature already owns; it preserves existing ownership/export checks and requires the full configuration matrix if implementation actually introduces a configuration reader. No settled runtime design, new scope, pipeline order, or author ruling was changed. Both downstream documents have dated refresh logs; the next spec has `refresh_required: false` and a handoff message. No contested refresh item remains.

Review 2 is linked to this review and marked implemented. The current spec records iteration 3, `status: implemented`, and `completed: true`. Its historical design text remains a snapshot. The feature directory has not been moved, and no commit was made.
