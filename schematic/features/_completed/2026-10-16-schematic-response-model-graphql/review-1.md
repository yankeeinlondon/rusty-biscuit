---
$schema: feature-review.yaml
ready: false
findings:
    - 'R1-F1 (high): OpenAI stream events decode as the wrong variants'
    - 'R1-F2 (high): Referenced oneOf schemas bypass response-shape classification'
observations:
    - Bitbucket JSON object files can look like empty directories
    - Generated provider tests have a separate local recipe
human_in_the_loop: null
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-22T19:57:07-07:00"
spec: 2026-10-16-schematic-response-model-graphql/spec.md
implemented: true
log: /Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-16-schematic-response-model-graphql/log.md
implemented_by: claude/opus
description: "A **feature** review of `2026-10-16-schematic-response-model-graphql/spec.md`"
feature: 2026-10-16-schematic-response-model-graphql/review-1.md
next: 2026-10-16-schematic-response-model-graphql/review-2.md
---

# Review 1

The feature is **not production ready**. Two high findings remain: the new OpenAI stream returns incorrect event variants for ordinary provider replies, and OpenAPI import skips the required response alternatives when their schema is referenced rather than inline. Neither repair depends on an author decision. This is the first review; there are no previous findings to close or carry forward.

Two separate agents reviewed the implementation log and specification for decisions, unresolved choices, and deferrals before implementation review began. Neither found a blocking decision under this review's three-part test. Course 2 applies. The choices below are adopted for this cycle; the author may revisit them later without interrupting repairs.

## Decisions adopted for author review

These choices come from the specification's planning defaults, its design, and the implementation log's phase decision sections and final review agenda. Provider evidence below means the evidence recorded in that log; this review made no live provider calls.

