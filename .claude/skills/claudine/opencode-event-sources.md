---
hash: 39a0c5d58ef53df2-dc0521dad5458b1b
---

# OpenCode Event Sources

OpenCode is the only provider Claudine wraps where **stdout NDJSON alone is
insufficient** to drive the live renderer, the silence watchdog, and the
end-of-run summary. The stream is "DONE-only": tool starts and subagent
starts are never on the wire — Claudine sees the after-the-fact `tool_use`
and never sees a paired request-side event. To compensate, Claudine
classifies the structured **stderr log stream** (opted into with
`--print-logs --log-level INFO`) and promotes selected log records to
`SemanticEvent`s alongside the NDJSON ones.

The two streams are a **Dual-Source Contract**: stdout NDJSON carries the
canonical session outcome and token usage; stderr carries the lifecycle
beat (boot, sessions, LLM calls, step loops, permissions, HTTP) that lets
the renderer report progress while NDJSON is silent.

## Why the stderr source is mandatory for OpenCode

- **No `tool_start`.** OpenCode emits `tool_use` only after a tool reaches
  `completed` / `error`. There is no request-side event, so
  `LiveMetricsState.in_flight` is never populated for OpenCode and the
  in-flight gate of `step_timeout` is a no-op.
- **No native `task_started`.** Although the variant is recognised by the
  protocol parser, current OpenCode releases do not emit it. Subagent
  lifecycle visibility comes from stderr `service=session ... parentID=...`
  records and the subsequent `service=session.prompt ... exiting loop`
  closure.
- **No `init` payload in many runs.** The primary provider/model identity
  for the run is reliably reachable only via the first
  `service=llm ... mode=primary stream` stderr line.
- **Long synthesis silences.** Between the last `tool_use` and the next
  `text` chunk the NDJSON stream can go silent for minutes; the raw-byte
  heartbeat protects the silence watchdog but does not feed semantic
  rendering. Stderr fills that semantic gap.

See [timeouts.md](../../../claudine/docs/topics/timeouts.md) for the full
silence-rule story and the per-step `provider_status` grace.

## Configuration

The wrapper enables structured stderr by passing two flags to every
non-interactive `opencode run` invocation:

```
--print-logs
--log-level INFO
```

INFO is the minimum level that includes the lifecycle records below
(`SessionCreated`, `LlmCall`, `StepLoop`, `StepExit`, `PermissionEvaluated`,
`HttpResponse`). `service=bus` lines are aggressively filtered before
classification — they refresh the raw-byte heartbeat but never emit a
semantic event.

## Signal → Classification → SemanticEvent

The table below is the authoritative mapping from stderr log lines (as
classified by [`classify_lifecycle`](../../../claudine/lib/src/stream/logs/opencode/errors.rs))
to the semantic events emitted by [`OpenCodeLogBridge`](../../../claudine/lib/src/stream/logs/opencode/reasoning.rs).

