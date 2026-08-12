---
$schema: ./_schema.yaml
created: 2026-07-05
last_updated: 2026-07-06
agent: codex
model: default
docs: https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/
records:
  - id: stream-model_resolved-system-init
    signal: model_resolved
    source: stream
    locator: "type=system,subtype=init"
    detection: declarative
    priority: 10
    match_path: subtype
    match_op: eq
    match_value: init
    distinguish: "Current Qwen Code `stream-json` emits a first system initialization record with the selected model, session id, tools, permission mode, and CLI version. This is metadata, not a completion result."
    vocabulary: ["init"]
    since: "0.19.6"
    confidence: source_code
    evidence: ./fixtures/qwen/system-init-model-version.jsonl
  - id: stream-model_resolved-system-session_start
    signal: model_resolved
    source: stream
    locator: "type=system,subtype=session_start"
    detection: declarative
    priority: 20
    match_path: subtype
    match_op: eq
    match_value: session_start
    distinguish: "This covers documented/observed session-start system records that carry `model` at top level. It is distinct from current headless `subtype=init` and from dual-output capability handshakes where session metadata is nested under `data`."
    vocabulary: ["session_start"]
    confidence: observed
    evidence: ./fixtures/qwen/system-session-start-model.jsonl
  - id: stream-model_resolved-init-legacy
    signal: model_resolved
    source: stream
    locator: "type=init"
    detection: declarative
    priority: 30
    match_path: type
    match_op: eq
    match_value: init
    distinguish: "This compatibility spelling appears in the seeded Qwen corpus and Claudine's existing Qwen parser. Current upstream `v0.19.6` headless output uses `system/subtype=init` instead."
    vocabulary: ["init"]
    confidence: observed
    evidence: ./fixtures/qwen/init-model.jsonl
  - id: stream-rate_limited-error-type
    signal: rate_limited
    source: stream
    locator: "type=error"
    detection: declarative
    priority: 40
    match_path: error.type
    match_op: eq
    match_value: rate_limit
    distinguish: "This is the observed Qwen error envelope with a structured `rate_limit` discriminator. Current source has rate-limit classifiers, but ordinary headless failures more often surface as terminal `result` errors or stderr diagnostics."
    vocabulary: ["rate_limit"]
    confidence: observed
    evidence: ./fixtures/qwen/error-rate-limit.jsonl
  - id: stream-auth_invalid-result-missing-api-key
    signal: auth_invalid
    source: stream
    locator: "type=result,is_error=true"
    detection: declarative
    priority: 50
    match_path: error.message
    match_op: substring_ci
    match_value: "Missing API key"
    distinguish: "Qwen validates non-interactive auth before the main loop. In `stream-json` mode missing or invalid preflight auth is emitted as a terminal `result` error, not as a separate `error` event."
    vocabulary: ["error_during_execution"]
    since: "0.19.6"
    confidence: source_code
    evidence: ./fixtures/qwen/result-auth-missing-api-key.jsonl
  - id: stream-tokens_consumed-result-usage
    signal: tokens_consumed
    source: stream
    locator: "type=result"
    detection: declarative
    priority: 60
    match_path: usage.input_tokens
    match_op: exists
    distinguish: "`result.usage` is the terminal aggregate token envelope for current headless `stream-json`. It differs from assistant-message `message.usage`, which is per assistant message, and from compatibility `summary.token_usage`."
    vocabulary: ["result", "success", "error_during_execution"]
    since: "0.19.6"
    confidence: source_code
    evidence: ./fixtures/qwen/result-usage.jsonl
  - id: stream-tokens_consumed-summary-token_usage
    signal: tokens_consumed
    source: stream
    locator: "type=summary"
    detection: declarative
    priority: 70
    match_path: token_usage.input_tokens
    match_op: exists
    distinguish: "This compatibility spelling uses `summary.token_usage` rather than current `result.usage`. It should not shadow the current result record."
    vocabulary: ["summary"]
    confidence: observed
    evidence: ./fixtures/qwen/summary-token-usage.jsonl
  - id: stream-runaway_repetition-result-loop
    signal: runaway_repetition
    source: stream
    locator: "type=result,is_error=true"
    detection: bespoke
    priority: 55
    distinguish: "Qwen has native loop detectors for repeated tool calls, chanting text, repetitive thoughts, read loops, stagnation, and tool-call caps. The terminal stream only carries a human message; classification should parse the loop type and treat this as provider-native guard output, not as Claudine's own guard firing."
    vocabulary: ["CONSECUTIVE_IDENTICAL_TOOL_CALLS", "CHANTING_IDENTICAL_SENTENCES", "REPETITIVE_THOUGHTS", "READ_FILE_LOOP", "ACTION_STAGNATION", "SHELL_COMMAND_STAGNATION", "GLOBAL_TOOL_CALL_DUPLICATE", "ALTERNATING_TOOL_CALL_PATTERN", "TURN_TOOL_CALL_CAP", "INVALID_TOOL_PARAMS_STAGNATION"]
    since: "0.19.6"
    confidence: source_code
    evidence: ./fixtures/qwen/result-loop-detected.jsonl
