---
$schema: ./_schema.yaml
created: 2026-07-05
last_updated: 2026-07-09
agent: codex
model: default
docs: https://code.claude.com/docs/en/agent-sdk/typescript
records:
  - id: stream-usage_cap_approaching-usage
    signal: usage_cap_approaching
    source: stream
    locator: "type=rate_limit_event"
    detection: declarative
    priority: 10
    match_path: rate_limit_info.rateLimitType
    match_op: eq
    match_value: usage
    distinguish: "`rateLimitType: usage` is the generic account-wide usage marker: it names neither a model tier nor a window duration, so it decomposes to `model: All` with no `timeframe`. A `rate_limit_event` carrying a `rateLimitType` is the informative approaching channel; active throttling arrives via `is_throttled`/`assistant.error` instead."
    vocabulary: ["usage", "five_hour", "seven_day", "seven_day_opus", "seven_day_sonnet", "seven_day_overage_included", "overage"]
    confidence: observed
    evidence: ./fixtures/claude/rate-limit-info-approaching.jsonl
    notes: "Cap axes are decomposed from `rateLimitType` per the provider-neutral cap model (more-struture Cluster 0). This seeded fixture pairs `rateLimitType: usage` with the observed `status: approaching_limit`."
  - id: stream-usage_cap_approaching-seven_day
    signal: usage_cap_approaching
    source: stream
    locator: "type=rate_limit_event"
    detection: declarative
    priority: 11
    match_path: rate_limit_info.rateLimitType
    match_op: eq
    match_value: seven_day
    distinguish: "`rateLimitType: seven_day` is the account-wide 7-day window (`model: All`, `timeframe: 604800s`), distinct from the model-scoped `seven_day_opus`/`seven_day_sonnet` variants."
    vocabulary: ["usage", "five_hour", "seven_day", "seven_day_opus", "seven_day_sonnet", "seven_day_overage_included", "overage"]
    confidence: source_code
    evidence: ./fixtures/claude/rate-limit-info-allowed-warning-seven-day.jsonl
    notes: "SDK `0.3.201` lists `seven_day` in `SDKRateLimitInfo.rateLimitType`; the seeded fixture pairs it with `status: allowed_warning`."
  - id: stream-usage_cap_approaching-five_hour
    signal: usage_cap_approaching
    source: stream
    locator: "type=rate_limit_event"
    detection: declarative
    priority: 12
    match_path: rate_limit_info.rateLimitType
    match_op: eq
    match_value: five_hour
    distinguish: "`rateLimitType: five_hour` is the account-wide 5-hour window (`model: All`, `timeframe: 18000s`)."
    vocabulary: ["usage", "five_hour", "seven_day", "seven_day_opus", "seven_day_sonnet", "seven_day_overage_included", "overage"]
    confidence: documented
    evidence: ./fixtures/claude/rate-limit-info-five-hour.jsonl
    notes: "SDK `0.3.201` declares `five_hour` in `SDKRateLimitInfo.rateLimitType`; no scrubbed real stream captured it, so the evidence is a synthetic fixture mirroring the observed `seven_day` shape."
  - id: stream-usage_cap_approaching-seven_day_opus
    signal: usage_cap_approaching
    source: stream
    locator: "type=rate_limit_event"
    detection: declarative
    priority: 13
    match_path: rate_limit_info.rateLimitType
    match_op: eq
    match_value: seven_day_opus
    distinguish: "`rateLimitType: seven_day_opus` is the Opus-scoped 7-day window (`model: Specific(opus)`, `timeframe: 604800s`) — the model tier is pinned in research via `literal: opus` because the wire bundles model and window into one token."
    vocabulary: ["usage", "five_hour", "seven_day", "seven_day_opus", "seven_day_sonnet", "seven_day_overage_included", "overage"]
    confidence: documented
    evidence: ./fixtures/claude/rate-limit-info-seven-day-opus.jsonl
    notes: "SDK `0.3.201` declares `seven_day_opus`; evidence is a synthetic fixture (no scrubbed real stream captured it) mirroring the observed `seven_day` shape."
  - id: stream-usage_cap_approaching-seven_day_sonnet
    signal: usage_cap_approaching
    source: stream
    locator: "type=rate_limit_event"
    detection: declarative
    priority: 14
    match_path: rate_limit_info.rateLimitType
    match_op: eq
    match_value: seven_day_sonnet
    distinguish: "`rateLimitType: seven_day_sonnet` is the Sonnet-scoped 7-day window (`model: Specific(sonnet)`, `timeframe: 604800s`)."
    vocabulary: ["usage", "five_hour", "seven_day", "seven_day_opus", "seven_day_sonnet", "seven_day_overage_included", "overage"]
    confidence: documented
    evidence: ./fixtures/claude/rate-limit-info-seven-day-sonnet.jsonl
    notes: "SDK `0.3.201` declares `seven_day_sonnet`; evidence is a synthetic fixture (no scrubbed real stream captured it) mirroring the observed `seven_day` shape."
  - id: stream-rate_limited-throttled
    signal: rate_limited
    source: stream
    locator: "type=rate_limit_event"
    detection: declarative
    priority: 30
    match_path: is_throttled
    match_op: eq
    match_value: "true"
    distinguish: "This is active throttling and may include `retry_after_ms`; it differs from usage-cap warnings because the provider is rejecting or delaying work now."
    vocabulary: ["true", "false"]
    confidence: observed
    evidence: ./fixtures/claude/rate-limit-throttled-message.jsonl
    notes: "Legacy/observed stream shape; the current SDK's typed `SDKRateLimitEvent` uses nested `rate_limit_info` instead."
  - id: stream-rate_limited-assistant-error
    signal: rate_limited
    source: stream
    locator: "type=assistant.error"
    detection: declarative
    priority: 40
    match_path: error.type
    match_op: eq
    match_value: rate_limit
    distinguish: "This is a terminal assistant error with a rate-limit discriminator, not a nonterminal `rate_limit_event` warning."
    vocabulary: ["authentication_failed", "oauth_org_not_allowed", "billing_error", "rate_limit", "overloaded", "invalid_request", "model_not_found", "server_error", "unknown", "max_output_tokens"]
    confidence: source_code
    evidence: ./fixtures/claude/assistant-error-rate-limit.jsonl
  - id: stream-no_funds-error-billing
    signal: no_funds
    source: stream
    locator: "type=error"
    detection: declarative
    priority: 50
    match_path: error.type
    match_op: eq
    match_value: billing_error
    distinguish: "A top-level `error` with `billing_error` is an account-credit/billing failure. It should not be collapsed into generic rate limiting even though both are provider-side account limits."
    vocabulary: ["authentication_failed", "oauth_org_not_allowed", "billing_error", "rate_limit", "overloaded", "invalid_request", "model_not_found", "server_error", "unknown", "max_output_tokens"]
    confidence: observed
    evidence: ./fixtures/claude/error-billing.jsonl
  - id: stream-no_funds-assistant-billing
    signal: no_funds
    source: stream
    locator: "type=assistant"
    detection: declarative
    priority: 60
    match_path: error
    match_op: eq
    match_value: billing_error
    distinguish: "Newer Claude Code can emit a synthetic assistant message carrying `error: billing_error` and human text, followed by a `result` with `is_error: true`."
    vocabulary: ["authentication_failed", "oauth_org_not_allowed", "billing_error", "rate_limit", "overloaded", "invalid_request", "model_not_found", "server_error", "unknown", "max_output_tokens"]
    confidence: source_code
    evidence: ./fixtures/claude/billing-error-synthetic-result.jsonl
  - id: stream-model_resolved-init
    signal: model_resolved
    source: stream
    locator: "type=init"
    detection: declarative
    priority: 70
    match_path: type
    match_op: eq
    match_value: init
    distinguish: "`init` starts the structured stream and carries the resolved model for the session. A later `system`/`init` SDK frame can also carry metadata, but no scrubbed provider-version fixture exists yet."
    vocabulary: ["init", "system"]
    confidence: observed
    evidence: ./fixtures/claude/init-model.jsonl
  - id: stream-auth_kind_detected-init
    signal: auth_kind_detected
    source: stream
    locator: "type=init"
    detection: declarative
    priority: 80
    match_path: apiKeySource
    match_op: exists
    distinguish: "`apiKeySource` is authentication metadata on the same init frame as the resolved model. It identifies the credential source, not whether authentication succeeded."
    vocabulary: ["ANTHROPIC_API_KEY", "CLAUDE_CODE_OAUTH_TOKEN", "none", "unknown"]
    confidence: observed
    evidence: ./fixtures/claude/init-model.jsonl
  - id: stream-tokens_consumed-total_cost_usd
    signal: tokens_consumed
    source: stream
    locator: "type=result"
    detection: declarative
    priority: 90
    match_path: total_cost_usd
    match_op: exists
    distinguish: "Current result envelopes use `total_cost_usd`; this record should win over legacy `cost_usd` when both are present."
    vocabulary: ["success", "error_during_execution", "error_max_turns", "error_max_budget_usd", "error_max_structured_output_retries"]
    confidence: source_code
    evidence: ./fixtures/claude/result-usage-cost-fields.jsonl
  - id: stream-tokens_consumed-cost_usd-legacy
    signal: tokens_consumed
    source: stream
    locator: "type=result"
    detection: declarative
    priority: 100
    match_path: cost_usd
    match_op: exists
    distinguish: "Legacy result envelopes used `cost_usd`; keep it lower priority than `total_cost_usd` because Claudine's parser prefers the newer field."
    vocabulary: ["success", "error_during_execution", "error_max_turns", "error_max_budget_usd", "error_max_structured_output_retries"]
    confidence: observed
    evidence: ./fixtures/claude/result-usage-cost-fields.jsonl
  - id: stream-session_tainted-result-error
    signal: session_tainted
    source: stream
    locator: "type=result"
    detection: bespoke
    priority: 110
    match_path: is_error
    match_op: eq
    match_value: "true"
    distinguish: "`result.is_error=true` marks a failed terminal envelope even when `subtype` is `success` or prior assistant text looked normal. Correct classification depends on whether an earlier error was already emitted."
    vocabulary: ["true", "false"]
    confidence: observed
    evidence: ./fixtures/claude/billing-error-synthetic-result.jsonl