| Stderr line shape | `LogClassification` variant | `SemanticEvent` emitted | Notes |
|---|---|---|---|
| `service=default version=... opencode` | `BootBanner { version }` | *(none)* | Parsed and counted; never promoted. |
| `service=session id=... title=... created` *(no `parentID`)* | `SessionCreated { id, parent_id: None }` | `SessionStart { session_id, model: None }` | Cross-stream dedup: skipped if stdout already emitted any event, or if a primary session was already emitted. |
| `service=session id=... parentID=... title=... created` | `SessionCreated { id, parent_id: Some(_) }` | `SubagentStart { name: title, id }` | Tracked in `child_sessions` so the matching `StepExit` can synthesize `SubagentStop`. |
| `service=llm providerID=... modelID=... mode=... ... stream` | `LlmCall { provider_id, model_id, mode, is_stream: true }` | `Info { message: "llm_call_start", extra: { provider_id, model_id, mode, is_stream, agent?, small?, session_id? } }` | The first `mode=primary` observation backfills `summary.model` and the primary provider id. |
| `service=session.prompt session.id=... step=N loop` | `StepLoop { session_id, step }` | `Info { message: "step_loop step=N session=<id>", extra: { session_id, step } }` | OpenCode emits one of these per HTTP-span boundary inside the same reasoning step; the bridge dedupes on `(session_id, step)` so only genuine step transitions surface. The dedup entry is cleared on `StepExit` so a follow-up prompt on the same session starts fresh. |
| `service=session.prompt session.id=... exiting loop` | `StepExit { session_id }` | `Info { message: "exiting_loop session=<id>", extra: { session_id } }` + (if `session_id` is a tracked child) `SubagentStop { name, id }` | The longer keyword is matched first so "exiting loop" never falls through to `StepLoop`. Child stop fires at most once per child. |
| `service=permission permission=... pattern=... action=... evaluated` | `PermissionEvaluated { permission, pattern, action }` | `Info { message: "permission_evaluated", extra: { permission, pattern, action } }` | |
| `service=default http.method=... http.url=... http.status=... ... Sent HTTP response` | `HttpResponse { method, url, status, duration_ms }` | `Info { message: "http_response", extra: { method, url, status, duration_ms } }` | `duration_ms` is sourced from the first `logSpan.http.span.*` tag. |
| `service=bus ...` | *(filtered)* | *(none)* | Refreshes `last_byte_at` only. |

Failure-shaped lines (`ProviderLimit`, `MalformedAsset`, `ApiFailure`,
`AuthFailure`, `UncaughtError`) continue to route through their existing
`Warning` / `Error` handlers and are documented in
[`reasoning.rs`](../../../claudine/lib/src/stream/logs/opencode/reasoning.rs).

## Failure Classifications

The `ProviderLimit` classification replaces the earlier monolithic `RateLimit`
variant with a four-kind model that distinguishes **provider capacity** from
**account consumption** on two independent axes.

| `ProviderLimitKind` | Axis | Terminal? | Semantic Event | Notes |
|---|---|---|---|---|
| `Overloaded` | Provider capacity | No | `Warning { "server overloaded; will retry" }` | The provider's servers are busy. Transient, retryable, not a cap. Does not set `state.rate_limit` or trigger early termination. |
| `RateLimited` | Account consumption (speed) | No | `Warning { "request throttled; will retry" }` | This account sent requests too fast. Transient, retryable. Does not set `state.rate_limit` or trigger early termination. |
| `UsageCap` | Account consumption (allowance) | Yes | `Error { "Usage limit reached..." }` | The account's usage allowance is exhausted. Sets `state.rate_limit` and triggers early termination regardless of whether stdout output was already observed. |
| `RetriesExhausted` | Provider capacity + account consumption | Yes | `Error { "provider 429s did not clear after retries" }` | A 429 wrapped in `AI_RetryError` / `maxRetriesExceeded` — the call failed after exhausting retries. Sets `state.rate_limit` and triggers early termination. |

### Resolution order

The classifier applies these rules in strict priority order inside
`classify_llm_failure`:

1. **Cap with context** — `has_cap` ( `"code":"1308"`, `exceeded_current_quota_error`, or `"Usage limit reached"` ) **AND** the record carries an `error` tag → `UsageCap`.
2. **Retry exhaustion** — `status_code == 429` AND (`AI_RetryError` OR `maxRetriesExceeded`) → `RetriesExhausted`.
3. **Cap without context** — `has_cap` but NO `error` tag → non-fatal `ApiFailure` (advisory path).
4. **Plain overload** — `status_code == 429` AND `is_overload` (case-insensitive match for `overload` / `engine_overloaded_error`) → `Overloaded`.
5. **Plain rate limit** — `status_code == 429` with none of the above → `RateLimited`.

The **error-context gate** (`record.tags.get("error").is_some()`) is the
primary defense against false-positive termination. Only the presence of an
`error` tag proves the line came from an OpenCode error envelope rather than
echoed or quoted text.

### The `kimi-for-coding` gap

