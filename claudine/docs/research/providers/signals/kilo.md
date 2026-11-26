---
$schema: ./_schema.yaml
created: 2026-07-05
last_updated: 2026-07-06
agent: codex
model: default
records:
  - id: stream-usage_capped-promotion-limit
    signal: usage_capped
    source: stream
    locator: "global SSE /event payload.type=session.error; error.data.responseBody contains PROMOTION_MODEL_LIMIT_REACHED"
    detection: declarative
    priority: 10
    match_path: payload.properties.error.data.responseBody
    match_op: substring_ci
    match_value: PROMOTION_MODEL_LIMIT_REACHED
    distinguish: "Kilo-specific promotion limits are non-retryable APIError payloads and must be classified before generic authentication or retry records."
    vocabulary: ["PROMOTION_MODEL_LIMIT_REACHED", "PAID_MODEL_AUTH_REQUIRED"]
    confidence: source_code
    evidence: ./fixtures/kilo/stream-session-error-promotion-limit.json
  - id: stream-no_funds-gateway-402
    signal: no_funds
    source: stream
    locator: "global SSE /event payload.type=session.error; error.data.responseBody contains insufficient balance"
    detection: declarative
    priority: 20
    match_path: payload.properties.error.data.responseBody
    match_op: substring_ci
    match_value: "Insufficient balance"
    distinguish: "Gateway balance failures are billing failures, not rate limits; the documented HTTP 402 body is preserved inside the assistant APIError responseBody."
    vocabulary: ["402", "Insufficient balance"]
    confidence: documented
    evidence: ./fixtures/kilo/stream-session-error-insufficient-balance.json
  - id: stream-auth_invalid-paid-model-auth-required
    signal: auth_invalid
    source: stream
    locator: "global SSE /event payload.type=session.error; error.data.responseBody contains PAID_MODEL_AUTH_REQUIRED"
    detection: declarative
    priority: 30
    match_path: payload.properties.error.data.responseBody
    match_op: substring_ci
    match_value: PAID_MODEL_AUTH_REQUIRED
    distinguish: "Paid-model sign-in failures are Kilo APIError response bodies; they are different from usage caps because the code asks the user to authenticate rather than wait or add credits."
    vocabulary: ["PAID_MODEL_AUTH_REQUIRED", "PROMOTION_MODEL_LIMIT_REACHED"]
    confidence: source_code
    evidence: ./fixtures/kilo/stream-session-error-paid-auth-required.json
  - id: stream-rate_limited-status-retry
    signal: rate_limited
    source: stream
    locator: "global SSE /event payload.type=session.status; status.type=retry; status.message contains rate limit"
    detection: declarative
    priority: 40
    match_path: payload.properties.status.message
    match_op: substring_ci
    match_value: "rate limit"
    distinguish: "The status channel is a generic retry surface; classify it as rate-limited only when the provider-supplied retry message names a rate limit."
    vocabulary: ["idle", "retry", "busy", "offline"]
    confidence: observed
    evidence: ./fixtures/kilo/stream-session-status-rate-limit.json
  - id: stream-provider_overloaded-status-retry
    signal: provider_overloaded
    source: stream
    locator: "global SSE /event payload.type=session.status; status.type=retry; status.message contains overloaded"
    detection: declarative
    priority: 50
    match_path: payload.properties.status.message
    match_op: substring_ci
    match_value: overloaded
    distinguish: "The status channel is a generic retry surface; overload classification depends on overload wording in the retry message."
    vocabulary: ["idle", "retry", "busy", "offline"]
    confidence: observed
    evidence: ./fixtures/kilo/stream-session-status-overloaded.json
  - id: stream-generation_retried
    signal: generation_retried
    source: stream
    locator: "global SSE /event payload.type=session.next.retried"
    detection: declarative
    priority: 60
    match_path: payload.type
    match_op: eq
    match_value: session.next.retried
    distinguish: "This is the structural retry event emitted by the experimental event system; status.retry is the UI status side-channel for the same retry period."
    vocabulary: ["session.next.retried"]
    confidence: source_code
    evidence: ./fixtures/kilo/stream-session-retried.json
  - id: stream-human_input_requested-offline
    signal: human_input_requested
    source: stream
    locator: "global SSE /event payload.type=session.status; status.type=offline"
    detection: declarative
    priority: 70
    match_path: payload.properties.status.type
    match_op: eq
    match_value: offline
    distinguish: "Kilo added the `offline` status variant with a question request id; this is a reserved Claudine signal because it means progress is blocked on a user answer."
    vocabulary: ["idle", "retry", "busy", "offline"]
    confidence: source_code
    evidence: ./fixtures/kilo/stream-session-status-offline-question.json
  - id: stream-interrupted-aborted
    signal: interrupted
    source: stream
    locator: "global SSE /event payload.type=session.error; error.name=MessageAbortedError"
    detection: declarative
    priority: 80
    match_path: payload.properties.error.name
    match_op: eq
    match_value: MessageAbortedError
    distinguish: "Manual aborts are represented as MessageAbortedError and are intentionally suppressed by Kilo notification code; classify them as interruptions, not provider failures."
    vocabulary: ["MessageAbortedError", "APIError", "AuthError", "OutputLengthError", "ContextOverflowError", "StructuredOutputError"]
    confidence: source_code
    evidence: ./fixtures/kilo/stream-session-error-aborted.json
  - id: stream-model_resolved-model-switched
    signal: model_resolved
    source: stream
    locator: "global SSE /event payload.type=session.next.model.switched"
    detection: declarative
    priority: 90
    match_path: payload.type
    match_op: eq
    match_value: session.next.model.switched
    distinguish: "Model-switched events carry the selected provider/model/variant for the session. They do not prove a fallback unless compared with an earlier requested model."
    vocabulary: ["session.next.model.switched"]
    confidence: source_code
    evidence: ./fixtures/kilo/stream-model-switched.json
  - id: stream-tokens_consumed-step-ended
    signal: tokens_consumed
    source: stream
    locator: "global SSE /event payload.type=session.next.step.ended"
    detection: declarative
    priority: 100
    match_path: payload.type
    match_op: eq
    match_value: session.next.step.ended
    distinguish: "Step-ended events are the streaming metering surface; stored step-finish parts are the persisted equivalent."
    vocabulary: ["session.next.step.ended"]
    confidence: source_code
    evidence: ./fixtures/kilo/stream-session-step-ended.json
  - id: sqlite-tokens_consumed-step-finish
    signal: tokens_consumed
    source: sqlite
    locator: "SELECT data FROM part WHERE json_extract(data, '$.type') = 'step-finish'"
    detection: declarative
    priority: 10
    match_path: row.data.type
    match_op: eq
    match_value: step-finish
    distinguish: "Stored step-finish parts are persisted token/cost metering rows; session.next.step.ended is the live event equivalent."
    vocabulary: ["step-finish"]
    confidence: source_code
    evidence: ./fixtures/kilo/sqlite-part-step-finish.json
  - id: sqlite-model_resolved-step-finish-routed
    signal: model_resolved
    source: sqlite
    locator: "SELECT data FROM part WHERE json_extract(data, '$.type') = 'step-finish' AND json_extract(data, '$.model.modelID') IS NOT NULL"
    detection: declarative
    priority: 20
    match_path: row.data.model.providerID
    match_op: substring_ci
    match_value: kilo
    distinguish: "Kilo writes routed auto-model resolution onto step-finish parts when provider metadata reveals the concrete model; this is not a fallback signal by itself."
    vocabulary: ["step-finish"]
    confidence: source_code
    evidence: ./fixtures/kilo/sqlite-part-step-finish.json
  - id: sqlite-interrupted-aborted
    signal: interrupted
    source: sqlite
    locator: "SELECT data FROM message WHERE json_extract(data, '$.error.name') = 'MessageAbortedError'"
    detection: declarative
    priority: 30
    match_path: row.data.error.name
    match_op: eq
    match_value: MessageAbortedError
    distinguish: "Persisted assistant messages with MessageAbortedError are interrupted sessions, not failed provider responses."
    vocabulary: ["MessageAbortedError", "APIError", "AuthError", "OutputLengthError", "ContextOverflowError", "StructuredOutputError"]
    confidence: source_code
    evidence: ./fixtures/kilo/sqlite-message-aborted.json
