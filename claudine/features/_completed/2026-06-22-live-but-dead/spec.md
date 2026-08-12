---
status: ready for planning and implementation
created: 2026-06-22
area: claudine
packages:
    - claudine
reviewed: true
review_iterations: 3
---

# Live-but-Dead Guard: Stalled-Generation Detection

This spec captures a real failure investigation and commits to an
OpenCode-scoped progress guard. The guard is deliberately separate from
Claudine's two general timeout rules: it does not redefine
`timeout`/`step_timeout`, and it does not weaken the raw-byte heartbeat that
protects legitimate long OpenCode synthesis windows.

Reader note: the reviewed design keeps Option B from the draft but tightens the
contract around the existing timeout documentation. Today
[`docs/topics/timeouts.md`](../../docs/topics/timeouts.md) says Claudine has
exactly two user-facing timeout rules and exactly two timeout env vars:
`CLAUDINE_TIMEOUT` and `CLAUDINE_STEP_TIMEOUT`. This feature intentionally adds
an **OpenCode provider backstop**, not a third general timeout. Any public
`stall_timeout` surface added by this feature must be documented as an
OpenCode stalled-generation guard and must update the timeout docs/CLI reference
so the "exactly two" wording does not become stale.

## Problem

A wrapped OpenCode run with `zai-coding-plan/glm-5.2` hung indefinitely. The
parent session sat on `⏳ Awaiting subagent` for 40+ minutes while the wrapper
output streamed a steady beat of `llm_call_start` badges and never terminated:

```text
 llm_call_start zai-coding-plan/glm-5.2 (mode=all, agent=rust-developer)
 step_loop step=70 session=ses_10ea40010ffeUlahGfHA4R7Mmv
 llm_call_start zai-coding-plan/glm-5.2 (mode=all, agent=rust-developer)
 llm_call_start zai-coding-plan/glm-5.2 (mode=all, agent=rust-developer)
 llm_call_start zai-coding-plan/glm-5.2 (mode=all, agent=rust-developer)
 ⏳ Awaiting subagent: "Implement Finding 2 ... (@rust-developer subagent)" (22m 7s)
```

### What actually happened (from the OpenCode store)

- Parent session `ses_10eb6a3c…` (agent `build`) spawned subagent
  `ses_10ea4001…` (agent `rust-developer`) via the `task` tool at
  22:03:18 UTC.
- The subagent ran **70 healthy steps** (41 reads, 14 greps, 10 bash, 8 edits)
  iterating on an `E0505` borrow-checker error in `loop_engine.rs`. Step 70
  finished at **22:09:43 UTC**.
- At 22:09:43.959 the subagent's next assistant message
  (`msg_ef161e017…`) was created — but it has **no `time.completed`, zero
  tokens, no finish reason, and no error**. The generation was launched and
  **never returned a response**.
- **Zero database parts were written in either session after 22:09:43.** Total
  forward-progress stall.
- The parent's `task` tool part is frozen at `state.status = "running"`. Because
  the subagent never reports back, the parent waits forever.

The repeated `llm_call_start` lines are OpenCode **retrying** the dropped
generation. The provider returned neither a response nor an error — the worst
failure mode: a silent drop.

### How this differs from the usage-cap hang

This is **not** the case fixed in
[`2026-06-21-opencode-log-fix`](../../fixes/2026-06-21-opencode-log-fix/spec.md).
That fix added the `RepeatedStreamError` backstop, which counts consecutive
`message="stream error"` records and aborts after 5. Here there are **zero
`stream error` records** — the generation hangs/drops without ever emitting an
error envelope. The only on-wire signal is repeated `llm_call_start` with no
completion. The existing backstop's counter never increments, so it cannot fire.

## Why every existing guard misses it

