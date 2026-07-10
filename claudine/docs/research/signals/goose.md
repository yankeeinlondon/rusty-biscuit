---
$schema: ./_schema.yaml
created: 2026-07-05
last_updated: 2026-07-06
agent: codex
model: default
docs: https://goose-docs.ai/docs/guides/running-tasks/
records:
  - id: stream-no_funds-credits_exhausted
    signal: no_funds
    source: stream
    locator: "type=message; message.content[0].type=systemNotification"
    detection: declarative
    priority: 10
    match_path: message.content[0].notificationType
    match_op: eq
    match_value: creditsExhausted
    distinguish: "This is Goose's explicit credits-exhausted system notification, produced from `ProviderError::CreditsExhausted`. It is stronger than matching provider prose for billing or quota text."
    vocabulary: ["thinkingMessage", "inlineMessage", "creditsExhausted"]
    confidence: source_code
    evidence: ./fixtures/goose/stream-credits-exhausted.jsonl
  - id: stream-auth_invalid-provider-error-text
    signal: auth_invalid
    source: stream
    locator: "type=message"
    detection: declarative
    priority: 20
    match_path: message.content[0].text
    match_op: substring_ci
    match_value: "Ran into this error: Authentication error:"
    distinguish: "Goose catches most provider errors inside the agent loop and emits assistant text. This record keys on Goose's `ProviderError::Authentication` display prefix inside Goose's fixed catch-all wrapper, not provider-specific wording."
    vocabulary: ["Authentication error", "Context length exceeded", "Rate limit exceeded", "Server error", "Network error", "Request failed", "Execution error", "Usage data error", "Unsupported operation", "Endpoint not found (404)", "Credits exhausted", "Provider refused request"]
    confidence: source_code
    evidence: ./fixtures/goose/stream-authentication-error-message.jsonl
  - id: stream-rate_limited-provider-error-text
    signal: rate_limited
    source: stream
    locator: "type=message"
    detection: declarative
    priority: 30
    match_path: message.content[0].text
    match_op: substring_ci
    match_value: "Ran into this error: Rate limit exceeded:"
    distinguish: "This is the typed `ProviderError::RateLimitExceeded` display string after Goose wraps it in assistant text. It differs from usage caps and no-funds because Goose has no usage-cap warning event and credits exhaustion has a dedicated system notification."
    vocabulary: ["Authentication error", "Context length exceeded", "Rate limit exceeded", "Server error", "Network error", "Request failed", "Execution error", "Usage data error", "Unsupported operation", "Endpoint not found (404)", "Credits exhausted", "Provider refused request"]
    confidence: source_code
    evidence: ./fixtures/goose/stream-rate-limit-error-message.jsonl
  - id: stream-provider_overloaded-server-error-text
    signal: provider_overloaded
    source: stream
    locator: "type=message"
    detection: declarative
    priority: 40
    match_path: message.content[0].text
    match_op: substring_ci
    match_value: "Ran into this error: Server error:"
    distinguish: "Goose normalizes provider 5xx-style failures to `ProviderError::ServerError`. It does not expose an overload-specific enum, so this is a coarse transient provider-infrastructure signal."
    vocabulary: ["Authentication error", "Context length exceeded", "Rate limit exceeded", "Server error", "Network error", "Request failed", "Execution error", "Usage data error", "Unsupported operation", "Endpoint not found (404)", "Credits exhausted", "Provider refused request"]
    confidence: source_code
    evidence: ./fixtures/goose/stream-server-error-message.jsonl
  - id: stream-tokens_consumed-complete
    signal: tokens_consumed
    source: stream
    locator: "type=complete"
    detection: declarative
    priority: 60
    match_path: type
    match_op: eq
    match_value: complete
    distinguish: "`complete` is the terminal `stream-json` event and carries accumulated token totals when Goose can read the session. It is not an error or provider status frame."
    vocabulary: ["message", "notification", "error", "complete"]
    confidence: source_code
    evidence: ./fixtures/goose/stream-complete-usage.jsonl
  - id: stream-session_tainted-error-then-complete
    signal: session_tainted
    source: stream
    locator: "type=error followed by type=complete"
    detection: bespoke
    priority: 70
    match_path: type
    match_op: eq
    match_value: error
    distinguish: "In `stream-json`, `handle_agent_error` can emit an `error` frame, break the loop, and the outer code still emits `complete`. Correct outcome classification requires remembering the earlier error."
    vocabulary: ["message", "notification", "error", "complete"]
    confidence: source_code
    evidence: ./fixtures/goose/stream-error-then-complete.jsonl
  - id: sqlite-tokens_consumed-usage_ledger
    signal: tokens_consumed
    source: sqlite
    locator: "SELECT input_tokens, output_tokens, total_tokens, cache_read_tokens, cache_write_tokens, cost FROM usage_ledger WHERE session_id = ?"
    detection: declarative
    priority: 10
    match_path: total_tokens
    match_op: exists
    distinguish: "The `usage_ledger` table is Goose's durable per-message usage ledger. It differs from the stream `complete` event because it can include model, cache-token, cost, and compaction fields for historical rows."
    vocabulary: ["provider_reported", "estimated", "carried_forward"]
    confidence: source_code
    evidence: ./fixtures/goose/sqlite-usage-ledger-row.json