| Topic | Adopted behavior, alternative, and reason |
| --- | --- |
| Stream ends | Use named data/event/field/connection-close conditions. Consume terminators by default; retain them when requested. Retained Ollama final chunks preserve the modeled statistics. Always delivering terminators would attempt to decode OpenAI's `[DONE]` as JSON. |
| Stream errors | Yield `Result<Item, StreamError>`. Malformed items retain their bytes and allow the next complete frame; interruption, early end, timeout, deadline, and frame limit terminate. Stopping at every malformed item would contradict the recommended recovery behavior. |
| Framing and timing | Use SSE, NDJSON, and raw byte framing; `max_frame_bytes` is the builder setting. Envelope elapsed time ends at headers. A per-request timeout becomes an idle bound after headers; the shared deadline remains the overall bound. Raw audio is bytes rather than SSE. |
| Response selection | Choose by status, normalized media type, and declared JSON tag; select empty on 204; allow one text/binary fallback. Request `stream` values remain ordinary caller inputs, not routing instructions. Missing or invalid media headers do not select a fallback; body interruption retains its stronger classification. |
| Single-shape compatibility | Content type and optional statuses document/export a lone shape without introducing a new runtime rejection. Strict media checking would restore a guard the earlier runtime design removed. |
| Error tables | Exact and inclusive-range matches cover 400–599, with class shorthand. Merge API and endpoint entries, replace an identical match, then choose exact/narrower/wider specificity. Endpoint-first precedence is the explicitly considered alternative; it would let a broad endpoint range hide an API exact match. API `ErrorSet::none()` supplies no defaults; endpoint opt-out disables inheritance. |
| Public names | Keep the landed success/failure enum names, stored author-selected success variants, and suffixes for colliding error variants. Ollama's buffered models use `GenerateCompletion` and `ChatCompletion` to avoid generated enum collisions. Array/object tags may be inferred where type information supports it; explicit declarations remain authoritative. |
| Runtime ownership | Codecs and selection live in the runtime; generated clients supply tables, types, and forwarding implementations. Error tables belong to endpoint specifications, not error types. The runtime's `ErrorBody<ApiMarker>` uses a local API marker to allow implementations for model types owned by another crate under Rust's trait implementation rules. Wrapper types for every error would defeat the plain single-error-type contract. |
| Failures inside HTTP success | Keep the runtime's private type-erased carrier for declared failures originating in a decoded 2xx body, including GraphQL. It avoids a disruptive additional associated type while preserving the public typed failure. |
| GraphQL authoring | Keep `GraphQlOperation`, typed variables/data, explicit optional operation name, and omitted variables for `()`. Documents are sent verbatim. Null wrapper `errors` is malformed; null optional error paths/locations are treated as absent. Errors precede typed-data decoding; only failure-free data reaches a typed mutator. |
| GraphQL addresses | Ordered parsed-path replacement/append rules derive the address, preserving prefixes. A full URL override still obeys origin policy. Recognizing Enterprise by `/api/v3` rather than a hostname follows the specified default and supports private installations. |
| GraphQL pagination | Adopt separate `Relay::forward(root).collection(item)` and `Relay::backward(root)` presets and the shared pager's data-root hook. One pager does not switch directions. A combined backward leg needs separate page-size inputs and continuation signals in the existing model; standalone backward paging is the smallest consistent implementation of the optional backward support. The log discloses this departure from the proposed spelling and combined-leg design. |
| Pager variables | Authors supply a concrete variables type whose managed fields are optional; definition validation rejects a schema requiring those fields. The generator does not rewrite an existing Rust model or introduce an untyped variables interface. Null or omitted initial cursors remain valid. |
| Export/import forms | Keep serde-backed extension fields within bundle version 1, endpoint-level imported errors, and exact-status expansion when a range is partly superseded. Standard OpenAPI fields remain authoritative for authentication/deprecation; default `SameOrigin` is omitted. A version bump, error hoisting, or redundant default metadata offers no required benefit with no external format consumers. |
| LM Studio | Adopt the logged provider correction: model download is a JSON job object, while chat supports JSON and SSE ending at `chat.end`. The spec's download-SSE premise is contradicted by the recorded provider documentation. Implementing that premise would knowingly misdescribe the endpoint. Removed obsolete progress types are part of this correction. |
| Other provider details | Keep retained Ollama `done=true` or progress `status=success` ends. Keep the spec-required OpenAI-compatible `[DONE]` despite the log's disclosed lack of provider verification. ElevenLabs audio types remain undeclared where evidence does not establish a fixed media type; timestamped streaming uses NDJSON based on the recorded SDK evidence. Mapping format-changing query values to media types is additional scope. |
| Migration compatibility | Keep the OpenAI import overlay that preserves unmigrated public response types while adding `GetResponse` streaming. Accept the logged `CreateRealtimeCall` correction from `()` to bytes: its SDP reply was previously discarded. A second legacy importer or wholesale migration of other endpoints would broaden this change substantially. This acceptance does **not** excuse the newly reachable stream decoder defect in R1-F1. |
| Bitbucket limitation | Keep `Directory` plus binary `File` fallback, as R32a explicitly permits when research finds no separating wire tag. A JSON file can select directory decoding, and an object can decode as an empty listing. The limitation is disclosed rather than adding a speculative protocol discriminator. See Observations. |
| Hooks and diagnostics | No migrated incompatible hooks existed to remove. GraphQL errors retain provider fields without additional redaction; stored metadata/display retain the existing credential rules. The runtime ABI advances to 7 for the new generated contracts. |

Allowed deferrals remain deferred: GraphQL subscriptions, incremental delivery, batching, document validation, runnable GraphQL Postman output, broader extension/import fidelity, migration of sniff's handwritten GraphQL client, and other endpoints outside the named migration set. Unsupported provider event forms and unmodeled Ollama statistics remain limited to what the log discloses; the retained modeled final statistics are tested. No CI changes, lifecycle moves, commits, or pushes are part of this review.

## Findings

### R1-F1 (high): OpenAI stream events decode as the wrong variants

**Authority: spec R32b** requires `GetResponse` to expose an SSE stream of registered `BetaResponseStreamEvent`; R11c requires typed provider error events, and R24 requires correct changed public responses. This feature makes the older imported decoder reachable through its new streaming endpoint, so its age does not put this defect outside the review boundary. A normal text event is returned as an audio event, and a completion or error event loses its payload.