extractions:
  - record: stream-usage_capped-promotion-limit
    field: message
    path: payload.properties.error.data.responseBody
  - record: stream-no_funds-gateway-402
    field: message
    path: payload.properties.error.data.responseBody
  - record: stream-auth_invalid-paid-model-auth-required
    field: message
    path: payload.properties.error.data.responseBody
  - record: stream-rate_limited-status-retry
    field: message
    path: payload.properties.status.message
  - record: stream-rate_limited-status-retry
    field: attempt
    path: payload.properties.status.attempt
  - record: stream-rate_limited-status-retry
    field: next
    path: payload.properties.status.next
    unit: unix_millis
    zone: unspecified
    notes: "The code names this `next`; tests use millisecond values such as 5000, but the field is not documented as an absolute timestamp or duration."
  - record: stream-provider_overloaded-status-retry
    field: message
    path: payload.properties.status.message
  - record: stream-provider_overloaded-status-retry
    field: attempt
    path: payload.properties.status.attempt
  - record: stream-provider_overloaded-status-retry
    field: next
    path: payload.properties.status.next
    unit: unix_millis
    zone: unspecified
    notes: "The code names this `next`; tests use millisecond values such as 1000, but the field is not documented as an absolute timestamp or duration."
  - record: stream-generation_retried
    field: attempt
    path: payload.properties.attempt
  - record: stream-generation_retried
    field: message
    path: payload.properties.error.message
  - record: stream-human_input_requested-offline
    field: request_id
    path: payload.properties.status.requestID
  - record: stream-human_input_requested-offline
    field: prompt
    path: payload.properties.status.message
  - record: stream-interrupted-aborted
    field: message
    path: payload.properties.error.data.message
  - record: stream-model_resolved-model-switched
    field: provider
    path: payload.properties.model.providerID
  - record: stream-model_resolved-model-switched
    field: resolved
    path: payload.properties.model.id
  - record: stream-model_resolved-model-switched
    field: variant
    path: payload.properties.model.variant
  - record: stream-model_resolved-model-switched
    field: occurred_at
    path: payload.properties.timestamp
    unit: iso8601
    zone: utc
  - record: stream-tokens_consumed-step-ended
    field: input
    path: payload.properties.tokens.input
    unit: tokens
  - record: stream-tokens_consumed-step-ended
    field: output
    path: payload.properties.tokens.output
    unit: tokens
  - record: stream-tokens_consumed-step-ended
    field: reasoning
    path: payload.properties.tokens.reasoning
    unit: tokens
  - record: stream-tokens_consumed-step-ended
    field: cache_read
    path: payload.properties.tokens.cache.read
    unit: tokens
  - record: stream-tokens_consumed-step-ended
    field: cache_write
    path: payload.properties.tokens.cache.write
    unit: tokens
  - record: stream-tokens_consumed-step-ended
    field: cost
    path: payload.properties.cost
    unit: usd
  - record: sqlite-tokens_consumed-step-finish
    field: total
    path: row.data.tokens.total
    unit: tokens
  - record: sqlite-tokens_consumed-step-finish
    field: input
    path: row.data.tokens.input
    unit: tokens
  - record: sqlite-tokens_consumed-step-finish
    field: output
    path: row.data.tokens.output
    unit: tokens
  - record: sqlite-tokens_consumed-step-finish
    field: reasoning
    path: row.data.tokens.reasoning
    unit: tokens
  - record: sqlite-tokens_consumed-step-finish
    field: cache_read
    path: row.data.tokens.cache.read
    unit: tokens
  - record: sqlite-tokens_consumed-step-finish
    field: cache_write
    path: row.data.tokens.cache.write
    unit: tokens
  - record: sqlite-tokens_consumed-step-finish
    field: cost
    path: row.data.cost
    unit: usd
  - record: sqlite-model_resolved-step-finish-routed
    field: provider
    path: row.data.model.providerID
  - record: sqlite-model_resolved-step-finish-routed
    field: resolved
    path: row.data.model.modelID
  - record: sqlite-interrupted-aborted
    field: message
    path: row.data.error.data.message
