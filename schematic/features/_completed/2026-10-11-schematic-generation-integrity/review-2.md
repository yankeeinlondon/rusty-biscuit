---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-12T09:22:52-07:00"
spec: "2026-10-11-schematic-generation-integrity/spec.md"
feature: "2026-10-11-schematic-generation-integrity/review-2.md"
previous: "2026-10-11-schematic-generation-integrity/review-1.md"
log: "schematic/features/2026-10-11-schematic-generation-integrity/log.md"
implemented: true
implemented_by: claude/opus
description: "A **feature** review of `2026-10-11-schematic-generation-integrity/spec.md`"
ready: false
findings:
  - "high: Unsupported optional Postman inputs do not fail their output"
observations:
  - "Dependent migration exceeded the original scope after a ruling's premise proved false"
  - "Existing main-suite tests exceeded the local time budget"
human_review: false
has_blocked_findings: false
blocked: false
next: "2026-10-11-schematic-generation-integrity/review-3.md"
---

# Review 2

The feature is **not production ready**: the Postman validation backstop does not enforce the specification for unsupported optional inputs. The previous review's two high findings are implemented. A separate unusual-input defect is medium and does not block readiness.

## Previous findings and human rulings

| Authority | Verification | Result |
| --- | --- | --- |
| `review-1.md`, item 1: same-API duplicate routes must be errors without exceptions | All three validators report errors; the removed warning constant is absent. Public batch tests prove no output is written for a duplicate route. GitHub now has one `GetGitTree` with optional `recursive`; Bitbucket has one binary `GetSource`. Generated Rust, OpenAPI, and Postman drift checks pass. | Implemented |
| `review-1.md`, item 1: withdraw REST selectors and amend later features | The route key remains method plus normalized path. The definition, response-model, and fidelity specs and plans record the required endpoint merges, response distinctions, parameter locations, and REST selector withdrawal. WebSocket selectors remain separate. Docs and skills describe duplicate-route errors. | Implemented |
| `review-1.md`, item 2: delete the obsolete generated folder and record completion | All five files are staged deletions, the folder is absent, and the implementation log records completion. The historical Cargo-machete report remains unchanged as ruled. | Implemented |

The author's ruling overrides the spec's proposed future REST selector hook. Multi-shape responses and parameter-location validation remain assigned to the later features identified in the ruling; this review does not require implementing them here.

## Findings

### High: Unsupported optional Postman inputs do not fail their output

**Authority: spec R10:** “A feature one output lacks fails validation for that output only; the others still generate.” The parent decision on request-side support also retains per-output errors as the backstop. Neither sentence exempts optional parameters. The prior review disclosed warning-only behavior as information; its human rulings did not accept that exception.

**Defect class:** unsupported input projections are treated as successful when the input is optional, allowing an output to omit a declared capability.

In `schematic-gen`, [check_unsent_params](../../gen/src/validation.rs:608), which detects parameters Postman cannot carry, chooses `Warning` when `required` is false. The batch generator consequently publishes the collection and reports `[OK] postman output`. This is reached by the shipped OpenAI `BetaCreateResponse` definition's optional `openai-beta` header, and by six other shipped OpenAI endpoints. The collection has no corresponding header input, so users cannot configure the declared option through the generated request.

**Reproduction and complete sibling sweep:** copied the actual OpenAI `BetaCreateResponse` definition into a one-endpoint batch, retained its schema registry, and varied only the parameter location and requiredness. Invoked the public batch generator, inspected the written collection, and repeated every case through the single-API and grouped Postman writers. Rust and OpenAPI succeeded for every case. The embedded-query warning is unrelated and remains allowed by R33.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Batch Postman projection; [single-API writer](../../gen/src/export/postman/mod.rs:69); [grouped writer](../../gen/src/export/postman/mod.rs:128) | Optional header, including the unchanged live definition | All three succeed; parameter absent from request headers; batch emits warning | Postman fails before writing until this input is supported |
| Same three sites | Required header | All three reject; batch writes no Postman collection | Reject; clean |
| Same three sites | Optional cookie | All three succeed; parameter absent from request | Postman fails before writing until this input is supported |
| Same three sites | Required cookie | All three reject; batch writes no Postman collection | Reject; clean |
| Same three sites | Optional and required query controls | All three succeed; parameter present in URL query declarations | Support; clean |
| Batch Rust and OpenAPI projections | The same six cases | Both outputs succeed; generated fields/OpenAPI parameters retain the declared location | Independent successful outputs; clean |