extractions:
  - record: stream-no_funds-credits_exhausted
    field: message
    path: message.content[0].msg
  - record: stream-no_funds-credits_exhausted
    field: top_up_url
    path: message.content[0].data.top_up_url
  - record: stream-auth_invalid-provider-error-text
    field: message
    path: message.content[0].text
  - record: stream-rate_limited-provider-error-text
    field: message
    path: message.content[0].text
  - record: stream-provider_overloaded-server-error-text
    field: message
    path: message.content[0].text
  - record: stream-tokens_consumed-complete
    field: total
    path: total_tokens
    unit: tokens
  - record: stream-tokens_consumed-complete
    field: input
    path: input_tokens
    unit: tokens
  - record: stream-tokens_consumed-complete
    field: output
    path: output_tokens
    unit: tokens
  - record: sqlite-tokens_consumed-usage_ledger
    field: total
    path: total_tokens
    unit: tokens
  - record: sqlite-tokens_consumed-usage_ledger
    field: input
    path: input_tokens
    unit: tokens
  - record: sqlite-tokens_consumed-usage_ledger
    field: output
    path: output_tokens
    unit: tokens
  - record: sqlite-tokens_consumed-usage_ledger
    field: cache_read
    path: cache_read_tokens
    unit: tokens
  - record: sqlite-tokens_consumed-usage_ledger
    field: cache_write
    path: cache_write_tokens
    unit: tokens
  - record: sqlite-tokens_consumed-usage_ledger
    field: cost
    path: cost
    unit: usd
  - record: sqlite-tokens_consumed-usage_ledger
    field: timestamp
    path: created_timestamp
    unit: unix_seconds
    zone: utc
    notes: "Inserted with SQLite `strftime('%s','now')`; imported legacy message timestamps elsewhere may use milliseconds, but usage ledger rows are seconds."
bespoke_rationale:
  - "stream-session_tainted-error-then-complete: Goose can emit a stream `error` frame and then still emit the terminal `complete` frame after the loop. Detecting taint requires cross-record state, not a single-payload match."
gaps:
  - "All goose fixtures are source_shape provenance (serializer-faithful shapes derived verbatim from block/goose source at 65eed515559af22dde2ba965335e331422f60c26, not live captures) per the 2026-07-06 provenance ruling; live captures should replace them via harvest when goose runs under claudine."
  - "retries_exhausted has NO record: RetryManager's 'Maximum retry attempts (...) exceeded.' message never reaches any output surface at commit 65eed515 (the MaxAttemptsReached path returns without yielding an AgentEvent::Message, and emit_stream_event only fires on yielded messages) — the signal is wire-invisible; re-check on future goose versions."
  - "Goose docs document `--output-format json|stream-json` but do not publish a complete JSON schema for `StreamEvent`, `MessageContent`, or session SQLite rows; source is the authority for field names."
  - "Goose has typed `ProviderError::RateLimitExceeded` and optional `retry_delay`, but the CLI stream path wraps most provider errors into assistant text and does not expose `retry_delay` in `stream-json` frames."
  - "Goose has no native `usage_cap_approaching` or `usage_capped` signal distinct from rate limits or credits exhausted in the inspected code."
  - "Goose has no overload-specific enum; `provider_overloaded` is mapped conservatively from `ProviderError::ServerError` and should be treated as coarse."
  - "Goose's model and provider selections are stored in session rows (`provider_name`, `model_config_json`) and config, but the CLI stream has no stable `model_resolved`, `model_fallback`, or `provider_version` event."
  - "Ctrl+C cancels the in-process token and may add tool-response repair messages, but there is no stable stream `interrupted` terminal event."
  - "Action-required tool confirmations and elicitations are intercepted by the CLI before normal stream emission; non-interactive elicitation returns an error. This is not yet a clean `human_input_requested` stream record."