**Defect class:** untagged event alternatives accept another event's fields because their `type` members are unrestricted strings and decoding picks the first compatible alternative.

In `schematic-definitions`, the response overlay [adds the stream type to GetResponse](/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/definitions/src/openai_pagination.rs:60). The imported [BetaResponseStreamEvent](/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/definitions/src/openai/types.rs:33872) tries audio delta and audio done first. These require only a small subset of fields carried by other events. Neither checks that `type` has the event's declared value.

A temporary test copied the existing generated-provider stream fixture, substituted the following ordinary text event, and sent it through the generated OpenAI client with `get_response("resp_1").stream(true).anonymous()`:

```text
data: {"type":"response.output_text.delta","item_id":"i","output_index":0,"content_index":0,"delta":"hi","sequence_number":3,"logprobs":[]}

```

The returned item was:

```text
BetaResponseAudioDeltaEvent {
    type_: "response.output_text.delta", sequence_number: 3, delta: "hi", ...
}
```

The expected item is the text-delta variant, retaining its item identity, indexes, and log probabilities. The implementation log also disclosed completion payload loss, but classified fixing the decoder as additional scope. That classification does not hold once this feature promises the affected stream.

The sweep serialized one concrete value for **each of the 53 declared event alternatives**, set its documented event `type`, and deserialized through the public event enum. **51 selected the wrong variant; only the first two audio alternatives selected themselves.** These structural round trips supplement the realistic generated-client reproduction; they are not claimed as live provider validation. The complete per-alternative instance ledger follows below.

Other sites within the migration boundary were checked for the same class:

| Child ID | Site and shape checked | Observed result | Expected result / clean invariant |
| --- | --- | --- | --- |
| R1-F1.54 | Anthropic message events and nested deltas | Clean; generated-client text/error fixtures pass | Both enums use a `type` tag, so structurally overlapping payloads cannot select another named event |
| R1-F1.55 | LM Studio chat events | Clean by declared invariant | Explicit `type` tags select chat/load/prompt/reasoning/error alternatives |
| R1-F1.56 | Ollama generate item | Clean; normal/final/error fixture tests pass | Error alternative requires `error`; ordinary chunks lack it and require their own chunk fields |
| R1-F1.57 | Ollama chat item | Clean; normal/final/error fixture tests pass | Same explicit error-first/disjoint-field contract |
| R1-F1.58 | Ollama pull and push item | Clean; progress/error fixture tests pass | Error requires `error`; progress requires `status` |
| R1-F1.59 | Ollama create item | Clean; progress/error fixture tests pass | Same explicit progress/error distinction |
| R1-F1.60 | Ollama OpenAI-compatible chat and completion chunks | Clean for this class | Each framing item is a concrete chunk struct, not an overlapping event union |
| R1-F1.61 | ElevenLabs timestamp chunks and raw audio | Clean for this class | A timestamp chunk is one concrete struct; raw audio has no JSON event-alternative selection |
| R1-F1.62 | Generated GetResponse response selection and stream projection | SSE shape selected correctly, but text becomes audio | Preserve the correct typed stream item; media selection cannot repair an already misclassified event |
| R1-F1.63 | Existing generated-provider OpenAI test | Passes with audio-only events | Add ordinary text, completed-response, and provider-error coverage; choosing only the two clean alternatives misses the defect |

Repair the imported event decoding at its generation source and regenerate owned artifacts. Enforce the declared event discriminator before accepting an alternative; do not hand-edit generated models or add a decoder workaround to every endpoint. Keep the repair focused on the newly exposed contract, and preserve the migration guard for unrelated endpoints. The required improvement is correct event identity and payload retention, not a new general protection layer. Add generated-client checks for text, completion, and error events plus a guard covering every declared event alternative.

### R1-F2 (high): Referenced oneOf schemas bypass response-shape classification

**Authority: spec R28** requires importing a JSON `oneOf` separable by a discriminator or array/object shape as several JSON success shapes. It does not restrict this to inline schemas. Moving a reusable response schema into `components.schemas` is ordinary OpenAPI authoring, and currently changes the imported public response from two tagged alternatives to one untagged named shape without a diagnostic.