These two parameter locations are every branch of the changed unsupported-input detector. Text and binary bodies and parameter-key authentication have separate Postman projections; their existing conversion tests pass, and they do not use the optionality decision.

**Repair:** make unsupported header and cookie parameters errors regardless of requiredness, preserving Rust/OpenAPI success and warning-only embedded-query behavior. Update the tests that currently endorse seven Postman warnings and the relevant docs. Supporting both projections is also valid, but is unnecessary to satisfy this backstop. Add public-result tests for both locations and both requiredness states, checking that a rejected output leaves existing files unchanged. No new protection layer or synthetic payload matrix is needed.

### Medium: Nested query arrays escape the required validation

**Authority: spec R26:** “reject unsupported nested array shapes during validation rather than dropping them.” The array query projection also changed under R28. This is nonblocking because no shipped definition declares nested arrays.

**Defect class:** validation rejects an unsupported parameter shape at some locations but allows the same shape through another location, producing unusable generated code.

In `schematic-gen`, [check_named_params](../../gen/src/validation.rs:548), the parameter-shape validator, checks header and cookie arrays only. [generate_path_format](../../gen/src/codegen/request_structs/shared.rs:130), which collects query pairs, calls `to_string()` on each outer element. For `Array(Array(Boolean))`, that element is `Vec<bool>`, which has no string conversion; validation succeeds, files are written, and offline compilation fails with `E0599`.

**Reproduction and complete sibling sweep:** copied the live GitHub `GetGitTree` endpoint, changed its existing `recursive` parameter to an array, and tested a one-dimensional control and nested shape at each location. Passed each definition through the public validator and generated every accepted case into an isolated crate using the existing fresh-output helper; compiled with `cargo check --offline --locked`.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Query validator and query-pair projection | `Array(Boolean)` | Accepted; generated crate compiles | Accept; clean |
| Query validator and query-pair projection | `Array(Array(Boolean))` | No diagnostic; generated crate fails with `E0599` at `value.to_string()` | Reject before publication |
| Header validator and header projection | Both shapes | Flat array compiles; nested array rejected | Clean |
| Cookie validator and cookie projection | Both shapes | Flat array compiles; nested array rejected | Clean |

Extend the existing nested-array check to query parameters; do not introduce nested-array serialization. This is a small validation correction. If supporting that unusual shape would require a larger serialization design, document the limitation instead of engineering support in this cycle.

## Observations

### Dependent migration exceeded the original scope after a ruling's premise proved false

R31 prohibited dependent edits, but the endpoint-merging ruling incorrectly stated that no outside caller used the removed requests. The repair migrated the affected `sniff` GitHub and Bitbucket calls and documented the departure. That migration keeps consumers compiling and is narrowly tied to the ruling's endpoint removals. GitHub now explicitly sends `recursive=true`, correcting the old constructor's omission; Bitbucket decodes directory bytes into the existing listing model. This review does not request reversal or another confirmation. Future endpoint-removal work should inventory callers before relying on an absence claim.

### Existing main-suite tests exceeded the local time budget

Three existing tests exceeded five seconds in this run: `e2e_generation::shared_module_contains_paginated_trait` (5.894 seconds), `output::tests::split_to_single_file_transition_cleans_up` (7.226 seconds), and `e2e_generation::multiple_apis_generate_together` (6.274 seconds). These are outside the new test modules, and this review has no matched pre-change measurement establishing a regression. They warrant a separate performance investigation under the repository's five-second rule. The deliberately slow fresh-output compile tests are explicitly required by the spec and selected by their live recipe.

