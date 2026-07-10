---
$schema: ./_schema.yaml
created: 2026-07-05
last_updated: 2026-07-06
agent: codex
model: default
docs: https://opencode.ai/docs/cli/
records:
  - id: stderr_promoted-usage_capped-legacy
    signal: usage_capped
    source: stderr_promoted
    locator: "legacy OpenCode log header; LogClassification::ProviderLimit(kind=UsageCap)"
    detection: declarative
    priority: 10
    match_path: kind
    match_op: eq
    match_value: UsageCap
    distinguish: "Usage-cap records must run before retry-exhausted and generic 429 records because OpenCode can wrap cap responses in AI_RetryError/maxRetriesExceeded envelopes."
    vocabulary: ["UsageCap", "RetriesExhausted", "Overloaded", "RateLimited"]
    until: "v1.17.7"
    confidence: source_code
    evidence: ./fixtures/opencode/usage-cap-legacy-retry-wrapped.txt
    notes: "Legacy log lines use `LEVEL <timestamp> +<delta>ms service=llm ... error=<JSON>`. Priority evidence: ./fixtures/opencode/usage-cap-wins-over-retries.txt (cap+retry markers on one line → cap wins); negative twin for the advisory branch: ./fixtures/opencode/usage-cap-advisory-no-error-tag.txt; matches the serialized LogClassification output of claudine's promoted-stderr classifier (glue mode); detection vocabulary lives in errors.rs"
  - id: stderr_promoted-usage_capped-1178
    signal: usage_capped
    source: stderr_promoted
    locator: "timestamp=... level=ERROR message=\"stream error\"; LogClassification::ProviderLimit(kind=UsageCap)"
    detection: declarative
    priority: 11
    match_path: kind
    match_op: eq
    match_value: UsageCap
    distinguish: "This is the OpenCode 1.17.8-style flat-string stream-error shape; it lacks `service=llm` and carries the provider payload in the dotted `error.error` key."
    vocabulary: ["UsageCap", "RetriesExhausted", "Overloaded", "RateLimited"]
    since: "v1.17.8"
    confidence: source_code
    evidence: ./fixtures/opencode/stream-error-1178-usage-cap.txt
    notes: "Matches the serialized LogClassification output of claudine's promoted-stderr classifier (glue mode); detection vocabulary lives in errors.rs"
  - id: stderr_promoted-retries_exhausted-429
    signal: retries_exhausted
    source: stderr_promoted
    locator: "LogClassification::ProviderLimit(kind=RetriesExhausted)"
    detection: declarative
    priority: 20
    match_path: kind
    match_op: eq
    match_value: RetriesExhausted
    distinguish: "A 429 with `AI_RetryError` or `maxRetriesExceeded` but without usage-cap vocabulary is retry exhaustion, not an account cap."
    vocabulary: ["UsageCap", "RetriesExhausted", "Overloaded", "RateLimited"]
    confidence: source_code
    evidence: ./fixtures/opencode/429-retry-exhausted.txt
    notes: "Matches the serialized LogClassification output of claudine's promoted-stderr classifier (glue mode); detection vocabulary lives in errors.rs"
  - id: stderr_promoted-provider_overloaded-429
    signal: provider_overloaded
    source: stderr_promoted
    locator: "LogClassification::ProviderLimit(kind=Overloaded)"
    detection: declarative
    priority: 30
    match_path: kind
    match_op: eq
    match_value: Overloaded
    distinguish: "A plain 429 with overload vocabulary (`overload`, `engine_overloaded_error`) is provider capacity pressure, not an account rate limit."
    vocabulary: ["UsageCap", "RetriesExhausted", "Overloaded", "RateLimited"]
    confidence: source_code
    evidence: ./fixtures/opencode/429-overload.txt
    notes: "Matches the serialized LogClassification output of claudine's promoted-stderr classifier (glue mode); detection vocabulary lives in errors.rs"
  - id: stderr_promoted-rate_limited-429
    signal: rate_limited
    source: stderr_promoted
    locator: "LogClassification::ProviderLimit(kind=RateLimited)"
    detection: declarative
    priority: 40
    match_path: kind
    match_op: eq
    match_value: RateLimited
    distinguish: "Generic 429 records are only rate limits after usage-cap, retry-exhaustion, and overload classifiers have failed."
    vocabulary: ["UsageCap", "RetriesExhausted", "Overloaded", "RateLimited"]
    confidence: source_code
    evidence: ./fixtures/opencode/429-plain-rate-limited.txt
    notes: "Matches the serialized LogClassification output of claudine's promoted-stderr classifier (glue mode); detection vocabulary lives in errors.rs"
  - id: stderr_promoted-auth_invalid-invalid-key
    signal: auth_invalid
    source: stderr_promoted
    locator: "LogClassification::AuthFailure"
    detection: declarative
    priority: 5
    match_path: classification
    match_op: eq
    match_value: AuthFailure
    distinguish: "AuthenticationError/unauthorized/fetch-failed LLM records are auth failures rather than transient API failures; the classifier evaluates the auth branch before all five limit branches (errors.rs:326-334)."
    vocabulary: ["AuthFailure"]
    confidence: source_code
    evidence: ./fixtures/opencode/auth-failure-invalid-key.txt
    notes: "Matches the serialized LogClassification output of claudine's promoted-stderr classifier (glue mode); detection vocabulary lives in errors.rs"
  - id: stderr_promoted-provider_version-boot
    signal: provider_version
    source: stderr_promoted
    locator: "LogClassification::BootBanner"
    detection: declarative
    priority: 60
    match_path: classification
    match_op: eq
    match_value: BootBanner
    distinguish: "Boot banners are default-service lifecycle records with a `version` tag and trailing `opencode` keyword; they are not LLM-call starts."
    vocabulary: ["BootBanner"]
    confidence: source_code
    evidence: ./fixtures/opencode/version-announcement.txt
    notes: "Matches the serialized LogClassification output of claudine's promoted-stderr classifier (glue mode); detection vocabulary lives in errors.rs"
  - id: stderr_promoted-model_resolved-llm-call
    signal: model_resolved
    source: stderr_promoted
    locator: "LogClassification::LlmCall"
    detection: declarative
    priority: 70
    match_path: classification
    match_op: eq
    match_value: LlmCall
    distinguish: "`message=stream` with providerID/modelID is a call start and model-resolution observation; `message=\"stream error\"` is a failure path handled by the limit classifiers."
    vocabulary: ["LlmCall"]
    confidence: source_code
    evidence: ./fixtures/opencode/stream-start-1178.txt
    notes: "Matches the serialized LogClassification output of claudine's promoted-stderr classifier (glue mode); detection vocabulary lives in errors.rs"