extractions:
  - record: stream-model_resolved-system-init
    field: resolved
    path: model
  - record: stream-model_resolved-system-init
    field: session_id
    path: session_id
  - record: stream-model_resolved-system-session_start
    field: resolved
    path: model
  - record: stream-model_resolved-system-session_start
    field: session_id
    path: session_id
  - record: stream-model_resolved-init-legacy
    field: resolved
    path: model
  - record: stream-model_resolved-init-legacy
    field: session_id
    path: session_id
  - record: stream-rate_limited-error-type
    field: message
    path: error.message
  - record: stream-auth_invalid-result-missing-api-key
    field: message
    path: error.message
  - record: stream-tokens_consumed-result-usage
    field: input
    path: usage.input_tokens
    unit: tokens
  - record: stream-tokens_consumed-result-usage
    field: output
    path: usage.output_tokens
    unit: tokens
  - record: stream-tokens_consumed-summary-token_usage
    field: input
    path: token_usage.input_tokens
    unit: tokens
  - record: stream-tokens_consumed-summary-token_usage
    field: output
    path: token_usage.output_tokens
    unit: tokens
bespoke_rationale:
  - "stream-runaway_repetition-result-loop: this maps Qwen's provider-native loop detector to Claudine's guard taxonomy. The stream carries a formatted prose message, and reliable extraction of which native loop detector fired requires parsing text and correlating it with Qwen's `LoopType` vocabulary rather than a single stable discriminator path."
