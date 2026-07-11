---
$schema: ./_schema.yaml
created: 2026-07-05
last_updated: 2026-07-06
agent: codex
model: default
docs: https://geminicli.com/docs/cli/headless/
records:
  - id: stream-model_resolved-init
    signal: model_resolved
    source: stream
    locator: "type=init"
    detection: declarative
    priority: 10
    match_path: type
    match_op: eq
    match_value: init
    distinguish: "Gemini CLI emits `init` once at the start of `--output-format stream-json`; it carries the session id and the configured model selector. It is metadata, not a completion or token event."
    vocabulary: ["init"]
    since: "0.1.20"
    confidence: source_code
    evidence: ./fixtures/gemini/init-model-resolved.jsonl
    notes: "The model can be an alias/router such as `auto-gemini-3`; per-model usage in the terminal result is needed to learn which concrete models served the run."
  - id: stream-tokens_consumed-result-stats
    signal: tokens_consumed
    source: stream
    locator: "type=result"
    detection: declarative
    priority: 30
    match_path: stats.total_tokens
    match_op: exists
    distinguish: "`result.stats` is the terminal aggregate usage envelope for stream-json. It differs from ordinary `message` chunks and from `error` warnings because it is emitted at final outcome time."
    vocabulary: ["result"]
    since: "0.1.20"
    confidence: source_code
    evidence: ./fixtures/gemini/result-usage-stats.jsonl
  - id: stream-turn_limit_reached-result-error-type
    signal: turn_limit_reached
    source: stream
    locator: "type=result,status=error"
    detection: declarative
    priority: 20
    match_path: error.type
    match_op: in
    match_values: ["FatalTurnLimitedError", "FatalTurnLimited"]
    distinguish: "This is Gemini CLI's configured max-session-turns failure on the terminal `result` event. It is distinct from nonfatal `error` events such as loop warnings because the result status is `error` and the nested error type names a fatal turn-limit condition."
    vocabulary: ["FatalTurnLimitedError", "FatalTurnLimited"]
    confidence: source_code
    evidence: ./fixtures/gemini/result-turn-limit.jsonl
extractions:
  - record: stream-model_resolved-init
    field: resolved
    path: model
  - record: stream-model_resolved-init
    field: session_id
    path: session_id
  - record: stream-tokens_consumed-result-stats
    field: total
    path: stats.total_tokens
    unit: tokens
  - record: stream-tokens_consumed-result-stats
    field: input
    path: stats.input_tokens
    unit: tokens
  - record: stream-tokens_consumed-result-stats
    field: output
    path: stats.output_tokens
    unit: tokens
  - record: stream-tokens_consumed-result-stats
    field: cache_read
    path: stats.cached
    unit: tokens
  - record: stream-tokens_consumed-result-stats
    field: uncached_input
    path: stats.input
    unit: tokens
  - record: stream-turn_limit_reached-result-error-type
    field: message
    path: error.message
bespoke_rationale: []
gaps:
  - "Gemini CLI `stream-json` emits `init.session_id`, and the CLI supports `--resume`, but `init` is already used for `model_resolved`; recording `session_resumable` on the same single-payload discriminator would overlap with `model_resolved` under first-match-wins evaluation. Claudine may need multi-signal emission from one payload before this can become a generated declarative record."
  - "Current source has internal `GeminiEventType.Retry`, but the noninteractive stream-json projection does not emit a retry event. No `generation_retried` stream record is available."
  - "Current source has internal model routing, fallback, and availability services. Public stream-json exposes the configured model in `init.model` and aggregate `result.stats.models`, but not a structured fallback event with failed model, fallback model, and reason. No reliable `model_fallback` record is available."
  - "Current source can surface rate-limit-like conditions as API errors and, in the ADK agent-session path, nonfatal agent errors with `status === RESOURCE_EXHAUSTED` become stream-json `error` events with `severity: error`; the status itself is not emitted. No committed rate-limit or usage-cap fixture exists, so `rate_limited`, `usage_cap_approaching`, and `usage_capped` remain unrecorded."
  - "Authentication failures are validated before the main noninteractive loop. JSON output is handled specially, but `stream-json` auth validation exits with code 53 after logging diagnostics rather than emitting a structured stream event. No fixture-backed `auth_invalid` record is available."
  - "Gemini CLI exposes native ACP. ACP session updates include usage, model usage, available commands, and auth-required failures in source, but no committed Gemini ACP fixture exists in the signal corpus. ACP records should be added only after scrubbed session/update fixtures are captured."
  - "The stream-json formatter emits ISO 8601 UTC timestamps, but the current records do not extract timestamps because the normalized payload fields for these signals do not require them."
