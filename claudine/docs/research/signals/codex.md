---
$schema: ./_schema.yaml
created: 2026-07-05
last_updated: 2026-07-06
agent: codex
model: default
docs: https://developers.openai.com/codex/noninteractive
records:
  - id: stream-usage_capped-turn_failed-message
    signal: usage_capped
    source: stream
    locator: "type=turn.failed"
    detection: declarative
    priority: 10
    match_path: error.message
    match_op: substring_ci
    match_value: "you've hit your usage limit"
    distinguish: "This is Codex's account/session usage-cap copy on a failed turn. It must win over generic `rate_limit` records because Codex also classifies the same payload as a rate-limit-shaped error."
    vocabulary: ["rate_limit"]
    confidence: source_code
    evidence: ./fixtures/codex/turn-failed-usage-limit.jsonl
    notes: "The current Codex source formats `UsageLimitReachedError` as usage-limit prose; the seeded fixture preserves the nested compatibility shape Claudine already parses."
  - id: stream-usage_capped-error-message
    signal: usage_capped
    source: stream
    locator: "type=error"
    detection: declarative
    priority: 20
    match_path: error_message
    match_op: substring_ci
    match_value: "you've hit your usage limit"
    distinguish: "This catches the duplicate top-level terminal error emitted after the `turn.failed` usage-cap payload. It is more specific than generic `error_type=rate_limit`."
    vocabulary: ["rate_limit"]
    confidence: observed
    evidence: ./fixtures/codex/turn-failed-usage-limit.jsonl
  - id: stream-rate_limited-error_type
    signal: rate_limited
    source: stream
    locator: "type=error"
    detection: declarative
    priority: 30
    match_path: error_type
    match_op: eq
    match_value: rate_limit
    distinguish: "This is a generic rate-limit stream error such as `Too many requests`; usage-cap wording is handled by the lower-priority-number usage-capped records first."
    vocabulary: ["rate_limit"]
    confidence: observed
    evidence: ./fixtures/codex/error-rate-limit.jsonl
  - id: stream-tokens_consumed-turn_completed
    signal: tokens_consumed
    source: stream
    locator: "type=turn.completed"
    detection: declarative
    priority: 40
    match_path: usage.input_tokens
    match_op: exists
    distinguish: "`turn.completed` is the terminal usage envelope for `codex exec --json`; it differs from app-server live token-count notifications because exec only emits the flattened total when the turn completes."
    vocabulary: ["turn.completed"]
    since: rust-v0.142.5
    confidence: source_code
    evidence: ./fixtures/codex/turn-completed-usage-reasoning-docs.jsonl
    notes: "Usage shape source-confirmed against exec_events.rs at rust-v0.142.5; the evidence fixture line is docs-sourced (developers.openai.com), so the fixture proves the documented example, the source citation proves the shape."
  - id: stream-session_resumable-thread_started
    signal: session_resumable
    source: stream
    locator: "type=thread.started"
    detection: declarative
    priority: 50
    match_path: type
    match_op: eq
    match_value: thread.started
    distinguish: "`thread.started.thread_id` is the identifier accepted by `codex exec resume <SESSION_ID>`; it is a resumability affordance, not a success or usage signal."
    vocabulary: ["thread.started"]
    confidence: documented
    evidence: ./fixtures/codex/thread-started-docs.jsonl
extractions:
  - record: stream-usage_capped-turn_failed-message
    field: message
    path: error.message
  - record: stream-usage_capped-error-message
    field: message
    path: error_message
  - record: stream-rate_limited-error_type
    field: message
    path: error_message
  - record: stream-tokens_consumed-turn_completed
    field: input
    path: usage.input_tokens
    unit: tokens
  - record: stream-tokens_consumed-turn_completed
    field: cache_read
    path: usage.cached_input_tokens
    unit: tokens
  - record: stream-tokens_consumed-turn_completed
    field: output
    path: usage.output_tokens
    unit: tokens
  - record: stream-tokens_consumed-turn_completed
    field: reasoning_output
    path: usage.reasoning_output_tokens
    unit: tokens
  - record: stream-session_resumable-thread_started
    field: session_id
    path: thread_id