gaps:
  - "Current Qwen `v0.19.6` has source-code-confirmed rate-limit and retry classifiers for HTTP 429, 503, 1302, 1305, 529 capacity overload, transport failures, `Throttling.AllocationQuota`, and Qwen OAuth free-tier quota exhaustion, but the primary headless stream does not expose those classifier fields as stable JSON records. Only the seeded `type=error,error.type=rate_limit` fixture is recorded."
  - "No fixture-backed stream record was found for `usage_cap_approaching` or `usage_capped`. Qwen's internal quota helpers distinguish free allocated quota exhaustion and allocated quota exceeded from temporary throttling, but terminal stream output currently collapses this to error text."
  - "`no_funds` is likely expressible for Qwen OAuth free-tier quota and provider billing failures, but no scrubbed stream fixture proves a stable message or discriminator. It is left unrecorded rather than inferred from generic quota text."
  - "Current headless `system/subtype=init` includes both `model` and `qwen_code_version`; recording both `model_resolved` and `provider_version` from the same payload would lose one signal under Claudine's first-match-wins generated detection model. `provider_version` is documented in the body but omitted from frontmatter until multi-signal emission from one payload exists."
  - "Qwen session ids are resumable through `--continue` and `--resume`, and docs state sessions are project-scoped JSONL under `~/.qwen/projects/<sanitized-cwd>/chats`. `session_resumable` is omitted for the same first-match-wins reason as `provider_version`: the same system/init payload is already the model-resolution record."
  - "Qwen result payloads collect `permission_denials[]` for execution-denied tool calls, but the shape records `tool_name`, `tool_use_id`, and `tool_input` without a stable read-versus-write discriminator. `permission_denied_read` and `permission_denied_write` need hand classification by tool name and input schema before they can be recorded."
  - "Qwen's `control_request` stream messages can request `can_use_tool` decisions in SDK/stream-json control mode, but this is a permission mediation channel rather than Claudine's reserved `human_input_requested` signal. No human-input prompt record is emitted."
  - "Qwen native cancellation, max-session-turns, max-wall-time, and max-tool-calls use process exit paths and stderr/JSON-mode error handling. `stream-json` does not consistently emit terminal result envelopes for all of these paths, so `interrupted`, `turn_limit_reached`, `timeout`, and `runaway_volume` remain wrapper/exit-layer gaps. Claudine's wrapper-side bespoke exit mapping (FatalTurnLimitedError 53 → turn_limit_reached, FatalBudgetExceededError 55 → session_time_limit_reached, FatalCancellationError 130 → interrupted, via the synthesized exit-source payload) landed 2026-07-06 but still awaits captured evidence (harvest/E6); no records or fixtures exist yet."
  - "The compatibility `init` and `summary` fixtures are proven by the local seeded corpus and Claudine parser support, but current upstream `v0.19.6` headless source favors `system/subtype=init` and `result.usage`. Maintainers should treat the compatibility records as drift tolerance rather than current upstream contract."
  - "usage.cache_read_input_tokens and usage.total_tokens (and the summary.token_usage equivalents) are source-confirmed optional fields with no fixture evidence; extractions deferred until a fixture carries them."
  - "claudine's parser (protocol/qwen.rs:66-80) accepts system/subtype=session_start and legacy init but ignores the current upstream system/subtype=init — the priority-10 record requires a parser change to be consumable."
changes:
  - "2026-07-06: noted in gaps that claudine's wrapper-side bespoke exit-code mapping (53/55/130) landed and awaits captured evidence."
  - "2026-07-06: rewrote both tokens_consumed numeric-regex presence proxies (result.usage and summary.token_usage input_tokens) to `match_op: exists`."
  - "2026-07-06: renamed the three model_resolved extraction fields `model` → `resolved` to match the canonical SignalEvent::ModelResolved payload field."
requires_claudine_update: true
reason: "Qwen signal detection is codegen-wired and this research adds Qwen stream records for model resolution, rate limits, auth preflight failures, token metering, and a native loop-guard catalog entry while documenting first-match-wins and stream-projection gaps that generated detection must respect."
---

# Qwen Code Signal Detection

## Overview

Qwen Code's most useful Claudine-facing signal surface is headless `--output-format stream-json`, which writes line-delimited JSON messages to stdout. The official headless documentation treats this as a first-class automation contract: system records open the session, assistant/user/tool records carry conversation and tool flow, and terminal `result` records carry execution summary fields. Current source at `v0.19.6` shows the stream is produced by `StreamJsonOutputAdapter`, which serializes each JSON message as one stdout line.

The stream is stable enough for model/session metadata, terminal token usage, auth-preflight terminal errors, execution-denied tool accounting, and provider-native loop-guard terminal errors. It is not a lossless projection of Qwen's internal retry, quota, rate-limit, cancellation, or budget machinery. Those internals are source-code-confirmed but usually collapse to `result.error.message`, stderr, or process exit code before Claudine can read a typed stream discriminator.

## Signal Surfaces

### Stream Events

`qwen -p ... --output-format stream-json` emits newline-delimited JSON objects on stdout. Official docs show `system`, `assistant`, and `result` events, and note that `--include-partial-messages` adds `stream_event` payloads such as `message_start` and `content_block_delta`. Current source writes each message directly from `StreamJsonOutputAdapter.emitMessageImpl`.

The current headless initialization payload is `type: system`, `subtype: init`. `buildSystemMessage` includes `session_id`, `cwd`, `tools`, `mcp_servers`, `model`, `permission_mode`, `slash_commands`, `qwen_code_version`, and `agents`. The official docs still show a `system/subtype=session_start` example, and Qwen's dual-output bridge emits `system/subtype=session_start` with a nested capability handshake under `data`; the seeded Claudine corpus also has a top-level `system/session_start` model fixture. The document records both current and compatibility model-resolution spellings separately.