changes:
  - "2026-07-06: rewrote the result stats.total_tokens numeric-regex presence proxy to `match_op: exists`."
  - "2026-07-06: renamed the model_resolved extraction field `model` → `resolved` to match the canonical SignalEvent::ModelResolved payload field."
requires_claudine_update: true
reason: "Gemini signal detection is codegen-wired and this research adds fixture-backed stream records for Gemini model resolution, token metering, and turn-limit failures while documenting several native but currently unrecorded signal gaps."
---

# Gemini CLI Signal Detection

## Overview

Gemini CLI's primary wrapper-grade signal surface is headless `--output-format stream-json`, which writes newline-delimited JSON events to stdout. The official headless documentation names the public event vocabulary as `init`, `message`, `tool_use`, `tool_result`, `error`, and `result`; current source defines the same public stream event union in `packages/core/src/output/types.ts` and emits it from the noninteractive CLI projection.

The stream is stable enough for session metadata, terminal token usage, nonfatal warnings, tool envelopes, terminal errors, and max-turn failures. It is not a lossless projection of Gemini CLI internals. Internal turn events include retry, user-cancelled, model-info, invalid-stream, agent-blocked, context-window, and routing/fallback concepts, but most of those are folded into generic stream-json `error` or terminal `result` events, or are not emitted at all.

## Signal Surfaces

### Stream Events

`gemini -p ... --output-format stream-json` emits JSONL on stdout. The source-defined public event vocabulary is:

| Event | Shape | Signal relevance |
| --- | --- | --- |
| `init` | `timestamp`, `session_id`, `model` | Model selector and resumable session handle. |
| `message` | `timestamp`, `role`, `content`, optional `delta` | Text streaming; no normalized operational signal in this topic. |
| `tool_use` | `timestamp`, `tool_name`, `tool_id`, `parameters` | Tool request envelope; not mapped to the current signal taxonomy. |
| `tool_result` | `timestamp`, `tool_id`, `status`, optional `output`, optional `error` | Tool result envelope; future read/write permission mapping may use this if provider errors become path-typed. |
| `error` | `timestamp`, `severity`, `message` | Nonfatal warnings and system errors. Current source emits loop warnings, agent-blocked warnings, invalid-stream errors, and ADK agent-session resource-exhausted severity. |
| `result` | `timestamp`, `status`, optional `error`, optional `stats` | Terminal outcome; carries aggregate token usage, per-model token usage, turn-limit errors, cancellation errors, and other fatal errors. |

The output contract is first-class documentation, not just diagnostics. Timestamps are ISO 8601 strings with embedded UTC offsets in observed fixtures (`Z`).

### Internal Turn Events

The core `ServerGeminiStreamEvent` union is richer than the public stream-json vocabulary. It includes `content`, `tool_call_request`, `tool_call_response`, `tool_call_confirmation`, `user_cancelled`, `error`, `chat_compressed`, `thought`, `max_session_turns`, `finished`, `loop_detected`, `citation`, `retry`, `context_window_will_overflow`, `invalid_stream`, `model_info`, `agent_execution_stopped`, and `agent_execution_blocked`.

These are source-code-confirmed, but they are not the direct Claudine wrapper surface unless Gemini CLI starts exposing them. In the legacy noninteractive path, `retry` is consumed and never projected; `loop_detected` becomes a stream-json `error` with `severity: warning`; `max_session_turns` becomes a stream-json `error` warning during the loop or a terminal `result` error when the configured turn cap trips; and generic internal `error` is thrown to the top-level error handler, which emits a terminal `result` with `status: error`.

### ACP Streams

Gemini CLI has native ACP support through `gemini --acp`. Source inspection shows session updates for command availability, model selection, usage totals, and per-model usage. ACP authentication failures can produce auth-required request errors before a session starts.

This document does not add ACP frontmatter records because the current Gemini signal fixture corpus does not include scrubbed ACP `session/update` payloads. ACP should be researched as a separate source once fixtures are captured; its usage summaries are likely valuable because they preserve per-turn model usage in a protocol contract rather than only in the final stream-json stats.

### Session Logs, App Logs, and stderr Diagnostics

Gemini CLI persists local session data under `~/.gemini/tmp/{project_id}/chats/` and can emit diagnostics through stderr, debug logging, and optional activity logging. Those surfaces are useful for retrospective debugging but are not the primary machine stream for Claudine's direct wrapper. This signal document therefore treats stream-json as the authoritative detection source and records log/stderr gaps rather than matching free-form diagnostics.

## Usage and Rate Limits

No fixture-backed `usage_cap_approaching`, `usage_capped`, or `rate_limited` record is available from the public stream-json surface.