**Defect class:** response classification inspects only inline schema nodes before deciding whether a resolvable reference contains multiple distinguishable alternatives.

In `schematic-define`, [json_shapes](/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/define/src/openapi/import/mappings/classify.rs:204), which constructs imported JSON response alternatives, matches only `ReferenceOr::Item`. A reference goes straight to the single named-schema path. The resolver is available but is not used before that decision.

The reproduction starts with the real exporter's fixture used by `response_openapi`, retaining its endpoints and error responses. The control's response schema is:

```json
{
  "oneOf": [
    {"$ref": "#/components/schemas/Directory"},
    {"$ref": "#/components/schemas/File"}
  ],
  "discriminator": {
    "propertyName": "type",
    "mapping": {
      "dir": "#/components/schemas/Directory",
      "file": "#/components/schemas/File"
    }
  }
}
```

Move that same object, unchanged, to `components.schemas.Reply`, and replace its old location with `{"$ref":"#/components/schemas/Reply"}`. Through the public `OpenApiImport::build()` result, the inline control has two tagged response shapes; the reference has one `Json(Named("Reply"))` shape with no JSON tag. Repeat with an object/array pair without a discriminator: the same loss occurs.

| Child ID | Site / shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| R1-F2.1 | Direct response, inline discriminator `oneOf` | Two field-tagged alternatives; clean | Two field-tagged alternatives |
| R1-F2.2 | Direct response, referenced discriminator `oneOf` | One untagged named shape | Two field-tagged alternatives |
| R1-F2.3 | Referenced response, inline discriminator `oneOf` | Two field-tagged alternatives; clean | Two field-tagged alternatives |
| R1-F2.4 | Referenced response, referenced discriminator `oneOf` | One untagged named shape | Two field-tagged alternatives |
| R1-F2.5 | Direct response, inline object/array `oneOf` | Object and array alternatives; clean | Object and array alternatives |
| R1-F2.6 | Direct response, referenced object/array `oneOf` | One untagged named shape | Object and array alternatives |
| R1-F2.7 | Referenced response, inline object/array `oneOf` | Object and array alternatives; clean | Object and array alternatives |
| R1-F2.8 | Referenced response, referenced object/array `oneOf` | One untagged named shape | Object and array alternatives |
| R1-F2.9 | Error response projection through the same JSON reader | Single error shape by contract | Clean with respect to splitting: the error table intentionally holds one shape per status, and diagnoses unsupported multiple alternatives rather than generating success variants |
| R1-F2.10 | Generator success-type decision consuming the imported shape count | Referenced result takes the single-shape branch | Imported separable successes must reach the multi-shape branch |
| R1-F2.11 | OpenAPI response export consuming the imported shape list | A single shape exports its named reference; several shapes form a `oneOf` | Preserve the imported response alternatives and their tags in extension/output projections |

Rows 1–8 were run through public import, not merely a private parser. Rows 9–11 trace the value to its downstream decisions: the error reader shares `json_shapes` but deliberately collapses multiple error alternatives; the generator's [success_code](/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/gen/src/codegen/responses.rs:36) chooses its codec by the shape count; the exporter's [map_responses](/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/define/src/openapi/export/responses.rs:91) projects that same list. Those consumers need no speculative new policy: the missing resolution is at classification.

The existing response-input matrix covers absence, null, wrong field type, invalid/all-invalid collection members, empty values, duplicate keys/spellings, and trailing content through JSON and YAML loaders. Those tests pass; this is a missing **valid reference form**, not a request to reject a new set of unusual malformed inputs. Preserve that matrix while adding inline/referenced controls for both separability methods. Resolve the media schema before inspecting `oneOf`, retain the original named fallback when separation is impossible, and keep located diagnostics for unsupported cases.

## Observations

**Bitbucket JSON object files can look like empty directories.** R32a expressly allows retaining the ambiguous JSON-listing/binary-file shape set when no wire tag separates them. The implementation log records that a JSON object file can deserialize to an empty listing, beyond the spec's predicted malformed-listing error. The shape note documents this limitation, and sniff's forced call site uses raw bytes. Treat it as a documented limitation under that authorization; a speculative discriminator or metadata request would cost more than this review can justify.