bespoke_rationale: []
gaps:
  - "No official Kilo Code documentation was found for local CLI signal/event payloads. Official docs describe the CLI, VS Code architecture, and Gateway billing, but the event schemas used here come from source code."
  - "The `session.status.retry.next` unit is not named in the schema. Tests and UI usage indicate millisecond values, but the field is not clearly documented as an absolute timestamp or relative delay."
  - "Kilo Gateway HTTP 402 insufficient-balance responses are documented. The local runtime passes upstream statuses through for edit/FIM handlers and assistant APIError objects preserve responseBody, but the exact chat-completions no-funds body is not exhaustively enumerated in local source."
  - "PermissionDeniedError carries ruleset data and the originating request carries permission keys such as `read` and `edit`; mapping that to `permission_denied_read` versus `permission_denied_write` needs cross-record/tool context, so no declarative record is emitted here."
  - "Kilo auto models can resolve to concrete routed models, and documentation says Efficient can fall back to Balanced, but no shipped first-class model_fallback event was found in the inspected source."
  - "Session resumption is supported operationally through stored sessions, children/forks, and editor-owned server recovery, but no explicit `session_resumable` signal payload was found."
  - "session.next.* records (generation_retried, tokens_consumed step-ended, model_resolved model-switched) are gated behind flags.experimentalEventSystem (processor.ts dual-write) — default installs may never emit them."
  - "Base.timestamp is V2Schema.DateTimeUtcFromMillis (encoded epoch-millis) while the runtime publishes a DateTime — whether the SSE wire carries ISO or millis is unpinned; the occurred_at extraction's unit: iso8601 is plausible but unverified."
  - "step-finish rows carry both tokens_consumed and model_resolved payloads; under group-level first-match-wins the sqlite-model_resolved-step-finish-routed record (priority 20) is unreachable behind sqlite-tokens_consumed-step-finish (priority 10) — needs the per-signal evaluation ruling."