changes:
  - "2026-07-06: rewrote the usage_ledger total_tokens numeric-regex presence proxy to `match_op: exists`."
requires_claudine_update: true
reason: "Goose has source-backed stream and SQLite signal records that Claudine can codegen for token usage, no-funds, provider error classes, recipe retry exhaustion, and stream taint detection."
---

# Goose CLI Signal Detection

## Overview

Goose CLI exposes operational signals through `goose run --output-format stream-json`, `goose run --output-format json`, persisted SQLite session storage, and diagnostic text. The `stream-json` surface is documented for automation, but its exact enum variants and nested message shapes are defined in Rust source rather than in a published schema.

The inspected upstream revision is `65eed515559af22dde2ba965335e331422f60c26` from `block/goose` on 2026-07-05. The latest release tags visible through GitHub were `v1.41.0` and `v2.0.0-rc-*`; this research uses commit permalinks because the inspected default branch is not identical to a stable release tag.

## Signal Surfaces

### Stream Events

`goose run --output-format stream-json` emits newline-delimited JSON objects to stdout. Source defines a tagged `StreamEvent` enum with `message`, `notification`, `error`, and `complete` variants. The terminal `complete` variant carries `total_tokens`, `input_tokens`, and `output_tokens` from the session manager when available.

`message` events embed Goose's provider-agnostic conversation `Message` shape. `MessageContent` is tagged with camelCase `type` values; `systemNotification` includes `notificationType`, `msg`, and optional `data`.

### JSON Output

`goose run --output-format json` prints one pretty JSON object after execution. Its `metadata` object has `total_tokens`, `input_tokens`, `output_tokens`, and `status: "completed"`. The current signals schema has no separate `json` source, so the structured frontmatter records target `stream` and `sqlite` surfaces instead.

### SQLite Session Store

Goose stores sessions in `sessions.db` under its data directory. Source creates `sessions`, `messages`, and `usage_ledger` tables. `usage_ledger` rows contain per-message token and cost fields: `input_tokens`, `output_tokens`, `total_tokens`, `cache_read_tokens`, `cache_write_tokens`, `cost`, `cost_source`, and `created_timestamp`.

This store is first-party and durable, but it is an internal application database rather than a documented interoperability contract.

### stderr Diagnostics

Non-JSON/text mode renders errors to stderr and status/progress to the terminal. In `stream-json`, `handle_agent_error` emits a structured `error` event to stdout and suppresses the plain stderr error line. The research records use the structured stream event where possible and do not promote free-form stderr diagnostics.

### ACP

Goose contains both ACP client/provider code and server code. As an ACP provider wrapper, it maps ACP prompt response `usage` into Goose `ProviderUsage`, and observes ACP `session/update` `usage_update` frames for context size. This is relevant to Goose as a host of other ACP agents, not the normal Goose CLI provider stream, so no ACP frontmatter record is added in this first pass.

## Usage and Rate Limits

### `tokens_consumed`

The strongest CLI stream token signal is `type=complete`. The source `StreamEvent::Complete` variant includes `total_tokens`, `input_tokens`, and `output_tokens`, and `process_agent_response` fills them from `session.accumulated_usage` with fallback to the current session `usage`.

The durable SQLite surface is `usage_ledger`. Source creates the table with token, cache-token, cost, `cost_source`, and `is_compaction` columns, and `record_usage_metrics` inserts a row with `created_timestamp` from SQLite `strftime('%s','now')`. That timestamp is Unix seconds in UTC.