**Generated provider tests have a separate local recipe.** The concrete provider-stream tests live in the excluded `schematic-schema` crate and run with `just test-schema`; they have a declared target and a live recipe, and all 65 schema tests passed in this review. The log records that current CI does not run that separate suite. Fresh-generation fixtures cover the shared generated/runtime boundary, but provider-specific event mistakes can escape them, as R1-F1 demonstrates. CI changes are outside this specification's authorization; consider test placement when scheduling that work rather than adding a matrix cell in this repair.

## Verification and requirement coverage

The feature's new behavior is HTTP decoding, code generation, and in-process definition validation. **L1** mock-server, fixture, and compiler checks are the appropriate verification level; it adds no keyboard encoding, terminal rendering, or focus-dependent interaction requiring L2/L3. Existing terminal tests are not evidence for or against these network contracts.

| Requirements | Strongest relevant checks inspected | Review result |
| --- | --- | --- |
| R1–R4, R13a, R23 | Definition validation; runtime `shapes`; generator response tests | L1 covers status/media/tag/fallback selection, ambiguous definitions, and unchanged request values |
| R6–R12 | Runtime `streams`/`timing`/hooks; generated provider streams; compile-fail fixtures | L1 covers incremental delivery, all end forms, retention, malformed recovery, framing boundaries, ownership/drop, frame limit, idle timeout and shared deadline; provider event identity has R1-F1 |
| R16–R21 | Definition error-table tests; runtime `error_routes`; generated error declarations | L1 covers merged specificity, opt-out, malformed/undeclared errors, raw retention, and single/multiple failure types |
| R22, R24–R29, R44 | Definition baselines/bundles, response export/import matrices, generator artifact drift/import overlay | L1 and source audit; referenced separable schemas have R1-F2 |
| R5, R30–R34 | Provider definition tests and generated-schema `streaming_providers` | L1 with the adopted evidence-based provider choices and Bitbucket limitation above; OpenAI coverage misses text/completion/error identity |
| R35–R39, R41–R43 | GraphQL definition/runtime/address/hook tests; generated operation fixture and sniff fixture parity | L1 checks typed requests, object variables, verbatim document, wrapper-before-data classification, HTTP errors, errors/partial data, addresses/origins, hook order, and preserved provider fields |
| R40 | Relay definition tests, runtime `graphql_pager`, generated pager fixture and compile-fail surface | L1 with the adopted separate-direction presets; error pages do not append partial data |

Commands run during this review, after removing temporary reproduction tests:

- `just test schematic-runtime`: **451 passed, 1 skipped**.
- `just test schematic-define schematic-definitions schematic-gen`: **2,169 passed, 14 skipped**. This includes the response input matrices and bundle loader tests, baseline comparisons, artifact drift, and import-overlay reproduction.
- `just test-schema` in `schematic`: **65 passed**, including the existing provider stream tests and compiler diagnostic checks.
- `just test-fresh-output slow_fixture_apis_compile_and_send_offline` in `schematic`: **1 passed** (8.9 seconds), compiling fresh generated clients and exercising their request, GraphQL, pagination, hook, and sniff-fixture parity behavior.

Temporary probes were compiled by declared targets and executed with nextest. The generated-client text probe failed as reported in R1-F1; the exhaustive event probe found 51/53 misclassifications; the public import probe found four failures and four clean controls as reported in R1-F2. All probe edits were removed. No production sources or generated artifacts were changed by this review. Cross-platform and full lint/regeneration evidence remains the implementation log's reported evidence; this review does not claim to have repeated those checks.

## OpenAI event instance ledger for R1-F1

Each row is a separate declared alternative of the public stream event enum. The expected result is that same alternative with its payload retained. A row that selects itself is a clean control. The structural sweep uses serialized concrete event values with the corresponding documented `type`; the generated-client text reproduction above establishes realistic reachability.

