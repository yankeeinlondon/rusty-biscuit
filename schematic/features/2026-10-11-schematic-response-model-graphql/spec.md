---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
status: draft-spec
reviewed: false
clarified: false
implemented: false
human_review: false
message_to_agent: ''
parent: ../../reviews/2026-07-31-macro-review/spec.md
depends_on:
    - 2026-10-11-schematic-definition-vocabulary
refresh_required: true
related: []
---

# Schematic response model v2 and GraphQL

## Problem

Today an endpoint's response is one value of `ApiResponse` in `schematic-define`
(`schematic/define/src/response.rs`): `Json(Schema)`, `Text`, `Binary`, or
`Empty`. That is too small for the APIs the repository already defines. The
facts below are from the parent review (section 5, 5b, and decisions D6 and
D36-D38) unless marked as checked in code.

1. **Streaming has no representation.** Ollama and LM Studio annotate
   `ApiResponse::Binary` with `// SSE streaming` or `// Streaming NDJSON`
   comments (checked: `schematic/definitions/src/ollama/mod.rs` lines 145-327,
   `schematic/definitions/src/lmstudio/mod.rs` lines 125 and 173). Anthropic's
   `CreateMessage` declares a plain JSON response that cannot decode once the
   request sets `stream: true`. ElevenLabs' `StreamSpeech` is declared exactly
   like its non-streaming sibling, so the generated method buffers the whole
   body (parent section 5). The parent reports ten Ollama types stranded by
   this; that count is unverified here.
2. **Same-address endpoints cannot coexist.** Ollama and Anthropic pick the
   streaming or non-streaming reply with a request-body field (`stream`). The
   current rule forbids two endpoints on one `(method, path)`, and the OpenAPI
   exporter merges on `(path, method)`.
3. **Error bodies have no home.** `ApiResponse` is a single value, not a
   status-to-shape map. The exporter can emit only 200 and 204. A correct,
   tested type such as `artificial_analysis` `RateLimitError` exists with no slot
   to live in.
4. **Text and binary responses carry no content type** although their request
   twins do. The OpenAPI exporter reports every ElevenLabs audio endpoint as
   `application/octet-stream`, and import collapses `audio/mpeg` into the same
   variant.
5. **There is no GraphQL support.** Nothing under `schematic/` references
   GraphQL. sniff calls GitHub's GraphQL API with hand-written code
   (checked: `sniff/lib/src/remote/check_status.rs`, `focused.rs`,
   `blocking.rs`), including the check-status query whose fixtures live in
   `sniff/lib/tests/fixtures/remote/graphql_check_status/`. GitHub.com serves
   REST at the base address and GraphQL at `/graphql`; GitHub Enterprise serves
   REST under `/api/v3` and GraphQL under `/api/graphql`.
6. **The type change breaks everything that reads it.** Changing the type of
   `Endpoint::response` breaks the vendor-extension format, both exporters, the
   importer, and the generator (parent section 5b item 2). Files that name
   `ApiResponse` today include `schematic/define/src/openapi/export/responses.rs`,
   `schematic/define/src/openapi/import/mappings/responses.rs`,
   `schematic/define/src/openapi/extensions.rs`,
   `schematic/gen/src/export/openapi.rs`, `schematic/gen/src/export/postman/`,
   `schematic/gen/src/validation.rs`, and the `schematic/gen/src/codegen/`
   modules (checked by search; the list is a starting point, not exhaustive).

## Goal and non-goals

**Goal.** An API author can say, in the definition, what an endpoint really
returns: one or more success shapes (JSON, text, binary, empty, or a stream),
each with an optional content type, plus a typed list of error shapes mapped from
status codes. Streaming and non-streaming variants at one address are two typed
endpoints. A GraphQL operation is a first-class definition item that generates
into the same client crate and reports GraphQL errors as typed failures. The
generated client, the OpenAPI and Postman exporters, and the OpenAPI importer all
agree on the new vocabulary, and the migrated Ollama, LM Studio, Anthropic, and
ElevenLabs definitions use it.

**Non-goals.**

- WebSocket (spec `schematic-websocket`).
- Exporter and importer fidelity beyond what the vocabulary change forces: the
  schemars bridge, the `x-schematic` round trip, Postman example bodies, and the
  AsyncAPI exporter belong to `schematic-fidelity-polish`.
- Parsing or validating GraphQL documents (D17: the string is sent as written).
- GraphQL batching (D34 says no decision addresses it; this spec does not add it).
- Retiring sniff's hand-written GitHub or GitLab client (tracked by sniff's own
  spec, starting after the pagination spec; D50 item 6).
- Changing the envelope, `TransportError`, credentials, deadline, or cross-origin
  runtime (spec `schematic-runtime-envelope`), the pagination model (spec
  `schematic-pagination`), the builder and registration machinery (spec
  `schematic-definition-api`, 4a), or the definition vocabulary (spec
  `schematic-definition-vocabulary`, 4b), except to extend them as stated below.
- Rewriting any definition other than the four named APIs (Ollama, LM Studio,
  Anthropic, ElevenLabs). The other roughly 519 endpoint literals (OpenAI alone has
  265) were rewritten once by spec 4a and are not touched again here.
- Generating typed convenience wrapper methods per endpoint (D53 rules none are
  generated; nothing in this spec adds or promises them).

## Background: decisions this spec implements

Provenance: parent `schematic/reviews/2026-07-31-macro-review/spec.md` (section 15
unless noted). The text below restates what the implementer needs; the parent is
not required reading.

- **D6. Response vocabulary v2 (success set plus error set).** Each endpoint
  declares one or more success shapes and a separate list of error shapes. A
  success shape is one of: JSON with a type, text, binary, empty, or a stream
  with SSE or NDJSON framing and an item type; each has an optional content
  type. The generated success type is a plain type for the common case, or a
  small generated enum when several success shapes exist. Declared error bodies
  decode on demand from the error. This replaces the type of `Endpoint::response`
  and is breaking for both exporters, the importer, and all endpoint definitions.
- **D3 and D7 (context from earlier specs).** Every send returns
  `Result<Response<T, E>, TransportError>`; `T` is the declared success type and
  `E` the declared failure type; the envelope holds status, headers, metadata,
  and an outcome of `Success(T)` or `Failure(ApiError<E>)`, with the raw body
  kept. One generic send serves every endpoint including streaming and GraphQL
  (typed-send spike, `spikes/typed-send/findings.md`; reported, not re-run).
- **D25 and D26.** A 2xx body that fails to decode is a failure inside the
  envelope with its real status and kind `MalformedSuccess`; a body cut off after
  the headers is a failure inside the envelope with kind `BodyInterrupted`, status,
  headers, and partial bytes.
- **D36. Same-address streaming and non-streaming.** Two endpoint definitions on
  the same method and path, each declaring a **selector** (a request-body field
  value, for example `stream=false` versus `stream=true`). The uniqueness rule
  becomes `(method, path, selector)`. OpenAPI export merges them into one
  operation whose response lists both content types (`application/json`,
  `text/event-stream`). Planning details left open: selector syntax, naming of
  the two generated request types, how the merged request schema shows the
  selector.
- **D37. Stream end.** The definition declares the end condition from a small
  vocabulary: **data line equals X** (OpenAI `[DONE]`), **event named X**
  (Anthropic `message_stop`), **field equals X** (Ollama `done=true`), or
  **connection close** (the default). A stream that closes before its declared
  end yields an explicit "ended early" error item. Planning details left open:
  exact names of the forms and the shape of the "ended early" item.