changes:
  - "2026-07-06: renamed extraction fields to the canonical SignalEvent payload names — model_resolved `model` → `resolved` (stream and sqlite) and human_input_requested `message` → `prompt`."
requires_claudine_update: true
reason: "Kilo exposes new source-code-backed stream and SQLite signal records that Claudine's generated detection catalog does not currently include."
---

# Kilo Code Signal Detection

## Overview

Kilo Code is an open-source agent platform with a CLI runtime, VS Code extension, JetBrains plugin, local HTTP/SSE server, and Kilo Gateway integration. The strongest machine-readable signal surfaces are not user-facing prose logs; they are source-defined Effect schemas in the embedded OpenCode-derived runtime: global Server-Sent Events, session status events, experimental `session.next.*` events, assistant message/part JSON stored in SQLite, and structured API error objects.

The local runtime is mature enough for source-code-first extraction, but the signal contract is mostly internal. Official Kilo docs describe the CLI, editor architecture, and gateway billing behavior; they do not publish a stable signal taxonomy or stream schema for Claudine-style detection. The records above therefore use source-code confidence for schemas and documented confidence only where Kilo Gateway docs are the authority.

## Signal Surfaces

### Global SSE Stream

The local server exposes a global event stream as text/event-stream. Source defines `/event` for instance events and `/global/event` for process-wide global events; the global handler serializes each event as JSON in an SSE `message` frame. Kilo's VS Code architecture docs describe the editor client using generated SDK calls plus global SSE against an editor-owned `kilo serve --port 0` child process.

The source-backed event payloads relevant to signals are:

