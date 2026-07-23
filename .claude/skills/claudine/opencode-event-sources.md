---
hash: ef46db3751d8e999-4c29486ecc27d639
last_updated: 2026-07-22
---

# OpenCode Event Sources

## Contents

- Why the stderr source is mandatory for OpenCode
- Configuration
- Signal → Classification → SemanticEvent
- Failure Classifications
- Deduplication strategy
- Watchdog interaction
- End-of-run summary enrichment
- Related files

Use heading search to jump to the listed subsystem.


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

See [timeouts.md](timeouts.md) for the full
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
classified by `classify_lifecycle`)
to the semantic events emitted by `OpenCodeLogBridge`.

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
`bridge/mod.rs`.

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

1. **Cap with context** — `has_cap` ( `"code":"1308"`, `exceeded_current_quota_error`, or `"Usage limit reached"` ) **AND** the record carries an error-context tag → `UsageCap`.
2. **Retry exhaustion** — `status_code == 429` AND (`AI_RetryError` OR `maxRetriesExceeded`) → `RetriesExhausted`.
3. **Cap without context** — `has_cap` but no error-context tag → non-fatal `ApiFailure` (advisory path).
4. **Plain overload** — `status_code == 429` AND `is_overload` (case-insensitive match for `overload` / `engine_overloaded_error`) → `Overloaded`.
5. **Plain rate limit** — `status_code == 429` with none of the above → `RateLimited`.

The failure path runs whenever the **effective** service is `llm`/`provider` —
the literal `service=` tag, or, when absent, the value inferred from the
`message` tag. OpenCode **1.17.8** emits stream failures as `message="stream
error"` with no `service=` tag and the payload nested under `error.error="…"`
(a flat string, not the legacy `error={JSON}` envelope), so both the
service-inference and the error-context lookup must cover that shape.

The **error-context gate** (`error_context(record).is_some()`, checking `error`,
`error.error`, then `err`) is the primary defense against false-positive
termination. Only the presence of an error-context tag proves the line came
from an OpenCode error envelope rather than echoed or quoted text.

### Repeated-stream-error backstop

Independent of classification, the bridge counts consecutive `message="stream
error"` records with no intervening step advance. Crossing
`MAX_CONSECUTIVE_STREAM_ERRORS` (5) emits a terminal
`Error { "provider stream failed N times…" }` and fires
`EarlyTermination::RepeatedStreamError` (`error_kind = "repeated_stream_error"`,
maps to `ProcessTermination::Aborted` → fail-fast `AgentFailure`). This bounds
OpenCode's unbounded backoff retries so a *future* error vocabulary the
classifier does not recognize as terminal still degrades to a bounded abort
rather than an indefinite hang. The counter resets on any genuine step
transition. See `fixes/_completed/2026-06-21-opencode-log-fix`.

### Stalled-generation backstop (live-but-dead guard)

A second, independent stderr backstop catches the **live-but-dead** shape: a
run that keeps retrying a *dropped generation*. OpenCode re-emits a
`service=llm ... stream` (`LlmCall` with `is_stream == true`) record on every
retry, and those retries keep the byte heartbeat alive, so `step_timeout`
never fires — yet no assistant text, tool call, or step advance is produced.
The classifier sees no terminal error, so `RepeatedStreamError` does not fire
either; the run is alive on the wire but producing nothing.

The bridge counts streamed `LlmCall` records since the last progress event in
`generation_count_since_progress`. The guard trips only when **both**
conditions hold on the same `llm_call_start`:

1. **Retry churn** — `generation_count_since_progress >= MAX_GENERATIONS_WITHOUT_PROGRESS` (`4`).
2. **Progress silence** — `now - last_progress_at >= stall_timeout` (default `10m`).

Either condition alone never fires — the count condition exempts a single
legitimately-slow generation, and a long tool emitting **no** `LlmCall`
records never trips even past `stall_timeout`. The two conditions are
anti-correlated with healthy output: any stdout-origin progress would reset
the count before it could reach the threshold.

**Reset taxonomy.** The two progress clocks (`last_progress_at` +
`generation_count_since_progress`) live in a shared `StalledGenerationProgress`
cell so both producers can clear them. On the **stderr** bridge,
`reset_stalled_generation_progress` advances `last_progress_at` and zeroes the
count on the bridge-visible progress class — a genuine `StepLoop` advance
(after the `(session_id, step)` dedup passes), `StepExit`, and subagent
lifecycle (`SubagentStart` / `SubagentStop`). On the **stdout** NDJSON stream
(a different reader thread), a `StalledProgressObserverSink` built from the same
cell resets it for each progress-class semantic event
(`SemanticEvent::is_stdout_progress_class`: `OutputText`, `Reasoning`,
`ToolCall`, `ToolResult`, `SubagentStart`, `SubagentStop`, `FileChange`,
`PlanUpdate`). Without the stdout-side reset a run could make real stdout
progress and still trip on a later `llm_call_start`, because the stderr bridge
never sees stdout events. Liveness-only events do **not** reset on either
producer: another `LlmCall`, a deduped/repeated `StepLoop` for the same
`(session_id, step)`, `http_response`, `permission_evaluated`, `service=bus`
lines, raw bytes, and the stdout `Info`/`Warning`/`Error`/session-turn-envelope
events.

On trip the bridge emits a terminal `SemanticEvent::Error`
(`SemanticErrorKind::AgentNative`, label **"Stalled Generation"**, carrying
`generation_count` and the stall duration plus safe OpenCode metadata —
session id, step, agent, provider id, model id, mode; never prompt text or
tool payloads) and fires `EarlyTermination::StalledGeneration`
(`error_kind = "stalled_generation"`, maps to `ProcessTermination::Aborted` →
fail-fast `AgentFailure`, **never** `handle_timeout:`). The two backstops are
fully independent: `LlmCall` churn never clears `consecutive_stream_errors`,
and a `stream error` never clears `generation_count_since_progress`. Both
share the bridge's single `fire_early_termination` idempotency, so at most one
terminal abort is emitted per bridge. See
[timeouts.md — OpenCode stalled-generation backstop](timeouts.md#opencode-stalled-generation-backstop)
and the spec.

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

- `claudine/cli/src/commands/wrap/profile/opencode.rs` — argv configuration (`--print-logs --log-level INFO`).
- `claudine/lib/src/stream/logs/opencode/events.rs` — JSONL header / body parser and `LogClassification` enum.
- `claudine/lib/src/stream/logs/opencode/classify/mod.rs` — `classify` / `classify_raw` / failure classifiers and `merge_rate_limit`.
- `claudine/lib/src/stream/logs/opencode/bridge/mod.rs` — `OpenCodeLogBridge` (stderr bridge).
- `claudine/lib/src/stream/logs/opencode/state.rs` — `SharedStderrState` and `merge_stderr_state_into_summary`.
- `claudine/lib/src/stream/providers/opencode.rs` — NDJSON parser; no longer synthesizes `SubagentStart`/`SubagentStop` from `task` tool_use.
- [`timeouts.md`](timeouts.md) — `step_timeout`, byte heartbeat, per-step grace.
- `claudine/docs/research/agent-cli/opencode.md` — research source for the stderr schema.