---
status: draft
created: 2026-06-21
area: claudine
packages:
    - claudine
review_iterations: 2
---

# OpenCode 1.17.8 `stream error` Usage-Cap Detection Drift

## Problem

A wrapped OpenCode run with `zai-coding-plan/glm-5.2` hung indefinitely (~42
minutes before the user pressed Ctrl+C). The wrapper output showed only repeated
`llm_call_start` badges and a growing `Awaiting subagent` spinner:

```text
 llm_call_start zai-coding-plan/glm-5.2 (mode=primary, agent=build)
 ⏳ Awaiting subagent: "New session - 2026-06-22T04:05:30.498Z" (2m 29s)
 llm_call_start zai-coding-plan/glm-5.2 (mode=primary, agent=build)
 ⏳ Awaiting subagent: "New session - 2026-06-22T04:05:30.498Z" (3m 29s)
 ...
 ⏳ Awaiting subagent: "New session - 2026-06-22T04:05:30.498Z" (42m 59s)
```

This was not a true hang. The `build` subagent hit a terminal provider usage
cap, and OpenCode retried it under unbounded exponential backoff. Claudine never
classified the cap, so it never terminated, and the retries themselves kept the
silence timeout from firing.

Session `ses_1127ec2f`, `~/.local/share/opencode/log/opencode.log`. Every stream
failure in the session was identical and terminal:

```text
timestamp=2026-06-22T04:07:15.161Z level=ERROR run=da37e0dd message="stream error" \
  providerID=zai-coding-plan modelID=glm-5.2 session.id=ses_1127ec2fdffepaJc2kEnX093eo \
  small=false agent=build mode=primary \
  error.error="AI_APICallError: Usage limit reached for 5 hour. Your limit will reset at 2026-06-22 13:59:38"
```

The cap reset (`13:59:38`) was ~10 hours past session start, so every retry was
futile. Observed retry gaps doubled cleanly — 3s, 5s, 10s, 17s, 33s, 65s, 129s,
~4m, ~8.5m, ~17m — OpenCode's internal backoff with no observed upper bound.

## Cause

OpenCode 1.17.8 changed its stderr error-log shape. The new `message="stream
error"` record slips through `claudine/lib/src/stream/logs/opencode/errors.rs`
`classify(...)` on three independent points:

1. **No `service=` tag.** `classify(...)` only invokes `classify_llm_failure`
   when the literal `service` tag is `llm` or `provider` (errors.rs:38). The new
   line carries `message="stream error"` with no `service=`. The
   `infer_service_from_message` shim (events/errors) that compensates for the
   new format maps `message="stream"` (the call *start*) to service `llm`, but
   has no arm for `message="stream error"`, and it is only consulted from
   `classify_lifecycle`, never from the error path in `classify(...)`.
2. **Error payload key moved.** Errors used to arrive as `error={…JSON…}`. The
   new shape is a flat `error.error="…"` string (the body parser keeps
   `error.error` as a single dotted key, because `.` is an ident byte, and the
   `error`/`err` swallow rule at events.rs:271 does not match `error.error`).
   Consequently `classify_llm_failure`'s `has_error_context`
   (`record.tags.contains_key("error")`) is false, and `summarize_error_json` /
   `extract_reset_at` read an `error` JSON envelope that no longer exists.
3. **New cap dialect.** The message is `Usage limit reached for 5 hour` (the
   `for N hour` qualifier is new). The existing `Usage limit reached` substring
   needle (errors.rs:312) would still match, but only if the classifier path is
   reached — which, per (1), it is not.

Net effect: the record classifies as `Unclassified`. A correctly-detected
`UsageCap` fires a terminal `SemanticEvent::Error { terminal: true }`
(reasoning.rs:435) that requests early termination on the first error
(`04:07:15`).

### Why the timeouts did not catch it

`step_timeout` is 30m of *stream silence*. Each OpenCode retry re-emits
`message="stream"`, which classifies as `LlmCall` (rendering `llm_call_start`)
and resets the silence timer. The largest observed retry gap (~17m) never
crossed 30m, so the timeout never tripped. No wall-clock `timeout` was set, so
that backstop was also absent. See
[Timeouts](../../docs/topics/timeouts.md).

## Goals

1. An OpenCode `message="stream error"` record carrying a usage-cap signal must
   classify as `LogClassification::ProviderLimit { kind: UsageCap, .. }`, even
   when the line has no `service=` tag and the payload is under `error.error`.
2. That classification must drive the existing terminal early-termination path
   so the wrapper aborts the session on the first cap error rather than waiting
   through OpenCode's retries.
3. `reset_at`, `provider_id`, and `model_id` must still be extracted from the
   new line shape so the rendered message keeps its reset-time context.
4. Add a defense-in-depth backstop so a repeated, unclassified terminal stream
   error cannot indefinitely out-wait the silence timeout via retry-driven
   `llm_call_start` resets.

## Non-Goals

- Do not change the happy-path `message="stream"` (call start) handling that
  renders `llm_call_start`.
- Do not redesign the `LogClassification` taxonomy or the semantic-event bridge.
- Do not alter `step_timeout` / `timeout` precedence or env-var semantics beyond
  the targeted backstop in Goal 4.
- Do not add provider-specific cap vocabulary beyond what is needed to cover the
  observed ZAI `Usage limit reached for N hour` dialect (the existing substring
  needle already covers the core phrase).

## Proposed Design

All changes are in `claudine/lib/src/stream/logs/opencode/`.

### 1. Route `message="stream error"` into the failure classifier

In `errors.rs` `classify(...)`, stop gating `classify_llm_failure` solely on the
literal `service` tag. Treat a record as an LLM-failure candidate when either:

- the literal `service` tag is `llm` / `provider` (existing behavior); or
- the inferred service is `llm` (reuse `infer_service_from_message`); or
- `message` (tag or trailing) is `stream error` with `providerID` + `modelID`
  present.

Extend `infer_service_from_message` with a `"stream error"` arm that returns
`llm` when `providerID` and `modelID` are present, mirroring the existing
`"stream"` arm. Then have `classify(...)` consult the inferred service for the
failure path, not just `classify_lifecycle`.

### 2. Accept the flat `error.error` payload

`classify_llm_failure` and its helpers currently assume the error text lives in
the `error` tag (JSON) or `record.raw`. Update them so the `error.error` tag is
a recognized error-context source:

- `has_error_context` should be true when `error`, `err`, **or** `error.error`
  is present.
- The cap/needle scan already runs over `record.raw`, so `Usage limit reached`
  continues to match. Confirm `for 5 hour` does not defeat the substring match
  (it does not).
- `extract_reset_at` already scans `haystack` (`record.raw`) with `RESET_AT_RE`
  (`reset at YYYY-MM-DD HH:MM:SS`), which matches the new line. Verify it
  resolves `2026-06-22 13:59:38`.
- When building the `ProviderLimit`, populate `provider_error` from
  `error.error` (falling back to `error` / raw) and keep `provider_id` /
  `model_id` from the `providerID` / `modelID` tags.
- `status_code` will be `None` for this shape (no `statusCode`); that is
  acceptable — `UsageCap` terminality does not depend on a status code.

Resolution order must remain: cap-with-context wins over retries-exhausted, as
documented at errors.rs:318.

### 3. Silence-timeout backstop for repeated terminal stream errors

Independent of classification, add a guard so consecutive identical
`message="stream error"` records do not let retry-driven `llm_call_start`
events reset the silence timer forever. Preferred approach (least invasive):
once a stream error is classified terminal (`UsageCap` / `RetriesExhausted`),
the existing terminal `SemanticEvent::Error` already aborts — so with fix (1)+(2)
this path is covered for the cap case. The backstop targets the *residual*
unclassified case:

- Track a per-session counter of consecutive `stream error` records that did not
  advance a step (no intervening `StepLoop` / successful stream completion).
- When the counter crosses a threshold (proposed: 5), emit a terminal
  `SemanticEvent::Error { terminal: true, kind: ApiRemote }` with a message
  noting repeated stream errors, so the wrapper aborts even if the specific
  error vocabulary is unrecognized.

Keep the threshold a named constant with a comment tying it to this fix. This
ensures a *future* OpenCode format drift degrades to a bounded failure, not an
indefinite hang.

## Tests

### Unit — classification (`errors.rs`)

Build fixtures from the real captured line.

1. The new line shape classifies as `ProviderLimit { kind: UsageCap, .. }`:

   ```text
   timestamp=2026-06-22T04:07:15.161Z level=ERROR run=da37e0dd message="stream error" \
     providerID=zai-coding-plan modelID=glm-5.2 session.id=ses_x small=false \
     agent=build mode=primary \
     error.error="AI_APICallError: Usage limit reached for 5 hour. Your limit will reset at 2026-06-22 13:59:38"
   ```

   Assert: `kind == UsageCap`, `reset_at` parses to `2026-06-22 13:59:38Z`,
   `provider_id == "zai-coding-plan"`, `model_id == "glm-5.2"`, and
   `provider_error` contains `Usage limit reached`.

2. The matching call-start line still classifies as `LlmCall` (no regression):

   ```text
   ... message="stream" providerID=zai-coding-plan modelID=glm-5.2 ... agent=build mode=primary
   ```

3. The legacy `service=llm error={…JSON…
   code:1308…}` fixtures (existing tests) still classify as `UsageCap` — keep
   them green to prove backward compatibility.

### Unit — semantic bridge (`reasoning.rs`)

4. Ingesting the new `stream error` line emits a terminal
   `SemanticEvent::Error { terminal: true }` requesting early termination, even
   if stdout has already been seen (mirror the existing
   `early-termination signal expected for UsageCap` test at reasoning.rs:1221).

### Unit — backstop

5. Feeding N consecutive *unrecognized* `stream error` records (a synthetic
   vocabulary the cap needles do not match) with no intervening step advance
   emits a terminal error once the threshold is crossed; fewer than the
   threshold does not.

### Manual validation

Record one manual check against the captured log line (or a replay fixture
derived from it) confirming the wrapper aborts on the first cap error instead of
spinning on `Awaiting subagent`.

## Acceptance Criteria

1. The captured `message="stream error"` + `error.error="… Usage limit reached
   for 5 hour …"` line classifies as `ProviderLimit { kind: UsageCap }` with
   correct `reset_at`, `provider_id`, `model_id`.
2. That classification drives a terminal `SemanticEvent::Error { terminal: true }`
   and the wrapper terminates on the first cap error.
3. The `message="stream"` call-start path is unchanged (`LlmCall` /
   `llm_call_start`).
4. Legacy `service=llm error={JSON}` cap fixtures still classify as `UsageCap`.
5. Repeated unrecognized terminal stream errors trip a bounded backstop rather
   than hanging indefinitely.
6. `just test` passes in the `claudine` package area.

## Implementation Notes

- Source of truth for the drift is OpenCode **1.17.8** (`version=1.17.8` in the
  session's `created` log line). Note the version in a code comment so future
  drift is traceable.
- Reuse `infer_service_from_message`, `extract_reset_at`, and the existing
  needle scan rather than adding parallel matchers.
- Do not run `cargo fmt` in write mode as part of this fix unless explicitly
  requested.
- Background reference: memory `project_claudine_opencode_streamerror_format_drift`,
  and prior art in `project_claudine_opencode_error_parsing`.
