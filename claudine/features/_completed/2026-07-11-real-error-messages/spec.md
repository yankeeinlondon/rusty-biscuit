---
implemented: true
review_iterations: 3
---
# Real Error Messages on Failure

## Problem

When a provider exits non-zero, the `err.msg` rendered in lifecycle `failure`
events is:

> `agent exited with error code 1 (attempt 1)`

This is utterly useless. The operator learns nothing about *why* the run failed
— not the provider's own error text, not the stderr output, not which guard
tripped. Worse, the `(attempt 1)` suffix implies a retry is imminent when, in
the common case (no `retry`/`resume` lifecycle control configured), the run
exits after the first failure and `attempt` never increments.

## Root Cause

The message is constructed at
`claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:986-990`:

```rust
claudine::harness::FailureEvent::AgentFailure => {
    format!(
        "agent exited with error code {} (attempt {attempt})",
        outcome.exit_code
    )
}
```

It uses only `outcome.exit_code` (a raw integer) and `attempt` (a counter that
only increments via lifecycle `retry`/`resume` control actions).

### Data available but unused

The `AttemptOutcome` struct (`claudine/lib/src/harness/model.rs:68`) already
carries:

| Field | Type | Content |
|-------|------|---------|
| `stderr_text` | `Option<String>` | Provider stderr output (already noise-filtered — the capture/stream paths apply the per-provider `stderr_noise_prefixes` before storing it) |
| `error_kind` | `Option<String>` | Classification label — guard/timeout labels (`step_timeout`, `runaway_repetition`, …) **and** provider-semantic kinds the stream parsers set (`rate_limit`, `billing_error`, `unauthorized`, …) |
| `guard_context` | `Option<GuardContext>` | Structured guard trip detail (pattern, cycle length, volume counters, stall duration) |
| `final_response` | `String` | Assistant's last text |
| `termination` | `ProcessTermination` | How it ended (`LaunchFailed`, `Aborted`, `TimedOut`, `Completed`) |

Note the `AttemptOutcome.error_kind` doc comment currently claims it is only
populated for guard/timeout labels — in reality `summary.error_kind` also
carries provider-semantic kinds set by every stream parser (see
`claude.rs:196`, `codex.rs:182`, `opencode.rs:261`, `kimi.rs:299`, …). The
doc comment should be corrected as part of this work; those recognized labels
already project `err.code`/`err.category` facets via `code_for_error_kind`,
which is why the message cascade below only needs to fix `err.msg`.

### Data dropped before reaching `AttemptOutcome`

`StreamExecutionSummary` (`claudine/lib/src/stream/summary.rs:57`) additionally
has:

| Field | Type | Content |
|-------|------|---------|
| `error_message` | `Option<String>` | Provider's own error message |
| `stderr_diagnostics` | `Option<StderrDiagnostics>` | Structured stderr **counters** (`rate_limit_events`, `auth_failures`, …) — classification hints only; it carries no text and is **not** a message source |
| `is_error` | `bool` | Whether the provider flagged the session as errored |

Every provider stream parser populates `error_message`:

| Provider | Source |
|----------|--------|
| Claude | `claude.rs:351` — `detail.and_then(\|d\| d.message)` |
| Codex | `codex.rs:586` — `self.error_message` |
| OpenCode | `opencode.rs:517` — `self.error_message` |
| Gemini | `gemini.rs:412` — `self.error_message` |
| Kimi | `kimi.rs:1077` — `self.error_message` |
| Qwen | `qwen.rs:311` — `self.error_message` |
| Pi | `pi.rs:372` — `self.error_message` |
| Antigravity | `antigravity.rs:222` — `this.error_message` |

Typical contents: `"Too many requests"`, `"Insufficient credits"`,
`"API timeout"`, `"Billing error"`.

But `build_attempt_outcome` (`claudine/lib/src/harness/runtime.rs:55-68`) and
the wrapper's `execute_harness_attempt`
(`claudine/cli/src/commands/wrap/harness_orch/attempt.rs:409-418`) **never
propagate `error_message`** to `AttemptOutcome`. It is dropped on the floor.

### The `(attempt N)` suffix

The `attempt` counter only increments when a lifecycle `retry`/`resume` control
action dispatches `NextAttempt`
(`claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs:309`).
Without a recovery action configured, `attempt` is always 1, the run exits
after the first failure, and `(attempt 1)` implies retries that never happen.

## Solution