extractions:
  - record: stderr_promoted-usage_capped-legacy
    field: message
    path: provider_error
  - record: stderr_promoted-usage_capped-legacy
    field: provider
    path: provider_id
  - record: stderr_promoted-usage_capped-legacy
    field: model
    path: model_id
  - record: stderr_promoted-usage_capped-legacy
    field: lifts_at
    path: reset_at
    unit: iso8601
    zone: utc
    notes: "OpenCode provider text has no zone; Claudine's promoted record currently interprets the parsed value as UTC."
  - record: stderr_promoted-usage_capped-1178
    field: message
    path: provider_error
  - record: stderr_promoted-usage_capped-1178
    field: provider
    path: provider_id
  - record: stderr_promoted-usage_capped-1178
    field: model
    path: model_id
  - record: stderr_promoted-usage_capped-1178
    field: lifts_at
    path: reset_at
    unit: iso8601
    zone: utc
    notes: "OpenCode provider text has no zone; Claudine's promoted record currently interprets the parsed value as UTC."
  - record: stderr_promoted-retries_exhausted-429
    field: message
    path: provider_error
  - record: stderr_promoted-retries_exhausted-429
    field: provider
    path: provider_id
  - record: stderr_promoted-retries_exhausted-429
    field: model
    path: model_id
  - record: stderr_promoted-provider_overloaded-429
    field: message
    path: provider_error
  - record: stderr_promoted-provider_overloaded-429
    field: provider
    path: provider_id
  - record: stderr_promoted-provider_overloaded-429
    field: model
    path: model_id
  - record: stderr_promoted-rate_limited-429
    field: message
    path: provider_error
  - record: stderr_promoted-rate_limited-429
    field: provider
    path: provider_id
  - record: stderr_promoted-rate_limited-429
    field: model
    path: model_id
  - record: stderr_promoted-auth_invalid-invalid-key
    field: message
    path: message
  - record: stderr_promoted-provider_version-boot
    field: version
    path: version
  - record: stderr_promoted-model_resolved-llm-call
    field: provider
    path: provider_id
  - record: stderr_promoted-model_resolved-llm-call
    field: resolved
    path: model_id
