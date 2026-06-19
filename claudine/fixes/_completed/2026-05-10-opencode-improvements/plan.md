---
source_files_during_phase_1:
  - claudine/lib/src/stream/providers/opencode.rs
  - claudine/lib/src/stream/protocol/opencode.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/stream/protocol/opencode.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/stream/progress.rs
  - claudine/cli/src/commands/wrap/exec/watchdog.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/cli/src/commands/wrap/exec/subagent_watchdog.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs
  - claudine/cli/src/commands/wrap/exec/watchdog.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/cli/src/commands/wrap/exec/watchdog.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - claudine/docs/topics/timeouts.md
  - claudine/.claude/skills/claudine/SKILL.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - claudine/.claude/skills/claudine/SKILL.md
packages:
  - claudine
---

# OpenCode Subagent Visibility & Per-Step Grace

## Problem statement

The 2026-05-10 `opencode-timeout-regression` fix added two compensating
layers for OpenCode's sparse parent NDJSON stream — a raw-byte
heartbeat and a `provider_status` silence-grace. The grace, however,
only applies **until the first `step_finish` is observed**. After that
boundary fires once for the session, the silence rule re-engages
unconditionally and a long mid-stream subagent dispatch can still
trigger a false `step_timeout` kill.

A concrete example observed on 2026-05-10: a `compose prompts/commit.md`
run on OpenCode + Minimax 2.7 dispatched 6 concurrent `task` (subagent)
tool calls in a single step. 5 of 6 completions streamed back as
`tool_use` events; the 6th took longer than the 30-minute
`step_timeout`. The wrapper killed the parent process even though the
git commits had landed successfully. Two structural facts make this
class of failure hard to diagnose:

1. **The current grace fires only once per session.** Once any
   `step_finish` has been seen, all subsequent steps lose the silence
   protection.
2. **The breach diagnostic shows "no outstanding subagents".** Because
   OpenCode never emits `tool_start` or `task_started` for the `task`
   tool, `LiveMetricsState.in_flight_subagents` is empty at the moment
   of breach, even though subagent work was clearly in flight. The
   rendered `Agent Error` block gives the user no signal about what
   was actually happening.

## Root cause

Per [`claudine/lib/src/stream/providers/opencode.rs`](../../lib/src/stream/providers/opencode.rs)
(`handle_tool_use_completed`, lines 309-348) and the
[OpenCode non-interactive research](../../docs/research/non-interactive-sessions/opencode.md):

- OpenCode emits `tool_use` events **only after** a tool reaches
  `completed` or `error`. There is no paired `tool_start`.
- OpenCode does not emit `task_started` / `task_completed` /
  `task_progress` events on the `run --format json` stream. The
  parser's existing handlers for those types never fire from current
  OpenCode releases. Subagent dispatch is just the `task` *tool*.

The combination means:

- `LiveMetricsState.in_flight_subagents` (populated by
  `SemanticEvent::SubagentStart`) stays empty for the entire
  OpenCode session.
- `WatchdogState.active` (populated by the live sink at
  [`live_semantic_sink/mod.rs:675-676`](../../cli/src/commands/wrap/live_semantic_sink/mod.rs))
  stays empty for the same reason.
- The `step_timeout` breach diagnostic at
  [`evaluate_timeout_tick`](../../cli/src/commands/wrap/exec/watchdog.rs)
  enumerates `outstanding_at_breach()`, which returns an empty vec.

And the existing one-shot `provider_status` grace at
[`watchdog.rs:487-510`](../../cli/src/commands/wrap/exec/watchdog.rs)
flips off forever the moment the first `step_finish` is parsed:

```rust
let provider_status_seen = g.provider_status.is_some();
// ...
if provider == Provider::OpenCode && !provider_status_seen { suppress }
```

This was deliberate in the regression fix (the goal there was to
protect the cold-start window). It needs to be widened to also protect
each subsequent step's in-flight window.

## Goal

Reduce false `step_timeout` kills on OpenCode for multi-step flows
that dispatch subagents mid-stream, **without**:

- Disabling the silence rule entirely for OpenCode.
- Weakening the wall-clock `timeout` backstop.
- Introducing new user-facing env vars or frontmatter knobs.
- Promoting "watchdog" to user-facing nomenclature anywhere.
- Adding fragile parent-text pattern matching (e.g. "Dispatching N
  subagents").

When a kill *does* happen, give the user a substantively richer breach
diagnostic that names the subagent work observed in the current step.

## Fix shape

Two independent improvements, both scoped to OpenCode:

### Layer A — Synthesize subagent lifecycle from `task` tool completions

When the OpenCode parser handles a `tool_use` completion for the
`task` tool, emit a `SemanticEvent::SubagentStart` immediately followed
by a `SemanticEvent::SubagentStop` in addition to the existing
`ToolResult`. The `SubagentStart` is reconstructed from
`part.state.time.start` so `LiveMetricsState` and `WatchdogState` see
a realistic `started_at`.

This does **not** help with in-flight detection during the silence
window (we still only learn about a Task at completion). What it does
is:

1. Increment `subagent_done_count` so the breach diagnostic can report
   "N subagents observed in this session".
2. Cache the last-completion timestamp per step so the breach
   diagnostic can report "last subagent completion: 3m ago" — a
   strong hint that other parallel subagents are likely still in
   flight even though we have no direct evidence.
3. Make the synthesized `name` field useful — the `task` tool input
   carries a human-readable `description` / `subagent_type` we can
   surface.

### Layer B — Per-step `provider_status` grace reset

Replace the one-shot "any `step_finish` ever observed" guard with a
**per-step** grace. The silence rule is suppressed during any window
between `step_start` and the next `step_finish`. The semantics become:

- `step_start` → enter "step in flight" state.
- `step_finish` → exit "step in flight" state.
- Silence rule fires only when NOT in "step in flight" state AND
  (existing predicates) hold.
- Wall-clock `timeout` rule fires unconditionally.
- Byte heartbeat continues to refresh activity regardless of step
  state.

The motivation is the same as the original `provider_status` grace:
OpenCode steps are atomic units of model work; mid-step silence is
not evidence of a hang. The byte heartbeat from
`2026-05-10-opencode-timeout-regression` already protects against
genuine zero-byte hangs. The wall-clock `timeout` protects against
runaway steps.

This guard fires only on the OpenCode provider. Other providers do
not need it because their richer event surface populates `in_flight`
and `in_flight_subagents` correctly.

### Out of scope (deliberately)

- **Parent-text pattern matching** ("Dispatching N concurrent
  subagents" → push N entries into `in_flight_subagents`). Rejected
  in the prior regression fix as too prompt/model-dependent. Same
  rejection here.
- **OpenCode server SSE bridge** — connecting the wrapper to a running
  `opencode serve` for richer event visibility. Architecturally
  appealing but a much larger lift and requires the server to be
  running. Park for a future feature.
- **Generalizing per-step grace to other providers.** Goose, Kimi,
  and Qwen also have sparse streams, but no incidents have been
  reported on them. Defer until evidence appears.
- **Reworking the `task` tool's stream rendering** (`→ Task(...)` vs
  `← Task(...)`). The 2026-04-16 stderr-surface fix already handles
  the canonical display; this plan does not change rendering.

## Phases

### Phase 1 — Synthesize SubagentStart/SubagentStop for `task` tool

In `claudine/lib/src/stream/providers/opencode.rs`, extend
`handle_tool_use_completed` to detect when the resolved tool name
equals `"task"` (case-insensitive). When it does:

1. Extract `started_at` from `part.state.time.start` if present
   (epoch ms → `Instant` via the existing timestamp utilities; if
   unavailable, fall back to "now").
2. Extract a display name preferring `state.input.description` →
   `state.input.subagent_type` → `state.input.prompt` (truncated) →
   `"task"`.
3. Extract the subagent session id from `state.metadata.sessionId`
   (per the research doc, the `task` tool attaches child-session
   metadata; surface it as the SubagentStart `id`).
4. Emit `SemanticEvent::SubagentStart { id, name, extra }` before
   the existing `SemanticEvent::ToolResult`.
5. Emit `SemanticEvent::SubagentStop { id, name, status, extra }`
   after the existing `SemanticEvent::ToolResult`, copying `status`
   from `resolved.status`.

The synthesized events must include an `extra.synthesized = true`
marker so consumers (and future debuggers) can tell these did not
come from a native `task_started` / `task_completed` payload.

This change does NOT remove the existing `ToolResult` emission — the
tool's appearance in `→ Task(...)` / `← Task(...)` lines is preserved
exactly as today.

### Phase 2 — Surface `task` tool metadata in the protocol layer

In `claudine/lib/src/stream/protocol/opencode.rs`, extend
`OpenCodeToolFields` (or the nested `state`) with an optional
`metadata` value so `state.metadata.sessionId` and `state.time` are
typed rather than re-parsed from `serde_json::Value`. Two helper
methods on the resolved view:

- `task_subagent_id(&self) -> Option<&str>` — returns
  `state.metadata.sessionId` when present.
- `task_started_at_epoch_ms(&self) -> Option<u64>` — returns
  `state.time.start` when present.

Keep all fields optional, no `deny_unknown_fields`, and add unit
tests for the new accessors in the same `mod tests` block.

### Phase 3 — Per-step `provider_status` grace

1. Add a boolean `step_in_flight: bool` (or equivalent) on
   `LiveMetricsState` in
   [`claudine/lib/src/stream/progress.rs`](../../lib/src/stream/progress.rs)
   that toggles on `step_start` and off on `step_finish`. Existing
   `step_finish` extra-marker handling at lines 274-283 already
   distinguishes the finish phase; mirror that for `start`.
2. In `evaluate_timeout_tick` at
   [`watchdog.rs:487-510`](../../cli/src/commands/wrap/exec/watchdog.rs),
   replace the single-shot `provider_status.is_some()` guard with a
   composite predicate:
   - If provider is OpenCode AND `step_in_flight` is true → suppress
     silence rule.
   - If provider is OpenCode AND `provider_status` is `None` → also
     suppress (preserve cold-start protection from the prior fix).
   - Otherwise → evaluate silence rule as today.
3. Wall-clock `timeout` evaluation is unchanged.

Files:

- `claudine/lib/src/stream/progress.rs` — add `step_in_flight`,
  update `observe_event` to toggle it.
- `claudine/cli/src/commands/wrap/exec/watchdog.rs` — widen the
  guard.
- `claudine/cli/src/commands/wrap/exec/timeouts.rs` — if the
  helper shape moves; otherwise no change.

### Phase 4 — Enrich the breach diagnostic

In the `step_timeout` breach render path
([`watchdog.rs:render_watchdog_error_to_stream`](../../cli/src/commands/wrap/exec/watchdog.rs)),
when the provider is OpenCode and the standard
`outstanding_at_breach()` snapshot is empty, also include:

- `subagent_done_count` — count of subagents observed (synthesized
  in Phase 1).
- Elapsed since last subagent completion (if any).
- Whether `step_in_flight` was true at the moment of breach.
- **Recent subagent descriptions** — the names/descriptions of the
  last N (default 5) subagent completions, drawn from a small
  bounded ring-buffer on `WatchdogState`.

#### Recent-subagent ring-buffer

Add a `recent_subagents: VecDeque<RecentSubagentInfo>` field on
`WatchdogState` (capacity 5, configurable internally) populated when
`subagent_stopped` fires. Each entry captures:

```rust
struct RecentSubagentInfo {
    id: SubagentId,
    name: Option<String>,
    description: Option<String>,  // from task.input.description / subagent_type
    completed_at: Instant,
    status: Option<String>,
}
```

The live sink at
[`live_semantic_sink/mod.rs:675-676`](../../cli/src/commands/wrap/live_semantic_sink/mod.rs)
already forwards `SubagentStart` to `WatchdogState`; extend the
`SubagentStop` arm symmetrically and pull `description` from the
event's `extra.description` field (populated by the Phase 1
synthesizer).