extractions:
  - record: stream-usage_cap_approaching-usage
    field: resets_at
    path: rate_limit_info.resetsAt
    unit: unix_seconds
    zone: utc
  - record: stream-usage_cap_approaching-seven_day
    field: resets_at
    path: rate_limit_info.resetsAt
    unit: unix_seconds
    zone: utc
  - record: stream-usage_cap_approaching-seven_day
    field: timeframe
    literal: "604800"
    unit: duration_secs
  - record: stream-usage_cap_approaching-five_hour
    field: resets_at
    path: rate_limit_info.resetsAt
    unit: unix_seconds
    zone: utc
  - record: stream-usage_cap_approaching-five_hour
    field: timeframe
    literal: "18000"
    unit: duration_secs
  - record: stream-usage_cap_approaching-seven_day_opus
    field: resets_at
    path: rate_limit_info.resetsAt
    unit: unix_seconds
    zone: utc
  - record: stream-usage_cap_approaching-seven_day_opus
    field: model
    literal: opus
  - record: stream-usage_cap_approaching-seven_day_opus
    field: timeframe
    literal: "604800"
    unit: duration_secs
  - record: stream-usage_cap_approaching-seven_day_sonnet
    field: resets_at
    path: rate_limit_info.resetsAt
    unit: unix_seconds
    zone: utc
  - record: stream-usage_cap_approaching-seven_day_sonnet
    field: model
    literal: sonnet
  - record: stream-usage_cap_approaching-seven_day_sonnet
    field: timeframe
    literal: "604800"
    unit: duration_secs
  - record: stream-rate_limited-throttled
    field: retry_after
    path: retry_after_ms
    unit: duration_millis
  - record: stream-rate_limited-throttled
    field: message
    path: message
  - record: stream-rate_limited-assistant-error
    field: message
    path: error.message
  - record: stream-no_funds-error-billing
    field: message
    path: error.message
  - record: stream-no_funds-assistant-billing
    field: message
    path: message.content[0].text
  - record: stream-model_resolved-init
    field: resolved
    path: model
  - record: stream-auth_kind_detected-init
    field: auth_kind
    path: apiKeySource
  - record: stream-tokens_consumed-total_cost_usd
    field: input
    path: usage.input_tokens
    unit: tokens
  - record: stream-tokens_consumed-total_cost_usd
    field: output
    path: usage.output_tokens
    unit: tokens
  - record: stream-tokens_consumed-total_cost_usd
    field: cache_read
    path: usage.cache_read_input_tokens
    unit: tokens
  - record: stream-tokens_consumed-total_cost_usd
    field: cost
    path: total_cost_usd
    unit: usd
  - record: stream-tokens_consumed-cost_usd-legacy
    field: input
    path: usage.input_tokens
    unit: tokens
  - record: stream-tokens_consumed-cost_usd-legacy
    field: output
    path: usage.output_tokens
    unit: tokens
  - record: stream-tokens_consumed-cost_usd-legacy
    field: cache_read
    path: usage.cache_read_input_tokens
    unit: tokens
  - record: stream-tokens_consumed-cost_usd-legacy
    field: cost
    path: cost_usd
    unit: usd