### `rate_limited`

Goose's normalized provider error enum includes `ProviderError::RateLimitExceeded { details, retry_delay }`, whose display string starts with `Rate limit exceeded:`. In the normal agent loop, most provider errors are caught and yielded as assistant text with Goose's wrapper prefix `Ran into this error: ...`. The CLI stream does not preserve the `retry_delay` field in that assistant text, so the frontmatter record extracts only the message.

### Usage Caps

No source-inspected Goose event distinguishes `usage_cap_approaching` or `usage_capped` from rate limiting or credits exhaustion. Provider-specific quota prose may appear inside error details, but that is not a Goose-owned discriminator.

## Billing and Funds

### `no_funds`

Goose has a typed `ProviderError::CreditsExhausted { details, top_up_url }`. The agent loop converts it into a `MessageContent::SystemNotification` with `SystemNotificationType::CreditsExhausted` and data containing `top_up_url`. In `stream-json`, this appears as a `message` event whose first content item has `type: "systemNotification"` and `notificationType: "creditsExhausted"`.

This is the most stable billing/funds signal in Goose because it is independent of provider-specific wording.

## Authentication and Authorization

### `auth_invalid`

`ProviderError::Authentication` is a typed enum variant with display prefix `Authentication error:`. Goose's generic provider-error branch wraps it in assistant text. The frontmatter record matches Goose's fixed prefix sequence, `Ran into this error: Authentication error:`, rather than matching raw provider prose such as "invalid API key".

### Permission Denials

Goose has a policy and permission-confirmation model with `Permission::{AlwaysAllow, AllowOnce, Cancel, DenyOnce, AlwaysDeny}`. Tool confirmations are intercepted before ordinary stream emission; in non-interactive headless mode Approve/SmartApprove is treated as invalid configuration, while Auto can auto-allow. There is no stable read/write-denied stream signal that maps cleanly to `permission_denied_read` or `permission_denied_write`.

## Provider Availability

### `provider_overloaded`

The source enum has `ProviderError::ServerError(String)` with display prefix `Server error:`. Goose does not expose an overload-specific discriminator. The record maps this to `provider_overloaded` as a coarse transient provider-infrastructure signal, not as proof of an explicit overloaded status.

Network errors are deliberately not mapped to provider overload: Goose has a separate `ProviderError::NetworkError` branch, and that points to connectivity rather than provider saturation.

## Retries and Taint

### `retries_exhausted`

Goose recipe retry logic lives in `RetryManager`. When success checks keep failing and the configured maximum is reached, it appends `Maximum retry attempts (...) exceeded.` to the in-loop conversation — but source tracing at commit 65eed515 shows that message **never reaches any output surface**: on `MaxAttemptsReached` the agent wrapper returns without yielding an `AgentEvent::Message`, and `emit_stream_event` only fires on yielded messages. The retry result enum vocabulary is `Skipped`, `MaxAttemptsReached`, `SuccessChecksPassed`, and `Retried`.

No detection record is emitted for this signal: a stream fixture for it would be wire-impossible, and there is nothing observable to match. Recipe retry exhaustion is currently invisible on the wire; if a future goose version surfaces it, the signal is recipe-wrapper exhaustion, not a provider rate-limit retry.

### `session_tainted`

In `stream-json`, `handle_agent_error` emits an `error` event for an unhandled agent error. After the stream loop breaks, the surrounding `process_agent_response` code still emits a terminal `complete` event. This means a stream can contain an `error` event followed by a `complete` event; because `StreamEvent::Complete` serializes `total_tokens` unconditionally, the trailing frame is at minimum `{"type":"complete","total_tokens":null}` (never a bare `{"type":"complete"}`). Claudine must remember the earlier error and treat the session outcome as failed.

This requires bespoke detection because no single payload proves the contradiction.

## Model Resolution

Goose sessions persist `provider_name` and `model_config_json`, and provider implementations can resolve model configuration internally. The CLI `stream-json` output has no first-class `model_resolved`, `model_fallback`, or `provider_version` event in the inspected code. `goose --version` is a CLI command surface, not a session stream signal.

## Interruption and Recovery