bespoke_rationale: []
gaps:
  - "The official CLI docs document `--format json`, `--print-logs`, and `--log-level`, but do not define signal semantics or error taxonomy; source code and fixtures are required for the records above."
  - "OpenCode default-branch source exposes token usage on assistant/step-finish message shapes, but this topic has no committed `run --format=json` fixture proving a `step_finish` or assistant message payload. No `tokens_consumed` frontmatter record is emitted yet."
  - "OpenCode default-branch source exposes `session.status` retry payloads with `attempt`, `message`, optional `action.reason`, and `next` epoch milliseconds; no committed JSON stream fixture proves the direct CLI payload, so `generation_retried`, `usage_capped` retry-action variants, and reset extraction from `next` remain gaps."
  - "OpenCode has typed `permission.asked` / `permission.replied` events and `run` auto-rejects permissions unless permission skipping is enabled, but no fixture proves a read-vs-write denial payload suitable for `permission_denied_read` or `permission_denied_write`."
  - "OpenCode session creation and session IDs are visible in events/logs, but no fixture proves resume semantics in a provider stream. `session_resumable` is therefore not recorded."
  - "Provider reset timestamps inside usage-cap prose do not include an explicit timezone. Claudine's current promoted record parses them as UTC; a provider-local-zone confirmation is still needed."
  - "The Git remote tags visible for `anomalyco/opencode` and `sst/opencode` stop at `v1.4.14`, while the official site changelog documents `v1.17.x`. Version drift records use the observed fixture/source terms `pre-1.17.8` and `v1.17.8` but should be rechecked when release tags are available."
  - "No 1.17.8-format fixture exists for retries_exhausted / provider_overloaded / rate_limited / auth_invalid — only usage_capped has the legacy+1.17.8 pair; ./fixtures/opencode/error-new-format-inline-json.txt is uncited."
changes:
  - "2026-07-06: renamed the model_resolved extraction field `model` → `resolved` to match the canonical SignalEvent::ModelResolved payload field (the `model` fields on cap/limit records are unchanged supplementary evidence)."
requires_claudine_update: true
reason: "OpenCode signal detection is codegen-wired and this research adds fixture-backed promoted-stderr records for usage caps, retry exhaustion, rate limits, overloads, auth failure, provider version, and model resolution, plus gaps for stream JSON signals Claudine should capture next."
---

# OpenCode CLI Signal Detection

## Overview

OpenCode exposes three useful signal surfaces for Claudine: promoted stderr logs from `--print-logs --log-level ...`, the `run --format=json` stdout event projection, and the local/server event stream used by the CLI and SDK. The official CLI documentation lists the relevant flags but does not define a signal taxonomy. Source code and fixtures are therefore the authority for operational detection.

The most fixture-ready surface today is stderr log promotion. OpenCode emits structured key/value logs, and Claudine parses those lines into `LogClassification` variants. The JSON event stream is richer for token usage, retries, permissions, and session state, but this topic does not yet have committed OpenCode JSON fixtures for those shapes, so the structured frontmatter records stay limited to promoted stderr detections.

## Signal Surfaces

### Promoted stderr Logs

OpenCode has global `--print-logs` and `--log-level` flags. On the current default branch, those flags set `OPENCODE_PRINT_LOGS` and `OPENCODE_LOG_LEVEL` before command execution. The official CLI docs list the same flags. Claudine treats this as a promoted stderr contract because the wrapper intentionally enables the log stream for machine supervision.

Two log header dialects matter:

| Dialect | Shape | Notes |
| --- | --- | --- |
| Legacy | `ERROR 2026-04-15T19:26:02 +3054ms service=llm ... error={...}` | Timestamp has no fractional seconds and includes a delta field. |
| Newer | `timestamp=2026-06-22T04:07:15.161Z level=ERROR ... message="stream error" ... error.error="..."` | Used by the seeded 1.17.8 fixture; often omits `service=` and stores provider text under a dotted key. |

Claudine's parser preserves unknown tags and normalizes both dialects into `OpenCodeLogRecord` with `level`, `timestamp`, `delta_ms`, `tags`, `message`, and `raw`. Classification then emits `ProviderLimit`, `AuthFailure`, `BootBanner`, `LlmCall`, `SessionCreated`, `PermissionEvaluated`, and related variants.

### JSON stdout Stream

`opencode run --format=json` writes newline-delimited JSON to stdout. The CLI `emit` helper stamps each event with `type`, `timestamp: Date.now()`, `sessionID`, and event-specific data. It currently promotes `tool_use`, `step_start`, `step_finish`, `text`, `reasoning`, and `error`.

This stream is the right future home for `tokens_consumed`: the older v1 message schema has `step-finish` parts with `cost` and `tokens.input/output/reasoning/cache.read/cache.write`, and assistant messages carry the same totals. The current schema package likewise defines assistant `tokens` as input, output, reasoning, and cache read/write. No committed `run --format=json` fixture exists in `fixtures/opencode`, so no frontmatter token record is emitted yet.