## Verification and fidelity

Executed `just test` in `schematic/`: **1,776 main-suite tests passed**, with 9 excluded/ignored tests; the separately selected **four fresh-output tests passed**. The original fresh-output run took 43.7 seconds. `just check-tier-coverage schematic` reported **zero stranded tests**. Temporary review probes exercised both findings through public APIs and were removed afterward; no implementation or generated artifact was changed by this review. No application windows were opened.

| User-observable requirements | Boundary and strongest evidence | Assessment |
| --- | --- | --- |
| R1–R7, R34: complete groups, retained exports, ownership, refusal, failure reporting, dry run | Filesystem and shipped CLI: actual temporary files, refusal/no-mutation checks, injected publication failure and retry; retained-plus-regenerated output compiled by rustc (F2) | Appropriate boundary crossed |
| R8–R13, R33: scoped diagnostics, output independence, identifiers and route rules | Public batch API and CLI exit/output/files (F2 for CLI/filesystem); manufactured definitions and expected diagnostics (F1 for rule logic) | Appropriate evidence, except the high finding |
| R14–R15, R20: unique operation ids, links, deterministic artifacts | In-process OpenAPI document/link tests and fresh-to-committed comparisons (F1 for export semantics); generated Rust compiled by rustc (F2) | Appropriate for specified document invariants; no provider-wire promise inferred |
| R16–R19: isolated offline compilation, normal-run selection, compiler-error control, fixture/real coverage | Cargo and rustc compile real output outside checkout; deliberately broken output fails with compiler diagnostics; checkout witness verifies no rewrite (F2) | Appropriate boundary crossed |
| R21–R26: bodies, header/cookie/query inputs, credential sources and precedence | Fresh generated clients compiled and exercised against dynamically bound local HTTP servers; exact received bodies, headers, query values and no-send failures asserted (F2) | Appropriate for generator-defined request mechanics; nested-array exception recorded above |
| R27: credential Debug suppression | Explicit sentinel keys checked on generated clients (F1 logic exercised in compiled fixture); request has no client credential storage | Appropriate for this narrow contract; broader redaction remains deferred |
| R28–R29: query separation and wire equivalence | Compiled generated clients send requests to a raw loopback recorder and compare request lines with the pre-change golden capture (F2) | Appropriate; this is equivalence to the old generator, not provider-originated F3 evidence |
| R30–R32: cleanup, consumer compatibility, feature counts | Deleted paths inspected; current generation/compile checks pass; implementation log records scoped dependent gates and counts | No new blocking gap; dependent migration departure noted above |
| R35: build-owner placement and archive exclusion | Workflow step runs the fresh-output recipe before archive production; the shared slow-test exclusion applies to consumer execution and expected listings; contract test passes (F1 configuration evidence) | Placement inspected; hosted CI execution remains pending and is not a readiness blocker |

No live-provider claim requires F3 in this generator feature: the spec explicitly chooses local server tests for request mechanics. The changed provider-specific endpoint definitions and `sniff` migration retain the assumption that GitHub accepts `recursive=true` and Bitbucket source routes return JSON directory listings or raw file bytes; local tests do not establish live-provider behavior.

The configuration robustness matrix is not applicable: this feature reads generated Rust source as an inventory grammar and checks an ownership marker in manifests; it adds no configuration, manifest-field, or lockfile deserializer. The fixture helper copies a lockfile and delegates format interpretation to Cargo. Rust inventory refusals are exercised separately.

## Closure

`ready: false` follows the single high finding. Neither finding needs a human decision. Review 1 is marked implemented and linked to this review; the spec advances to review iteration 2 with status `review-findings`. The next spec and plan are not refreshed because this review is not ready.