Goose listens for Ctrl+C with `tokio::signal::ctrl_c()` and cancels a `CancellationToken`. If a tool call is pending, `handle_interrupted_messages` can add synthetic tool responses such as "Interrupted by the user to make a correction"; otherwise interruption is internal state and terminal prompt behavior. There is no stable `interrupted` stream event.

Goose can resume sessions via CLI options and session manager APIs, but this is a command capability rather than an emitted `session_resumable` signal in the inspected stream.

## Human Input

Goose represents tool confirmations and elicitation requests as `MessageContent::ActionRequired`. The CLI intercepts these messages before emitting ordinary stream events: interactive sessions prompt the user, non-interactive elicitation errors, and non-interactive tool confirmation is either auto-allowed in Auto mode or rejected as invalid configuration in Approve/SmartApprove modes. This is researchable for future `human_input_requested`, but current `stream-json` does not provide a clean emitted event.

## Version Drift

No version drift row is added because this first pass inspected the current default-branch commit only. Release tags visible through GitHub include `v1.41.0`, `v1.40.0`, and `v2.0.0-rc-*`; the source tree inspected here is pinned by commit hash rather than by a matching release tag.

## Quirks and Gaps

- Source code is the authority for stream event names and message content fields; official docs describe the existence of JSON output modes but not the full schema.
- `ProviderError::RateLimitExceeded` carries `retry_delay`, but the CLI stream record cannot extract it after Goose converts the error to assistant text.
- `ProviderError::ServerError` is only a coarse overload proxy; Goose does not distinguish "overloaded" from other server failures.
- Stream `complete` is not a success marker by itself. An earlier `error` frame taints the session even if `complete` follows.
- SQLite `usage_ledger` is durable and first-party but internal. Consumers should expect schema drift across Goose releases.
- No local Goose binary was available for live fixture capture in this workspace.

## Changelog

Initial signal research document for Goose CLI.

## Sources

- [Goose running tasks documentation](https://goose-docs.ai/docs/guides/running-tasks/)
- [Goose CLI command documentation](https://goose-docs.ai/docs/guides/goose-cli-commands/)
- [`StreamEvent`, JSON metadata, and stream completion source](https://github.com/block/goose/blob/65eed515559af22dde2ba965335e331422f60c26/crates/goose-cli/src/session/mod.rs#L63-L100)
- [`process_agent_response` stream-json emission and completion source](https://github.com/block/goose/blob/65eed515559af22dde2ba965335e331422f60c26/crates/goose-cli/src/session/mod.rs#L1149-L1430)
- [`handle_agent_error` stream error emission source](https://github.com/block/goose/blob/65eed515559af22dde2ba965335e331422f60c26/crates/goose-cli/src/session/mod.rs#L2186-L2194)
- [`MessageContent`, `ActionRequiredData`, and `SystemNotificationType` source](https://github.com/block/goose/blob/65eed515559af22dde2ba965335e331422f60c26/crates/goose-provider-types/src/conversation/message.rs#L200-L283)
- [`ProviderError` enum and telemetry vocabulary source](https://github.com/block/goose/blob/65eed515559af22dde2ba965335e331422f60c26/crates/goose-provider-types/src/errors.rs#L7-L74)
- [`CreditsExhausted` conversion to system notification source](https://github.com/block/goose/blob/65eed515559af22dde2ba965335e331422f60c26/crates/goose/src/agents/agent.rs#L2458-L2479)
- [`RetryManager` exhaustion behavior source](https://github.com/block/goose/blob/65eed515559af22dde2ba965335e331422f60c26/crates/goose/src/agents/retry.rs#L1-L136)
- [`sessions` and `usage_ledger` table creation source](https://github.com/block/goose/blob/65eed515559af22dde2ba965335e331422f60c26/crates/goose/src/session/session_manager.rs#L936-L1008)
- [`record_usage_metrics` SQLite write source](https://github.com/block/goose/blob/65eed515559af22dde2ba965335e331422f60c26/crates/goose/src/session/session_manager.rs#L2091-L2176)
- [ACP usage mapping source](https://github.com/block/goose/blob/65eed515559af22dde2ba965335e331422f60c26/crates/goose/src/acp/provider.rs#L638-L650)
