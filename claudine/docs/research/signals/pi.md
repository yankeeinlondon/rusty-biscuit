---
$schema: ./_schema.yaml
created: 2026-07-05
last_updated: 2026-07-06
agent: codex
model: default
docs: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/json.md
records:
  - id: stream-usage_capped-quota
    signal: usage_capped
    source: stream
    locator: "type=message_end,message.role=assistant,message.stopReason=error"
    detection: declarative
    priority: 10
    match_path: message.errorMessage
    match_op: substring_ci
    match_value: "quota exceeded"
    distinguish: "Pi's stock JSONL stream has no quota enum. This record catches quota/account-limit provider failures only after higher-priority non-retryable usage-cap text has been checked and before generic rate-limit retry text."
    vocabulary: ["stop", "length", "toolUse", "error", "aborted"]
    confidence: source_code
    evidence: ./fixtures/pi/stream-usage-capped-quota.jsonl
  - id: stream-no_funds-billing
    signal: no_funds
    source: stream
    locator: "type=message_end,message.role=assistant,message.stopReason=error"
    detection: declarative
    priority: 20
    match_path: message.errorMessage
    match_op: substring_ci
    match_value: "billing"
    distinguish: "Pi's retry classifier treats billing text as non-retryable provider-limit exhaustion. Keep this before generic retry/rate-limit records because billing failures are account funding problems, not transient throttles."
    vocabulary: ["stop", "length", "toolUse", "error", "aborted"]
    confidence: source_code
    evidence: ./fixtures/pi/stream-no-funds-billing.jsonl
  - id: stream-auth_invalid-no-api-key
    signal: auth_invalid
    source: stream
    locator: "type=message_end,message.role=assistant,message.stopReason=error"
    detection: declarative
    priority: 30
    match_path: message.errorMessage
    match_op: substring_ci
    match_value: "No API key found"
    distinguish: "This is Pi's pre-request auth guidance for API-key providers. It is emitted as an assistant error message in JSON mode, not as a typed auth event."
    vocabulary: ["stop", "length", "toolUse", "error", "aborted"]
    confidence: source_code
    evidence: ./fixtures/pi/stream-auth-invalid-no-api-key.jsonl
  - id: stream-auth_invalid-oauth
    signal: auth_invalid
    source: stream
    locator: "type=message_end,message.role=assistant,message.stopReason=error"
    detection: declarative
    priority: 40
    match_path: message.errorMessage
    match_op: substring_ci
    match_value: "Authentication failed for"
    distinguish: "This is Pi's OAuth/stored-credential failure text. It is distinct from missing API-key guidance and asks the user to re-run `/login`."
    vocabulary: ["stop", "length", "toolUse", "error", "aborted"]
    confidence: source_code
    evidence: ./fixtures/pi/stream-auth-invalid-oauth.jsonl
  - id: stream-rate_limited-message
    signal: rate_limited
    source: stream
    locator: "type=message_end,message.role=assistant,message.stopReason=error"
    detection: declarative
    priority: 50
    match_path: message.errorMessage
    match_op: substring_ci
    match_value: "rate limit"
    distinguish: "Pi classifies `rate.?limit`, `too many requests`, and HTTP 429 text as retryable provider errors. This record intentionally comes after quota/billing records because Pi's own retry classifier excludes account-limit strings from retry."
    vocabulary: ["stop", "length", "toolUse", "error", "aborted"]
    confidence: source_code
    evidence: ./fixtures/pi/stream-rate-limited-message.jsonl
  - id: stream-provider_overloaded-message
    signal: provider_overloaded
    source: stream
    locator: "type=message_end,message.role=assistant,message.stopReason=error"
    detection: declarative
    priority: 60
    match_path: message.errorMessage
    match_op: substring_ci
    match_value: "overloaded"
    distinguish: "Provider overload is one member of Pi's retryable provider-error text classifier. It is separated from rate-limit text so Claudine can surface provider capacity failures distinctly."
    vocabulary: ["stop", "length", "toolUse", "error", "aborted"]
    confidence: source_code
    evidence: ./fixtures/pi/stream-provider-overloaded-message.jsonl
  - id: stream-interrupted-aborted
    signal: interrupted
    source: stream
    locator: "type=message_end,message.role=assistant"
    detection: declarative
    priority: 70
    match_path: message.stopReason
    match_op: eq
    match_value: "aborted"
    distinguish: "Assistant `stopReason=aborted` is Pi's stream-level cancellation result. Process-level SIGTERM/SIGHUP in print mode can exit before a JSON event is emitted, so wrappers still need exit handling."
    vocabulary: ["stop", "length", "toolUse", "error", "aborted"]
    confidence: source_code
    evidence: ./fixtures/pi/stream-interrupted-aborted.jsonl
  - id: stream-tokens_consumed-message_end
    signal: tokens_consumed
    source: stream
    locator: "type=message_end,message.role=assistant"
    detection: declarative
    priority: 80
    match_path: message.usage.totalTokens
    match_op: exists
    distinguish: "Assistant `message_end` carries the finalized per-response `Usage` object. It should run after error-class records so zero-usage provider failures are not reduced to token metering."
    vocabulary: ["message_end"]
    confidence: source_code
    evidence: ./fixtures/pi/stream-message-end-usage.jsonl
  - id: stream-generation_retried-auto_retry_start
    signal: generation_retried
    source: stream
    locator: "type=auto_retry_start"
    detection: declarative
    priority: 90
    match_path: type
    match_op: eq
    match_value: "auto_retry_start"
    distinguish: "Pi emits this before sleeping and restarting a retryable assistant failure. It is wrapper-level retry, not a provider-internal retry hidden inside an SDK."
    vocabulary: ["auto_retry_start"]
    confidence: source_code
    evidence: ./fixtures/pi/stream-auto-retry-start.jsonl
  - id: stream-retries_exhausted-auto_retry_end
    signal: retries_exhausted
    source: stream
    locator: "type=auto_retry_end,success=false"
    detection: bespoke
    priority: 100
    distinguish: "A failed `auto_retry_end` only means the retry sequence ended unsuccessfully. Correctly classifying exhausted budget requires correlating the prior `auto_retry_start.attempt`, `maxAttempts`, and whether the final error is not `Retry cancelled`."
    vocabulary: ["auto_retry_end"]
    confidence: source_code
    evidence: ./fixtures/pi/stream-auto-retry-exhausted.jsonl
  - id: session_log-model_resolved-model_change
    signal: model_resolved
    source: session_log
    locator: "~/.pi/agent/sessions/*.jsonl type=model_change"
    detection: declarative
    priority: 10
    match_path: type
    match_op: eq
    match_value: "model_change"
    distinguish: "Session `model_change` entries persist explicit model selection. They are durable log records, not live stream events; assistant messages can also imply the active model but overlap with token usage."
    vocabulary: ["model_change"]
    confidence: source_code
    evidence: ./fixtures/pi/session-log-model-change.jsonl
  - id: session_log-tokens_consumed-message
    signal: tokens_consumed
    source: session_log
    locator: "~/.pi/agent/sessions/*.jsonl type=message,message.role=assistant"
    detection: declarative
    priority: 20
    match_path: message.usage.totalTokens
    match_op: exists
    distinguish: "Pi persists assistant messages to session JSONL on `message_end`. This is retrospective token metering and should not replace the live stream record when Claudine is supervising a running process."
    vocabulary: ["message"]
    confidence: source_code
    evidence: ./fixtures/pi/session-log-message-usage.jsonl
  - id: session_log-session_resumable-header
    signal: session_resumable
    source: session_log
    locator: "~/.pi/agent/sessions/*.jsonl first line type=session"
    detection: declarative
    priority: 30
    match_path: type
    match_op: eq
    match_value: "session"
    distinguish: "A Pi session file begins with a session header carrying an id, timestamp, and cwd. The header proves a persisted session exists; actual resume eligibility can still depend on CLI mode and file availability."
    vocabulary: ["session"]
    confidence: source_code
    evidence: ./fixtures/pi/session-log-header-resumable.jsonl
  - id: stream-human_input_requested-extension-ui
    signal: human_input_requested
    source: stream
    locator: "RPC stdout type=extension_ui_request"
    detection: declarative
    priority: 110
    match_path: method
    match_op: in
    match_values: [select, confirm, input, editor]
    distinguish: "Only RPC mode exposes extension UI requests as JSONL stdout records. Stock `--mode json` print mode streams AgentSessionEvent records and does not include this RPC-only envelope."
    vocabulary: ["select", "confirm", "input", "editor", "notify", "setStatus", "setWidget"]
    confidence: source_code
    evidence: ./fixtures/pi/rpc-extension-ui-request.jsonl
  - id: exit-auth_invalid-no-models
    signal: auth_invalid
    source: exit
    locator: "1"
    detection: declarative
    priority: 10
    match_path: stderr_tail
    match_op: substring_ci
    match_value: "No models available"
    distinguish: "Non-interactive startup exits before a session stream is available when no authenticated/configured model can be selected. The wrapper must classify the exit payload rather than wait for JSONL."
    vocabulary: ["1"]
    confidence: source_code
    evidence: ./fixtures/pi/exit-no-models-available.json