bespoke_rationale:
  - "stream-session_tainted-result-error: A single `result.is_error=true` payload proves the terminal envelope is failed, but the taint semantics depend on cross-record state: whether a prior `error`, `assistant.error`, or synthetic assistant billing frame already claimed failure and whether any downstream wrapper would otherwise classify the run as successful."
gaps:
  - "No scrubbed fixture yet for current SDK `SDKRateLimitInfo.status='rejected'`, `errorCode='credits_required'`, `overageDisabledReason='out_of_credits'`, or overage credit fields, so current no-funds/usage-capped rate-limit-info records are documented but not placed in frontmatter."
  - "No scrubbed fixture yet for `SDKAPIRetryMessage` / `system` `subtype='api_retry'`; generation_retried is source-confirmed by SDK types but omitted from records until a fixture exists."
  - "No scrubbed fixture yet for `SDKModelRefusalFallbackMessage`; model_fallback is source-confirmed by SDK types but omitted from records until a fixture exists."
  - "No scrubbed fixture yet for `SDKPermissionDeniedMessage` or non-empty `result.permission_denials`; permission_denied_read/write mapping needs tool-name/path semantics from live payloads."
  - "No scrubbed fixture yet for `SDKSystemMessage` with `claude_code_version`; provider_version is source-confirmed by SDK types but omitted from records until a fixture exists."
  - "No scrubbed fixture yet for `SDKControlElicitationRequest`, `SDKElicitationCompleteMessage`, or `request_user_dialog`; human_input_requested is documented/source-confirmed but omitted from records until an emitted stream/control fixture exists."
  - "Current SDK `0.3.201` types list `SDKRateLimitInfo.status` as `allowed | allowed_warning | rejected`, while seeded fixtures include observed `approaching_limit` and legacy `is_throttled`; Claudine must keep compatibility with both shapes."
  - "Older rate_limit_info.status vocabulary: claudine's parser (protocol/claude.rs resolved_is_throttled) treats status in {limited, blocked} and overageStatus == blocked as throttled; these members were absent from the observed vocabulary and need fixture evidence."
  - "Top-level resetsAt / reset_at (seconds) spellings are supported by claudine's parser (protocol/claude.rs:338-341, resolved_reset_at) but no fixture exercises either; the usage_capped record consequently has no lifts_at extraction."
