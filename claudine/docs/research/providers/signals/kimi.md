---
$schema: ./_schema.yaml
created: 2026-07-05
last_updated: 2026-07-06
agent: codex
model: default
docs: https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html
records:
  - id: stream-provider_version-init
    signal: provider_version
    source: stream
    locator: "JSON-RPC initialize result.server"
    detection: declarative
    priority: 10
    match_path: result.server.name
    match_op: eq
    match_value: "Kimi Code CLI"
    distinguish: "The initialize response is metadata from the wire server, not a turn event. It carries both the Kimi server version and wire protocol version."
    vocabulary: ["Kimi Code CLI"]
    confidence: source_code
    evidence: ./fixtures/kimi/wire-protocol-110.jsonl
  - id: stream-auth_invalid-auth_expired
    signal: auth_invalid
    source: stream
    locator: "JSON-RPC error.code=-32004"
    detection: declarative
    priority: 20
    match_path: error.code
    match_op: regex
    match_value: "^-32004$"
    distinguish: "`-32004` is Kimi wire mode's `AUTH_EXPIRED` code. It is narrower than generic chat-provider errors and should be treated as an OAuth re-login requirement."
    vocabulary: ["-32004"]
    confidence: source_code
    evidence: ./fixtures/kimi/wire-auth-expired.jsonl
  - id: stream-rate_limited-step_retry-429
    signal: rate_limited
    source: stream
    locator: "params.type=StepRetry"
    detection: declarative
    priority: 30
    match_path: params.payload.status_code
    match_op: regex
    match_value: "^429$"
    distinguish: "A `StepRetry` with HTTP status 429 is specifically a rate-limit retry. It must win over the broader `generation_retried` record, which also matches `StepRetry`."
    vocabulary: ["APIStatusError", "429"]
    since: "1.10"
    confidence: source_code
    evidence: ./fixtures/kimi/step-retry-rate-limit.jsonl
    notes: "since is a wire-protocol version, not a kimi-cli package version."
  - id: stream-generation_retried-step_retry
    signal: generation_retried
    source: stream
    locator: "params.type=StepRetry"
    detection: declarative
    priority: 40
    match_path: params.type
    match_op: eq
    match_value: StepRetry
    distinguish: "`StepRetry` is emitted only when the current step attempt failed and will be retried. More specific retry causes, such as HTTP 429, should be evaluated first."
    vocabulary: ["StepRetry", "APIEmptyResponseError", "APIStatusError"]
    since: "1.10"
    confidence: source_code
    evidence: ./fixtures/kimi/wire-protocol-110.jsonl
    notes: "since is a wire-protocol version, not a kimi-cli package version."
  - id: stream-tokens_consumed-status_update
    signal: tokens_consumed
    source: stream
    locator: "params.type=StatusUpdate"
    detection: declarative
    priority: 50
    match_path: params.payload.token_usage.output
    match_op: exists
    distinguish: "`StatusUpdate.token_usage` is the step usage envelope. Other `StatusUpdate` messages may only update plan mode or MCP state and should not be counted as token-metering events."
    vocabulary: ["StatusUpdate"]
    confidence: source_code
    evidence: ./fixtures/kimi/status-update-token-usage.jsonl
  - id: stream-interrupted-cancelled-result
    signal: interrupted
    source: stream
    locator: "JSON-RPC prompt result.status=cancelled"
    detection: declarative
    priority: 60
    match_path: result.status
    match_op: eq
    match_value: cancelled
    distinguish: "`cancelled` is the terminal prompt result returned after `RunCancelled`; it differs from an ordinary `TurnEnd`, which can still be emitted during orderly cancellation cleanup."
    vocabulary: ["cancelled"]
    confidence: source_code
    evidence: ./fixtures/kimi/wire-cancelled-interrupt.jsonl
  - id: stream-turn_limit_reached-max_steps
    signal: turn_limit_reached
    source: stream
    locator: "JSON-RPC prompt result.status=max_steps_reached"
    detection: declarative
    priority: 70
    match_path: result.status
    match_op: eq
    match_value: max_steps_reached
    distinguish: "Kimi names this terminal status `max_steps_reached`; it is the provider-native loop limit outcome and maps to Claudine's turn-limit family rather than a token or wall-clock timeout."
    vocabulary: ["max_steps_reached"]
    confidence: source_code
    evidence: ./fixtures/kimi/wire-max-steps-reached.jsonl
  - id: stream-human_input_requested-question_request
    signal: human_input_requested
    source: stream
    locator: "params.type=QuestionRequest"
    detection: declarative
    priority: 80
    match_path: params.type
    match_op: eq
    match_value: QuestionRequest
    distinguish: "`QuestionRequest` is a wire request that blocks until the client returns `QuestionResponse`; it is not an approval request or tool result."
    vocabulary: ["QuestionRequest"]
    since: "1.4"
    confidence: source_code
    evidence: ./fixtures/kimi/wire-question-request.jsonl
    notes: "since is a wire-protocol version, not a kimi-cli package version."