Terminal `result` records are built by `BaseJsonOutputAdapter.buildResultMessage`. Success records have `subtype: success`, `is_error: false`, `duration_ms`, `duration_api_ms`, `num_turns`, `result`, `usage`, and `permission_denials`. Error records have `subtype: error_during_execution` by default, `is_error: true`, the same timing/usage/permission fields, and `error.message`. `usage` is an `ExtendedUsage` object with `input_tokens`, `output_tokens`, optional cache fields, optional `total_tokens`, and optional service/tool metadata.

### JSON Output

`--output-format json` buffers the same message objects into one JSON array and writes them when the session completes. The structured shapes are useful for research confirmation, but Claudine's direct wrapper stream source should prefer `stream-json` because it preserves event ordering and allows live supervision.

### stderr Diagnostics and Exit

Qwen still uses stderr and process exit for several operational signals. Non-interactive cancellation uses `FatalCancellationError` and exit code 130. Max-session-turns uses `FatalTurnLimitedError` and exit code 53. Run-level budgets use `FatalBudgetExceededError` and exit code 55. These paths are documented and source-code-confirmed, but they do not consistently produce stream records before process termination, so they are not frontmatter records except where a terminal result fixture exists.

### Session Logs and App Logs

Qwen stores project-scoped chats under `~/.qwen/projects/<sanitized-cwd>/chats`, as documented in the headless resume section. Existing Claudine logging research covers Qwen transcripts and debug logs. This signal document focuses on live stream detection because logs are retrospective and can lag or outlive the wrapped process.

### Dual Output and Control Messages

Qwen's dual-output bridge can mirror stream-json events to a file descriptor or file while the TUI remains active. Its `session_start` handshake advertises `protocol_version: 1` and supported events: `system`, `user`, `assistant`, `stream_event`, `result`, `control_request`, and `control_response`. Control requests include `can_use_tool`, which is permission mediation rather than a general human-input prompt.

## Usage and Rate Limits

Qwen has strong internal rate-limit classification but a weaker public stream projection.

The fixture-backed stream record is the observed `type=error` envelope:

| Signal | Source | Locator | Match | Extraction |
| --- | --- | --- | --- | --- |
| `rate_limited` | stream | `type=error` | `error.type == rate_limit` | `message` from `error.message` |

Current source confirms broader retry and rate-limit semantics. `isRateLimitError` treats HTTP/status codes `429`, `503`, `1302`, and `1305` as rate-limit or throttling codes; `classifyRetryError` classifies HTTP 529 as `capacity-overload`, 401/403 as `auth-error`, 5xx as retryable server errors, `Throttling.AllocationQuota` as provider-business fail-fast, and Qwen OAuth free-tier quota exhaustion as `qwen-oauth-free-tier-quota`. The classifier also extracts provider code, provider message, request id, transport kind, and transport code from known HTTP/SSE/error shapes.

Those fields are diagnostic internals, not current stream fields. A stream `result.error.message` may contain rate-limit or quota prose, but without a committed current fixture and stable discriminator it would be unsafe to generate broad substring records for `usage_capped`, `no_funds`, `provider_overloaded`, or `retries_exhausted`.

## Token Metering

`tokens_consumed` is recorded from terminal usage envelopes.

| Signal | Source | Locator | Match | Extracted fields |
| --- | --- | --- | --- | --- |
| `tokens_consumed` | stream | `type=result` | `usage.input_tokens` is numeric | `input`, `output`, optional `cache_read`, optional `total` |
| `tokens_consumed` | stream | `type=summary` | `token_usage.input_tokens` is numeric | Compatibility spelling for older/observed streams |

Current Qwen source defines `Usage` as `input_tokens`, `output_tokens`, optional `cache_creation_input_tokens`, optional `cache_read_input_tokens`, and optional `total_tokens`. `ExtendedUsage` adds optional server-tool usage, service tier, and cache-creation detail. Current `result.usage` is the authoritative headless aggregate; the `summary.token_usage` fixture is retained as compatibility evidence because Claudine already accepts that spelling.