### Server and SDK Events

OpenCode's server exposes events through the SDK/event subscription used by `run`. The current source imports shared schema definitions for `session.status`; `retry` status has `attempt`, `message`, optional `action`, and `next`, while `idle` and `busy` are the other status variants. This surface can support `generation_retried` and perhaps future usage-cap action detection, but it needs committed stdout/SSE fixtures before it should drive declarative records.

### Local Logs and Storage

Official troubleshooting docs say logs are written under `~/.local/share/opencode/log/` on macOS/Linux and `%USERPROFILE%\.local\share\opencode\log` on Windows, with the most recent 10 log files kept. Local storage also contains auth data and project/session data. These stores are useful for retrospective enrichment, but Claudine's operational wrapper should prefer live stderr promotion and stdout JSON.

### ACP

OpenCode has ACP code and changelog entries for ACP work, but this research did not find fixture-backed ACP signal records for Claudine's taxonomy. ACP interruption/cancel and usage-update behavior should be researched separately if Claudine adds an ACP source.

## Usage and Rate Limits

The promoted stderr classifier has five ordered LLM-failure branches. The ordering is operationally important:

| Order | Classifier branch | Signal |
| --- | --- | --- |
| 1 | cap vocabulary plus error context | `usage_capped` |
| 2 | HTTP 429 plus `AI_RetryError` or `maxRetriesExceeded` | `retries_exhausted` |
| 3 | cap vocabulary without error context | non-fatal API failure, not a structured cap record |
| 4 | HTTP 429 plus overload vocabulary | `provider_overloaded` |
| 5 | remaining HTTP 429 | `rate_limited` |

Usage caps are recognized from provider vocabulary including ZAI code `1308`, `exceeded_current_quota_error`, `Usage limit reached`, `reached your usage limit`, and `billing cycle`. The Kimi billing-cycle dialect is a real HTTP 403, so the classifier preserves the actual status code instead of forcing all caps to 429.

Retry exhaustion is separate from a usage cap. A line with both cap vocabulary and retry-exhaustion wrappers must resolve to `usage_capped`; a line with retry exhaustion and 429 but no cap vocabulary resolves to `retries_exhausted`.

The current OpenCode retry policy also has native retry metadata. It computes retry delay from `retry-after-ms`, `retry-after`, or exponential backoff, and current source distinguishes `free_tier_limit` from `account_rate_limit` actions for OpenCode Go responses. That data should become stream records after JSON fixtures are captured.

## Authentication and Authorization

Promoted stderr maps authentication failures to `auth_invalid` when the LLM/provider failure line contains `AuthenticationError`, `unauthorized`, `Unauthorized`, or an LLM `fetch failed` condition. The fixture-backed invalid-key line proves the current `AuthFailure` record.

Authorization and permissions are more nuanced. OpenCode has typed `permission.asked` and `permission.replied` events. The non-interactive `run` command auto-rejects permission prompts unless permission skipping is enabled, but the available fixtures do not include a denial payload with enough tool/path detail to split `permission_denied_read` from `permission_denied_write`. The body records the surface; the frontmatter leaves those signals as gaps.

## Model Resolution and Provider Version

OpenCode stderr logs expose model resolution at LLM call start. A `message=stream` / `service=llm` record with `providerID` and `modelID` classifies as `LlmCall`. Claudine records this as `model_resolved` because it is the provider/model pair actually used for the call. The 1.17.8 negative twin matters: `message="stream error"` is not a model-resolution line; it routes to failure classification.

Provider version is exposed by the boot banner. The classifier recognizes a default-service lifecycle line with a `version` tag and trailing `opencode` keyword as `BootBanner`, and extracts the version string.

No fixture-backed `model_fallback` signal was found. OpenCode has model switching/message schema concepts, but the observed records do not prove an automatic fallback from one model to another.

## Token Metering

OpenCode has structured token fields, but they are not frontmatter-ready in this topic. In the older v1 message schema, `StepFinishPart` carries `cost` and `tokens.total/input/output/reasoning/cache.read/cache.write`; assistant messages also carry `cost` and token totals. The current shared schema package defines assistant `tokens.input`, `tokens.output`, `tokens.reasoning`, and `tokens.cache.read/write`.

Because the committed OpenCode corpus currently contains `.txt` stderr fixtures only, no `tokens_consumed` record is emitted. The next evidence capture should run `opencode run --format=json` through a small successful prompt and retain the `step_finish` or assistant update line with user content scrubbed.