- **D38. Error shape mapping.** The API declares a default error list of
  (status or range to shape); endpoints add, override, or opt out (a
  `no_declared_errors` form); the most specific wins (exact status over range
  over default); an unmatched status is an `Undeclared` failure that keeps the raw
  body. Planning details left open: range syntax and overlapping ranges. This
  supersedes the spike's endpoint-level status range (S8).
- **D48 (context).** An endpoint with no declared error body uses an uninhabited
  placeholder error type, so error replies arrive as kind `Undeclared` with
  status, headers, and raw body.
- **D17. GraphQL authoring.** A GraphQL operation is a string held in the
  definition with typed variables and typed response data. Schematic does not
  parse or validate GraphQL; the provider's server is the validator. A mismatch
  between the declared data type and the reply surfaces as a decode failure in
  the envelope.
- **D29. GraphQL errors and partial data.** Errors and partial data inside an
  HTTP 200 reply go on the **failure** side as
  `Declared(GraphQlFailure { errors, partial_data })`, with status 200 and the raw
  body kept, so `?` and `into_result()` cannot silently ignore them.
- **D34. GraphQL home.** GraphQL lives inside `schematic-define` (an operation
  primitive) and `schematic-gen` (generated into the same client crate), reusing
  the envelope, credentials, cross-origin rule, deadline, and pagination. Typed
  variables and data bind through the D2 definition context. Planning details:
  the primitive's name, the pagination vocabulary for GraphQL connections;
  batching is not addressed.
- **D35. GraphQL address.** The definition carries a rule that derives the GraphQL
  address from the client's base address (GitHub.com: append `/graphql`; an
  Enterprise base ending in `/api/v3`: replace that ending with `/api/graphql`),
  and the caller may override it. Planning details: how the rule is represented
  and how it recognizes the Enterprise base.
- **D2 and D44 (context from spec 4a).** Typed registration through a definition
  context; defaults come from the type; containers reuse the element's
  registration; `serde_json::Value` registers no component; aliases are explicit.
- **D14, D42, D43 (context from spec 3).** The pagination model has four strategy
  kinds, request inputs that may target the query, a header, or a JSON path in the
  body, and item selection by path (for example `edges[*].node`). The parent
  research names Relay connections as a pattern the model must cover.
- **D10 backstop (context from spec 1).** A declared feature must either reach the
  selected output or produce a diagnostic identifying API, endpoint, feature, and
  unsupported target.
- **D49 item 5 (context from spec 2).** Spec 2 owns the `ResponseKind` runtime
  guard and records its disposition (kept, changed, or removed). This spec makes no
  decision about the guard; it only builds against whatever spec 2 landed.
- **D50 and D51.** This is spec 5 of 8 (1 generation integrity, 2 runtime envelope,
  3 pagination, 4a definition API, 4b definition vocabulary, 5 this spec, 6
  websocket, 7 fidelity polish). It depends on spec 4b, which depends on 4a, whose
  builders already expose placeholders (`.returns(...)`, `.errors(...)`) that this
  spec widens. The spec is implemented unattended, then reviewed ("review 1").
- **D53 (context from spec 4a).** No per-endpoint typed convenience wrapper methods
  are generated.

## Requirements

Each requirement is testable. "Validated by" names the kind of test and the
package. L1 means `just test`; L2 means `just test-l2` and must never take window
focus. Package names are the crates under `schematic/`: `schematic-define`
(`define/`), `schematic-definitions` (`definitions/`), `schematic-gen` (`gen/`),
`schematic-schema` (`schema/`, generated), and the runtime package created by spec
2 (working name `schematic-runtime`; use whatever name spec 2 landed).

### Success shapes (D6)

**R1.** An endpoint declares a non-empty set of success shapes. Each shape is
exactly one of JSON-with-type, text, binary, empty, or stream (with a framing and
an item type), and may carry an optional content type. A definition with an empty
set, or two shapes that cannot be told apart by the rule in R3, fails validation
with a diagnostic naming the API and endpoint.
*Validated by:* L1 unit tests in `schematic-define` (construction and validation).

**R2.** The generated success type `T` is the shape's plain type when the endpoint
declares one shape (JSON: the registered Rust type; text: `String`; binary: bytes;
empty: `()`; stream: an item stream), and a generated enum with one variant per
shape when it declares several.
*Validated by:* L1 generator-output assertions in `schematic-gen` (one single-shape
and one multi-shape fixture); compile check of the generated fixture crate.

**R3.** For a multi-shape endpoint the client chooses the variant from the
response's `Content-Type` (media type only, parameters ignored, case-insensitive)
matched against each shape's declared content type. A response that matches no
declared content type is a failure inside the envelope with kind
`UnexpectedContentType`, status, headers, and the raw body kept. `ErrorKind` is
`#[non_exhaustive]`; `UnexpectedContentType` is this spec's addition to it (**PROPOSED**:
spec 2's PD17 excludes it, so the two are reconciled in Refresh notes and ruled on in
review 1). A single-shape
endpoint does not check the content type; a body that does not decode is
`MalformedSuccess` (D25).
*Validated by:* L1 mock-server tests (generated fixture client against a local
mock) in `schematic-gen` or the runtime package.

**R4.** Binary and text shapes record a content type that the OpenAPI exporter
reports (R27) and the importer preserves (R28). A declared content type is
documentation and a selector for R3; it does not cause the client to reject a
binary or text body whose actual content type differs.
*Validated by:* L1 in `schematic-define`/`schematic-gen` (export) plus the R3 mock
test with a mismatched content type on a single-shape binary endpoint.

**R5.** The `ElevenLabs` text-to-speech, sound-generation, history-audio, and
voice-sample endpoints that currently declare `ApiResponse::Binary` declare a
binary shape with an audio content type, and the exported OpenAPI for them no
longer reports `application/octet-stream`.
*Validated by:* L1 export test in `schematic-gen` over the real `schematic-definitions`
ElevenLabs API.

### Streams (D6, D37)

**R6.** A stream shape declares a framing (SSE or NDJSON), an item type, and an end
condition. The generated success type for it is a stream of
`Result<Item, StreamError>` (names are Proposed; see Design).
*Validated by:* L1 generator-output assertion plus the mock-server tests below.

**R7.** Items are delivered incrementally: a mock server sends several events or
lines with deliberate delays and the test observes the first decoded item before the
mock has finished sending.
*Validated by:* L1 mock-server test in the runtime package and one through a
generated fixture client in `schematic-gen`.

**R8.** The end condition is one of: data line equals X, event named X, field equals
X, or connection close (the default). For each form, a mock stream that reaches the
declared end finishes cleanly with no error item. For all three terminator forms
(data line, event, and field equals) the terminating line, event, or item is consumed
as the end marker and not delivered as an item. A field-equals item (Ollama
`done=true`, which carries the evaluation statistics) is therefore not delivered, and
those statistics are not available to the caller (Default D-18; the alternative,
delivering it as the last item, is on the Human review agenda for the author to rule
on at review 1).
*Validated by:* L1 mock-server tests, one per form, in the runtime package.

**R9.** A stream that closes before its declared end yields exactly one explicit
"ended early" error item and then ends. It is distinct from a clean end, from a
malformed item, and from a transport interruption. A connection-close stream never
reports "ended early".
*Validated by:* L1 mock-server test (mock closes early) in the runtime package.