changes:
  - "2026-07-06: rewrote the total_cost_usd, cost_usd, and apiKeySource numeric/any-value regex presence proxies to `match_op: exists`; retry_after extraction now uses the new `duration_millis` unit, and the unit-lie gap entry plus body quirk are removed."
  - "2026-07-06: renamed extraction fields to the canonical SignalEvent payload names — usage_cap_approaching `lifts_at` → `resets_at` (the variant field is `resets_at`; `lifts_at` belongs to usage_capped) and model_resolved `model` → `resolved`."
  - "2026-07-09: replaced the two status-matched usage_cap_approaching records with one record per `rateLimitType` SDK token (usage, seven_day, five_hour, seven_day_opus, seven_day_sonnet), each decomposing the combined token into the provider-neutral cap axes via `ExtractStrategy::Literal` (model tier + timeframe seconds) per more-struture Cluster 0. The `window` extraction is retired. five_hour/seven_day_opus/seven_day_sonnet carry synthetic fixtures (SDK-declared tokens with no scrubbed real stream)."
requires_claudine_update: true
reason: "Claude Code signal detection is codegen-wired and this research adds declarative records for usage-cap warnings, throttling, billing/no-funds, model/auth metadata, token usage cost spellings, and a bespoke session-taint rule."
---

# Claude Code Signal Detection