Current source does contain rate-limit-shaped behavior. In the ADK agent-session noninteractive path, nonfatal agent errors with `event.status === 'RESOURCE_EXHAUSTED'` are projected as stream-json `error` events with `severity: 'error'`, but the emitted payload contains only `severity` and a stripped message. The status discriminator is lost, so a generated declarative rule would have to match arbitrary message text unless a fixture proves stable provider copy.

Fatal API errors flow through `handleError`, which emits a terminal `result` with `status: 'error'`, `error.type` from `getErrorType(error)`, a formatted message from `parseAndFormatApiError`, and zero-duration stats. This is a useful future source for `rate_limited`, `usage_capped`, `provider_overloaded`, and `no_funds`, but no committed Gemini fixture currently proves the exact terminal shape for those failures.

## Token Metering

`tokens_consumed` is emitted on the terminal `result` event when `stats.total_tokens` is present. The stream-json formatter converts telemetry `SessionMetrics` into aggregate `StreamStats` and per-model `ModelStreamStats`.

| Field | Unit | Notes |
| --- | --- | --- |
| `stats.total_tokens` | tokens | Sum of per-model total token counts. |
| `stats.input_tokens` | tokens | Sum of per-model prompt token counts. |
| `stats.output_tokens` | tokens | Sum of per-model candidate/output token counts. |
| `stats.cached` | tokens | Cache-read token count. |
| `stats.input` | tokens | Non-cached input token count in Gemini's stream-json naming. |
| `stats.duration_ms` | milliseconds | Session duration; not extracted into `tokens_consumed` because it is not a token quantity. |
| `stats.tool_calls` | requests | Tool-call count; not extracted into `tokens_consumed` in the current record. |
| `stats.models.<model>.total_tokens` | tokens | Per-model total token count; dynamic model keys cannot be addressed by the restricted declarative extraction path. |

The seeded live fixture with `stats.models` shows a session configured as `auto-gemini-3` that consumed tokens on both `gemini-2.5-flash-lite` and `gemini-3-flash-preview`. That proves per-model metering, but not a structured model-fallback event.

## Authentication and Authorization

No `auth_invalid` record is fixture-backed for stream-json. Noninteractive auth validation runs before the main stream loop. If no auth type is configured, Gemini CLI asks for a configured auth method or one of the environment variables `GEMINI_API_KEY`, `GOOGLE_GENAI_USE_VERTEXAI`, or `GOOGLE_GENAI_USE_GCA`. For JSON output, auth validation delegates to the common error handler; for stream-json, the validation path logs an error and exits with the fatal-authentication exit code instead of emitting a structured stream-json event.

No reliable `permission_denied_read` or `permission_denied_write` stream record was found. Tool errors can appear in `tool_result.error.type` and ADK agent-session tests use `permission_denied` as tool response data, but the public stream payload does not distinguish read versus write denial with a stable path field in the current fixture corpus.

## Model Resolution and Fallback

`model_resolved` is recorded from `init.model`. This is the configured session model or alias returned by `config.getModel()`, not necessarily the exact backend model used for every request. For aliases and routers such as `auto-gemini-3`, `result.stats.models` is the better evidence of actual model usage.

Model fallback is source-code-confirmed internally but not publicly structured in stream-json. The routing and availability code can mark models unavailable for `quota` or `capacity`, build fallback policy context, and choose alternate models. The noninteractive stream-json result can reveal multiple model keys in `stats.models`, but it does not identify a failed model, fallback model, or fallback reason. That makes `model_fallback` unsuitable for a declarative record today.

`provider_version` is not emitted in stream-json. It is available through CLI/package metadata (`gemini --version` or package files), but not as a provider stream event.

## Generation Health

Internal `GeminiEventType.Retry` exists and is yielded when the low-level response stream emits `retry`, but the legacy noninteractive projection skips it. The ADK event translator also returns no agent event for retry. There is therefore no fixture-backed `generation_retried` public stream record.

Loop detection is emitted as stream-json `error` with `severity: warning` and message `Loop detected, stopping execution` in current source. Claudine's taxonomy does not have a provider-native loop-warning signal distinct from the internal `runaway_repetition` guard, so this document does not map it. Treating a provider warning as Claudine's internal runaway guard would be misleading unless Claudine adds a provider loop-warning signal or a bespoke mapping.

Invalid stream is projected as a stream-json `error` with `severity: error` and a fixed message, then the terminal `result` has `status: error` without a nested error object in the legacy path. It is not enough evidence for `repeated_stream_error`, which requires repeated stream errors within a correlation window.

## Interruption and Recovery

Gemini CLI supports resuming sessions with `--resume`, and stream-json `init.session_id` provides the handle. This document leaves `session_resumable` as a gap rather than a frontmatter record because the same `init` payload is already the `model_resolved` record and the current generated detection model is first-match-wins.