extractions:
  - record: stream-provider_version-init
    field: version
    path: result.server.version
  - record: stream-provider_version-init
    field: protocol_version
    path: result.protocol_version
  - record: stream-auth_invalid-auth_expired
    field: message
    path: error.message
  - record: stream-rate_limited-step_retry-429
    field: attempt
    path: params.payload.next_attempt
    unit: requests
  - record: stream-rate_limited-step_retry-429
    field: max_attempts
    path: params.payload.max_attempts
    unit: requests
  - record: stream-rate_limited-step_retry-429
    field: retry_after
    path: params.payload.wait_s
    unit: duration_secs
  - record: stream-rate_limited-step_retry-429
    field: status_code
    path: params.payload.status_code
  - record: stream-rate_limited-step_retry-429
    field: error_type
    path: params.payload.error_type
  - record: stream-generation_retried-step_retry
    field: attempt
    path: params.payload.next_attempt
    unit: requests
  - record: stream-generation_retried-step_retry
    field: max_attempts
    path: params.payload.max_attempts
    unit: requests
  - record: stream-generation_retried-step_retry
    field: wait
    path: params.payload.wait_s
    unit: duration_secs
  - record: stream-generation_retried-step_retry
    field: error_type
    path: params.payload.error_type
  - record: stream-generation_retried-step_retry
    field: status_code
    path: params.payload.status_code
  - record: stream-tokens_consumed-status_update
    field: input_other
    path: params.payload.token_usage.input_other
    unit: tokens
  - record: stream-tokens_consumed-status_update
    field: output
    path: params.payload.token_usage.output
    unit: tokens
  - record: stream-tokens_consumed-status_update
    field: cache_read
    path: params.payload.token_usage.input_cache_read
    unit: tokens
  - record: stream-tokens_consumed-status_update
    field: cache_creation
    path: params.payload.token_usage.input_cache_creation
    unit: tokens
  - record: stream-tokens_consumed-status_update
    field: context_tokens
    path: params.payload.context_tokens
    unit: tokens
  - record: stream-tokens_consumed-status_update
    field: max_context_tokens
    path: params.payload.max_context_tokens
    unit: tokens
  - record: stream-tokens_consumed-status_update
    field: message_id
    path: params.payload.message_id
  - record: stream-interrupted-cancelled-result
    field: status
    path: result.status
  - record: stream-turn_limit_reached-max_steps
    field: limit
    path: result.steps
    unit: requests
  - record: stream-human_input_requested-question_request
    field: request_id
    path: params.payload.id
  - record: stream-human_input_requested-question_request
    field: tool_call_id
    path: params.payload.tool_call_id