extractions:
  - record: stream-usage_capped-quota
    field: message
    path: message.errorMessage
  - record: stream-no_funds-billing
    field: message
    path: message.errorMessage
  - record: stream-auth_invalid-no-api-key
    field: message
    path: message.errorMessage
  - record: stream-auth_invalid-oauth
    field: message
    path: message.errorMessage
  - record: stream-rate_limited-message
    field: message
    path: message.errorMessage
  - record: stream-provider_overloaded-message
    field: message
    path: message.errorMessage
  - record: stream-interrupted-aborted
    field: message
    path: message.errorMessage
  - record: stream-tokens_consumed-message_end
    field: input
    path: message.usage.input
    unit: tokens
  - record: stream-tokens_consumed-message_end
    field: output
    path: message.usage.output
    unit: tokens
  - record: stream-tokens_consumed-message_end
    field: cache_read
    path: message.usage.cacheRead
    unit: tokens
  - record: stream-tokens_consumed-message_end
    field: cache_write
    path: message.usage.cacheWrite
    unit: tokens
  - record: stream-tokens_consumed-message_end
    field: total
    path: message.usage.totalTokens
    unit: tokens
  - record: stream-generation_retried-auto_retry_start
    field: attempt
    path: attempt
  - record: stream-generation_retried-auto_retry_start
    field: max_attempts
    path: maxAttempts
  - record: stream-generation_retried-auto_retry_start
    field: wait
    path: delayMs
    unit: duration_millis
  - record: stream-generation_retried-auto_retry_start
    field: message
    path: errorMessage
  - record: session_log-model_resolved-model_change
    field: provider
    path: provider
  - record: session_log-model_resolved-model_change
    field: resolved
    path: modelId
  - record: session_log-model_resolved-model_change
    field: timestamp
    path: timestamp
    unit: iso8601
    zone: utc
  - record: session_log-tokens_consumed-message
    field: input
    path: message.usage.input
    unit: tokens
  - record: session_log-tokens_consumed-message
    field: output
    path: message.usage.output
    unit: tokens
  - record: session_log-tokens_consumed-message
    field: cache_read
    path: message.usage.cacheRead
    unit: tokens
  - record: session_log-tokens_consumed-message
    field: cache_write
    path: message.usage.cacheWrite
    unit: tokens
  - record: session_log-tokens_consumed-message
    field: total
    path: message.usage.totalTokens
    unit: tokens
  - record: session_log-session_resumable-header
    field: session_id
    path: id
  - record: session_log-session_resumable-header
    field: created_at
    path: timestamp
    unit: iso8601
    zone: utc
  - record: session_log-session_resumable-header
    field: cwd
    path: cwd
  - record: stream-human_input_requested-extension-ui
    field: request_id
    path: id
  - record: stream-human_input_requested-extension-ui
    field: method
    path: method
  - record: stream-human_input_requested-extension-ui
    field: prompt
    path: message
  - record: exit-auth_invalid-no-models
    field: message
    path: stderr_tail