## Interruption and Recovery

OpenCode represents retry state natively as `session.status` with `type: retry`, `attempt`, `message`, optional `action`, and `next`. This can map to Claudine's `generation_retried` once a direct stream fixture exists. Terminal retry exhaustion is already represented through promoted stderr when an LLM error line carries both HTTP 429 and retry-exhaustion vocabulary.

OpenCode also has abort/error paths in message schemas and processor code, but no committed fixture proves a provider-emitted `interrupted` signal. Claudine should continue to treat Ctrl+C, SIGTERM, and wrapper cancellation as process-level interruption unless a provider event is captured.

Session IDs appear in logs and JSON events, and OpenCode has session continuation commands, but this research did not find a fixture proving a resumability contract in the signal stream. `session_resumable` remains a gap.

## Version Drift

The critical drift is the stderr header and stream-error payload format:

| Version range | Shape | Impact |
| --- | --- | --- |
| Through observed pre-1.17.8 fixtures | `LEVEL <timestamp> +<delta>ms service=llm ... error=<JSON>` | Service tag and JSON `error=` context are available directly. |
| Observed at 1.17.8 | `timestamp=<iso>Z level=ERROR ... message="stream error" ... error.error="<flat string>"` | No `service=` tag; Claudine infers `llm` from `message` plus `providerID`/`modelID` and reads `error.error`. |

The Git remotes available during this run exposed tags only through `v1.4.14`, but the official OpenCode changelog documents `v1.17.x` releases. The 1.17.8 fixture and Claudine parser comments are therefore treated as observed drift evidence, with a gap to recheck against release tags when they become available.

The current default branch also changed retry semantics by adding structured retry `action` metadata for OpenCode Go limits. That is not yet represented in frontmatter because no JSON fixture exists.

## Quirks and Gaps

Usage caps can look like rate limits. Always run cap detection before generic 429 detection.

Provider reset prose has no timezone. Claudine currently parses reset text as UTC; this is deterministic but may not match the provider's intended local zone.

`--format=json` is promising but under-fixtured. Capture successful `step_finish`, retry `session.status`, auth failure, permission denied, and interruption payloads before adding stream records.

`usage_cap_approaching`, `no_funds`, `model_fallback`, `unsupported_protocol_version`, `turn_limit_reached`, `session_time_limit_reached`, `human_input_requested`, and `session_resumable` were not fixture-backed for OpenCode in this pass.

## Changelog

Initial OpenCode signal research document created on 2026-07-05.

## Sources

- [OpenCode CLI documentation](https://opencode.ai/docs/cli/)
- [OpenCode troubleshooting: logs and storage](https://opencode.ai/docs/troubleshooting/)
- [OpenCode changelog](https://opencode.ai/changelog)
- [OpenCode default-branch CLI flags, commit e0ec9be](https://github.com/anomalyco/opencode/blob/e0ec9be238a1495454e46426665323af25273b63/packages/opencode/src/index.ts#L53-L68)
- [OpenCode default-branch `run --format=json`, commit e0ec9be](https://github.com/anomalyco/opencode/blob/e0ec9be238a1495454e46426665323af25273b63/packages/opencode/src/cli/cmd/run.ts#L174-L179)
- [OpenCode default-branch JSON event emission, commit e0ec9be](https://github.com/anomalyco/opencode/blob/e0ec9be238a1495454e46426665323af25273b63/packages/opencode/src/cli/cmd/run.ts#L678-L786)
- [OpenCode default-branch retry policy, commit e0ec9be](https://github.com/anomalyco/opencode/blob/e0ec9be238a1495454e46426665323af25273b63/packages/opencode/src/session/retry.ts#L10-L190)
- [OpenCode default-branch session status schema, commit e0ec9be](https://github.com/anomalyco/opencode/blob/e0ec9be238a1495454e46426665323af25273b63/packages/schema/src/session-status-event.ts#L9-L41)
- [OpenCode default-branch assistant token schema, commit e0ec9be](https://github.com/anomalyco/opencode/blob/e0ec9be238a1495454e46426665323af25273b63/packages/schema/src/session-message.ts#L164-L189)
- [OpenCode v1.4.14 message-v2 schema, tag v1.4.14](https://github.com/anomalyco/opencode/blob/v1.4.14/packages/opencode/src/session/message-v2.ts#L253-L271)
- [Claudine OpenCode stderr promoted event types](../../../lib/src/stream/logs/opencode/events.rs)
- [Claudine OpenCode stderr classification](../../../lib/src/stream/logs/opencode/errors.rs)