bespoke_rationale: []
gaps:
  - "Kimi wire `StatusUpdate.context_usage` is documented as a ratio between 0 and 1, while Claudine's current extraction unit vocabulary has `percent` but no `ratio`. The frontmatter extracts token counts and leaves the ratio in prose until the normalized payload contract decides how to represent it."
  - "No fixture-backed `usage_cap_approaching` or `usage_capped` wire event was found. Kimi may surface account caps through provider error messages or HTTP status codes, but source inspection did not reveal a stable cap-specific discriminator in wire mode."
  - "No `no_funds` record is emitted. Interactive shell UI has special handling for HTTP 402, but the wire server currently returns generic `CHAT_PROVIDER_ERROR` for non-401 `APIStatusError`; a billing fixture is needed before matching message text."
  - "Kimi classifies HTTP 500, 502, 503, and 504 as retryable and emits `StepRetry`, but the wire payload does not distinguish provider overload from generic transient server failure. `provider_overloaded` is therefore left unrecorded."
  - "Kimi ACP initializes with `session_capabilities.resume`, but no committed ACP initialize fixture exists in this corpus. `session_resumable` should be added from ACP only after scrubbed ACP fixtures are captured."
  - "Kimi wire mode supports `ApprovalRequest` and `ApprovalResponse.response=reject`, but those are human/client approval mechanics, not provider-denied read/write errors with stable resource paths. No `permission_denied_read` or `permission_denied_write` record is available."
  - "No model resolution or model fallback wire event was found. The selected model lives in runtime/config state; the initialize result exposes server identity and protocol version, not the active model id."
  - "Kimi ACP consumes `StatusUpdate` and `StepRetry` internally rather than forwarding them as ACP session updates in the current adapter. Wire mode remains the better signal surface for tokens and retries."
  - "The wire docs are first-class, but some source events are ahead of the rendered docs: current source includes `Notification` and `MCPLoadingBegin`/`MCPLoadingEnd` in the event union while the inspected English wire-mode page's event list omits MCP loading events."
  - "unsupported_protocol_version is synthesized wrapper-side by claudine when initialize's result.protocol_version falls outside {1.9, 1.10} — a negative match the declarative grammar cannot express; record as wrapper-bespoke when the engine lands."
  - "auth_kind_detected is unrecorded — no wire surface announcing the auth mechanism was identified."
changes:
  - "2026-07-06: rewrote the StatusUpdate token_usage.output numeric-regex presence proxy to `match_op: exists`."
  - "2026-07-06: renamed extraction fields to the canonical SignalEvent payload names — rate_limited `wait` → `retry_after` and turn_limit_reached `steps` → `limit` (the generation_retried record keeps `wait`, which is canonical there)."
requires_claudine_update: true
reason: "Kimi signal detection is codegen-wired and this research adds wire-stream records for provider version, auth expiry, rate limiting, retry, token metering, interruption, max-step limits, and reserved human-input requests."
---

# Kimi Code CLI Signal Detection

## Overview

Kimi Code CLI exposes its strongest machine-readable signal surface through wire mode, a JSON-RPC stream used by `kimi --wire` and the wire server. Wire messages are emitted as JSON-RPC `event`, `request`, success, and error messages. The typed payload vocabulary is defined in source as Pydantic models and documented in the official wire-mode reference. The seeded Claudine fixtures for Kimi all use this wire stream.

Kimi also has an ACP server, hook payloads, app/web APIs, session files, and terminal UI diagnostics. Those surfaces are useful context, but current source shows that ACP consumes several operational wire events internally and hooks are extension points rather than provider signal contracts. For Claudine wrapper-grade detection today, wire JSON-RPC should be treated as the primary structured source.

## Signal Surfaces

### Wire Stream

Wire mode emits JSON-RPC messages. Events and requests place a Kimi envelope under `params` with `type` and `payload`; prompt completion uses JSON-RPC success responses with a `result.status`; failures use JSON-RPC error responses with `error.code` and `error.message`.

The source-defined event union includes `TurnBegin`, `SteerInput`, `TurnEnd`, `StepBegin`, `StepInterrupted`, `StepRetry`, hook lifecycle events, compaction events, MCP loading events, `StatusUpdate`, `Notification`, content and tool events, approval responses, subagent events, plan display, and side-question begin/end events. Request messages include `ApprovalRequest`, `ToolCallRequest`, `QuestionRequest`, and `HookRequest`.

The official wire-mode docs describe the message transport as `event` and `request` methods carrying `{ "type": "...", "payload": ... }`. They also document `StepRetry`, `StatusUpdate`, `TokenUsage`, `ApprovalRequest`, `QuestionRequest`, and response shapes. The docs label `StepRetry` as added in Wire 1.10 and `QuestionRequest` as added in Wire 1.4.