bespoke_rationale:
  - "stream-retries_exhausted-auto_retry_end: `auto_retry_end` with `success:false` can be emitted either after the retry budget is exhausted or when a user cancels retry sleep. Correct classification needs correlation with preceding `auto_retry_start.attempt/maxAttempts`, the final assistant error, and `finalError != Retry cancelled`, so it is temporal/cross-record state."
gaps:
  - "Pi has source-code-confirmed retryable and non-retryable provider error string classifiers, but it does not expose structured error categories for rate limits, quota, overload, billing, or auth in the JSONL event contract. The stream records therefore match stable Pi-normalized message text and classifier vocabulary rather than provider-native enum fields."
  - "No stock Pi record was found for `usage_cap_approaching`. Account quota discovery is handled by optional extensions and provider-private APIs, not by the core stream."
  - "No stock Pi record was found for `auth_kind_detected`; auth resolution can return a source label internally, but the JSONL stream does not emit it."
  - "Provider version is available from `--version` and the package `VERSION` constant, but it is not present in `--mode json` or session logs. Recording `provider_version` would require an exit/stderr wrapper probe outside the provider stream."
  - "Pi model fallback is source-code-confirmed for missing/unavailable restored models and custom model ids, but fallback messages are printed startup diagnostics or returned SDK metadata, not stable JSONL stream records."
  - "Assistant messages include `responseModel` when a provider reports a concrete model different from the requested model. That can imply model resolution/fallback, but matching it would overlap the same `message_end` payload used for token metering under Claudine's first-match-wins detector."
  - "Pi has no built-in permission prompt or deny stream. Tool errors can appear in `tool_execution_end` with `isError:true`, but there is no stable read-versus-write permission-denied discriminator."
  - "Pi's internal provider request timeout and subprocess signal handlers can end a run without a JSONL timeout signal. Claudine should keep its own `timeout` and `step_timeout` guards."
  - "RPC `extension_ui_request` is recorded as `human_input_requested`, but this is RPC-mode stdout rather than ordinary `--mode json` print-mode output. Claudine must decide whether a Pi integration uses RPC mode before enabling this record."
  - "Fixture provenance: the two auth fixtures and exit-no-models-available.json carry verbatim Pi source message templates; stream-usage-capped-quota / stream-no-funds-billing / stream-rate-limited-message / stream-provider-overloaded-message / rpc-extension-ui-request use envelope shapes faithful to the cited source types but exemplar provider text — shape-synthesized, not live captures."