The expected user-facing prose becomes something like:

```
Step timeout (30m) exceeded.

5 subagents observed in this step. Last completion 4m ago. A step
boundary was still open at the time of timeout — the parent agent
may have been waiting on a parallel subagent that has not yet
returned.

Recent subagents:
  ← Commit wrap CLI refactor (4m ago, success)
  ← Commit claudine docs (5m ago, success)
  ← Commit providers refactor (6m ago, success)
  ← Commit wrap tests (7m ago, success)
  ← Commit system_prompt lib refactor (8m ago, success)
```

vs. today:

```
Step timeout (30m) exceeded.

No outstanding subagents.
```

`LiveSemanticSink` already carries the lib-side `LiveMetricsState`;
the ring-buffer lives on `WatchdogState` (CLI-side) since it's
diagnostic-only and never crosses the lib boundary.

### Phase 5 — Tests

Author the following in
[`claudine/cli/tests/wrap_commands.rs`](../../cli/tests/wrap_commands.rs)
and the protocol parser's `mod tests`:

1. **`opencode_task_completion_synthesizes_subagent_lifecycle`** —
   feed a single `tool_use` line for `tool: "task"` with
   `state.metadata.sessionId` and `state.time.start` set; assert
   that the recorded events include `SubagentStart` →
   `ToolResult` → `SubagentStop` in order, with matching `id` and
   non-zero `subagent_done_count` on the finish summary.

2. **`opencode_non_task_tool_does_not_synthesize_subagent_lifecycle`** —
   regression guard for the `bash` / `read` / `write` tools: feed
   the same shape with `tool: "bash"`; assert no `SubagentStart` /
   `SubagentStop` is emitted.

3. **`opencode_step_in_flight_suppresses_silence`** — drive the
   wrapper with `step_start` → silence past `step_timeout` → assert
   no breach fires. Then emit `step_finish` and re-silence past
   `step_timeout` → assert the breach DOES fire.

4. **`opencode_step_in_flight_resets_per_step`** — drive two
   `step_start` / `step_finish` cycles with silence in the middle of
   the second; assert the silence rule is suppressed for both
   in-flight windows.