### ACP Streams

Kimi's ACP server negotiates ACP protocol version 1, reports Kimi implementation metadata, declares `load_session`, MCP, and session resume capabilities, and supplies a terminal-auth method for login. The ACP prompt adapter iterates over the same internal Kimi wire messages, but in current source it ignores `StatusUpdate` and `StepRetry`, forwards text/thinking/tool streams, resolves unsupported `QuestionRequest` with an empty answer, and maps OAuth 401 failures to ACP `auth_required`.

This means ACP is not currently equivalent to the wire stream for Claudine's signal taxonomy. It can prove future `session_resumable` and `auth_invalid` mappings, but this document does not add ACP records without committed ACP fixtures.

### Hooks

Kimi hooks are documented as a beta lifecycle extension mechanism. Hook commands receive JSON on stdin and can run for `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `UserPromptSubmit`, `Stop`, `StopFailure`, `SessionStart`, `SessionEnd`, subagent events, compaction events, and notifications. Source builders produce payloads such as `hook_event_name`, `tool_name`, `tool_input`, `error_type`, `error_message`, `source`, and `reason`.

Hooks are not the primary detection surface for this signal document. They are user-configurable side effects and policy gates. A `StopFailure` hook can observe generic error type/message text, and approval hooks can block actions, but neither provides a stable provider-owned read/write permission-denial signal with resource paths.

### Session Logs and App Logs

Kimi records wire conversations and session metadata for replay, export/import, web UI, and session management. The local store is useful for retrospective analysis and resumption, but the source-backed real-time semantics are the wire messages themselves. This document therefore records stream rules against JSON-RPC wire payloads rather than local file paths.

### stderr Diagnostics

The terminal UI renders human-readable messages for retry, auth, and billing-like errors. For example, the UI labels a `StepRetry` with status 429 as a rate-limit retry. These diagnostics are not treated as first-class detection records because wire mode already exposes the structured payload.

## Usage and Rate Limits

Kimi emits retry information through `StepRetry`. The typed payload has:

| Field | Meaning | Unit |
| --- | --- | --- |
| `n` | Step number | count |
| `next_attempt` | Next attempt number | requests |
| `max_attempts` | Maximum attempts for the step | requests |
| `wait_s` | Backoff before retry | seconds |
| `error_type` | Exception class name | enum-like string |
| `status_code` | HTTP status code when available | HTTP status |

`rate_limited` is recorded when `params.type == "StepRetry"` and `params.payload.status_code == 429`. Source confirms that 429 is retryable, the wire model carries the status code, and the terminal UI renders that status as rate limiting. The new scrubbed fixture captures the canonical JSON-RPC event shape.

`generation_retried` is recorded from the broader `StepRetry` event. The seeded Wire 1.10 fixture shows a retry after `APIEmptyResponseError` with status 500. Kimi also retries connection errors, timeouts, empty responses, and HTTP 500/502/503/504. Those cases should not be promoted to `provider_overloaded` without a more specific discriminator because the payload does not distinguish overload from other transient server failures.

No reliable `usage_cap_approaching` or `usage_capped` wire mapping was found. Kimi may receive cap-shaped messages from a configured model provider, but current wire types do not expose a cap-specific code or structured reset window.

## Token Metering

Kimi emits `tokens_consumed` through `StatusUpdate.token_usage`. Source constructs this event after each LLM step from `result.usage`, includes the message id and plan-mode state, and enriches it with current context token counters when usage is present.

The documented `TokenUsage` fields are:

| Field | Unit | Notes |
| --- | --- | --- |
| `token_usage.input_other` | tokens | Input tokens excluding cache read and cache creation. |
| `token_usage.output` | tokens | Output tokens. |
| `token_usage.input_cache_read` | tokens | Cached input tokens. |
| `token_usage.input_cache_creation` | tokens | Cache-creation input tokens, currently provider-dependent. |
| `context_tokens` | tokens | Current context size when usage is available. |
| `max_context_tokens` | tokens | Maximum context capacity when usage is available. |

`context_usage` is documented as a ratio between 0 and 1. Because Claudine's current unit vocabulary has `percent` but not `ratio`, the frontmatter does not extract that field yet.

## Authentication and Authorization

Kimi wire mode maps OAuth 401 failures during prompt handling to JSON-RPC error code `-32004`, named `AUTH_EXPIRED` in source. The source message asks the user to run `/login`; the seeded fixture uses equivalent copy asking for `kimi login`. This record maps to `auth_invalid` because the existing credentials are expired or invalid for the provider session.

For non-OAuth providers, the same 401 is deliberately returned as generic `CHAT_PROVIDER_ERROR` rather than `AUTH_EXPIRED`, because a Kimi login flow would be misleading for API-key-based providers. That generic branch is left unrecorded until fixtures prove stable provider-specific message text for invalid API keys.

No `permission_denied_read` or `permission_denied_write` record is available. Kimi has `ApprovalRequest` and `ApprovalResponse.response` values `approve`, `approve_for_session`, and `reject`, and hooks can block tool use. Those are approval and policy surfaces, not provider-owned read/write denial events with a normalized resource path.

## Model Resolution

Kimi wire initialization reports `result.server.name`, `result.server.version`, and `result.protocol_version`, so `provider_version` is recorded from the initialize response. It does not report the active model id.

No reliable `model_resolved` or `model_fallback` event was found in the wire stream. Model selection lives in runtime and configuration objects, and provider-specific model fallback or routing is not emitted as a structured wire event in the inspected source.

## Interruption and Recovery

Kimi wire prompt handling returns `result.status == "cancelled"` when `RunCancelled` is raised. The seeded cancellation fixture shows an in-flight turn, a cancel response, a `TurnEnd`, and a terminal prompt result with `status: "cancelled"`. Claudine should treat the terminal prompt result as the provider-native `interrupted` signal, not the presence or absence of `TurnEnd`.

Kimi also returns `result.status == "max_steps_reached"` with a step count when `MaxStepsReached` is raised. This maps to Claudine's `turn_limit_reached` family because it is the provider's configured agent-loop limit rather than a wall-clock timeout or token cap.

Kimi ACP advertises session resume capabilities, and the ACP initialize response includes `SessionResumeCapabilities`. No ACP fixture is committed for this topic, so `session_resumable` remains a gap rather than a frontmatter record.

## Human Input Requests

Wire mode supports `QuestionRequest` as a structured request that blocks until the client returns `QuestionResponse`. It is gated by `capabilities.supports_question: true` during wire initialization; if unsupported, the `AskUserQuestion` tool is hidden from the model's tool list. The source type and docs define question items, option labels/descriptions, and multi-select support.

This maps to Claudine's reserved `human_input_requested` signal. It is recorded because the provider has a first-class native request, and the fixture captures the scrubbed request envelope without user content beyond the signal shape.

## Version Drift

Wire protocol drift matters more than CLI package version for signal detection.

| Area | Since | Current behavior |
| --- | --- | --- |
| `QuestionRequest` | Wire 1.4 | Structured question request/response for human input. |
| `StepRetry` | Wire 1.10 | Retry event with attempt counts, wait seconds, exception class, and optional HTTP status. |
| Initialize capabilities | Wire 1.10 fixture | `supports_question` and `supports_plan_mode` appear in the seeded initialize fixture. |
| Provider version | Kimi package 1.48.0 source | `pyproject.toml` reports `kimi-cli` 1.48.0; initialize fixtures show server versions 1.38.0 and 1.47.0 from earlier captures. |

The inspected source checkout is tag `1.48.0` at commit `2c34efbbc6c7cfe40770623281e87c138ff8eb6c`.

## Quirks and Gaps

Kimi has two protocol layers with similar vocabulary. Wire mode is Kimi-specific JSON-RPC and is the current fixture-backed source for tokens and retries. ACP is a standards-based integration surface, but Kimi's ACP adapter does not forward every wire signal.

`StepRetry` can be both a retry signal and a cause-specific signal. Priority must evaluate status-code-specific records, such as 429 rate limiting, before the generic retry record.

`StatusUpdate` is a partial state update. A `StatusUpdate` with only `plan_mode` or `mcp_status` should not be treated as token metering; the record requires `token_usage.output`.

Kimi's retryable status list includes 500, 502, 503, and 504. Without a stable provider overload code or message, mapping all of those to `provider_overloaded` would over-classify transient failures.

No fixture-backed records were found for usage caps, cap warnings, no funds, invalid static API key, read/write permission denial, model resolution, model fallback, provider overload, retries exhausted, repeated stream error, session taint, or ACP resumability.

## Changelog

Fresh first-run research document; `changes` is empty.

## Sources

- [Kimi Code CLI wire-mode documentation](https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html)
- [Kimi Code CLI README, ACP integration](https://github.com/MoonshotAI/kimi-cli/blob/2c34efbbc6c7cfe40770623281e87c138ff8eb6c/README.md#L43-L56)
- [Wire message type definitions, `2c34efb`](https://github.com/MoonshotAI/kimi-cli/blob/2c34efbbc6c7cfe40770623281e87c138ff8eb6c/src/kimi_cli/wire/types.py#L37-L96)
- [StatusUpdate and Notification definitions, `2c34efb`](https://github.com/MoonshotAI/kimi-cli/blob/2c34efbbc6c7cfe40770623281e87c138ff8eb6c/src/kimi_cli/wire/types.py#L176-L210)
- [Wire event/request union and envelope serialization, `2c34efb`](https://github.com/MoonshotAI/kimi-cli/blob/2c34efbbc6c7cfe40770623281e87c138ff8eb6c/src/kimi_cli/wire/types.py#L516-L661)
- [Wire JSON-RPC error codes and statuses, `2c34efb`](https://github.com/MoonshotAI/kimi-cli/blob/2c34efbbc6c7cfe40770623281e87c138ff8eb6c/src/kimi_cli/wire/jsonrpc.py#L230-L263)
- [Wire prompt handler auth/cancel/max-step outcomes, `2c34efb`](https://github.com/MoonshotAI/kimi-cli/blob/2c34efbbc6c7cfe40770623281e87c138ff8eb6c/src/kimi_cli/wire/server.py#L684-L713)
- [Kimi API error classification, `2c34efb`](https://github.com/MoonshotAI/kimi-cli/blob/2c34efbbc6c7cfe40770623281e87c138ff8eb6c/src/kimi_cli/soul/kimisoul.py#L99-L136)
- [Kimi StatusUpdate emission, `2c34efb`](https://github.com/MoonshotAI/kimi-cli/blob/2c34efbbc6c7cfe40770623281e87c138ff8eb6c/src/kimi_cli/soul/kimisoul.py#L1128-L1148)
- [Kimi retryability and StepRetry emission, `2c34efb`](https://github.com/MoonshotAI/kimi-cli/blob/2c34efbbc6c7cfe40770623281e87c138ff8eb6c/src/kimi_cli/soul/kimisoul.py#L1395-L1522)
- [Kimi ACP initialize capabilities, `2c34efb`](https://github.com/MoonshotAI/kimi-cli/blob/2c34efbbc6c7cfe40770623281e87c138ff8eb6c/src/kimi_cli/acp/server.py#L42-L113)
- [Kimi ACP prompt adapter signal projection, `2c34efb`](https://github.com/MoonshotAI/kimi-cli/blob/2c34efbbc6c7cfe40770623281e87c138ff8eb6c/src/kimi_cli/acp/session.py#L155-L235)
- [Kimi ACP version negotiation, `2c34efb`](https://github.com/MoonshotAI/kimi-cli/blob/2c34efbbc6c7cfe40770623281e87c138ff8eb6c/src/kimi_cli/acp/version.py#L6-L45)
- [Kimi hook events documentation](https://moonshotai.github.io/kimi-cli/en/customization/hooks.html)
- [Kimi hook payload builders, `2c34efb`](https://github.com/MoonshotAI/kimi-cli/blob/2c34efbbc6c7cfe40770623281e87c138ff8eb6c/src/kimi_cli/hooks/events.py#L1-L130)
- [Kimi package metadata, `2c34efb`](https://github.com/MoonshotAI/kimi-cli/blob/2c34efbbc6c7cfe40770623281e87c138ff8eb6c/pyproject.toml#L1-L9)