changes:
  - "2026-07-06: rewrote both message.usage.totalTokens numeric-regex presence proxies (stream and session_log) to `match_op: exists`; delay_ms extraction now uses the new `duration_millis` unit; amended the quirk about inexpressible field presence."
  - "2026-07-06: renamed extraction fields to the canonical SignalEvent payload names — generation_retried `delay_ms` → `wait`, model_resolved `model` → `resolved`, and human_input_requested `message` → `prompt`."
requires_claudine_update: true
reason: "Pi signal detection is codegen-wired and this research adds Pi records for JSONL stream message errors, retry lifecycle events, token usage, session-log model and resume metadata, RPC extension input requests, and startup exit auth failures."
---

# Pi Signal Detection

## Overview

Pi exposes two useful signal surfaces for Claudine: the first-class JSONL stream produced by `pi --mode json` and the durable JSONL session files under the Pi agent directory. The stream is documented as "JSON Event Stream Mode" and source confirms that print mode writes each subscribed session event to stdout with `JSON.stringify(event)`. It is stable for assistant message completion, tool lifecycle, retry lifecycle, compaction, and queue updates.

Pi does not publish typed provider error enums in that stream. Provider failures are normalized into `AssistantMessage` objects with `stopReason: "error"` plus an `errorMessage` string, and Pi's retry policy classifies those strings with regex vocabularies. That makes token metering and retry lifecycle records strong source-code records, while quota/rate-limit/billing records are source-code-backed string detections rather than provider-native semantic fields.

## Signal Surfaces

### Stream Events