Assistant messages also carry `message.usage`, but this document does not record that as a separate `tokens_consumed` source because the terminal `result` is the aggregate outcome record and avoids double-counting per-message usage.

## Authentication and Authorization

Qwen validates non-interactive authentication before the main run loop. If validation fails and output format is `json` or `stream-json`, `validateNonInteractiveAuth` creates the corresponding JSON adapter, emits a terminal `result` with `is_error: true`, `duration_ms: 0`, `num_turns: 0`, and `error.message`, then exits with code 1.

The frontmatter records `auth_invalid` for the stable missing-API-key preflight string because the source and tests confirm this path. This covers missing credentials more than expired credentials; Qwen's upstream request failures for expired or rejected keys still need fresh scrubbed stream fixtures before Claudine can distinguish `auth_invalid` from generic `error_during_execution`.

Execution-denied tool calls are tracked in `permission_denials[]` on the terminal result. The type is `CLIPermissionDenial` with `tool_name`, `tool_use_id`, and `tool_input`. That is useful evidence, but it does not by itself distinguish read permission denial from write permission denial. A future detector can classify denial type by tool-name and input schema, likely as bespoke behavior rather than a single declarative path.

## Model Resolution and Provider Version

`model_resolved` is recorded from the session metadata fields on current `system/subtype=init`, observed `system/subtype=session_start`, and observed legacy `init` events.

Current `system/subtype=init` also carries `qwen_code_version`, which is a source-code-confirmed provider-version field. It is intentionally not recorded in frontmatter because the generated detector is first-match-wins: the same payload already carries `model_resolved`, and adding a `provider_version` record for the same single JSON object would make one of the signals unreachable unless Claudine learns to emit multiple signals from one payload.

Qwen supports dynamic model selection through `--model`, slash commands, and model-provider configuration, but no structured `model_fallback` stream event was found. Internal retry classification labels HTTP 529 capacity overload as retryable rather than fallback-eligible, and the source comment explicitly says model/provider fallback is not implemented in that path.

## Generation Health

Qwen has provider-native loop detection. The non-interactive runner formats loop detector output with messages beginning `Loop detection halted the run`, and the native `LoopType` vocabulary is:

| Loop type |
| --- |
| `CONSECUTIVE_IDENTICAL_TOOL_CALLS` |
| `CHANTING_IDENTICAL_SENTENCES` |
| `REPETITIVE_THOUGHTS` |
| `READ_FILE_LOOP` |
| `ACTION_STAGNATION` |
| `SHELL_COMMAND_STAGNATION` |
| `GLOBAL_TOOL_CALL_DUPLICATE` |
| `ALTERNATING_TOOL_CALL_PATTERN` |
| `TURN_TOOL_CALL_CAP` |
| `INVALID_TOOL_PARAMS_STAGNATION` |

This is recorded as a bespoke `runaway_repetition` mapping because it is Qwen's native equivalent of a guard signal. The stream carries a formatted error message rather than a typed `loop_type` field, so reliable extraction should stay hand-written.

Qwen also has retry diagnostics and retry-delay helpers, but no first-class stream record announces `generation_retried` or `retries_exhausted`. A debug log or telemetry stream may carry retry details, but no committed signal fixture proves a wrapper-grade source.

## Interruption and Recovery

The official headless docs state that Qwen can continue the most recent project session with `--continue` and can resume a specific session id with `--resume <sessionId>`. Session data is project-scoped JSONL under `~/.qwen/projects/<sanitized-cwd>/chats`.

The stream exposes `session_id` in the same system/init payload used for `model_resolved`. Because Claudine's generated signal tables are first-match-wins, this document does not add a `session_resumable` record from that same payload. Native cancellation uses `FatalCancellationError`, writes a cancellation message, and exits 130; no scrubbed stream fixture proves a structured `interrupted` payload.

## Version Drift

The main observed drift is in event names and metadata placement.