| Event type | Signal use |
| --- | --- |
| `session.status` | retry/backoff status, offline user-input status |
| `session.next.retried` | generation retry event |
| `session.next.step.ended` | token and cost metering |
| `session.next.model.switched` | selected model observation |
| `session.error` | assistant/session failures, including Kilo API errors and aborts |

The stream is a first-class local runtime API, but many event types are still described in code as part of an experimental event system.

### Session SQLite

Kilo stores sessions in SQLite tables named `session`, `message`, `part`, `session_message`, `permission`, and `todo`. The `part.data` JSON field stores typed message parts such as `step-finish`; the `message.data` JSON field stores assistant/user message data and errors. The schema has aggregate token columns on `session` and structured token/cost payloads on `step-finish` parts.

SQLite is a persistence surface, not a streaming contract. It is still useful for post-run Claudine detection because the schemas are source-defined and projectors persist `session.next.*` events into `session_message`.

### Session Status

`SessionStatus.Info` is an exhaustive union with `idle`, `retry`, `busy`, and Kilo's added `offline` variant. The `retry` variant carries `attempt`, `message`, optional action metadata, and `next`; `offline` carries a `requestID` and `message`. The service publishes `session.status` events whenever status changes.

The `retry` variant is semantically generic. Rate-limit and overload detection from this surface depends on the status message text. The `offline` variant is stronger: it represents a state where the session is waiting for user input, so it maps to Claudine's reserved `human_input_requested`.

### Assistant Errors

Assistant/session errors are structured by name. The schema includes `AuthError`, `OutputLengthError`, `MessageAbortedError`, `StructuredOutputError`, `ContextOverflowError`, and `APIError`. Kilo-specific API errors are parsed from `APIError.data.responseBody` and currently recognize `PAID_MODEL_AUTH_REQUIRED` and `PROMOTION_MODEL_LIMIT_REACHED`.

Kilo Gateway balance exhaustion is documented as HTTP 402 with an insufficient-balance JSON body. The local gateway handlers preserve upstream status/body for edit/FIM failures so clients can distinguish auth, credit, rate-limit, and server failures; assistant API errors can likewise carry provider response bodies.

## Usage and Rate Limits

### `usage_capped`

Kilo-specific promotion exhaustion is represented by an `APIError` whose `responseBody` contains `PROMOTION_MODEL_LIMIT_REACHED`. Source code declares this as one of two known Kilo error codes and maps it to user-facing copy that asks the user to sign up to continue. That is a usage cap rather than a retryable provider rate limit.

Locator: `payload.type=session.error`, `payload.properties.error.name=APIError`, and `payload.properties.error.data.responseBody` containing `PROMOTION_MODEL_LIMIT_REACHED`.

### `no_funds`

The Kilo Gateway billing docs state that paid-model requests return HTTP 402 when the account balance reaches zero, with an error message that says the balance is insufficient and includes a credits URL. The runtime's Kilo Gateway handlers intentionally pass upstream status through for edit/FIM failures rather than collapsing everything to 400, and assistant `APIError` values can carry `statusCode` and `responseBody`.

Locator: `payload.type=session.error`, `APIError.data.responseBody` containing `Insufficient balance`.

### `rate_limited`

Kilo's structural retry surfaces do not have a dedicated rate-limit enum. The `session.status` retry variant carries provider/wrapper text in `status.message`, and tests exercise a `"Rate limited"` message. Claudine can declaratively classify retry status as `rate_limited` only when that message contains rate-limit wording.

Locator: `payload.type=session.status`, `status.type=retry`, `status.message` containing `rate limit`.

### `provider_overloaded`

Provider overload is analogous to rate limiting on the status surface: no dedicated enum was found, but retry status text can identify overload. The UI strings include provider-hot wording such as Gemini being overloaded; classification must remain message-based unless Kilo adds a typed reason.

Locator: `payload.type=session.status`, `status.type=retry`, `status.message` containing `overloaded`.

## Authentication and Authorization

### `auth_invalid`

Kilo declares `PAID_MODEL_AUTH_REQUIRED` as a Kilo-specific API error code. The parser accepts both `{ error: { code } }` and `{ code }` response-body shapes. User-facing copy says the user needs to sign in to use the model and points to `/connect` or `kilo auth login`.