## Overview

Claude Code exposes signal-bearing data primarily through SDK and `stream-json` messages. The best contract surface is the TypeScript Agent SDK, whose `SDKMessage` union in `@anthropic-ai/claude-agent-sdk@0.3.201` includes result, system init, rate-limit, retry, permission-denied, model-fallback, elicitation, auth-status, and control-channel variants. The official SDK reference also documents that `query()` returns an async generator of `SDKMessage` values and that the package bundles a native Claude Code binary unless callers override the executable path.

The plain `claude -p --output-format stream-json --verbose` surface is useful but less completely documented than the SDK types. Seeded Claudine fixtures show older and observed shapes that are not all present in the current SDK union, including top-level `init`, top-level `error`, `assistant.error`, legacy `rate_limit_event.is_throttled`, observed `rate_limit_info.status: "approaching_limit"`, and legacy `result.cost_usd`. These are shipped/observed behavior, not roadmap behavior.

## Signal Surfaces

### Stream Events

Claude Code stream output is newline-delimited JSON on stdout in non-interactive `stream-json` mode and is normalized by the SDK as `SDKMessage`. Useful discriminators include `type`, `subtype` for system/result messages, `error` on assistant messages, nested `error.type` on error messages, and nested `rate_limit_info` on rate-limit events.

The SDK type union is a first-class contract. The lower-level `stream-json` shapes are operationally necessary because they have shipped in real streams and in Claudine's committed fixtures, but some are not fully represented in the newest SDK declarations.

### Session Logs

Claude Code stores local session transcripts, and the SDK exposes `listSessions()`, `getSessionMessages()`, and `getSessionInfo()`. The official TypeScript reference documents `createdAt` and `lastModified` as millisecond timestamps for session metadata. This topic did not add session-log records because the seeded signal corpus for Claude is stream-focused and no scrubbed transcript fixture was available for signal replay.

### Hook Payloads

Claude Code hooks receive JSON on stdin for command hooks or an HTTP request body for HTTP hooks. The hook reference lists lifecycle hooks such as `SessionStart`, `SessionEnd`, `StopFailure`, `PermissionDenied`, `Elicitation`, and `ElicitationResult`. These are first-class lifecycle surfaces, but the signal corpus does not yet contain scrubbed hook payload fixtures, so no hook records are emitted in frontmatter.

### Control and SDK Side Channels

The SDK control channel contains request/response records for permission checks, elicitation, user dialogs, interruption, usage, context usage, binary version, and model control. These are SDK-contract surfaces rather than plain stdout `stream-json` records. They are important for future Claudine integration, especially `human_input_requested`, `permission_denied_*`, `provider_version`, and usage dashboards, but they need fixture capture before becoming detection records.

## Usage and Rate Limits

Claude Code emits `rate_limit_event` for subscription usage windows and active throttling. Current SDK `SDKRateLimitInfo` declares `status` values `allowed`, `allowed_warning`, and `rejected`; optional fields include `resetsAt`, `rateLimitType`, `utilization`, `overageStatus`, `overageResetsAt`, `overageDisabledReason`, `errorCode`, and payment/credit booleans. Observed seeded fixtures additionally contain `status: "approaching_limit"` and legacy top-level `is_throttled`, `retry_after_ms`, and `message`.