The `kimi-for-coding` provider endpoint (used for the `k2p6` model family)
does **not** emit a confirmed cap type. A real allowance exhaustion on this
endpoint surfaces as `RetriesExhausted` (a 429 wrapped in `AI_RetryError`
after retries are exhausted), not `UsageCap`. This is a known limitation:
without a provider-side `1308` code or `exceeded_current_quota_error` signal,
the classifier cannot distinguish a coding-endpoint cap from a generic retry
exhaustion.

## Deduplication strategy

The dual sources can describe the same logical event. The bridge enforces
**first-arrival-wins** with structural guards:

- **Primary `SessionStart`.** The bridge emits at most one stderr-derived
  `SessionStart`. If the stdout NDJSON stream emits any semantic event
  before the stderr session record arrives, the stderr `SessionStart` is
  recorded but not re-emitted. The wrapper layer no longer synthesizes a
  subagent lifecycle from completed `task` tool_use payloads — the
  stderr-derived `SubagentStart`/`SubagentStop` pair is now the single
  source of truth.
- **Subagent stop idempotence.** Each child session is allowed exactly
  one `SubagentStop`. OpenCode emits an `exiting loop` record for every
  step within a child session; only the first crosses into a
  `SubagentStop` event. Subsequent `StepExit`s still emit the `Info`
  envelope but skip the subagent stop.
- **Summary backfill, not overwrite.** The merge path
  (`merge_stderr_state_into_summary`) only sets `summary.model` when it
  is `None`. A model identified on stdout (e.g. by an `init` payload)
  takes precedence over the stderr LLM-call observation.

## Watchdog interaction

Every stderr-derived `Info` event refreshes `last_event_at` via
`LiveMetricsState::observe_event` because `Info` is in the
`SemanticEvent::is_activity()` set. This means the silence rule
(`step_timeout`) sees genuine progress during long NDJSON silences —
typically the multi-minute closing-synthesis turn after a `task` fan-out.

The byte heartbeat (`last_byte_at`) still fires at the stderr reader
layer **before** the bridge runs, so even filtered (`service=bus`) lines
keep the silence clock fresh as a backstop. A child that produces zero
bytes on either channel still allows `step_timeout` to fire normally.

## End-of-run summary enrichment

When the stdout NDJSON stream does not include a model identity (the
"DONE-only" stream shape), `merge_stderr_state_into_summary` backfills
`summary.model` from the first `mode=primary` LLM-call observation on
stderr. The shared state also exposes `primary_provider_id` for downstream
consumers that need the provider attribution even when the wire-level
NDJSON omits it.

## Related files

- [`claudine/cli/src/commands/wrap/profile/opencode.rs`](../../../claudine/cli/src/commands/wrap/profile/opencode.rs) — argv configuration (`--print-logs --log-level INFO`).
- [`claudine/lib/src/stream/logs/opencode/events.rs`](../../../claudine/lib/src/stream/logs/opencode/events.rs) — JSONL header / body parser and `LogClassification` enum.
- [`claudine/lib/src/stream/logs/opencode/errors.rs`](../../../claudine/lib/src/stream/logs/opencode/errors.rs) — `classify` / `classify_lifecycle` / failure classifiers.
- [`claudine/lib/src/stream/logs/opencode/reasoning.rs`](../../../claudine/lib/src/stream/logs/opencode/reasoning.rs) — `OpenCodeLogBridge` and `merge_stderr_state_into_summary`.
- [`claudine/lib/src/stream/providers/opencode.rs`](../../../claudine/lib/src/stream/providers/opencode.rs) — NDJSON parser; no longer synthesizes `SubagentStart`/`SubagentStop` from `task` tool_use.
- [`claudine/docs/topics/timeouts.md`](../../../claudine/docs/topics/timeouts.md) — `step_timeout`, byte heartbeat, per-step grace.
- [`claudine/docs/research/agent-cli/opencode.md`](../../../claudine/docs/research/agent-cli/opencode.md) — research source for the stderr schema.