bespoke_rationale: []
gaps:
  - "Current Codex `rust-v0.142.5` exec source defines `ThreadErrorEvent` and `TurnFailedEvent.error` as message-only structs, while seeded Claudine fixtures include older/observed `error_type` and nested `error.type` discriminators. A fresh live failing run is needed for current `message`-only rate-limit and usage-cap fixtures."
  - "Codex core exposes richer app-server/internal error vocabulary (`UsageLimitExceeded`, `ServerOverloaded`, `Unauthorized`, `ResponseTooManyFailedAttempts`, `ResponseStreamDisconnected`, and related HTTP status data), but `codex exec --json` flattens most of that to message text. Records for provider_overloaded, retries_exhausted, auth_invalid, repeated_stream_error, and no_funds need committed exec JSONL fixtures before frontmatter detection can be made reliable."
  - "Codex core records rate-limit snapshots with percent-used windows, reset times, credits, plan type, and `rate_limit_reached_type`, but the exec JSONL stream does not expose those fields directly. Usage-cap reset extraction is therefore unavailable from the current primary stream."
  - "App-server docs and source expose `turn/completed` with `status: interrupted` and `TurnAbortedEvent.reason: interrupted`, but `codex exec --json` does not emit an interruption record in the current projection; Claudine should continue to infer Ctrl+C/process interruption at the wrapper layer unless it adopts app-server as a signal source."
  - "Current source maps `ModelRerouted` to a non-fatal `item.completed` error item whose message is formatted as `model rerouted: from -> to (reason)`. No scrubbed exec fixture exists, and the message is not structured enough for high-confidence model_fallback extraction."
  - "No exec stream fixture was found for permission-denied read/write. Codex permission prompts and declines are visible as item/status or app-server approval flows, but mapping read versus write denial requires live payloads with paths and decisions."
  - "No provider_version record is available from `codex exec --json`; version is discoverable through CLI/package metadata and local files, not the primary stream."
changes:
  - "2026-07-06: rewrote the turn_completed usage.input_tokens numeric-regex presence proxy to `match_op: exists`."
requires_claudine_update: true
reason: "Codex signal detection is codegen-wired and this research adds Codex stream records for usage caps, generic rate limits, token usage, and resumability while documenting current source drift that Claudine must account for."
---

# Codex CLI Signal Detection

## Overview

Codex CLI exposes the most useful wrapper-grade signal surface through `codex exec --json`, which writes newline-delimited JSON events to stdout. The official non-interactive documentation describes this mode as a JSONL stream with event types including `thread.started`, `turn.started`, `turn.completed`, `turn.failed`, `item.*`, and `error`; the current Rust source models the same stream as the `ThreadEvent` enum in `codex-rs/exec`.

The stream is mature enough for turn lifecycle, terminal token usage, resumability, and terminal errors, but it is intentionally flatter than Codex's app-server/internal protocol. The core protocol and app-server surfaces carry richer `CodexErrorInfo`, rate-limit snapshots, token-count notifications, interruption status, and model reroute notifications; the exec JSONL projection either omits them or turns them into message-only events. Detection should therefore prefer the exec stream for Claudine's direct wrapper and treat app-server details as future integration material unless Claudine explicitly starts Codex through `codex app-server`.

## Signal Surfaces

### Stream Events

`codex exec --json` emits JSON Lines on stdout. The current upstream `rust-v0.142.5` event enum is tagged by the top-level `type` field and includes `thread.started`, `turn.started`, `turn.completed`, `turn.failed`, `item.started`, `item.updated`, `item.completed`, and `error`. `turn.completed` carries a `usage` object with `input_tokens`, `cached_input_tokens`, `output_tokens`, and `reasoning_output_tokens`. `thread.started` carries `thread_id`, which the CLI can later resume.

The TypeScript SDK mirrors this contract in `sdk/typescript/src/events.ts` and `sdk/typescript/src/items.ts`. The SDK item vocabulary includes agent messages, reasoning, command execution, file changes, MCP tool calls, web search, todo lists, and item-level errors. The official non-interactive docs give the same practical contract and a sample stream.

