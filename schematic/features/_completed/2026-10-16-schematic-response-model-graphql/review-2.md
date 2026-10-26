---
$schema: feature-review.yaml
ready: true
findings: []
observations:
    - Unions without a declared discriminator still use first-match decoding
    - Inline array alternatives lose their schema on re-export
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-22T21:17:24-07:00"
spec: 2026-10-16-schematic-response-model-graphql/spec.md
implemented: true
implemented_by: claude/opus
description: "A **feature** review of `2026-10-16-schematic-response-model-graphql/spec.md`"
feature: 2026-10-16-schematic-response-model-graphql/review-2.md
previous: 2026-10-16-schematic-response-model-graphql/review-1.md
---

# Review 2

The feature is **production ready** within the specification's scope and the decisions accepted in the first review. Both previous high findings are verified fixed. There are no new findings, partial repairs, regressions, or blocked findings. The repair's additional sibling cases were checked as part of closing the original findings; they are not new defects being carried into another repair cycle.

This review read the specification, original implementation log, previous review, and repair's `Finding Ledger` in `log.md`, then checked the implementation and ran the relevant tests. The first review's accepted decisions remain in effect, including the provider corrections, separate forward/backward Relay presets, OpenAI migration overlay, and permitted import-fidelity limitations. No prerequisite changed that would reopen those decisions. No new design choice requires human confirmation.

## Previous Finding Verification

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: OpenAI stream events decode as the wrong variants | R1-F1.1–.53: every declared event alternative; .54–.61: other migrated providers; .62: generated stream projection; .63: text/completion/error coverage. Also checked repair ledger .64, the other imported unions, and assessed .65, undeclared-discriminator unions. | **verified fixed** | The public event guard decodes all 53 alternatives as themselves and compares their serialized payloads, with its list checked against the vendored document and an exhaustive Rust match. All three `openai_stream_events` tests pass. The generated-client stream test passes with audio, text, completed-response, and provider-error items. Other providers retain their tagged/disjoint/concrete contracts. The importer records discriminator metadata and the generator emits matching dispatch; its compiled fixture tests pass. | No required instance remains. Ledger .65 concerns request-side unions without a declared discriminator, outside the finding's required stream contract; see Observations. |
| R1-F2: Referenced oneOf schemas bypass response-shape classification | R1-F2.1–.8: direct/referenced response × inline/referenced schema × field-tagged/object-array separation; .9: error projection; .10: generation; .11: export. Also checked repair ledger .12–.15: enum-derived tags, variant names, OpenAI overlay, reference chains/cycles/fallback. Assessed .16, inline-array re-export. | **verified fixed** | All four public-import cell tests pass in JSON and YAML, each checking both separation methods. Error projection, generated multi-shape response selection, export/re-import, enum-derived tags, optional-tag fallback, chained references, and cycle tests pass. The OpenAI baseline and full re-import reproduction pass. | No required instance remains. Ledger .16 is the previously existing inline-array fidelity limitation, unchanged by reference resolution; see Observations. |

The verification below identifies how each child group was checked, rather than treating the ledger's completion flags as evidence.

| Instance IDs | Implementation and observed verification |
| --- | --- |
| R1-F1.1–.53 | In `schematic-definitions`, [the event guard](../../definitions/tests/openai_stream_events.rs) constructs each concrete alternative with its documented wire `type`, decodes through the public union, and checks both variant identity and payload retention. All 53 pass, including the original two clean audio controls. The ordinary text-delta reproduction also passes independently. |
| R1-F1.54 | Anthropic's message and nested delta enums explicitly use `#[serde(tag = "type")]`. Generated-provider text and error stream tests pass. |
| R1-F1.55 | LM Studio's chat-event enum explicitly uses `#[serde(tag = "type")]`; this repair does not change it. |
| R1-F1.56–.57 | Ollama generate/chat alternatives put the required `error` member first; normal chunks instead require their chunk fields. Generated-provider normal, final-statistics, and error-line tests pass. |
| R1-F1.58–.59 | Ollama pull/push/create distinguish required `error` from required progress `status`. Their declared termination behavior and progress/error tests remain passing. |
| R1-F1.60–.61 | Ollama's OpenAI-compatible chunks and ElevenLabs timestamp chunks are concrete structs; raw audio is bytes. There is no event-alternative choice to misclassify. Generated-provider chunk tests pass. |
| R1-F1.62–.63 | In `schematic-schema`, [the generated-provider test](../../schema/tests/streaming_providers.rs) receives a text delta with item/index/log-probability fields, a completed response with output text, and an error with code/message/parameter fields through `GetResponse`. Each reaches its correct public variant with those fields retained. |
| R1-F1.64 | Source inspection finds 47 generated discriminator dispatch implementations. The shared importer handles mapping values, enum values through references/composition/nested unions, and implicit component names. Its six tests pass. Optional-tag unions remain untagged with a warning; components already represented by free-form JSON cannot select the wrong Rust variant. The six compiled generator-fixture tests cover tag-directed selection, shared tags, payload serialization, rejected tags, and selected-alternative decoding errors. |
| R1-F2.1–.8 | In `schematic-define`, [the public-import cell tests](../../define/tests/response_openapi.rs) edit the real exporter's fixture by relocating schemas and responses. All eight combinations preserve the expected alternative list and tags, with explicit distinct variant names. JSON and YAML are independently imported and their public results compared. |
| R1-F2.9 | Referenced error unions retain their named type; inline error unions retain the existing single-shape fallback and warning. The dedicated public-import test checks both and confirms neither adds success alternatives. |
| R1-F2.10 | In `schematic-gen`, [the import-to-generation test](../../gen/tests/l1/openapi_import_test.rs) confirms a referenced separable union produces a response enum and the multi-shape decoder. The existing fresh-generation fixture separately compiles and exercises the shared multi-shape runtime boundary. |
| R1-F2.11 | The export/re-import test checks the exact `oneOf` members and discriminator mapping, then checks that importing again retains the response list. |
| R1-F2.12–.13 | Tests check enum-derived wire values instead of component-name guesses, optional-tag fallback, and distinct response variant names. The generator test confirms those names can be emitted successfully. |
| R1-F2.14 | The OpenAI overlay explicitly restores the complete `ConversationItem` response after generic import separates it. The historical OpenAI definition comparison passes, as do re-import reproduction and generated-artifact drift checks. |
| R1-F2.15 | Public-import tests check reference chains, a self-reference, a recursive union, and an inseparable referenced union. They preserve the named fallback where separation is impossible and check the located warning for the inseparable union. |