**R10.** A malformed item yields a distinct error item that keeps the raw bytes of
the item, and the stream continues with the next item (Default D-5). A response
whose status is not 2xx is never decoded as a stream; it takes the error path.
*Validated by:* L1 mock-server tests in the runtime package.

**R11.** A stream endpoint's envelope is available as soon as response headers
arrive; `elapsed` is the time to headers (the spike's recommendation; spec 2 owns
the field).
*Validated by:* L1 mock-server test asserting the envelope is returned before the
mock finishes sending.

**R12.** Spec 2 owns the hook mechanism (a marker trait implemented by
JSON-decoded endpoints) and the `ResponseKind` guard disposition. This spec only
extends that mechanism to the new codecs: the stream codecs (`Sse`, `Ndjson`,
`Bytes`) do not implement the marker, so a hook registered for a text, binary, or
stream endpoint is a compile error in the generated client, not a silent no-op; the
`GraphQl<D>` codec implements it per R41. Any existing definition among the four
migrated APIs that has such a hook is fixed (D-26).
*Validated by:* compile-fail test (trybuild) in the runtime package or
`schematic-gen`; an L1 search-based test that no definition in
`schematic-definitions` carries such a hook.

### Same-address endpoints (D36)

**R13.** An endpoint may declare a selector: a top-level request-body JSON field
name and a scalar value (bool, string, or number). Validation accepts two endpoints
on the same method and path only when every endpoint on that pair declares a
selector and no two share the same `(method, path, selector)`. Two endpoints with
the same triple, or two with no selectors on one pair, fail validation.
*Validated by:* L1 unit tests in `schematic-gen` validation (the module that today
forbids duplicates) and in `schematic-define`.

**R14.** The generated request for a selector endpoint sends its selector value in
the body, overriding whatever the caller's body value holds for that field; the
caller cannot send the opposite variant through the typed request.
*Validated by:* L1 mock-server test that records the received body for each of the
two endpoints.

**R15.** The OpenAPI exporter emits one operation for the pair, whose response lists
both content types and whose request schema shows the selector field as an enum of
the declared values. Export then import of the merged operation is stable (R28).
*Validated by:* L1 export test in `schematic-gen`/`schematic-define`.

### Error shapes (D38)

**R16.** A `RestApi` declares a default error list of (status match, shape) entries.
A status match is exact (`404`), a range (`400..=499` or the class form `4xx`), and
is limited to statuses 400 through 599 (Default D-6, D-8).
*Validated by:* L1 unit tests in `schematic-define`.

**R17.** An endpoint may add entries, replace an entry with the same status match,
or opt out entirely with a `no_declared_errors` form. An opted-out endpoint that also
adds entries fails validation. The effective table for an endpoint is the API
default list overlaid by the endpoint's entries (same match replaces; opt-out
empties it).
*Validated by:* L1 unit tests in `schematic-define`.

**R18.** Lookup chooses the most specific match: an exact status beats a range, and a
narrower range beats a wider one; this applies to the merged effective table, so an
endpoint's range does not displace an API-level exact entry for a status inside that
range (Default D-7). Two ranges of equal width that overlap on some status, and two
identical matches at one level, fail validation, naming the API, the endpoint (if any),
and the overlapping statuses.
*Validated by:* L1 unit tests (lookup matrix and the overlap diagnostics) in
`schematic-define`.

**R19.** At run time, a status matched by an exact entry, a range entry, and the API
default resolves in that order. The matched body decodes to the entry's shape and
arrives as `Declared(E)`. A body that does not decode as the matched shape is kind
`MalformedError` with the raw body. A status matched by nothing, or any status on an
opted-out endpoint, is kind `Undeclared` with status, headers, and raw body. In every
case the raw body is kept.
*Validated by:* L1 mock-server tests (one status matching all three levels; opt-out;
unmatched; undecodable declared body) in `schematic-gen`/runtime.

**R20.** The generated failure type `E` is the matched shape's type when the
effective table has one distinct shape, a generated enum with one variant per
distinct shape when it has several, and the uninhabited placeholder (D48) when the
endpoint is opted out or the table is empty.
*Validated by:* L1 generator-output assertions in `schematic-gen`; compile check of
the fixture crate.

**R21.** The OpenAPI exporter lists every declared error status on each operation
(R27); status 2xx shapes and error shapes never share a response key.
*Validated by:* L1 export test.

### Exporters, importer, and the vocabulary change

**R22.** `Endpoint::response` no longer has the pre-v2 type. This spec owns updating
spec 4a's JSON baselines and its byte-comparison allow-list for that change (spec 4a's
frozen tests are expected to differ exactly where `Endpoint::response` serializes, and
only there). Everything that read it
compiles against the new vocabulary: the generator (client, request structs, enum,
module docs, traits), validation, the OpenAPI exporter and importer, the Postman
exporter, and the vendor-extension serializer.
*Validated by:* workspace build through the repo recipes (`just test` from the repo
root, then `just test {pkg}` per touched package) and the existing L1 suites of
`schematic-define` and `schematic-gen`.

**R23.** Validation's duplicate rule is `(method, path, selector)` (R13) and endpoint
ids stay unique. Validation also rejects: a stream shape with a missing item type, a
selector field absent from a known body schema (when the schema is known), and a
multi-shape endpoint whose shapes are indistinguishable by R3.
*Validated by:* L1 unit tests in `schematic-gen` validation.

**R24.** Every endpoint in `schematic-definitions` still builds. Only the four named
APIs' definitions (Ollama, LM Studio, Anthropic, ElevenLabs) are edited; every other
endpoint keeps its call site untouched, because the `.returns`/`.errors` placeholders
spec 4a created are widened to accept the new vocabulary without changing existing
calls, and every generated single-shape JSON, text, binary, and empty endpoint keeps its
current Rust type.
*Validated by:* L1 diff-style test: regenerate `schematic-schema` and assert the set
of changed public types equals the migration list recorded in the implementation log;
spec 4a's source-scan test (no hand-written literal survives outside the builders)
stays green.

**R25.** `schematic-schema` is regenerated with the generator, never hand-edited, and
compiles; `just lint-schema` passes.
*Validated by:* `just lint-schema` and the L1 generated-code compile check from spec 1.

### OpenAPI and Postman (forced by the vocabulary change)

**R26.** The vendor extension (`x-schematic`) serializes the new response vocabulary
without panicking or dropping fields. It must carry the new response fields, the error
table, the selector, and (from specs 4a, 4b, and 2) rate-limit, deprecated, auth
override, endpoint `cross_origin`, and request-id, so that spec 7's subset covers
them. Reading it back is out of scope (the round trip
is `schematic-fidelity-polish`).
*Validated by:* L1 serialization test in `schematic-define`.

**R27.** The OpenAPI exporter emits, per operation: one response per declared success
shape keyed by status (default `200`, or `204` for empty), a `content` entry per
shape's content type (`application/json` default for JSON, `text/event-stream` for
SSE, `application/x-ndjson` for NDJSON, the declared type for text and binary,
`text/plain` and `application/octet-stream` only when none is declared), and one
response per declared error status. Class-aligned ranges export as `4XX`/`5XX`; other
ranges export as the expanded exact statuses (Default D-25).
*Validated by:* L1 export tests in `schematic-gen` with `openapiv3` round-trip parse of
the output.