Locator: `payload.type=session.error`, `APIError.data.responseBody` containing `PAID_MODEL_AUTH_REQUIRED`.

### Permission Denials

The permission service defines `PermissionDeniedError`, `PermissionRejectedError`, and `PermissionCorrectedError`. Tool code sends permission keys such as `read` and `edit`, and denials carry ruleset data. This is enough for a bespoke read/write permission mapper, but not enough for a single-payload declarative record unless Claudine also correlates the error to the originating permission request or tool call.

## Model Resolution

### `model_resolved`

Kilo emits `session.next.model.switched` when a user prompt changes the current provider/model/variant. The event payload carries `model.id`, `model.providerID`, and `model.variant`.

Kilo also writes routed auto-model metadata onto `step-finish` parts when `KiloRoutedModel.readAuto` can read the concrete model from provider metadata. This is a persisted model-resolution signal for auto models, not a model fallback signal by itself.

## Token Metering

### `tokens_consumed`

The live event-system metering surface is `session.next.step.ended`. Its schema carries `finish`, `cost`, and token counts for `input`, `output`, `reasoning`, and cache `read`/`write`.

The persisted equivalent is the `part` table's `step-finish` data. Kilo's message schema adds an optional `tokens.total`, the same token breakdown, `cost`, and optional routed `model`. The `session` table also has aggregate token columns, but the per-step part is the better Claudine extraction point because it preserves event-level metering.

## Retry and Recovery

### `generation_retried`

The event schema defines `session.next.retried` with `attempt` and a retry error object containing `message`, optional `statusCode`, `isRetryable`, optional `responseHeaders`, optional `responseBody`, and optional `metadata`. The processor publishes this event during retry policy handling when the experimental event system is enabled, and it also sets `session.status` to `retry`.

`session.next.retried` is the structural retry signal. `session.status.retry` is the UI/status side-channel and is better for message-based classifications such as `rate_limited`.

## Interruption and Recovery

### `interrupted`

Kilo normalizes DOM aborts into `MessageAbortedError` and stores/streams that error name on assistant/session errors. TUI notification code treats `MessageAbortedError` specially and suppresses attention for manual stops, confirming this is an interruption path rather than an ordinary provider failure.

Locator: `payload.type=session.error`, `error.name=MessageAbortedError`, or persisted `message.data.error.name=MessageAbortedError`.

### `human_input_requested`

Kilo adds `status.type=offline` with a `requestID` from the question schema and a message. This is a reserved Claudine signal today, but it is a useful future mapping because the provider is explicitly waiting for a human answer.

### `session_resumable`

Kilo persists sessions and exposes list/get/messages/fork APIs. The editor architecture also describes server recovery and shared editor-owned runtime behavior. No explicit resumable event or status flag was found, so there is no structured `session_resumable` record.

## Version Drift

This pass inspected Kilo Code package version `7.4.1` at commit `1fc8f066fd263455d77fed269a8bcfcd57309a55`. No tags were present in the shallow clone, so records do not carry `since`/`until` bounds.

The most important drift risk is inherited from Kilo's OpenCode-derived runtime: `session.next.*` is source-defined but the processor still calls it an experimental event system in comments. Kilo-specific deltas such as `SessionStatus.offline`, routed model metadata on `step-finish`, and Kilo API error codes should be watched on updates.

## Quirks and Gaps

Kilo Code is not only a CLI. The VS Code and JetBrains clients drive a local `kilo serve` runtime through generated SDK calls and SSE, while the CLI TUI consumes the same runtime. Claudine should not assume a simple stdout JSONL stream.

The `session.status.retry.next` field is typed as a non-negative integer. Tests and UI DTOs use millisecond-looking values such as `5000`, but the source schema does not say whether the value is an absolute epoch, relative delay, or monotonic timestamp. The frontmatter marks its zone as `unspecified`.