Claudine's seeded fixtures and local parser also retain compatibility with older or observed Codex shapes: `error_type`/`error_message`, nested `error.type`/`error.message`, `stream.error`, `turn.error`, `thread.created`, `duration_ms`, `status`, `stop_reason`, and `cache_read_input_tokens`. Those are useful for compatibility but are not all present in the current upstream `rust-v0.142.5` exec type definitions.

### App-Server Streams

`codex app-server` is a JSON-RPC-style server over stdio or WebSocket. Official docs describe server-initiated notifications for `thread/*`, `turn/*`, `item/*`, `serverRequest/resolved`, and `thread/tokenUsage/updated`. It is a richer contract than exec JSONL: failed turns include `error.message`, optional `codexErrorInfo`, and optional `additionalDetails`; token usage can update during the turn; and app-server clients participate in approval and server-request flows.

This surface is not represented as a separate source enum in the current signal schema. For this document, records stay focused on `codex exec --json` stream events. App-server behavior is documented in gaps where it proves that Codex has a native signal but the primary Claudine wrapper stream does not expose a fixture-ready payload.

### Session Logs and Local State

Codex persists session rollouts and local state under `CODEX_HOME`, including session JSONL, session indexes, and SQLite state/log databases. Existing Claudine agent-logging research documents those surfaces in detail. They are valuable for after-the-fact enrichment, but they are not the primary signal source for a non-interactive wrapper because they are local stores, may be live-locked, and do not replace stdout supervision.

### stderr Diagnostics and Hooks

`codex exec --json` uses stdout for machine events. Human stderr diagnostics are not a stable signal contract for the records in this document. Codex hooks and notification commands exist for lifecycle customization, but the official notify event currently targets turn completion rather than the operational failure taxonomy here.

## Usage and Rate Limits

Codex has several layers of usage and rate-limit semantics.

In the primary exec stream, the fixture-backed shipped/observed records are:

| Signal | Source | Locator | Match | Extraction |
| --- | --- | --- | --- | --- |
| `usage_capped` | stream | `type=turn.failed` | `error.message` contains `You've hit your usage limit` | `message` from `error.message` |
| `usage_capped` | stream | `type=error` | `error_message` contains `You've hit your usage limit` | `message` from `error_message` |
| `rate_limited` | stream | `type=error` | `error_type == rate_limit` | `message` from `error_message` |

The priority order matters. A usage-cap failure can also carry `rate_limit` as the provider's coarse error kind, so `usage_capped` records must run before the generic `rate_limited` record. The seeded usage-limit fixture shows a `turn.failed` payload followed by a duplicate top-level `error`; the local Claudine parser already deduplicates that pair as one terminal error.

Current upstream source confirms the underlying Codex error vocabulary is richer than the current exec JSONL contract. `CodexErrorInfo` includes `UsageLimitExceeded`, `ServerOverloaded`, `Unauthorized`, `ResponseStreamDisconnected`, and `ResponseTooManyFailedAttempts`; `UsageLimitReachedError` formats user-facing strings for plan-specific usage caps, workspace credit depletion, workspace spend caps, and retry/reset suffixes. Core tests also show rate-limit snapshots with `used_percent`, `window_minutes`, and `resets_at` captured from `x-codex-*` headers. These facts are source-code-confirmed but not fixture-backed in the exec stream, so they are tracked as gaps rather than frontmatter records.

`no_funds` is source-confirmed but not recorded. The core error enum has `QuotaExceeded`, and upstream tests map a Responses `insufficient_quota` failure to the user-facing message `Quota exceeded. Check your plan and billing details.` No committed exec JSONL fixture proves the primary stream shape for that failure.

## Token Metering

`tokens_consumed` is emitted by the primary exec stream on `turn.completed`. In current `rust-v0.142.5`, `Usage` has four token fields:

| Field | Unit | Notes |
| --- | --- | --- |
| `usage.input_tokens` | tokens | Total input tokens reported for the turn/session projection. |
| `usage.cached_input_tokens` | tokens | Cached input tokens. Older Claudine compatibility code also accepts `cache_read_input_tokens`. |
| `usage.output_tokens` | tokens | Output tokens. |
| `usage.reasoning_output_tokens` | tokens | Reasoning output tokens; present in the current docs and SDK/Rust type. |

The exec JSONL projection builds this usage from the last `ThreadTokenUsageUpdated` app-server notification and emits it only when the turn completes. App-server/core has both total and last token usage plus optional `model_context_window`, but the exec JSONL event intentionally exposes the flatter four-field `usage` object.

## Authentication and Authorization

No fixture-backed `auth_invalid` record is available for `codex exec --json`. Source confirms that internal/core errors can become `CodexErrorInfo::Unauthorized`, and client code carries auth debug tags such as `auth_error` and `auth_error_code`, but the current exec stream exposes only message-level terminal errors. A fresh failing-auth exec capture is required before Claudine can choose a reliable declarative match.

Permission prompts are visible in Codex as item or app-server approval flows rather than as direct denied-read/denied-write signals in the primary stream. The app-server docs describe command execution and file-change approvals with decisions including `accept`, `acceptForSession`, `decline`, and `cancel`. Claudine's local parser recognizes compatibility item types such as `permission_request`, `approval_request`, and `user_input_request`, but no scrubbed fixture proves a denied read/write payload with enough path semantics to map it into `permission_denied_read` or `permission_denied_write`.

## Model Resolution and Fallback

`codex exec --json` does not emit the resolved model in `thread.started`; it emits only the thread id. The underlying `SessionConfiguredEvent` includes `model`, `model_provider_id`, `service_tier`, permission profile, and working directory, but the exec JSONL projection's `thread_started_event` keeps only `thread_id`.

Codex does have a native model reroute signal internally. `ModelRerouteEvent` carries `from_model`, `to_model`, and `reason`, where the current `ModelRerouteReason` vocabulary is `high_risk_cyber_activity`. The exec JSONL projection currently converts that to an `item.completed` error item with message text like `model rerouted: <from> -> <to> (<reason>)`. Because there is no scrubbed exec fixture and the projection is not structured, `model_fallback` is left as a gap.

`provider_version` is not emitted in the exec JSONL stream. It can be learned through CLI package/version metadata or local update-check state, but not as a stream event record.

## Interruption and Recovery

The official non-interactive docs state that `thread.started.thread_id` can be used with `codex exec resume <SESSION_ID>`, so `session_resumable` is recorded from `thread.started`. This is a future-facing reserved signal in Claudine's taxonomy, but Codex's stream makes the resumability handle explicit.

Interruption is different. App-server docs say `turn/completed` can carry `turn.status` values including `interrupted`, and the source defines `TurnAbortedEvent.reason` with `interrupted` and `replaced`. The current exec JSONL projection handles `TurnStatus::Interrupted` by initiating shutdown without emitting a `turn.failed` or `turn.completed` JSONL event. For Claudine's direct exec wrapper, `interrupted` therefore remains a wrapper/process signal, not a provider stream record.

## Version Drift

The main drift is between Claudine's compatibility parser/seeded fixtures and current upstream `rust-v0.142.5` exec types.

| Area | Older or observed fixture shape | Current upstream shape | Impact |
| --- | --- | --- | --- |
| Top-level error | `{"type":"error","error_type":"rate_limit","error_message":"..."}` | `{"type":"error","message":"..."}` in `ThreadErrorEvent` | Keep compatibility records; capture fresh current failures before removing or replacing them. |
| Failed turn error | `turn.failed.error.type` and `turn.failed.error.message` | `turn.failed.error.message` only | Usage-cap message matching remains viable; kind extraction is not current-contract. |
| Stream error | `{"type":"stream.error","error":{...}}` | Not in current `ThreadEvent` enum | Treat as compatibility/legacy until a current source or live fixture proves it. |
| Usage cache field | `cached_input_tokens` and older `cache_read_input_tokens` accepted by Claudine | Current Rust/TypeScript docs use `cached_input_tokens` | Prefer `cached_input_tokens`; keep compatibility parser support. |
| Reasoning usage | Missing in some seeded/live fixtures | Current Rust/TypeScript/docs include `reasoning_output_tokens` | Detection should tolerate missing reasoning usage for older fixtures but extract it when present. |