`pi --mode json "<prompt>"` writes newline-delimited JSON objects to stdout. The first line is the session header (`type: "session"`, version, id, timestamp, cwd), then subscribed `AgentSessionEvent` objects as the run proceeds. Print mode writes these records directly to raw stdout.

The source union contains agent, turn, message, tool execution, queue, compaction, session info, thinking-level, and retry events. The underlying `AgentEvent` vocabulary is `agent_start`, `agent_end`, `turn_start`, `turn_end`, `message_start`, `message_update`, `message_end`, `tool_execution_start`, `tool_execution_update`, and `tool_execution_end`. `AgentSessionEvent` adds `queue_update`, `entry_appended`, `session_info_changed`, `thinking_level_changed`, `compaction_start`, `compaction_end`, `auto_retry_start`, and `auto_retry_end`.

Assistant `message_end` records are the main carrier for provider outcomes. The normalized `AssistantMessage` includes `api`, `provider`, `model`, optional `responseModel`, optional diagnostics, `usage`, `stopReason`, optional `errorMessage`, and `timestamp` in Unix milliseconds. The `stopReason` vocabulary is `stop`, `length`, `toolUse`, `error`, and `aborted`.

### Session Logs

Pi persists session data as JSONL under `~/.pi/agent/sessions` unless the session directory is overridden. The first line is a `SessionHeader` with `type: "session"`, `version`, `id`, `timestamp`, `cwd`, and optional `parentSession`. Later entries include `message`, `thinking_level_change`, `model_change`, `compaction`, `branch_summary`, `custom`, `custom_message`, `label`, and `session_info`.

Session logs are retrospective. They are valuable for `tokens_consumed`, `model_resolved`, and `session_resumable`, but Claudine should prefer stream records for live supervision because a session log may lag the running process and can be migrated on read.

### RPC Stream

Pi also has `--mode rpc`, a strict JSONL stdin/stdout protocol. Its stdout includes the same session events plus RPC responses and extension UI requests. `RpcExtensionUIRequest` can ask for `select`, `confirm`, `input`, or `editor` input, or send `notify`, `setStatus`, and `setWidget` UI updates. This is the only source-code-confirmed machine-readable human-input request surface.

The schema's source enum has no separate `rpc` value, so the frontmatter records this as `source: stream` with an RPC locator. A Claudine Pi wrapper should only enable that record when it launches Pi in RPC mode.

### stderr Diagnostics and Exit

Pi still uses stderr and process exit for some pre-stream failures. For non-interactive modes, `main.ts` exits with code `1` when no model is available before print/RPC streaming can start. Print mode also writes final assistant errors to stderr in text mode, and signal handlers can exit with 129 for SIGHUP or 143 for SIGTERM. These are wrapper-level `exit` detections rather than first-class Pi JSONL records.

## Usage and Rate Limits

Pi's core stream does not emit a typed usage-cap or rate-limit event. Instead, provider failures become assistant errors. Pi's retry classifier in `packages/ai/src/utils/retry.ts` treats `rate.?limit`, `too many requests`, `429`, `service unavailable`, `overloaded`, server errors, network errors, timeout text, premature stream endings, and explicit retry guidance as retryable. The same file excludes `GoUsageLimitError`, `FreeUsageLimitError`, "Monthly usage limit reached", "available balance", `insufficient_quota`, "out of budget", "quota exceeded", and "billing" from retryable errors.

The frontmatter records `usage_capped` for `quota exceeded`, `no_funds` for billing text, `rate_limited` for rate-limit text, and `provider_overloaded` for overloaded text. Their priorities intentionally run before token metering so a zero-usage provider failure is not classified as `tokens_consumed`, and usage/account exhaustion runs before generic retryable rate-limit handling.

No `usage_cap_approaching` record was found in stock Pi. Existing Pi usage research found optional quota extensions and provider-private APIs, but the core event stream does not surface warning thresholds.

## Retry Lifecycle

Pi emits `auto_retry_start` when it decides an assistant error is retryable. The record includes `attempt`, `maxAttempts`, `delayMs`, and `errorMessage`; the delay is milliseconds. On later success, Pi emits `auto_retry_end` with `success: true`. If the retry sequence ends unsuccessfully, it emits `auto_retry_end` with `success: false`, `attempt`, and `finalError`.