| Child ID | Alternative tested and expected | Observed alternative |
| --- | --- | --- |
| R1-F1.1 | `BetaResponseAudioDeltaEvent` | `BetaResponseAudioDeltaEvent` (clean) |
| R1-F1.2 | `BetaResponseAudioDoneEvent` | `BetaResponseAudioDoneEvent` (clean) |
| R1-F1.3 | `BetaResponseAudioTranscriptDeltaEvent` | `BetaResponseAudioDeltaEvent` |
| R1-F1.4 | `BetaResponseAudioTranscriptDoneEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.5 | `BetaResponseCodeInterpreterCallCodeDeltaEvent` | `BetaResponseAudioDeltaEvent` |
| R1-F1.6 | `BetaResponseCodeInterpreterCallCodeDoneEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.7 | `BetaResponseCodeInterpreterCallCompletedEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.8 | `BetaResponseCodeInterpreterCallInProgressEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.9 | `BetaResponseCodeInterpreterCallInterpretingEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.10 | `BetaResponseCompletedEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.11 | `BetaResponseContentPartAddedEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.12 | `BetaResponseContentPartDoneEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.13 | `BetaResponseCreatedEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.14 | `BetaResponseErrorEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.15 | `BetaResponseFileSearchCallCompletedEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.16 | `BetaResponseFileSearchCallInProgressEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.17 | `BetaResponseFileSearchCallSearchingEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.18 | `BetaResponseFunctionCallArgumentsDeltaEvent` | `BetaResponseAudioDeltaEvent` |
| R1-F1.19 | `BetaResponseFunctionCallArgumentsDoneEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.20 | `BetaResponseInProgressEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.21 | `BetaResponseFailedEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.22 | `BetaResponseIncompleteEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.23 | `BetaResponseOutputItemAddedEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.24 | `BetaResponseOutputItemDoneEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.25 | `BetaResponseReasoningSummaryPartAddedEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.26 | `BetaResponseReasoningSummaryPartDoneEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.27 | `BetaResponseReasoningSummaryTextDeltaEvent` | `BetaResponseAudioDeltaEvent` |
| R1-F1.28 | `BetaResponseReasoningSummaryTextDoneEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.29 | `BetaResponseReasoningTextDeltaEvent` | `BetaResponseAudioDeltaEvent` |
| R1-F1.30 | `BetaResponseReasoningTextDoneEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.31 | `BetaResponseRefusalDeltaEvent` | `BetaResponseAudioDeltaEvent` |
| R1-F1.32 | `BetaResponseRefusalDoneEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.33 | `BetaResponseTextDeltaEvent` | `BetaResponseAudioDeltaEvent` |
| R1-F1.34 | `BetaResponseTextDoneEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.35 | `BetaResponseWebSearchCallCompletedEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.36 | `BetaResponseWebSearchCallInProgressEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.37 | `BetaResponseWebSearchCallSearchingEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.38 | `BetaResponseImageGenCallCompletedEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.39 | `BetaResponseImageGenCallGeneratingEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.40 | `BetaResponseImageGenCallInProgressEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.41 | `BetaResponseImageGenCallPartialImageEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.42 | `BetaResponseMCPCallArgumentsDeltaEvent` | `BetaResponseAudioDeltaEvent` |
| R1-F1.43 | `BetaResponseMCPCallArgumentsDoneEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.44 | `BetaResponseMCPCallCompletedEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.45 | `BetaResponseMCPCallFailedEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.46 | `BetaResponseMCPCallInProgressEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.47 | `BetaResponseMCPListToolsCompletedEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.48 | `BetaResponseMCPListToolsFailedEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.49 | `BetaResponseMCPListToolsInProgressEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.50 | `BetaResponseOutputTextAnnotationAddedEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.51 | `BetaResponseQueuedEvent` | `BetaResponseAudioDoneEvent` |
| R1-F1.52 | `BetaResponseCustomToolCallInputDeltaEvent` | `BetaResponseAudioDeltaEvent` |
| R1-F1.53 | `BetaResponseCustomToolCallInputDoneEvent` | `BetaResponseAudioDoneEvent` |