5. **`opencode_byte_heartbeat_still_catches_zero_byte_hang`** —
   regression guard for the prior fix: emit `step_start`, then no
   bytes at all for `step_timeout + grace`; assert the breach DOES
   fire (the per-step grace must not override the byte-heartbeat
   semantics).

6. **`opencode_breach_diagnostic_names_subagent_count`** — drive a
   sequence with 3 synthesized Task completions, then a long silence
   that triggers the breach; assert the rendered breach prose
   contains the "3 subagents observed" wording.

7. **`watchdog_state_recent_subagents_ring_buffer_caps_at_5`** —
   unit test for the new ring-buffer on `WatchdogState`: push 7
   subagent completions, assert only the 5 most recent are
   retained, in completion order (newest first).

8. **`opencode_breach_diagnostic_lists_recent_subagent_descriptions`** —
   drive 5 synthesized Task completions with distinct
   `state.input.description` values, then a silence that triggers
   the breach; assert all 5 descriptions appear in the rendered
   breach prose in newest-first order.

Time-based tests use the existing 1s `CLAUDINE_WATCHDOG_INTERVAL`
override pattern.

### Phase 6 — Documentation

1. Update
   [`claudine/docs/topics/timeouts.md`](../../docs/topics/timeouts.md):
   - Add a sub-section under the OpenCode-specific notes explaining
     the per-step grace and how it composes with the byte-heartbeat
     and wall-clock timeout.
   - Note explicitly that the silence rule is suppressed *while a
     step is in flight on OpenCode*, but the wall-clock rule is not.
2. Update
   [`claudine/.claude/skills/claudine/SKILL.md`](../../.claude/skills/claudine/SKILL.md):
   - In the existing 2026-05-10 paragraph, document that the
     `provider_status` grace was widened from one-shot to per-step
     and that `task` tool completions now synthesize subagent
     lifecycle events.

## Verification checklist

Before merging:

- [x] All new tests in Phase 5 pass.
- [x] All existing `watchdog_*` tests in `wrap_commands.rs` still pass.
- [x] All existing OpenCode parser tests in `providers/opencode.rs`
      and `protocol/opencode.rs` still pass.
- [x] Clippy clean: `just lint claudine` and `just lint claudine-cli`.
- [x] No new user-facing env vars introduced.
- [x] No "watchdog" string appears in any user-facing surface added or
      changed by this fix.
- [ ] `claudine compose prompts/commit.md` on OpenCode + Minimax 2.7
      succeeds ≥ 9/10 runs in the user's environment.
- [x] A deliberately stuck `bash` tool (`sleep 99999`) still triggers
      `step_timeout` within the configured window plus grace, proving
      the per-step grace did not over-suppress.

## Risks

- **Per-step grace masks a real hang inside a step.** If OpenCode
  emits `step_start` and then truly hangs without ever emitting
  `step_finish`, the silence rule is suppressed indefinitely.
  Mitigations: (1) the wall-clock `timeout` is unaffected; (2) the
  byte heartbeat from the prior fix still catches genuine zero-byte
  hangs. Validate explicitly in Phase 5 test #5.
- **`state.metadata.sessionId` may be absent on older OpenCode
  releases.** Handle gracefully — synthesize without an `id` and let
  the in-flight key fall back to the tool's own `id`. Already
  covered by the existing `unwrap_or_default` paths in
  `LiveMetricsState::observe_event`.
- **`tool.toLowerCase() == "task"` matching is fragile if OpenCode
  ever renames the tool.** Acceptable: the consequence is reverting
  to today's behaviour (no subagent lifecycle synthesized), not a
  regression. Add a debug `tracing` log when the tool name is
  recognised so future operators can see the synthesis happening.
- **Synthesized events confuse downstream reporting.** The
  `extra.synthesized = true` marker is the escape hatch. The
  reporting layer at
  [`claudine/lib/src/reporting/`](../../lib/src/reporting/) does not
  currently distinguish synthesized events, but the marker is
  forward-compatible if a future query wants to exclude them.

## Resolved design decisions

- **Per-step grace cap** (rejected): the wall-clock `timeout`
  serves as the upper bound; adding a per-step cap reintroduces
  the false-kill class this plan is fixing.
- **Recent-subagent ring-buffer in breach prose** (accepted): the
  breach diagnostic names the 5 most recent subagent completions,
  newest first, with description / elapsed / status. Implemented
  as a bounded `VecDeque` on `WatchdogState` (Phase 4).

## Open questions

- Should the synthesized `SubagentStart`/`SubagentStop` events also
  fire for the `task` tool's `error` completion path? Recommendation:
  yes — `status: "error"` should propagate through unchanged so
  `subagent_done_count` increments the same way and the ring-buffer
  captures failed subagents distinctly.