`generation_retried` is declarative on `type=auto_retry_start`. `retries_exhausted` is bespoke because `auto_retry_end success=false` also covers user-cancelled retry sleep (`finalError: "Retry cancelled"`), and correct classification requires correlation with the preceding attempt count and terminal assistant error.

## Authentication and Authorization

Pi resolves auth before provider requests. Missing API-key providers produce "No API key found for <provider>" guidance; OAuth/stored-credential failures produce "Authentication failed for \"<provider>\". Credentials may have expired or network is unavailable. Run '/login <provider>' to re-authenticate." In JSON mode these can appear as assistant `message_end` errors after the session starts.

If no usable model is selected for a non-interactive run, startup exits before the stream is established and writes "No models available" guidance to stderr. The frontmatter records that as an `exit` auth-invalid detection.

Pi has no built-in permission-denied signal. Tool execution failures are visible through `tool_execution_end` and assistant/tool result messages, but stock Pi does not distinguish read denial from write denial because it intentionally does not include a built-in permission popup system.

## Token Metering

Every assistant message carries a `usage` object with `input`, `output`, `cacheRead`, `cacheWrite`, optional `cacheWrite1h`, optional `reasoning`, `totalTokens`, and a `cost` object with `input`, `output`, `cacheRead`, `cacheWrite`, and `total`. The source comments specify that assistant timestamps are Unix milliseconds.

The live stream record uses `type=message_end` and `message.usage.totalTokens`. The session-log record uses persisted `type=message` entries with the same assistant message shape. The stream record is operationally preferable; the session-log record is useful for retrospective indexing.

## Model Resolution

Pi persists explicit model changes as `model_change` session entries with `provider` and `modelId`. The session context builder also derives the active model from assistant messages when no explicit model-change entry is present. The frontmatter records only the durable `model_change` log entry to avoid overlapping assistant `message_end` token usage under first-match-wins detection.

Pi has two model fallback concepts. During model parsing, if the user supplies `--provider <provider> --model <unknown-id>`, Pi can build a custom fallback model id and return a warning. During session restore, if the saved model no longer exists or has no configured auth, Pi prints warnings and returns a fallback model. Both are source-code-confirmed, but neither is emitted as a stable JSONL signal in print mode, so `model_fallback` is a gap rather than a frontmatter record.

## Interruption and Recovery

Assistant `stopReason: "aborted"` is the stream-level cancellation result. Print-mode signal handlers also dispose the runtime and exit with 129 for SIGHUP or 143 for SIGTERM; those exits may happen without an assistant message. Claudine should therefore combine stream detection with its own process termination classification.

Session resumption is represented by session files. A session header with a stable `id`, timestamp, and cwd proves there is a persisted session object that Pi can load or show in `/resume`. The frontmatter records `session_resumable` on the session-log header, with the caveat that actual resume still depends on file availability and the launch mode.

## Human Input

Stock print-mode JSON does not request human input. RPC mode can emit `extension_ui_request` records for extension-driven UI. The method vocabulary in source is `select`, `confirm`, `input`, `editor`, `notify`, `setStatus`, and `setWidget`; only the first four actually request input. Claudine's reserved `human_input_requested` maps to this RPC-only surface.

## Version Drift

The inspected repository head was `2e4ad6a09423002f58b9a5dc2749f7db7929d0f0`. Existing local Pi research in this repo previously observed the npm package at `@earendil-works/pi-coding-agent 0.80.3` and an older local installed binary under the legacy `@mariozechner` scope. The current source still documents `--mode json` as the event stream and `--mode rpc` as Pi's JSONL RPC protocol.

No per-signal version drift was established for the records above. The docs link in `docs/json.md` still points at old `pi-mono` source URLs, while the current repository is `earendil-works/pi`; source files in this document use the current repository path and commit.

## Quirks and Gaps