Permission denial mapping needs correlation. The single error shape says `PermissionDeniedError`; the read/write distinction comes from the originating permission key (`read` or `edit`) in the request/tool code. That should be `bespoke` if Claudine needs it.

No shipped model-fallback signal was found. Kilo's docs say Auto Efficient may fall back to Balanced when it cannot decide confidently, but the inspected local runtime surfaces routed model resolution rather than an explicit fallback event.

## Changelog

Initial research document.

## Sources

- [Kilo Code repository at commit `1fc8f066fd263455d77fed269a8bcfcd57309a55`](https://github.com/Kilo-Org/kilocode/tree/1fc8f066fd263455d77fed269a8bcfcd57309a55)
- [Session event schema: `session.next.model.switched`, `session.next.step.ended`, `session.next.retried`](https://github.com/Kilo-Org/kilocode/blob/1fc8f066fd263455d77fed269a8bcfcd57309a55/packages/core/src/session-event.ts#L49-L135)
- [Retry event schema](https://github.com/Kilo-Org/kilocode/blob/1fc8f066fd263455d77fed269a8bcfcd57309a55/packages/core/src/session-event.ts#L309-L330)
- [Session status schema and event publisher](https://github.com/Kilo-Org/kilocode/blob/1fc8f066fd263455d77fed269a8bcfcd57309a55/packages/opencode/src/session/status.ts#L9-L49)
- [Message part and assistant error schemas](https://github.com/Kilo-Org/kilocode/blob/1fc8f066fd263455d77fed269a8bcfcd57309a55/packages/opencode/src/session/message-v2.ts#L213-L258)
- [Assistant error normalization for aborts and auth failures](https://github.com/Kilo-Org/kilocode/blob/1fc8f066fd263455d77fed269a8bcfcd57309a55/packages/opencode/src/session/message-v2.ts#L1217-L1287)
- [Session SQLite tables](https://github.com/Kilo-Org/kilocode/blob/1fc8f066fd263455d77fed269a8bcfcd57309a55/packages/opencode/src/session/session.sql.ts#L16-L92)
- [Step-ended event and step-finish persistence](https://github.com/Kilo-Org/kilocode/blob/1fc8f066fd263455d77fed269a8bcfcd57309a55/packages/opencode/src/session/processor.ts#L684-L735)
- [Retry processing and status updates](https://github.com/Kilo-Org/kilocode/blob/1fc8f066fd263455d77fed269a8bcfcd57309a55/packages/opencode/src/session/processor.ts#L1022-L1055)
- [Model-switched event publishing](https://github.com/Kilo-Org/kilocode/blob/1fc8f066fd263455d77fed269a8bcfcd57309a55/packages/opencode/src/session/prompt.ts#L813-L827)
- [Projectors that persist event-system records](https://github.com/Kilo-Org/kilocode/blob/1fc8f066fd263455d77fed269a8bcfcd57309a55/packages/opencode/src/session/projectors-next.ts#L152-L215)
- [Kilo-specific API error codes](https://github.com/Kilo-Org/kilocode/blob/1fc8f066fd263455d77fed269a8bcfcd57309a55/packages/opencode/src/kilocode/kilo-errors.ts#L4-L85)
- [Kilo Gateway profile/balance schema](https://github.com/Kilo-Org/kilocode/blob/1fc8f066fd263455d77fed269a8bcfcd57309a55/packages/opencode/src/kilocode/server/httpapi/groups/kilo-gateway.ts#L20-L47)
- [Kilo Gateway handler preserving upstream failure status/body](https://github.com/Kilo-Org/kilocode/blob/1fc8f066fd263455d77fed269a8bcfcd57309a55/packages/opencode/src/kilocode/server/httpapi/handlers/kilo-gateway.ts#L248-L262)
- [Kilo CLI docs](https://kilo.ai/docs/code-with-ai/platforms/cli)
- [Kilo VS Code extension architecture](https://kilo.ai/docs/contributing/architecture/vscode-extension)
- [Kilo Gateway usage and billing docs](https://kilo.ai/docs/gateway/usage-and-billing)