## Unblocked Findings

None.

## Blocked Findings

None.

## Observations

**Unions without a declared discriminator still use first-match decoding.** The repair deliberately uses an OpenAPI-declared discriminator; it does not infer one from a common constant-valued property. The repair ledger identifies nine top-level OpenAI unions without that keyword. Request-side examples can still select a structurally compatible first alternative. This does not violate the newly exposed `GetResponse` event contract, whose declared discriminator now works. The limitation is documented in the importer guide. Inferring an undeclared discriminator would introduce a broader policy; leave it for separately scheduled work.

**Inline array alternatives lose their schema on re-export.** The exporter fixture's inline array member imports as free-form JSON tagged as an array. Re-export retains the alternative's presence but loses its array schema, so a later import cannot infer the same array/object separation from standard fields. This behavior already existed for inline unions and is identical for referenced unions after the repair; the repair ledger discloses it. Broader import/export fidelity is an accepted deferral. It does not leave the original referenced-union classification finding open.

## Verification and requirement coverage

The feature changes definition validation, HTTP decoding, stream framing, import/export, and generated Rust APIs. In-process tests, local mock servers, and compiled generated fixtures are **Level 1**, the appropriate level for these contracts. It adds no terminal rendering, keyboard encoding, or focus-dependent requirement needing Level 2 or Level 3 evidence.

| Requirements | Verification inspected and rerun | Result |
| --- | --- | --- |
| Success shapes, status/media/tag selection, fallback, ordinary request values (R1–R4, R13a, R23) | Definition validation, runtime shape tests, response-generation tests, fresh generated-client fixture | Passing L1 checks; no new defect found |
| Streaming, framing, termination, recovery, ownership, timing, hooks, provider errors (R6–R12) | Runtime stream/timing/hook tests; generated-provider tests; compiled misuse diagnostics | Passing L1 checks; all 53 OpenAI event identities now verified |
| Declared HTTP errors and specificity (R16–R21) | Definition error-table tests, runtime error-route tests, generated declarations | Passing L1 checks |
| Migration, provider models, historical definitions, artifacts (R5, R22, R24–R25, R29–R34) | Provider definition tests, generated provider streams, historical definition comparisons, artifact drift and OpenAI re-import tests | Passing L1 checks within the reviewed packages; earlier accepted provider choices remain intact |
| OpenAPI import/export and persisted definitions (R26–R28, R44) | Public JSON/YAML import matrix, bundle JSON matrix, response export/import tests, repaired reference-form cells | Passing L1 checks; valid referenced response alternatives now preserved |
| GraphQL requests, responses, errors, addresses, hooks, fixture parity (R35–R39, R41–R43) | Definition/runtime tests and fresh generated-client fixture with sniff response fixtures | Passing L1 checks |
| Relay pagination (R40) | Definition tests, runtime GraphQL pager tests, generated pager fixture and compiler checks | Passing L1 checks under the previously accepted direction presets |

The response input matrix covers absent, null, wrong-type, invalid collection member, empty, duplicate, and trailing-content cases through the public importer. Its JSON and YAML cases pass. The bundle matrix also passes. Inspection of `serde(default)`, `Option`, `filter_map`, `unwrap_or_default`, and discarded parse results found no new permissive response-classification defect: the new optional discriminator metadata uses the existing non-null presence deserializer, and the resolver's empty mapping default means “no explicit mapping,” followed by the documented enum/name lookup. GraphQL's optional partial-data decode remains intentionally fallible without hiding server errors. New field-dispatch behavior is additionally checked for missing, non-string, unknown, and shared tag values.

Commands run for this review:

- `just test schematic-define schematic-definitions schematic-gen schematic-runtime`: **2,649 passed, 15 skipped**. This includes the new event guard, discriminator tests, public response import cells, input matrices, historical definition comparisons, artifact drift, and OpenAI re-import reproduction.
- `just test-schema` in `schematic`: **65 passed**, including generated-provider stream checks and generated API compiler diagnostics.
- `just test-fresh-output slow_fixture_apis_compile_and_send_offline` in `schematic`: **1 passed**, compiling fresh clients and exercising their offline mock-server behavior.
- `just test-compile-fail` in `schematic`: **1 passed**, checking the runtime's intended compiler diagnostics for invalid API use.
- `just check-tier-coverage schematic`: **0 stranded tests**. The new generator tests are declared by the consolidated L1 target; the definitions guard is an automatically discovered integration target.

This review does not claim new cross-platform, live-provider, full dependent-package, or lint evidence; those remain the implementation logs' reported checks and the repository's CI process. Cross-platform proof and a pending human review do not independently block readiness under this review's rules. This review changes only its report and the requested review/spec frontmatter; production sources and generated artifacts are unchanged.