Pi's most important quirk is that provider operational failures are string-normalized, not enum-normalized. Claudine can detect many critical failures, but the strongest machine-readable fields are `message.stopReason`, `message.usage`, retry lifecycle records, and session entries.

The schema's `match_op: exists` covers plain field presence, but the JSONPath schema still cannot express "field A differs from field B." That matters for `responseModel`: existence implies a concrete upstream model when it differs from the requested model, but a simple record would overlap token metering on the same `message_end` payload.

`delayMs` is milliseconds, but the signal schema currently has `duration_secs` and no duration-milliseconds unit. The extraction keeps `delay_ms` with a note and no unit.

`provider_version`, `auth_kind_detected`, `usage_cap_approaching`, `permission_denied_read`, `permission_denied_write`, `model_fallback`, `turn_limit_reached`, and `session_time_limit_reached` were not found as stable stock Pi JSONL records.

## Changelog

Initial Pi signal research document.

## Sources

- [Pi JSON Event Stream Mode docs](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/json.md)
- [Print mode JSON writer at `2e4ad6a`](https://github.com/earendil-works/pi/blob/2e4ad6a09423002f58b9a5dc2749f7db7929d0f0/packages/coding-agent/src/modes/print-mode.ts#L104-L116)
- [AgentSessionEvent union at `2e4ad6a`](https://github.com/earendil-works/pi/blob/2e4ad6a09423002f58b9a5dc2749f7db7929d0f0/packages/coding-agent/src/core/agent-session.ts#L126-L152)
- [AgentEvent union at `2e4ad6a`](https://github.com/earendil-works/pi/blob/2e4ad6a09423002f58b9a5dc2749f7db7929d0f0/packages/agent/src/types.ts#L413-L428)
- [AssistantMessage and Usage types at `2e4ad6a`](https://github.com/earendil-works/pi/blob/2e4ad6a09423002f58b9a5dc2749f7db7929d0f0/packages/ai/src/types.ts#L352-L395)
- [Retry classifier at `2e4ad6a`](https://github.com/earendil-works/pi/blob/2e4ad6a09423002f58b9a5dc2749f7db7929d0f0/packages/ai/src/utils/retry.ts#L7-L97)
- [Retry event emission at `2e4ad6a`](https://github.com/earendil-works/pi/blob/2e4ad6a09423002f58b9a5dc2749f7db7929d0f0/packages/coding-agent/src/core/agent-session.ts#L994-L1004)
- [Auth error generation at `2e4ad6a`](https://github.com/earendil-works/pi/blob/2e4ad6a09423002f58b9a5dc2749f7db7929d0f0/packages/coding-agent/src/core/agent-session.ts#L371-L395)
- [Startup no-model exit at `2e4ad6a`](https://github.com/earendil-works/pi/blob/2e4ad6a09423002f58b9a5dc2749f7db7929d0f0/packages/coding-agent/src/main.ts#L793-L798)
- [Session manager header and entries at `2e4ad6a`](https://github.com/earendil-works/pi/blob/2e4ad6a09423002f58b9a5dc2749f7db7929d0f0/packages/coding-agent/src/core/session-manager.ts#L30-L149)
- [Model change persistence at `2e4ad6a`](https://github.com/earendil-works/pi/blob/2e4ad6a09423002f58b9a5dc2749f7db7929d0f0/packages/coding-agent/src/core/session-manager.ts#L1001-L1012)
- [Model fallback paths at `2e4ad6a`](https://github.com/earendil-works/pi/blob/2e4ad6a09423002f58b9a5dc2749f7db7929d0f0/packages/coding-agent/src/core/model-resolver.ts#L163-L177)
- [RPC protocol and extension UI request types at `2e4ad6a`](https://github.com/earendil-works/pi/blob/2e4ad6a09423002f58b9a5dc2749f7db7929d0f0/packages/coding-agent/src/modes/rpc/rpc-types.ts#L20-L72)
- [RPC extension UI request vocabulary at `2e4ad6a`](https://github.com/earendil-works/pi/blob/2e4ad6a09423002f58b9a5dc2749f7db7929d0f0/packages/coding-agent/src/modes/rpc/rpc-types.ts#L229-L260)