`usage_cap_approaching` is keyed on `rate_limit_info.rateLimitType`, the combined window token. Under the provider-neutral cap model (more-struture Cluster 0) each token decomposes into the typed cap axes — `model` (`All`, or a `Specific` tier) and `timeframe` (a duration) — pinned in research via `ExtractStrategy::Literal` because the wire bundles both into one token: `usage` → `{All, no timeframe}`, `seven_day`/`five_hour` → `{All, 604800s/18000s}`, `seven_day_opus`/`seven_day_sonnet` → `{Specific(opus/sonnet), 604800s}`. Observed tokens are `usage` and `seven_day`; SDK `0.3.201` also declares `five_hour`, `seven_day_opus`, `seven_day_sonnet`, `seven_day_overage_included`, and `overage`. The overage spellings are billing state (out of the typed cap model) and carry no record. `resetsAt` is a Unix seconds timestamp in UTC. A `rate_limit_event` carrying a `rateLimitType` is the informative approaching channel; active throttling arrives via `is_throttled`/`assistant.error`.

`rate_limited` maps to active throttling. The legacy/observed shape is `type: "rate_limit_event"`, `is_throttled: true`, optionally `retry_after_ms` and `message`. A second terminal form is `type: "assistant.error"` with `error.type: "rate_limit"`.

`usage_capped` and `no_funds` are related but should not be conflated. The current SDK can express exhausted credits through rate-limit info fields such as `errorCode: "credits_required"` and `overageDisabledReason: "out_of_credits"`, but no scrubbed fixture exists yet. Fixture-backed `no_funds` records come from `billing_error` on top-level `error` and synthetic assistant envelopes. The synthetic assistant form carries a human message in `message.content[0].text` and is followed by a `result` with `is_error: true`, zero cost, zero usage, and `terminal_reason: "completed"`.

## Authentication and Authorization

The stream `init` frame carries `apiKeySource`, which is an `auth_kind_detected` signal. The fixture uses `ANTHROPIC_API_KEY`; SDK `AccountInfo` also carries `apiKeySource`, `tokenSource`, and an `apiProvider` enum with `firstParty`, `bedrock`, `vertex`, `foundry`, `anthropicAws`, `mantle`, and `gateway`.

Authentication failures are source-confirmed by the SDK `SDKAssistantMessageError` enum values `authentication_failed` and `oauth_org_not_allowed`, and the current Claudine parser classifies error kinds containing `auth` as configuration failures. No scrubbed Claude fixture exists for those exact payloads, so `auth_invalid` is not recorded in frontmatter.

Authorization and permission denial have two SDK surfaces. `SDKPermissionDeniedMessage` is emitted for auto-denied tool calls and carries `tool_name`, `tool_use_id`, optional `agent_id`, `decision_reason_type`, `decision_reason`, and `message`. Result envelopes also contain `permission_denials: SDKPermissionDenial[]`. The SDK does not directly split read vs write in the message type; Claudine should classify read/write from tool names and tool inputs once fixtures exist.

## Model Resolution

`model_resolved` is fixture-backed on stream `init.model`. The seeded fixture uses `claude-sonnet-4-20250514`, which is already a concrete version-style identifier rather than a vague alias.

`model_fallback` is source-confirmed in `SDKModelRefusalFallbackMessage`. The SDK says it is emitted when the primary model ends with a refusal and the turn is retried once on a fallback model; fields include `original_model`, `fallback_model`, `direction`, and refusal metadata. No scrubbed fixture exists, so no record is emitted yet.

`provider_version` is source-confirmed in `SDKSystemMessage.claude_code_version`. The committed live line that originally proved this included personal paths and local inventory, so it was not added to the fixture corpus. Capture a scrubbed `system`/`init` line before adding this record.

## Token Metering

`tokens_consumed` maps to terminal `result` envelopes. Current SDK result types require `total_cost_usd`, `usage`, `modelUsage`, `duration_ms`, `duration_api_ms`, `num_turns`, and `permission_denials`. Seeded fixtures show both current `total_cost_usd` and legacy `cost_usd`; Claudine's parser prefers `total_cost_usd` when both are present.

Useful extraction paths are `usage.input_tokens`, `usage.output_tokens`, `usage.cache_read_input_tokens`, and the cost field. Token counts are in tokens and costs are USD. `duration_ms` and `duration_api_ms` are millisecond durations.

## Retries, Overload, and Stream Failures