| Guard | Why it does not fire |
|---|---|
| `timeout` (wall-clock) | Opt-in; not set for this run. Would have worked, but it is a blunt instrument — it also kills legitimately long sessions. |
| `step_timeout` (stream-silence) | Defeated on **two** clocks. (1) The `llm_call_start` / `step_loop` stderr lines classify as `SemanticEvent::Info`, which `is_activity()` counts → `last_event_at` keeps advancing. (2) Those same lines are real bytes on stderr → `last_byte_at` keeps advancing. The ticker uses `max(last_event_at, last_byte_at)`, so silence never accrues. |
| `step_timeout` stuck-aware suppression | The in-flight subagent's `last_progress_at` is refreshed by the same heartbeat `Info` events, so it is classified **active**, never **stuck**. |
| `RepeatedStreamError` backstop | Counts `message="stream error"` records. This stall emits **none** — the generation drops silently. |
| `runaway` content guards | Trip on *volume* (flood) or *repetition* of `OutputText`/`Reasoning`. A stalled generation produces **no** output text at all — the opposite of a flood. |

**The crux:** the raw-byte heartbeat (introduced specifically to protect
OpenCode's long silent synthesis windows from false kills) is also what masks a
genuinely-dead-but-noisy loop. Any guard based on *liveness* (events arriving,
bytes flowing) is defeated, because the process is provably alive — it is just
making no forward progress. The guard must measure **progress**, not liveness.

## The discriminating signal

The fingerprint of this stall is precise and — importantly — distinguishable
from a legitimately long operation:

- A **stalled generation** emits repeated `llm_call_start` (each a fresh LLM
  generation attempt) with **no resulting progress** between them.
- A **legitimately long tool call** (e.g. a 20-minute test suite via `bash`)
  emits **no** `llm_call_start` at all while the tool runs — there is no model
  generation in flight. It is silent on the generation channel.

So "many `llm_call_start` with zero intervening progress" is unique to the
stall and will not false-trip on long tools. This is the signal the guard keys
on.

## Goals

1. Abort OpenCode's noisy-but-dead generation retry loop without waiting for an
   opt-in wall-clock `timeout`.
2. Preserve `step_timeout` behavior, including the raw-byte heartbeat and
   stderr `Info` activity classification.
3. Keep the first implementation OpenCode-scoped because the only known signal
   is OpenCode's structured stderr `LlmCall` classification.
4. Route the abort as fail-fast `AgentFailure` (`ProcessTermination::Aborted`),
   never as `TimedOut`, so lifecycle/handler recovery does not retry the same
   provider loop.
5. Emit structured summary data (`error_kind` + `guard_context`) that makes the
   cause distinguishable from `step_timeout`, `runaway_*`, and
   `repeated_stream_error`.

## Acceptance criteria

1. Repeated OpenCode `LlmCall` records with no progress-class event for at least
   `stall_timeout` and at least `MAX_GENERATIONS_WITHOUT_PROGRESS` attempts
   terminate the child with `EarlyTermination::StalledGeneration`.
2. The termination maps to `ProcessTermination::Aborted`,
   `summary.error_kind = "stalled_generation"`, and an `AgentFailure` failure
   event. It must not route through `handle_timeout:`.
3. The generated error message includes the generation-attempt count, elapsed
   progress silence, and, when available, session id, step, agent, provider id,
   model id, and mode.
4. `guard_context` includes at least `generation_count` and
   `stall_duration_ms`; it should include the OpenCode metadata above when
   present without leaking prompt text or tool payloads.
5. Any progress-class event resets the generation count and moves
   `last_progress_at` forward. Liveness-only events and raw bytes do not.
6. Long tools that produce no `llm_call_start` records do not trip this guard,
   even when they exceed `stall_timeout`.
7. Existing `RepeatedStreamError` tests still pass, and new tests cover the
   no-error/no-progress `llm_call_start` retry loop.
8. The timeout docs, OpenCode event-source docs, CLI help/reference, and
   frontmatter/completion metadata are updated for any new user-facing
   `stall_timeout` surface.
9. `just test` passes in the `claudine` package area.

### Progress vs. liveness taxonomy

Split the current `is_activity()` set into two classes:

**Progress-class** (genuine forward motion — advances a new `last_progress_at`
clock):

- `OutputText`, `Reasoning` — new model output
- `ToolCall`, `ToolResult` — tool lifecycle
- `SubagentStart`, `SubagentStop`
- `FileChange`, `PlanUpdate`
- `Info` `exiting_loop` (`StepExit`)
- `Info` `step_loop` **only when the step number advances** (the bridge
  already dedupes on `(session_id, step)`, so a true transition is detectable)

**Liveness-only** (process is alive but not progressing — does **not** advance
`last_progress_at`):

- `Info` `llm_call_start` (repeated for the same `(session, step)`)
- `Info` `http_response`
- `Info` `permission_evaluated`
- raw stderr/stdout bytes (`last_byte_at`)
- envelopes already excluded today: `SessionStart`, `TurnStart`,
  `TurnComplete`, `PermissionRequest`

`step_timeout` keeps using the existing liveness clocks unchanged (no
regression in its false-kill protection). The new guard reads only
`last_progress_at`.

Implementation detail: the first OpenCode-scoped implementation may keep this
taxonomy inside `OpenCodeLogBridge` rather than changing
`SemanticEvent::is_activity()`. In that case, "progress-class" means "bridge
input that must call the stalled-generation reset helper before/while emitting
the existing semantic event." Do not reclassify `Info` globally as
non-activity; that would be a behavior change to `step_timeout`.

## Proposed guard

A new **stalled-generation backstop** that trips when the run keeps attempting
generations without ever progressing.

Trip condition (both must hold, to defend against false positives):

1. **Retry churn** — `llm_call_start` count since the last progress-class event
   `>= N` (proposed `N = 4`). A healthy step emits one `llm_call_start` then
   makes progress, resetting the count to 0; only retries accumulate it.
2. **Progress silence** — `now - last_progress_at >= stall_timeout` (proposed
   default `10m`). Shorter than `step_timeout`'s 30m because the progress
   signal is stricter and the retry-churn condition already filters the
   ambiguous cases.

Both conditions reset the moment any progress-class event arrives.

On trip: emit a terminal `SemanticEvent::Error` ("provider attempted N
generations over M without progress; aborting") and fire a new
`EarlyTermination::StalledGeneration` → `ProcessTermination::Aborted` →
fail-fast `AgentFailure`. **Never** the `handle_timeout:` retry path — retrying
would reproduce the stall. This mirrors the `RepeatedStreamError` backstop's
termination shape exactly.

`error_kind = "stalled_generation"`, threaded into the `handle` payload
(`CLAUDINE_ERROR_KIND` env + JSON) like the other guards, with a
`guard_context` carrying the generation count and stall duration.

### State and reset semantics

Add bridge-local state, initially in
[`stream/logs/opencode/reasoning.rs`](../../lib/src/stream/logs/opencode/reasoning.rs):

```rust
struct StalledGenerationState {
    last_progress_at: Instant,
    generation_count_since_progress: u32,
    last_generation_context: Option<StalledGenerationContext>,
}
```

Use `Instant` for elapsed time so wall-clock changes cannot spuriously fire or
delay the guard. Tests should keep the detector logic in a small helper that
accepts `now: Instant`; avoid tests that sleep for real time.

`last_progress_at` initializes when the bridge is created or on the first
progress-class event, whichever is later. It does not wait for stdout NDJSON,
because this incident happened entirely while OpenCode stderr remained active.

Reset on:

- `OutputText`, `Reasoning`, `ToolCall`, `ToolResult`, `FileChange`,
  `PlanUpdate`, `SubagentStart`, `SubagentStop`
- `StepExit`
- `StepLoop` only when `last_step_per_session` accepts a new step value

Do not reset on:

- `LlmCall`
- repeated/deduped `StepLoop` for the same `(session_id, step)`
- `HttpResponse`
- `PermissionEvaluated`
- `BootBanner`, `Snapshot`, `Unclassified`, or filtered `service=bus`
- raw bytes before the bridge parses them

`StepExit` must clear `last_step_per_session` as it does today and reset the
stalled-generation counter. This makes a follow-up prompt on the same session
fresh while preserving the existing dedup contract documented in
[OpenCode Event Sources](../../../.claude/skills/claudine/opencode-event-sources.md).

### `LlmCall` counting

Count structured OpenCode `LogClassification::LlmCall` records where
`is_stream == true`. Each count stores the latest safe metadata for rendering
and `guard_context`: `session_id`, `step` when known from the current
dedup state, `agent`, `provider_id`, `model_id`, and `mode`.

The guard does not need provider request IDs or response payloads. Do not store
prompt text, tool inputs, HTTP URLs, authorization headers, or raw stderr lines
in the guard context.

The first `LlmCall` after progress is not enough to trip. This is intentional:
a single genuinely slow first generation can exceed 10 minutes on a struggling
endpoint. The count condition remains required.

### Termination plumbing

Add:

```rust
EarlyTermination::StalledGeneration {
    generation_count: u32,
    stall_duration: Duration,
    context: StalledGenerationContext,
}
```

Then update the same termination helpers that already cover
`RepeatedStreamError` and the runaway guards:

- `apply_early_termination_to_summary` writes
  `error_kind = "stalled_generation"` and a concise `error_message`.
- `early_termination_message` returns the same message written into the summary.
- `process_termination_from_early` maps it to `ProcessTermination::Aborted`.
- `early_termination_guard_context` returns the structured stalled-generation
  context.
- Tests that enumerate all `EarlyTermination` variants include the new variant.

Use `SemanticErrorKind::AgentNative` for the terminal semantic error. This is a
provider/wrapper failure, not a remote API classification like usage caps.

### Design options considered

**Option A — General progress-silence clock (a third timeout rule).**
Add `last_progress_at` at the stream layer and a `progress_timeout` rule
parallel to `step_timeout`, fired purely on progress silence.
*Rejected as the primary mechanism:* for OpenCode's DONE-only stream, a single
legitimately long tool call produces no progress events for its whole duration
and would false-trip. The retry-churn condition (Option B) is what makes the
signal safe; a pure progress clock lacks it.

**Option B — Stalled-generation backstop (recommended).**
The two-condition trip above. Precise (keys on the unique retry-churn
fingerprint), proven pattern (sits beside `RepeatedStreamError` in the OpenCode
log bridge), and immune to the long-tool false positive. Lives in
[`stream/logs/opencode/reasoning.rs`](../../lib/src/stream/logs/opencode/reasoning.rs)
initially because `llm_call_start` is an OpenCode-stderr-specific signal.

**Option C — Reclassify heartbeat `Info` as non-activity for `step_timeout`.**
Stop `llm_call_start` / `http_response` from advancing `last_event_at`.
*Insufficient alone:* the raw-byte clock still advances from the same stderr
lines, so `step_timeout` still never fires. Would require also gating the byte
heartbeat, which reintroduces the false-kill risk the byte clock was added to
prevent.

**Recommendation:** Option B, scoped to the OpenCode bridge first. If a second
provider later exhibits generation-retry churn, lift the progress/liveness
taxonomy and the backstop to the provider-general stream layer as a follow-up.

### Configuration

| Source | Knob | Default |
|---|---|---|
| CLI flag | `--stall-timeout <dur>` | — |
| Frontmatter | `stall_timeout: <dur>` | — |
| Env default | `CLAUDINE_OPENCODE_STALL_TIMEOUT` | `10m` |
| Built-in | — | `10m` |

- Same duration grammar and strict top-down precedence as `timeout` /
  `step_timeout`. `0s` disables the guard.
- The env var is OpenCode-scoped on purpose. A provider-general
  `CLAUDINE_STALL_TIMEOUT` would imply a third generic timeout rule and would
  conflict with the current timeout contract unless a broader design updates
  that contract first.
- The CLI flag and frontmatter key are provider-neutral names for author
  ergonomics, but the value only affects OpenCode until another provider has a
  proven equivalent signal. Non-OpenCode runs should accept the key silently as
  inert configuration or emit a debug trace only; do not warn users for portable
  prompt files.
- Generation-count threshold `N` ships as a constant
  (`MAX_GENERATIONS_WITHOUT_PROGRESS`, proposed `4`), mirroring
  `MAX_CONSECUTIVE_STREAM_ERRORS`. Promote to a knob only if real runs need it.

`stall_timeout_warn` is explicitly out of scope. The existing warning system is
for the two general timeout rules and has documented relational validation with
`timeout`/`step_timeout`; adding a warning companion here would expand the
frontmatter timing surface more than this incident needs.

## Out of scope

- Recovering or resuming the dropped generation. The guard aborts cleanly;
  resume is the provider's job.
- Changing `step_timeout`'s clocks or the byte heartbeat. Their false-kill
  protection is preserved unchanged.
- Changing `SemanticEvent::is_activity()` or making all `Info` events
  liveness-only. That would alter the existing silence rule.
- Provider-general generalization (Option B lifted to the stream layer) — a
  follow-up once a second provider needs it.
- The usage-cap variant already covered by `RepeatedStreamError`.
- A warning-only `stall_timeout_warn` surface.

## Documentation updates

Update these documents as part of implementation:

- [`docs/topics/timeouts.md`](../../docs/topics/timeouts.md): preserve the two
  general timeout-rule contract, add a short OpenCode stalled-generation
  backstop subsection near the content guards / OpenCode variant discussion,
  and include `stalled_generation` in the `Aborted` failure-event table.
- [OpenCode Event Sources](../../../.claude/skills/claudine/opencode-event-sources.md):
  document the new `LlmCall` retry-churn counter next to
  `RepeatedStreamError`.
- `.claude/skills/claudine/SKILL.md` and timeline docs: mention the new
  OpenCode live-but-dead guard and its `error_kind`.
- CLI/frontmatter reference and completion metadata: add `stall_timeout` /
  `--stall-timeout` with the OpenCode-only note.

## Tests

Add focused unit tests around the detector and bridge:

- Four streamed `LlmCall` records over `>= stall_timeout` with no progress fire
  `EarlyTermination::StalledGeneration`.
- Three records over the same duration do not fire.
- Four records under the duration do not fire.
- A progress-class event between `LlmCall` records resets count and time.
- Repeated/deduped `StepLoop` for the same `(session_id, step)` does not reset;
  a genuine step advance does reset.
- `HttpResponse`, `PermissionEvaluated`, filtered `service=bus`, and raw bytes
  do not reset.
- Long-tool shape with no `LlmCall` never trips this guard.
- `RepeatedStreamError` still fires independently for repeated
  `message="stream error"` records and is not reset by `LlmCall`.
- Summary/error-message/termination mapping tests include
  `StalledGeneration`.

## Open questions

1. **Single-generation dead air with heartbeats.** This guard only catches retry
   churn. If OpenCode emits one `LlmCall`, then only `HttpResponse` or
   `service=bus` heartbeats forever, the count condition will not fire and
   `step_timeout` may still be masked by bytes. Options:

   - **A. Keep this out of scope.**
     - Pros: solves the observed incident with the fewest false-positive risks;
       no new ambiguity around slow first generations.
     - Cons: a related noisy single-generation stall could still hang until an
       opt-in wall-clock timeout.
   - **B. Add a much longer single-generation threshold, e.g. 45m.**
     - Pros: bounds a second plausible live-but-dead shape.
     - Cons: starts to behave like a third timeout rule and may kill genuinely
       slow but active endpoints.
   - **C. Require a provider-specific "generation completed/failed" signal and
       track unmatched starts.**
     - Pros: semantically clean if OpenCode exposes the signal reliably.
     - Cons: current investigation did not identify a reliable completion
       envelope in the failing path.

   **Recommendation:** A. Keep the first implementation scoped to proven retry
   churn. Revisit only after a captured incident shows the single-generation
   heartbeat shape.

2. **Rendered badge wording.** Should the live stderr and final error block use
   a distinct `Stalled Generation` label or the generic agent-native error
   presentation?

   - **A. Distinct label.**
     - Pros: immediately distinguishes this from usage caps and stream errors;
       easier to search in logs and screenshots.
     - Cons: requires one more rendering branch.
   - **B. Reuse the repeated-stream-error presentation.**
     - Pros: smallest UI change.
     - Cons: hides the important distinction between error retries and silent
       generation retries.

   **Recommendation:** A. Use a distinct label while keeping
   `SemanticErrorKind::AgentNative` so color/style remains consistent with
   other provider-native failures.

## References

- Investigation session: `ses_10eb6a3cdffeRTxoa6t7bN8zZc` (parent),
  `ses_10ea40010ffeUlahGfHA4R7Mmv` (subagent), OpenCode store
  `~/.local/share/opencode/opencode.db`.
- [Timeouts](../../docs/topics/timeouts.md) — the two timeout rules, activity
  vocabulary, stuck-aware suppression.
- [OpenCode Event Sources](../../../.claude/skills/claudine/opencode-event-sources.md)
  — Dual-Source Contract, stderr promotion table, `RepeatedStreamError`
  backstop.
- [`2026-06-21-opencode-log-fix`](../../fixes/2026-06-21-opencode-log-fix/spec.md)
  — the usage-cap variant of this hang.