**R28.** The OpenAPI importer maps: `text/event-stream` to an SSE stream; `application/x-ndjson`
and `application/jsonl` to an NDJSON stream; `text/*` to text with that content type;
other non-JSON media types to binary with that content type; several content types
under one status to a multi-shape success; non-2xx responses to endpoint-level error
entries. Import followed by export preserves content types (the ElevenLabs audio case)
and does not hoist common errors to the API list (Default D-20).
*Validated by:* L1 import/export round-trip tests in `schematic-define`/`schematic-gen`.

**R29.** The Postman exporter compiles against the new vocabulary and records the first
success shape's content type; it needs no new fidelity beyond that.
*Validated by:* existing Postman golden tests in `schematic-gen` (updated as needed).

### Migration of definitions

**R30.** Ollama's native streaming endpoints (generate, chat, pull, push, create, and
any other that streams by default) are migrated to NDJSON stream shapes with item
types and the `done=true` end condition where the reply says so. For generate and chat
(which also support `stream: false`) a second endpoint with a non-streaming JSON shape
exists and the pair is distinguished by the selector (R13). Ollama's OpenAI-compatible
chat and completion endpoints use SSE with the data-line `[DONE]` end condition. The
ten stranded Ollama types referenced by the parent become reachable (each is used by a
registered endpoint shape or is recorded in the log as still unused with the reason).
Unverified: the exact endpoint list; the implementer derives it from
`schematic/definitions/src/ollama/mod.rs`.
*Validated by:* L1 definition tests in `schematic-definitions` (shape of each migrated
endpoint) plus generated-output compile; mock-server test through the generated Ollama
client for one NDJSON and one SSE endpoint.

**R31.** LM Studio's two endpoints annotated `// SSE streaming` become SSE stream shapes
with an item type and end condition (data line `[DONE]` unless the research doc
`schematic/docs/research/lm-studio-api.md` says otherwise).
*Validated by:* L1 definition tests in `schematic-definitions`.

**R32.** Anthropic `CreateMessage` becomes a selector pair: the non-streaming endpoint
(`stream=false`, JSON message) and a streaming endpoint (`stream=true`, SSE with item
type for the Messages stream events and end condition event named `message_stop`).
*Validated by:* L1 definition tests plus mock-server test through the generated
Anthropic client for both endpoints.

**R33.** ElevenLabs `StreamSpeech` declares a streaming binary shape so the generated
method no longer buffers the whole body (Default D-17: byte-chunk framing, an
**extension of D6's closed framing set (SSE, NDJSON), PROPOSED** and on the Human review
agenda). The
timestamps stream variant is NDJSON when its reply is line-delimited JSON; otherwise
it stays a JSON or text shape and the implementer logs why. Unverified: ElevenLabs'
reply framing; confirm against `schematic/docs/research/elevenlabs-api.md` and the
existing definitions.
*Validated by:* L1 definition test; mock-server test that the first audio chunk is
observed before the mock finishes.

**R34.** Dependents of `schematic-schema` that call a migrated endpoint compile and
pass their tests: `sniff/lib`, `biscuit-speaks/lib`, `model-citizen/lib`, and the five
homelab crates (`homelab/lib`, `eversolo-integration`, `arcam-amp-integration`,
`sony-receiver-integration`, `unfolded-integration-helper`). Per the parent, the
listed three have no Rust references to `schematic_schema` as of 2026-10-09
(unverified now); the implementer greps and edits only call sites whose success or
failure type changed.
*Validated by:* `just test {pkg}` from the repo root for each dependent package; the
implementation log lists every call site changed.

### GraphQL (D17, D29, D34, D35)

**R35.** `schematic-define` has a GraphQL operation primitive holding: an id, a
description, the document string, a variables type, a data type, and optional
pagination. Variables and data types are registered through the D2 definition context
(spec 4a); a variables value of the wrong type fails to compile at the call site, and a
data type that was never registered is a validation error. Operations belong to a
`RestApi` (the same API that owns the base address and auth).
*Validated by:* L1 unit tests in `schematic-define`; compile-fail test for the wrong
variables type; L1 validation test for the unregistered data type.

**R36.** `schematic-gen` generates a typed request for each operation into the same
client crate as the REST endpoints, sent through the same generic send, with the same
credentials, cross-origin rule, deadline, and envelope. The wire body is
`{"query": <document>, "variables": <typed variables>}` (and `operationName` only when
the author sets one; Default D-21). The document is sent exactly as written; the
generator does not parse it.
*Validated by:* L1 mock-server test asserting the recorded request body; a test that a
syntactically invalid document is sent unchanged.

**R37.** A GraphQL reply is decoded as follows. HTTP 200 with `data` and no (or empty)
`errors` is success with the typed data. HTTP 200 with a non-empty `errors` list is a
failure inside the envelope: `Declared(GraphQlFailure { errors, partial_data })`, status
200, raw body kept, where `partial_data` is the typed `data` when present and decodable
and absent otherwise. HTTP 200 with `errors` and no `data` is a failure with
`partial_data` absent. `?` and `into_result()` return `Err` for any failure. A 200 whose
body is not a GraphQL reply, or whose `data` does not decode as the declared type, is
`MalformedSuccess`. A non-2xx reply takes the REST error path (R19) using the API error
list.
*Validated by:* L1 mock-server tests: partial data with a forbidden-field error (using
the shape of the sniff fixtures `partial_errors.json` and `request_wide_errors.json`),
success, errors-only, malformed, and non-2xx.

**R38.** `GraphQlFailure` carries `errors` (each: message, optional path, optional
locations, optional extensions as JSON) and `partial_data: Option<D>`.
*Validated by:* L1 deserialization tests against the sniff fixtures
(`passed.json`, `partial_errors.json`, `request_wide_errors.json`,
`permission_failure.json`, `null_rollup.json`).

**R39.** The GraphQL address is derived from the client's base address by a rule in the
definition: for each rule in order, the first that matches wins. The GitHub rules are:
a base whose path ends in `/api/v3` has that ending replaced by `/api/graphql`;
otherwise `/graphql` is appended. The caller may override the full address on the client
builder (the managed builder from spec 2); an override beats the rule. An override on a
different origin than the base is subject to the API's cross-origin policy like any
other request (Default D-11).
*Validated by:* L1 tests with two mock bases (`.../api/v3` and a GitHub.com-style base
without it) asserting the final path for REST and GraphQL; override test; override to a
foreign origin under `same_origin` is refused.

**R40.** A GraphQL operation may declare pagination using the Relay connection pattern
through the existing pagination model (spec 3): a cursor strategy whose request inputs
are JSON paths into the request body (`variables.first`, `variables.after`), whose end
signal is `pageInfo.hasNextPage`, whose cursor is read from `pageInfo.endCursor`, and
whose items are selected by path (`nodes` or `edges[*].node`) under the operation's
data root (Default D-12). The generated pager exposes only the knobs the strategy
authors (as for REST).
*Validated by:* L1 mock-server test paging a two-page connection to the end and a
compile-fail test that an unauthored knob is not callable.

**R41.** Hooks on a GraphQL operation run on the typed `data`, not on the
`{data, errors}` wrapper: a response mutator receives the decoded data type, and the
client-global pre-response JSON hook is documented as seeing the wire body (the wrapper)
for GraphQL.
*Validated by:* L1 test that a mutator receives the data type (compile-checked) and
that the wire-level hook sees the wrapper.