### Part 1 — Propagate `error_message` to `AttemptOutcome`

Add `pub error_message: Option<String>` to `AttemptOutcome`
(`claudine/lib/src/harness/model.rs:68`).

Populate it in:

- `build_attempt_outcome` (`claudine/lib/src/harness/runtime.rs:55`) —
  `summary.error_message.clone()`
- `execute_harness_attempt` structured-stream path
  (`claudine/cli/src/commands/wrap/harness_orch/attempt.rs:293-303`) —
  `summary.error_message.clone()`
- Capture path (`attempt.rs:344-354`) — `None` (no stream parser)
- Interactive TUI path (`attempt.rs:382-392`) — `None` (no stream parser)

### Part 2 — Build a richer failure message

**RULED (2026-07-11):** the message construction moves out of the CLI into a
pure, unit-testable library function — e.g.
`claudine::harness::failure_message(&AttemptOutcome, attempt: u32) -> String`
— living next to `classify_failure` (in `harness/runtime.rs` or
`harness/report.rs`). Rationale:

- The cascade has real logic (priority selection, stderr line extraction,
  sanitization) that deserves table-style unit tests, not L2-only coverage.
- A single construction point serves both consumers: the
  `LifecycleErrorInfo` built for the `failure` event **and** the
  `report_unhandled_failure` stderr banner. They must never diverge.