## Quirks and Gaps

Codex's internal signal vocabulary is stronger than its exec JSONL projection. The app-server/core can classify usage limits, overloads, authorization failures, stream disconnections, too many failed attempts, rate-limit windows, credits, and model reroutes. The exec stream mostly surfaces terminal usage and message text.

Generic `rate_limit` is not enough to classify a cap. Usage-cap wording must be checked first because a cap can be encoded as a rate-limit-shaped error.

No reliable `usage_cap_approaching` record was found in the exec stream. Core stores percent-used windows and resets, and config has a `min_rate_limit_remaining_percent` setting, but no primary stream warning event or fixture proves an approaching-cap payload.

No reliable `no_funds`, `auth_invalid`, `provider_overloaded`, `retries_exhausted`, `model_resolved`, `model_fallback`, `permission_denied_read`, `permission_denied_write`, `provider_version`, `generation_retried`, `repeated_stream_error`, `human_input_requested`, or `interrupted` exec-stream record is fixture-backed today.

## Changelog

Fresh first-run research document; `changes` is empty.

## Sources

- [OpenAI Codex non-interactive mode documentation](https://developers.openai.com/codex/noninteractive)
- [OpenAI Codex app-server documentation](https://developers.openai.com/codex/app-server)
- [Codex `ThreadEvent` and `Usage` definitions, `rust-v0.142.5`](https://github.com/openai/codex/blob/rust-v0.142.5/codex-rs/exec/src/exec_events.rs#L8-L70)
- [Codex exec JSONL projection, `rust-v0.142.5`](https://github.com/openai/codex/blob/rust-v0.142.5/codex-rs/exec/src/event_processor_with_jsonl_output.rs#L117-L127)
- [Codex exec event projection for errors, reroutes, token usage, and turn completion, `rust-v0.142.5`](https://github.com/openai/codex/blob/rust-v0.142.5/codex-rs/exec/src/event_processor_with_jsonl_output.rs#L438-L550)
- [Codex TypeScript SDK event types, `rust-v0.142.5`](https://github.com/openai/codex/blob/rust-v0.142.5/sdk/typescript/src/events.ts#L20-L82)
- [Codex TypeScript SDK item types, `rust-v0.142.5`](https://github.com/openai/codex/blob/rust-v0.142.5/sdk/typescript/src/items.ts#L5-L128)
- [Codex protocol error vocabulary and rate-limit structures, `rust-v0.142.5`](https://github.com/openai/codex/blob/rust-v0.142.5/codex-rs/protocol/src/protocol.rs#L1690-L1725)
- [Codex model reroute and token/rate-limit protocol types, `rust-v0.142.5`](https://github.com/openai/codex/blob/rust-v0.142.5/codex-rs/protocol/src/protocol.rs#L1916-L2129)
- [Codex core error display and usage-limit formatting, `rust-v0.142.5`](https://github.com/openai/codex/blob/rust-v0.142.5/codex-rs/protocol/src/error.rs#L100-L132)
- [Codex `UsageLimitReachedError` display logic, `rust-v0.142.5`](https://github.com/openai/codex/blob/rust-v0.142.5/codex-rs/protocol/src/error.rs#L419-L520)
- [Codex core rate-limit snapshot test, `rust-v0.142.5`](https://github.com/openai/codex/blob/rust-v0.142.5/codex-rs/core/tests/suite/client.rs#L2964-L3185)
- [Codex quota-exceeded test, `rust-v0.142.5`](https://github.com/openai/codex/blob/rust-v0.142.5/codex-rs/core/tests/suite/quota_exceeded.rs#L15-L76)
- [Claudine Codex compatibility parser](../../../../lib/src/stream/protocol/codex.rs)
- [Claudine Codex stream fixtures](fixtures/codex/)