**R42.** GraphQL operations are not emitted by the OpenAPI or Postman exporters in this
spec. Each export that skips an operation produces a diagnostic identifying the API,
the operation, the feature ("GraphQL operation"), and the target output; the diagnostic
does not fail the run (D10 backstop).
*Validated by:* L1 test in `schematic-gen` exporting an API that has an operation and
asserting the diagnostic.

**R43.** sniff-facing parity: a test in `schematic-definitions` or `schematic-gen`
defines (in a test-only definition, not in the shipped definitions) the GitHub
check-status operation with its document and typed variables and data, decodes every
fixture in `sniff/lib/tests/fixtures/remote/graphql_check_status/` through the generated
path, and asserts the success, partial, and failure classification matches the fixture
names (`success`, `passed`, `partial_errors`, `request_wide_errors`, `permission_failure`,
`rate_limit`, `changed_head`, `null_rollup`). sniff itself is not edited by this spec.
*Validated by:* L1 test in `schematic-gen` reading the fixtures by repository path
(follow the `rust-testing` skill's forms for repository reads, and the
`docs/cicd/test-inputs.md` rule for declaring them).

### Cross-cutting

**R44.** Every new definition type this spec adds (success shapes, stream framing and
end conditions, status matches, the error set, the selector, the GraphQL operation and
address rule) derives `Serialize`, `Deserialize`, and `PartialEq` (and `Debug`/`Clone`
as the surrounding types do), because spec 4a's round-trip test serializes every
definition and now covers `Endpoint::response` and the `RestApi` error set. The earlier
claim of "no new serde obligation" was wrong.
*Validated by:* spec 4a's definition round-trip test over the real
`schematic-definitions`, including the four migrated APIs.

## Design

All symbol names are **Proposed** unless marked Decided. The only Decided items are
the behaviors in "Background" (D-numbers). Where specs 4a and 4b have already fixed a name or
signature (builder methods, the definition context), conform to it and log the
difference.

### `schematic-define` (package: schematic-define)