- The upcoming signal-assurance handling surface (see
  [Interaction with signal-assurance](#interaction-with-signal-assurance)
  below) can reuse the same builder from lib when it lands.

The CLI arms at `loop_control.rs:983-992` collapse to calls into this
function.

#### The cascade

**As implemented** (`harness/runtime.rs::failure_message`). The staleness
concern behind "guard outranks provider message on `Aborted`" turned out to
be structurally impossible: on the structured path,
`apply_early_termination_to_summary` **overwrites** `summary.error_message`
with the guard/timeout prose at trip time, so on `Aborted` the
`error_message` *is* the guard message (and richer than anything
reconstructable from `GuardContext`). The implemented order therefore
leads with `error_message`:

1. **Error message** (`outcome.error_message`) — the provider's own error
   text (`"Too many requests"`, `"Insufficient credits"`), or the wrapper's
   synthesized guard/timeout prose on the structured path.

2. **Guard context** (`outcome.guard_context`, when
   `termination == Aborted`) — render the trip from structured detail
   (exit-expression pattern, repetition cycle, volume counters, stall
   duration). Reached only on the capture path, which has no stream
   summary and therefore no synthesized `error_message`.

3. **Timeout phrasing** (when `termination == TimedOut`) — from
   `error_kind` (`"step_timeout"` vs wall-clock) plus the configured
   `timeout_secs`. Reached only on the capture/interactive paths (the
   structured path's timeout message arrives via source 1).

4. **Stderr text** (`outcome.stderr_text`) — the last non-empty line after
   sanitization (see [Message hygiene](#message-hygiene)); `stderr_text` is
   already noise-filtered by the per-provider `stderr_noise_prefixes`, so
   no second noise pass is needed.

5. **Termination label / fallback** — `"failed to launch provider process"`
   for `LaunchFailed`, `"aborted by content guard"` for a context-less
   `Aborted`, else the generic exit-code message.

The `(attempt N)` suffix is appended **only when `attempt > 1`** — uniformly,
whatever cascade step produced the text. When `attempt == 1` the suffix is
misleading (no retry may ever happen) and must be omitted.

#### Message hygiene

**RULED (2026-07-11):** `err.msg` feeds TTS (`say`), Discord/Slack/Signal
routes, desktop notifications, and the stderr banner. Provider
`error_message` and stderr lines can be multiline JSON blobs with ANSI
escapes; a 4 KiB JSON payload read aloud by `say` is a real failure mode.
The builder therefore enforces, on provider-derived text (cascade steps 2
and 3 — guard/termination/fallback messages are claudine-authored and
already short):

- **ANSI/OSC escape stripping** — reuse the existing escape-strip helper
  rather than adding a new regex.
- **Single line** — collapse to the first meaningful line of a multiline
  message; for the stderr fallback, the *last* non-empty line (providers
  print the fatal error last).
- **Length cap ~240 chars** — truncate with a trailing `…`. The full text
  remains available in `stderr_text`, the JSONL logs, and the stream
  summary; `err.msg` is the *headline*, not the archive.

#### Message shape

The message should read as a human-readable sentence, not a format string.
Examples of the desired output:

| Scenario | Current | Proposed |
|----------|---------|----------|
| Rate limit | `agent exited with error code 1 (attempt 1)` | `Too many requests` |
| Insufficient credits | `agent exited with error code 1 (attempt 1)` | `Insufficient credits` |
| Exit-expression guard | `agent exited with error code 1 (attempt 1)` | `exit expression matched: "STOPWIRE"` |
| Runaway repetition | `agent exited with error code 1 (attempt 1)` | `runaway repetition detected (cycle length 4, 35 repeats)` |
| Volume cap | `agent exited with error code 1 (attempt 1)` | `output volume cap exceeded (52,000 lines / 34 MiB)` |
| Stalled generation | `agent exited with error code 1 (attempt 1)` | `stalled generation (5 attempts without progress, 10m silence)` |
| Step timeout | `provider timed out (attempt 1)` | `step timeout (no output for 30m)` |
| Generic stderr | `agent exited with error code 1 (attempt 1)` | `<last non-trivial stderr line>` |
| Launch failed | `agent exited with error code 1 (attempt 1)` | `failed to launch provider process` |
| True fallback | `agent exited with error code 1 (attempt 1)` | `agent exited with error code 1` |
| Retry in progress | `agent exited with error code 1 (attempt 2)` | `Too many requests (attempt 2)` |

When the provider error message is present, it is the message. The exit code
and attempt number are secondary metadata already available via `err.code`
(faceted) and the `attempt` counter respectively — they do not belong in
`err.msg` unless they are the only information available.

### Part 3 — Existing message sites

The `Timeout` arm at `loop_control.rs:983-985` and the `_` catch-all at
`loop_control.rs:992` route through the same builder:

- **Timeout** — `"provider timed out (attempt {attempt})"` has the same
  misleading attempt suffix. The builder distinguishes which rule fired via
  `outcome.error_kind` (`"timeout"` = wall-clock vs `"step_timeout"` =
  stream silence) and phrases each accordingly. The *configured duration*
  (`step timeout (no output for 30m)`) is threaded via the new
  `AttemptOutcome.timeout_secs` (see Rulings §1).

- **Catch-all** (`ShellAuditDenied`) — `"failure on attempt {attempt}"` was
  dead in practice: `classify_failure` never returns `ShellAuditDenied`
  (shell-audit denials surface through their own typed error path before an
  attempt outcome exists). The per-event `match` disappeared entirely — the
  builder is termination-driven, and the call site keeps only the
  `classify_failure(...).is_some()` gate.

All messages apply the same `(attempt N)` policy: only when `attempt > 1`.

## Interaction with signal-assurance

**RULED (2026-07-11): this feature lands first, standalone.**

`claudine/features/2026-07-11-signal-assurance-and-handling/spec.md` §2.5
plans to extend the same seam — `classify_failure` grows into a
disposition-aware classification that consults the attempt-local drained
signal snapshot via a cross-kind precedence ladder, and drives configured
handling strategies before `finalize`.

The boundary between the two:

- **This spec is message-only.** The cascade reads `AttemptOutcome` fields
  exclusively — it does **not** consult signals, dispositions, or the
  arbitration ladder. No change to `classify_failure`, `FailureEvent`, or
  recovery routing.
- **Signal-assurance later composes with it.** When §2.5 lands, its
  signal-selected classification can feed richer inputs into (or phrase
  around) the same `failure_message` builder; the builder's placement in lib
  was chosen so that reuse is a call, not a port.
- If signal-assurance's implementation reorders this seam, the pure builder
  and its unit tests move with it unchanged — that is the point of Part 2's
  placement ruling.

The completed `2026-07-11-provider-errors-as-data` feature is related but orthogonal:
it migrates provider error *vocabulary* (the strings behind `error_kind`
classification) into data. It does not change `err.msg` construction; if it
later yields per-provider message normalization, that too slots in behind
the builder.

## Test Impact

### New unit tests (lib)

`failure_message` is a pure function; cover it with table-style unit tests
in the `harness` module:

- each cascade step in isolation (guard context, provider message, stderr
  fallback, termination label, exit-code fallback);
- precedence: guard context beats a present-but-stale `error_message` on
  `Aborted`; provider message beats stderr; stderr beats termination label;
- hygiene: ANSI stripping, multiline collapse (first line for
  `error_message`, last non-empty for stderr), 240-char truncation with `…`;
- attempt suffix: absent at `attempt == 1`, present at `attempt >= 2`, on
  every cascade step;
- timeout phrasing: `timeout` vs `step_timeout` via `error_kind`.

### Existing L2 assertions

One L2 test asserts on the current message format at
`claudine/cli/tests/level2_lifecycle_dispatch.rs:592`:

```rust
assert!(
    lines
        .iter()
        .any(|l| l.starts_with("err-msg=") && l.contains("agent exited with error code 99")),
    ...
);
```

The test's `stage(&doc, 99)` helper stages a fake provider that exits 99 with
no stream output, so the fallback path would still produce a message containing
the exit code. The assertion text should be updated to match the new fallback
message (no `(attempt 1)` suffix).

The finalize test at `level2_lifecycle_dispatch.rs:751` has a similar
assertion and should be updated in kind.

Sweep for any other L2/L1 assertions matching `"agent exited with error
code"`, `"provider timed out"`, or `"failure on attempt"` before landing.

## Rulings (2026-07-11, final)

1. **Timeout duration threading — RULED (a).** Add
   `timeout_secs: Option<u64>` to `AttemptOutcome`, populated on
   `TimedOut` terminations from `launch.timeout_config` at the wrapper
   attempt path (which rule fired is disambiguated via `error_kind`:
   `"timeout"` = wall-clock, `"step_timeout"` = stream silence). One
   optional field, set at a site that already has both values in hand.
   `build_attempt_outcome` leaves it `None` (the summary does not carry
   the configured duration), mirroring how `guard_context` is threaded.

2. **Redaction of provider-derived text — RULED: defer.** The 240-char
   single-line cap is sufficient risk reduction for now. A content
   redaction pass over what reaches outbound messaging routes belongs to
   the signal-assurance handling work, which formalizes that surface.

## Non-Goals

- Changing the `err.code` / `err.category` / `err.variant` faceted surface —
  those already carry structured classification and are unaffected by this
  work.
- Consulting signals, dispositions, or the arbitration ladder — that is
  signal-assurance §2.5 (see
  [Interaction with signal-assurance](#interaction-with-signal-assurance)).
- Changing the `LifecycleErrorInfo` struct shape beyond threading
  `error_message` through — the `msg` field is already a `String` and will
  simply carry better text.
- Adding new `err.*` fields — the provider error message belongs in the
  existing `err.msg`. If we later want it as a separate field (e.g.
  `err.detail.provider_message`), that is a separate enhancement (natural
  home: `DiagnosticFacets::from_code` currently projects `detail: null`
  for `error_kind`-labeled failures).
- Handling `is_error == true` with `exit_code == 0` — `classify_failure`
  does not fire on a zero exit, so a provider that flags an error but exits
  clean produces no `failure` event today. Changing that is a
  classification-semantics question for signal-assurance, not a message
  question.
- Using `final_response` as a message source — some providers narrate
  errors as assistant text, but it is too noisy to mine for a headline.
- Changing the `report_unhandled_failure` stderr banner — it already renders
  whatever `message` string it receives; the improvement is upstream in the
  message construction (and the banner now shares the single builder).

## Success Criteria

1. A provider that surfaces a structured `error_message` (rate limit, billing,
   auth, API failure) produces an `err.msg` containing that provider message
   text — not a generic exit-code string.

2. A content-guard trip (exit-expression, runaway repetition, volume cap,
   stalled generation) produces an `err.msg` describing the guard and its
   key parameters — a stale provider `error_message` cannot win on an
   `Aborted` termination (on the structured path the wrapper overwrites it
   with the guard prose at trip time; on the capture path there is none).

3. The `(attempt N)` suffix only appears when `attempt > 1`, across all
   three `FailureEvent` arms.

4. `AttemptOutcome.error_message` is populated from the structured-stream
   path and threaded into the failure message cascade; the
   `AttemptOutcome.error_kind` doc comment is corrected to acknowledge
   provider-semantic kinds.

5. The message is built by a single pure function in the `claudine` lib
   `harness` module, consumed by both the `failure` `LifecycleErrorInfo`
   and the `report_unhandled_failure` banner, with table-style unit tests
   covering cascade precedence, hygiene, and the attempt-suffix policy.

6. `err.msg` is always a single sanitized line of at most ~240 characters —
   safe for TTS, messaging routes, and the stderr banner.

7. The fallback (no provider message, no stderr, no guard context) still
   includes the exit code so the operator is never left without any signal.

8. All existing tests updated to match the new message shapes.