Cancellation is source-confirmed. Ctrl+C or abort paths produce `FatalCancellationError` terminal results in stream-json when the error handler runs. No committed cancellation fixture exists in the Gemini corpus, so no `interrupted` record is emitted in frontmatter.

## Version Drift

The seeded Gemini fixtures include `FatalTurnLimitedError` and `FatalTurnLimited` spellings for the same max-turn condition. Current source names the fatal error class `FatalTurnLimitedError`; Claudine's existing compatibility parser and seeded fixture preserve the shorter historical spelling. The `turn_limit_reached` record therefore uses `match_op: in` over both values.

The current upstream main checkout reports package version `0.51.0-nightly.20260625.g3fbf93e26` at commit `f7af4e5180cf92eea8190e383fd5daeeb2578c2d`. Existing local research notes mention installed `gemini` version `0.46.0`, so maintainers should expect headless stream behavior to drift quickly. The event vocabulary itself is stable across the official docs and current source.

## Quirks and Gaps

The `init.model` value is a configured selector. Do not assume it is the concrete billed model when the selector is an alias or router.

Dynamic `stats.models` keys cannot be expressed with Claudine's restricted extraction path because it intentionally disallows wildcards and filters. Per-model token usage may require bespoke extraction if Claudine wants model-level metering from Gemini stream-json.

Stream-json error events intentionally omit some internal discriminators. In particular, ADK agent-session `RESOURCE_EXHAUSTED` severity is collapsed to `severity: error` plus message text, and retry events are dropped.

No committed fixtures prove Gemini stream-json shapes for usage caps, rate-limit failures, exhausted retries, provider overload, no funds, invalid auth, read/write permission denial, native interruption, provider version, model fallback, human input requests, or ACP session updates.

## Changelog

Fresh first-run research document; `changes` is empty.

## Sources

- [Gemini CLI headless mode reference](https://geminicli.com/docs/cli/headless/)
- [Gemini CLI configuration reference](https://geminicli.com/docs/reference/configuration/)
- [Gemini CLI stream-json event types, `f7af4e5`](https://github.com/google-gemini/gemini-cli/blob/f7af4e5180cf92eea8190e383fd5daeeb2578c2d/packages/core/src/output/types.ts#L29-L117)
- [Gemini CLI stream-json formatter and stats conversion, `f7af4e5`](https://github.com/google-gemini/gemini-cli/blob/f7af4e5180cf92eea8190e383fd5daeeb2578c2d/packages/core/src/output/stream-json-formatter.ts#L14-L87)
- [Gemini CLI internal turn event union, `f7af4e5`](https://github.com/google-gemini/gemini-cli/blob/f7af4e5180cf92eea8190e383fd5daeeb2578c2d/packages/core/src/core/turn.ts#L55-L238)
- [Gemini CLI turn stream projection and retry/error handling, `f7af4e5`](https://github.com/google-gemini/gemini-cli/blob/f7af4e5180cf92eea8190e383fd5daeeb2578c2d/packages/core/src/core/turn.ts#L281-L443)
- [Gemini CLI noninteractive stream-json projection, `f7af4e5`](https://github.com/google-gemini/gemini-cli/blob/f7af4e5180cf92eea8190e383fd5daeeb2578c2d/packages/cli/src/nonInteractiveCli.ts#L246-L590)
- [Gemini CLI stream-json terminal error helpers, `f7af4e5`](https://github.com/google-gemini/gemini-cli/blob/f7af4e5180cf92eea8190e383fd5daeeb2578c2d/packages/cli/src/utils/errors.ts#L66-L244)
- [Gemini CLI noninteractive auth validation, `f7af4e5`](https://github.com/google-gemini/gemini-cli/blob/f7af4e5180cf92eea8190e383fd5daeeb2578c2d/packages/cli/src/validateNonInterActiveAuth.ts#L20-L64)
- [Gemini CLI ADK noninteractive stream-json projection, `f7af4e5`](https://github.com/google-gemini/gemini-cli/blob/f7af4e5180cf92eea8190e383fd5daeeb2578c2d/packages/cli/src/nonInteractiveCliAgentSession.ts#L244-L620)
- [Gemini CLI ACP session usage projection, `f7af4e5`](https://github.com/google-gemini/gemini-cli/blob/f7af4e5180cf92eea8190e383fd5daeeb2578c2d/packages/cli/src/acp/acpSession.ts#L354-L618)
- [Gemini CLI model availability and fallback policy helpers, `f7af4e5`](https://github.com/google-gemini/gemini-cli/blob/f7af4e5180cf92eea8190e383fd5daeeb2578c2d/packages/core/src/availability/policyHelpers.ts#L202-L338)
- [Claudine Gemini stream fixtures](fixtures/gemini/)