| Area | Current `v0.19.6` source | Observed or documented compatibility shape | Impact |
| --- | --- | --- | --- |
| Session metadata | `type=system`, `subtype=init`, top-level `model`, `qwen_code_version` | `type=system`, `subtype=session_start`, top-level `model`; legacy `type=init` | Keep separate model-resolution records with unique priorities. |
| Token usage | `type=result`, `usage.*` | `type=summary`, `token_usage.*` | Prefer result usage; keep summary compatibility. |
| Rate limit | Internal HTTP/SSE classifiers and terminal result/error text | Observed `type=error,error.type=rate_limit` | Record only the structured observed envelope; treat classifier-only facts as gaps. |
| Provider version | `qwen_code_version` on current system init | Dual-output puts version under `data.version` for `session_start` | Do not record until multi-signal payload emission is supported. |

## Quirks and Gaps

Qwen's internal classifier vocabulary is stronger than the public headless stream. Do not assume a generic `result.error.message` can safely distinguish usage caps, no funds, provider overload, retries exhausted, and rate limiting without fixture-backed provider copy.

The same current `system/subtype=init` object carries model, session id, and CLI version. With first-match-wins detection, one payload cannot currently yield `model_resolved`, `provider_version`, and `session_resumable`; this document records the model and lists the others as gaps.

Permission denials are accumulated in terminal results, but read/write classification requires tool-aware behavior. A denied `ReadFile` and denied `Edit` are semantically different even though both enter `permission_denials[]` with the same top-level shape.

Native max-session-turn, wall-time budget, tool-call budget, and cancellation paths are operationally important but mostly leave through stderr and exit codes. Claudine should continue to rely on wrapper/exit handling for these until Qwen emits consistent terminal stream envelopes.

The compatibility `init` and `summary` records are useful for drift tolerance, but the current upstream source should be considered authoritative for new captures.

## Changelog

Fresh first-run research document; `changes` is empty.

## Sources

- [Qwen Code headless mode documentation](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/)
- [Qwen Code repository](https://github.com/QwenLM/qwen-code)
- [Qwen `v0.19.6` stream-json adapter](https://github.com/QwenLM/qwen-code/blob/v0.19.6/packages/cli/src/nonInteractive/io/StreamJsonOutputAdapter.ts#L32-L120)
- [Qwen `v0.19.6` non-interactive message and result types](https://github.com/QwenLM/qwen-code/blob/v0.19.6/packages/cli/src/nonInteractive/types.ts#L19-L194)
- [Qwen `v0.19.6` result builder and permission-denial collection](https://github.com/QwenLM/qwen-code/blob/v0.19.6/packages/cli/src/nonInteractive/io/BaseJsonOutputAdapter.ts#L1027-L1241)
- [Qwen `v0.19.6` system initialization builder](https://github.com/QwenLM/qwen-code/blob/v0.19.6/packages/cli/src/utils/nonInteractiveHelpers.ts#L215-L259)
- [Qwen `v0.19.6` auth preflight stream-json error path](https://github.com/QwenLM/qwen-code/blob/v0.19.6/packages/cli/src/validateNonInterActiveAuth.ts#L16-L82)
- [Qwen `v0.19.6` rate-limit helper](https://github.com/QwenLM/qwen-code/blob/v0.19.6/packages/core/src/utils/rateLimit.ts#L11-L170)
- [Qwen `v0.19.6` retry error classifier](https://github.com/QwenLM/qwen-code/blob/v0.19.6/packages/core/src/utils/retryErrorClassification.ts#L12-L192)
- [Qwen `v0.19.6` quota detection helpers](https://github.com/QwenLM/qwen-code/blob/v0.19.6/packages/core/src/utils/quotaErrorDetection.ts#L86-L119)
- [Qwen `v0.19.6` cancellation, max-turn, and budget error handlers](https://github.com/QwenLM/qwen-code/blob/v0.19.6/packages/cli/src/utils/errors.ts#L249-L310)
- [Qwen `v0.19.6` dual-output protocol handshake](https://github.com/QwenLM/qwen-code/blob/v0.19.6/packages/cli/src/dualOutput/DualOutputBridge.ts#L26-L53)