The SDK declares `SDKAPIRetryMessage` as `type: "system"`, `subtype: "api_retry"` with `attempt`, `max_retries`, `retry_delay_ms`, `error_status`, and `error`. That maps naturally to `generation_retried`, and `error: "overloaded"` would support `provider_overloaded`, but the seeded Claude corpus does not include a scrubbed emitted retry frame.

The SDK error vocabulary includes `overloaded`, `server_error`, `unknown`, and `max_output_tokens`. Those values should remain in the vocabulary for error-classifying records, but only `rate_limit` and `billing_error` have fixture-backed records today.

## Interruption and Recovery

The SDK `Query` object exposes `interrupt()`, and control requests include `subtype: "interrupt"`. `Options.abortController` is documented as the cancellation path for a query. These are host/SDK control surfaces rather than provider-emitted terminal stream records in the current corpus, so `interrupted` is not recorded.

Session recovery is partially visible through SDK session APIs such as `listSessions()`, `getSessionMessages()`, `getSessionInfo()`, and resume/startup options, and hooks distinguish `SessionStart` matcher values such as `startup` and `resume`. There is no fixture-backed `session_resumable` record yet.

## Human Input

Claude Code has first-class user-input surfaces. Hooks include `Elicitation` and `ElicitationResult` for MCP tool elicitation, and SDK controls include `SDKControlElicitationRequest`, `SDKElicitationCompleteMessage`, `SDKControlRequestUserDialogRequest`, and `UserDialogRequest`. These map to the reserved `human_input_requested` signal, but detection needs a captured emitted payload because the path can be stream, hook, or control-channel depending on integration mode.

## Session Taint

Claude Code can emit a synthetic assistant billing error followed by a terminal `result` whose `subtype` is `success`, `terminal_reason` is `completed`, and `is_error` is `true`. A detector that only looks at process exit or `result.subtype` can incorrectly mark the run successful. Claudine should treat this as a tainted or failed session. Because the correct interpretation depends on cross-record state, the frontmatter record is bespoke.

## Version Drift

The main drift is between current SDK type declarations and observed stream fixtures. SDK `0.3.201` declares nested `SDKRateLimitEvent.rate_limit_info` with `status: allowed | allowed_warning | rejected`; observed fixtures include `approaching_limit`, legacy top-level `is_throttled`, `retry_after_ms`, and `message`.

Result cost spelling also drifted. Current SDK result types use `total_cost_usd`; older stream envelopes used `cost_usd`. Detection keeps both, with `total_cost_usd` ordered first.

The SDK union has expanded beyond the current seeded Claude stream corpus: model refusal fallback, permission denied, API retry, auth status, user dialogs, elicitation, binary version, context usage, and experimental usage control responses are source-confirmed but not yet fixture-backed.

## Quirks and Gaps

`apiKeySource` and `model` share the same `init` frame. If the generated engine is truly first-match-wins across all records rather than per-signal extraction, Claudine may need a multi-signal emission rule or a generated overlap exemption for metadata-bearing init frames.

`SDKPermissionDeniedMessage` does not say read vs write directly. Claudine should derive `permission_denied_read` or `permission_denied_write` from `tool_name` plus `tool_input` once captured fixtures show real tool names and path fields.

No local-session or hook fixtures were used for this first record set. The source surfaces are documented, but stream fixtures are the only replayable evidence for this document.

## Changelog

Fresh first-run document. `changes` is intentionally empty.

## Sources

- [Claude Code TypeScript Agent SDK reference](https://code.claude.com/docs/en/agent-sdk/typescript)
- [Claude Code hooks reference](https://code.claude.com/docs/en/hooks)
- [`@anthropic-ai/claude-agent-sdk@0.3.201` package](https://registry.npmjs.org/@anthropic-ai/claude-agent-sdk/-/claude-agent-sdk-0.3.201.tgz)
- Local inspected SDK declarations: `/tmp/claude-agent-sdk-0.3.201/package/sdk.d.ts`
- Local Claudine stream model: `claudine/lib/src/stream/protocol/claude.rs`
- Local Claudine stream parser: `claudine/lib/src/stream/providers/claude.rs`
- [Claude signal fixture corpus](./fixtures/README.md)