- **Success vocabulary (Proposed).** `ResponseShape` with variants `Json(Schema)`,
  `Text`, `Binary`, `Empty`, `Stream { framing: StreamFraming, item: Schema, end:
  StreamEnd }`; `SuccessShape { shape: ResponseShape, content_type: Option<String> }`;
  an endpoint holds a non-empty `Vec<SuccessShape>`. `StreamFraming` is `Sse`,
  `Ndjson`, and (Default D-17, PROPOSED extension of D6's closed set) `Bytes`.
  The definition struct is `Endpoint` in `schematic-define`; any new type here must not
  reuse that name. The runtime trait that generated request types implement is
  `EndpointSpec` (spec 2's default), never `Endpoint`.
- **`StreamEnd` (Proposed).** `DataLineEquals(String)`, `EventNamed(String)`,
  `FieldEquals { field: String, value: serde_json::Value }` (a dotted path allowed),
  `ConnectionClose` (the default).
- **Error vocabulary (Proposed).** `StatusMatch` (`Exact(u16)`, `Range(RangeInclusive<u16>)`,
  with a `class(4)` constructor); the error set is spec 4a's `ErrorSet`
  (not `ErrorTable` or `ErrorRule`), widened here to hold (`StatusMatch`, shape)
  entries, on `RestApi` (default list) and on `Endpoint` (additions, plus a
  `no_declared_errors` flag); a pure function that merges and resolves them. The
  `.returns`/`.errors` placeholders spec 4a created are widened to accept this
  vocabulary without touching any existing call site.
- **`Selector` (Proposed).** `{ field: String, value: SelectorValue }`, where
  `SelectorValue` is bool, string, or number; an optional endpoint field.
- **Validation (Proposed, Decided in rule).** Overlap and uniqueness checks of R16-R18
  and R23 live beside the types so the generator and the importer can reuse them.
- **GraphQL (Proposed).** `GraphQlOperation { id, description, document: String,
  variables: Schema-or-unit, data: Schema, pagination: Option<..>, operation_name:
  Option<String> }`; `GraphQlAddress` as an ordered list of `AddressRule`
  (`ReplaceSuffix { suffix, with }`, `Append(path)`) on `RestApi`, plus the API-level
  setting that a GraphQL surface exists. Registration of `variables` and `data` goes
  through the D2 context (spec 4a).
- **Replaced (Decided by D6).** `ApiResponse` and `Endpoint::response`. Whether
  `ApiResponse` survives as a thin constructor for the single-shape case (Proposed:
  yes, as builder sugar only) is left to the shape of spec 4a's builders; log it.

### `schematic-runtime` (package created by spec 2; extend it)

- **Codecs (Proposed).** `Sse<T>`, `Ndjson<T>`, `Bytes stream`, `GraphQl<D>`, and the
  shared multi-shape machinery (a content-type dispatcher the generated enum calls).
  Spec 2 may already have created `Json`, `Text`, `Binary`, `Empty`; check first and
  extend rather than duplicate (Default D-22).
- **`StreamError` (Proposed).** Item errors for a stream: `Malformed { raw }`,
  `EndedEarly`, `Interrupted` (a body break mid-stream).
- **`GraphQlFailure<D>` and `GraphQlError` (Proposed names; shape Decided by D29).**
- **Error decode (Proposed).** The runtime resolves a status against the endpoint's
  effective error table (generated as a constant lookup) and returns `Declared`,
  `MalformedError`, or `Undeclared`.

### `schematic-gen` (package: schematic-gen)

- **Client and request codegen (Proposed).** Per endpoint: `type Success`, `type
  Failure`, and the codec constant; a generated enum for multi-shape success and for
  multi-shape failure; selector injection in the request body; no per-endpoint wrapper
  method (spike cost finding: about 7 s extra per 550 calls).
- **Hooks.** Spec 2 owns the marker-trait mechanism; this spec extends it to the stream
  and GraphQL codecs (R12) and adds no mechanism of its own.
- **GraphQL codegen (Proposed).** A request type per operation implementing the same
  endpoint trait with the `GraphQl<D>` codec; address resolution from the rule and
  the caller override.
- **Validation (Proposed).** `validation.rs` gets the new duplicate rule, shape checks,
  and the skipped-feature diagnostics of R42.
- **Exporters (Proposed).** OpenAPI and Postman exporters read the new vocabulary;
  merging of selector endpoints; status expansion for ranges.

### OpenAPI import (package: schematic-define, `openapi/import`)

- **Mapping (Proposed).** `import/mappings/responses.rs` and `import/builder.rs` build
  `SuccessShape`s and endpoint error entries per R28.

### `schematic-definitions` (package: schematic-definitions)

- **Migration (Decided in scope by D6; shapes Proposed).** Ollama, LM Studio, Anthropic,
  ElevenLabs as in R5, R30-R33. No other API's definitions are edited. API-level default error lists are added only where the API already
  has a documented shared error type (Default D-19).

## Suggested phases

The plan may refine this outline. Every phase leaves the tree green (builds, L1 tests
for touched packages pass). Phases 2-4 are additive; phase 5 is the single atomic
switch of `Endpoint::response`.

1. **Baseline and inventory.** Read specs 4a and 4b outcomes (refresh notes). Record in the
   log: counts of endpoints per current `ApiResponse` variant, endpoints whose request
   body has a `stream` field, definitions with hooks, spec 2's runtime codec inventory,
   and the sniff fixtures present. No code change.
2. **Vocabulary and validation in `schematic-define`.** New types, error table
   resolution, selector, validation, all behind unit tests; the old `ApiResponse`
   path still compiles.
3. **Runtime codecs.** Sse, Ndjson, Bytes, GraphQl, end conditions, multi-shape
   dispatcher, error-table decode, `StreamError`, with mock-server tests in the
   runtime package.
4. **Generator.** Emit success and failure types, enums, selector injection, hook
   compile error, GraphQL requests and address; test against a generated fixture
   crate. The old path still works for definitions not yet migrated.
5. **Switch and migrate.** Change `Endpoint::response` to the new vocabulary; update
   validation, exporters, importer, Postman, extension serializer; migrate all
   the four named APIs by hand (no other definition is edited); update spec 4a's JSON
   baselines and byte-comparison allow-list; regenerate `schematic-schema`; fix
   dependents. Land only when the workspace builds.
6. **Exporters and importer behavior.** R27-R29, R42 with their tests, including the
   ElevenLabs content-type case and the selector merge.
7. **GraphQL.** Primitive, address rule, failure decode, pagination vocabulary,
   hooks on data, test-only check-status parity (R43).
8. **Docs, skills, drift, final verification.** Update README, `schematic/docs`
   topic pages, and `.claude/skills/schematic-define` (and any other affected skill);
   comment/docs drift pass; run the full recipes.

## Acceptance criteria and definition of done

The spec is "implementation complete, ready for review" when all of these hold:

1. Every requirement R1-R44 has a passing test of the kind named, or a logged
   deviation explaining why not.
2. The workspace builds and tests pass on macOS, Linux, native Windows, and WSL2 per
   the repository rules. The unattended run proves what its host can prove and records
   which environments were not exercised; it does not claim others (load the `os` skill
   before saying an OS cannot be tested from the current host).
3. `just test` and `just lint` pass for `schematic` and for every affected dependent
   package; `just lint-schema` passes; `just test-l2` is run where an affected package
   has L2 tests, and never takes window focus.
4. `schematic-schema` is regenerated by the generator (never hand-edited) and the
   regeneration is idempotent (a second run produces no diff).
5. Every dependent crate builds and its tests pass (R34).
6. README files, `schematic/docs` topic pages (the audience is a developer with no
   repository experience: lead with what the reader can do, a compact example per rule,
   a Mermaid diagram for the stream and error-resolution flows), and affected skills
   (`.claude/skills/schematic-define/` at least) describe the new behavior. A page that
   describes behavior that no longer exists is a defect of this change. `docs/` pages
   never name this feature by path or date.
7. A comments and docs drift pass is done for every symbol whose behavior changed;
   where code and comment disagree the code is assumed right and the drift is logged.
8. The implementation log (`implementation-log.md` in this directory) exists and lists
   every item of the Human review agenda.
9. The agent makes no commits unless the plan says otherwise, and never moves this spec
   to `_completed`.

## Pre-authorizations

The author must confirm these before the unattended run (D50 item 8); until confirmed,
none is assumed. Once confirmed, the run may do the following without pausing:

- Regenerate `schematic-schema` with the generator, including deleting generated files
  the generator itself reports as obsolete and owned.
- Add or extend codecs, types, and tests in the existing runtime package created by
  spec 2; add dev-dependencies needed for mock-server and compile-fail tests (use the
  mock-server crate spec 2 chose; default `wiremock`) and update `Cargo.lock`.
- Delete the stale `schematic/gen/schematic` tree only if it still exists and nothing
  references it (spec 1 may already have removed it).
- Edit dependent crates (`sniff`, `biscuit-speaks`, `model-citizen`, the five homelab
  crates) only where a migrated endpoint's success or failure type forces a change, and
  log each edit. Do not otherwise refactor them.
- Rewrite the endpoint definitions of the four named APIs (Ollama, LM Studio,
  Anthropic, ElevenLabs) in `schematic-definitions` by hand. No other API's definitions
  are edited.
- Update `schematic/docs/`, READMEs, and `.claude/skills/` pages as required by
  Drift Maintenance.
- Run the repo recipes (`just test`, `just test-l2` when it does not take window focus,
  `just lint`, `just lint-schema`, and `just test {pkg}` from the repo root).

Not pre-authorized: committing, pushing, changing CI configuration or scope, editing
sniff beyond forced call sites, adding a new workspace package, and modifying the
parent spec.

## Dependencies and ordering

- **Requires, landed first:** `2026-10-11-schematic-definition-vocabulary` (spec 4b),
  which in turn requires `2026-10-11-schematic-definition-api` (spec 4a): builders, the
  definition context and typed registration (D2, D44), the `.returns`/`.errors`
  placeholders, and the removal of the old pagination types. If those surfaces landed
  differently from what this spec assumes, conform to what landed and log it.
- **Transitively requires:** spec 2 (`2026-10-11-schematic-runtime-envelope`: envelope,
  typed send, the `EndpointSpec` trait, codecs, hook marker trait, managed builder,
  `send_raw`, cross-origin, the `ResponseKind` disposition), spec 3
  (`schematic-pagination`: the pagination model GraphQL pagination uses), and spec 4a.
  Spec 4b depends on them, so they are expected to be landed; the baseline phase
  verifies this and the run stops (and logs) if spec 2's envelope is absent. The
  frontmatter lists only spec 4b; the transitive needs (2, 3, 4a) are stated here.
- **Blocks:** spec 6 (WebSocket; independent of REST internals once spec 4a lands) and
  spec 7 (fidelity and polish), which refresh from this spec's review 1. Sniff retiring
  its hand-written provider client starts after spec 3 and reads this spec's GraphQL
  result.
- **Landing:** landing 2 (D30, D50 item 7).

## Planning-time details assigned to this spec

The parent assigns P9 (selector, end conditions, error mappings, GraphQL names) for
D34-D38 to this spec. Each default below is adopted by the implementer and logged. They
are not decisions until the author confirms them in review 1.

- **Default D-1 (selector syntax).** Parent leaves open. *Default if not ruled before
  planning:* a builder method `.selector("stream", false)` taking a field name and a
  scalar (bool, string, or number), top-level body field only. A selector on a method
  without a JSON body, or on a nested field, is a validation error.
- **Default D-2 (naming the two request types).** *Default if not ruled before planning:*
  the author names both endpoint ids explicitly; the convention used for the migrated
  definitions is `<Name>` for the non-streaming endpoint and `<Name>Stream` for the
  streaming one (for example `CreateMessage` and `CreateMessageStream`). The generated
  request type name follows the endpoint id as it does today.
- **Default D-3 (merged OpenAPI request schema).** *Default if not ruled before
  planning:* one operation, one request body schema (the shared body type) in which the
  selector property is shown as an enum of the declared values; the operation carries an
  `x-schematic-selector` extension listing each endpoint id and its value so that import
  can split it again (the splitting on import is not required by this spec).
- **Default D-4 (end-condition names).** *Default if not ruled before planning:*
  `StreamEnd::{DataLineEquals, EventNamed, FieldEquals, ConnectionClose}`; builder forms
  `.ends_at_data_line("[DONE]")`, `.ends_at_event("message_stop")`,
  `.ends_when_field("done", true)`, and the implicit default.
- **Default D-5 (the "ended early" item and item errors).** *Default if not ruled before
  planning:* the stream yields `Result<Item, StreamError>`; `StreamError` is
  `#[non_exhaustive]` with `Malformed { raw }`, `EndedEarly`, `Interrupted`. After
  `EndedEarly` or `Interrupted` the stream ends; after `Malformed` it continues.
- **Default D-6 (range syntax).** *Default if not ruled before planning:* `Exact(u16)` and
  inclusive `Range(lo..=hi)` with a `class(n)` shorthand (`class(4)` is `400..=499`);
  OpenAPI's literal `4XX` and `default` are import-time spellings only.
- **Default D-7 (overlapping ranges and level precedence).** *Default if not ruled before
  planning:* ranges may overlap; specificity is exact, then narrower range, then wider
  range; equal-width partial overlaps and duplicates are validation errors; an endpoint
  entry replaces an API entry only when the match is identical; otherwise specificity
  decides across the merged table. (See the Human review agenda: the parent says both
  "endpoints override" and "most specific wins".)
- **Default D-8 (error-status domain).** *Default if not ruled before planning:* 2xx is
  the success path; any other status is the failure path; declared matches are limited to
  400 through 599. A 3xx the managed client reports as a refused redirect keeps spec 2's
  kind.
- **Default D-9 (GraphQL primitive name).** *Default if not ruled before planning:*
  `GraphQlOperation` (the parent's illustrative name), with a `GraphQlOperation::query(id,
  document)` / `::mutation(id, document)` constructor pair (the kind is documentation
  and an OpenAPI-skip label only; it does not change the wire).
- **Default D-10 (address rule representation).** *Default if not ruled before planning:*
  `GraphQlAddress` is an ordered `Vec<AddressRule>`; the GitHub preset is
  `[ReplaceSuffix { "/api/v3" -> "/api/graphql" }, Append("/graphql")]`. A base is
  recognized as Enterprise purely by its path ending in `/api/v3` (trailing slash
  ignored).
- **Default D-11 (override and cross-origin).** *Default if not ruled before planning:* a
  client-builder method sets a full GraphQL URL; it is still checked against the API's
  cross-origin policy, so overriding to another origin requires a policy that allows that
  origin.
- **Default D-12 (GraphQL pagination vocabulary).** The pagination spike's default:
  Relay-style cursor via the existing Cursor strategy with JSON-path inputs (Relay-style
  cursors appear only in the pagination spike files, not as a parent decision). *Default if not ruled before
  planning:* a preset constructor `Strategy::relay_forward(root_path)` that expands to
  the cursor strategy of R40 (inputs `variables.first`/`variables.after`, more-signal
  `pageInfo.hasNextPage`, cursor `pageInfo.endCursor`, items `nodes`), a
  `.with_edges()` variant (items `edges[*].node`), and an optional backward leg
  (`last`/`before`, `hasPreviousPage`, `startCursor`). The variable names are
  overridable. Page-size maximum is author-declared as in D14.
- **Default D-13 (batching).** Not addressed by any decision. *Default if not ruled before
  planning:* not supported; log it as out of scope.
- **Default D-14 (multi-shape selection).** Parent is silent. *Default if not ruled before
  planning:* Content-Type match per R3; two shapes with no distinguishing content type
  fail validation.
- **Default D-15 (multi-shape enum naming).** *Default if not ruled before planning:*
  `<EndpointId>Response` with variants named after the shape (`Json`, `Text`, `Binary`,
  `Empty`, `Stream`), or after the content type when two shapes share a kind
  (`Json`/`JsonProblem`) -- the author may name variants in the definition.
- **Default D-16 (content type checks).** *Default if not ruled before planning:* none for
  single-shape endpoints; only R3 for multi-shape. SSE and NDJSON decoding do not require
  the matching response `Content-Type`.
- **Default D-17 (raw byte streams).** The parent's D6 stream framings are SSE and NDJSON
  only (a closed set), but the parent also lists ElevenLabs `stream_speech` as in scope.
  *PROPOSED default if not ruled before planning:* add a third framing `Bytes` (raw
  chunks) whose item is a byte chunk. This is an extension of D6's closed framing set and
  is flagged for review 1.
- **Default D-18 (field-equals delivery).** See R8. *Default if not ruled before
  planning:* parent-compliant. The matching item (Ollama `done=true`) is consumed as the
  end marker and not delivered, like the data-line and event terminators. Trade-off: that
  item carries Ollama's evaluation statistics, which are then unavailable to the caller.
  The alternative (deliver it as the last item) is on the Human review agenda.
- **Default D-19 (migration scope).** *Default if not ruled before planning:* hand-migrate
  Ollama, LM Studio, Anthropic, and ElevenLabs only; any other API found with streaming (a
  `stream` body field or an SSE/NDJSON reply, for example OpenAI chat completions) is
  recorded in the log and its definitions are not edited. API-level default error lists are added only for APIs that already define a
  shared error type.
- **Default D-20 (importer mapping).** See R28. Error entries stay endpoint-level; no
  hoisting to the API list.
- **Default D-21 (GraphQL `operationName`).** *Default if not ruled before planning:*
  omitted unless the author sets it on the operation.
- **Default D-22 (codec home).** *Default if not ruled before planning:* codecs and the
  dispatcher live in the runtime package; the generator emits only constants, marker
  impls, and the multi-shape and failure enums.
- **Default D-23 (failure enum naming).** *Default if not ruled before planning:*
  `<EndpointId>Error` with one variant per distinct shape named after the shape's type.
- **Default D-24 (`x-schematic` output).** *Default if not ruled before planning:* serialize
  the new types with serde in the extension; no stability promise beyond this spec.
- **Default D-25 (range export).** See R27: class-aligned ranges as `4XX`/`5XX`, other
  ranges expanded to exact statuses.
- **Default D-26 (hooks on migrated definitions).** *Default if not ruled before
  planning:* an existing definition with a hook on a text, binary, or stream endpoint has
  the hook removed and the removal logged.
- **Default D-27 (no-variables operation).** *Default if not ruled before planning:* `()` as
  the variables type serializes to an omitted `variables` field.
- **Default D-28 (sniff-facing checks).** See R43: a test-only parity definition, no sniff
  edit.
- **Default D-29 (GraphQL in exporters).** See R42: skip with a diagnostic.
- **Default D-30 (error `Debug` and redaction).** *Default if not ruled before planning:*
  error bodies follow D22 (stored metadata and `Display` are credential-free; a decoded
  error body prints as received in `Debug`); GraphQL `errors` are not redacted.

## Risks and failure modes

- **Spec 4a's builder surface may not fit.** If the `.returns(...)` and `.errors(...)`
  placeholders were shaped for a narrower vocabulary, widening them could force edits to
  the roughly 519 endpoint literals (OpenAI alone has 265) that spec 4a already rewrote,
  which D50's condition forbids. Mitigation: phase 1 compares the two and logs a mismatch
  before any edit; widening must leave call sites untouched and spec 4a's source-scan test
  green.
- **The `Endpoint::response` switch is large and atomic.** Phase 5 breaks the workspace
  until generator, exporters, importer, the four migrated definitions, spec 4a's JSON
  baselines, and dependents all compile.
  Mitigation: phases 2-4 are additive; phase 5 lands only when the build is green.
- **Behavior change on migration.** Making Ollama's `stream` default explicit can change
  wire behavior (Ollama streams by default when the field is omitted). Mitigation: the
  streaming endpoint always sends `stream: true`, and the non-streaming one `stream:
  false`, by the selector; mock tests record the bodies (R14).
- **Unverified provider facts.** The reply framing and end conditions of Ollama (including
  whether the `done` item, which carries the evaluation statistics, should be delivered
  rather than consumed), LM Studio, Anthropic, and ElevenLabs are
  taken from the parent and from the repository's research documents, not re-verified
  against live providers. Treat them as unverified and flag them in review 1.
- **Compile-time and binary-size cost.** Per-endpoint wrapper methods cost about 7 s at
  550 calls in the spike (reported). Pagers add about 1.77 MB at 157 endpoints
  (reported). GraphQL adds codecs but no per-endpoint wrapper.
- **GraphQL contract rests on the author's declared types.** With no validation (D17), a
  wrong data type surfaces as `MalformedSuccess` at run time. A reply that is mostly good
  but carries an error is on the failure side (D29); callers wanting partial data must
  match the kind.
- **Mock-server coverage is not provider coverage.** No test here contacts a real provider.
  Streaming over HTTP/2, cancel-on-drop of a stream, and non-UTF-8 headers were not tested by
  the spikes and are not required by this spec; a stream dropped mid-reply is expected to
  close its connection but that is unverified.
- **OpenAPI limits.** The `openapiv3` crate models OpenAPI 3.0, which has no streaming item
  schema; streams export with the item schema and an extension. Round-trip fidelity for
  streams is limited (R28 maps content types only).
- **sniff's migration depends on this work but not on this spec's code in place.** The
  parity test (R43) is the only guard that the generated GraphQL path can carry sniff's
  query; sniff's own spec starts after spec 3 and may find gaps this spec did not.
- **Windows and WSL2.** Mock-server tests bind local ports; the `os` skill lists the
  Windows and WSL2 traps. Evidence for those environments may not be producible from the
  unattended host; say so rather than claim it.

## Process

(Confirmed by the author; D50 and D51.)

1. This spec has a 1:1 plan. The plan is implemented end to end, phase by phase,
   **without a human in the loop**.
2. Then **review 1**: time is set aside for the author to review the decisions made during
   implementation and any open design questions.
3. Then the **next** spec and its plan (here: `schematic-websocket`, and
   `schematic-fidelity-polish`, which also reads this spec's exporter outcomes) are refreshed
   from the outcomes of review 1.
4. Then fix and review cycles repeat until the reviewer deems the spec production ready.
5. An agent's terminal state is "implementation complete, ready for review". Only the
   author moves a spec to `_completed`; the agent never does, and never runs
   `just complete`.

Because no human is available while the plan runs, the implementer:

- treats this spec as self-contained (the parent is provenance, not required reading);
- adopts every "Default if not ruled before planning" above and **logs it** when adopted;
- never pauses for a question; where something is not covered, chooses the smallest
  option consistent with Rule 2 (Simplicity First), logs it, and keeps going;
- logs each item of the Human review agenda **as it occurs**, not at the end, in
  `implementation-log.md` in this directory (frontmatter in the style of the worktree
  feature log: spec path, plan path, `implemented_by`, source files per phase, docs updated
  or created per phase, skills updated per phase, packages touched).

## Human review agenda

For review 1, `implementation-log.md` must record each of the following, with the phase,
what was chosen, the alternatives considered, and why:

1. **Every default adopted** (D-1 through D-30), including any that was changed because the
   code made the default unworkable.
2. **Every design choice not dictated by this spec**: symbol names and signatures that
   differ from the Proposed names; where each codec and type finally lives; builder-surface
   differences from specs 4a and 4b; the shape of `StreamError`, `GraphQlFailure`, and the
   generated enums.
3. **Every deviation** from a requirement, a decision, or a parent criterion, with the
   test that stands in for it. Known candidates: the `Bytes` framing as a PROPOSED
   extension of D6's closed framing set (D-17, R33); expanding non-class ranges on export
   (D-25).
4. **Every open design question found**, including, for the author to rule on at review 1:
   - **Ollama final item (D-18, R8):** the default consumes the `done=true` item as the end
     marker (parent-compliant), so its evaluation statistics are not available; the
     alternative is to deliver the final item as the last item. Surface the trade-off.
   - **`UnexpectedContentType` (R3):** this spec's PROPOSED addition to the
     `#[non_exhaustive]` `ErrorKind`; spec 2's PD17 excludes it.
   - **Malformed stream item (R10):** a malformed item yields an error item and the stream
     continues with the next item; confirm that continuing is wanted.
   - **Level precedence (D-7):** the parent says both "endpoint overrides" and "most
     specific wins"; whether endpoint entries should win outright (endpoint-first) or
     specificity decides across the merged table.
   - **`Bytes` framing (D-17, R33):** PROPOSED extension of D6's closed framing set.
   - **Selector scope (D-1, R13):** a top-level body field only; nested fields and
     non-JSON bodies are rejected.
   - Also: how the ElevenLabs
   `output_format` query parameter relates to the declared audio content type; whether a
   stream's `elapsed` should mean time to headers; what GraphQL batching would need;
   whether GraphQL operations should eventually appear in OpenAPI and Postman output.
5. **The inventory results** of phase 1 and **the final list of changed public types** (R24),
   plus every dependent call site edited (R34).
6. **Unverified provider facts** assumed in the migrations (Ollama, LM Studio, Anthropic,
   ElevenLabs framing, end conditions, and content types) and which research document or
   code each came from.
7. **Any hook removed** (D-26). The `ResponseKind` guard disposition belongs to spec 2.
8. **Which environments were exercised** (macOS, Linux, native Windows, WSL2) and which were
   not, with the reason.
9. **Any gap or contradiction found in the parent** while implementing.

## Refresh notes

This spec is a draft (`refresh_required: true`). Before its plan runs, refresh it from the
outcomes of the earlier specs' review 1:

- **Spec 2 (runtime and envelope):** the final names and locations of `Endpoint`, the
  codecs, `StreamError`-like types, `NoDeclaredError`, `ErrorKind` (which is `#[non_exhaustive]`; reconcile this spec's
  PROPOSED `UnexpectedContentType` with spec 2's PD17, which excludes it), the hook marker
  trait and `ResponseKind` disposition (spec 2's; this spec only extends the marker to
  stream and GraphQL codecs), the managed builder (where the
  GraphQL address override goes), `send_raw`, whether stream responses are acceptable as
  non-`Clone`/non-`Sync`, and `elapsed` for streams. Any of these may replace a Proposed
  name or remove a requirement.
- **Spec 3 (pagination):** the final pagination types and Relay preset; D-12 may be
  already satisfied, replaced, or require an input location for GraphQL variables that
  the model did not have.
- **Specs 4a and 4b (definition API and vocabulary):** the builder methods for responses
  and errors (the `.returns`/`.errors` placeholders and `ErrorSet`), the registration
  context API, the JSON baselines and byte-comparison allow-list this spec must update,
  the definition serialization round-trip test (R44), and the 4b vocabulary
  (rate-limit, deprecated, auth override, `cross_origin`, request-id) that R26's
  extension must carry. This is the largest source of change.
- **Spec 1 (generation integrity):** the diagnostics format for skipped features (R42)
  and the validation structure this spec extends.
- **Author rulings in review 1** on any default adopted by earlier specs that overlaps
  this one (cross-origin overrides, redaction, deadline), and on any of this spec's
  defaults the author has already seen.
- Re-check the counts and file lists stated as unverified (the ten stranded Ollama types,
  the files that name `ApiResponse`, dependents' call sites).
