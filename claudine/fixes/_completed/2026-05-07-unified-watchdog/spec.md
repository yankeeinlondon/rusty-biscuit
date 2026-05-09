---
fixed: 2026-05-07
agent: claude
---

# Fix: Unified watchdog `step_timeout` suppressed indefinitely by stuck in-flight tools/subagents

## Symptom

Two integration tests are permanently `#[ignore]`d because the unified watchdog's `step_timeout` rule never fires when tools or subagents are stuck:

- `watchdog_stream_idle_timeout_after_tool_call_hang` — a `tool_start` without a matching `tool_result` keeps `LiveMetrics::in_flight` populated, suppressing `step_timeout` forever.
- `watchdog_subagent_hang_terminates_and_names_stuck_ids` — subagents that started but never completed keep `LiveMetrics::in_flight_subagents` populated, suppressing `step_timeout` forever.

In both cases the parent stream is completely silent, but the watchdog treats "something is in flight" as "something is making progress" and never kills the run.

## Root cause

`claudine/cli/src/commands/wrap/exec/watchdog.rs::evaluate_timeout_tick` (and the legacy `timeouts.rs::detect_step_timeout`) suppress the silence rule with a single boolean:

```rust
let has_in_flight = !g.in_flight.is_empty() || !g.in_flight_subagents.is_empty();
if has_in_flight {
    return WatchdogTickResult::Ok;
}
```

This is correct for *healthy* long-running operations (e.g. a real `bash` tool that takes 30 s, or a `Task` subagent doing real work), but it is wrong for *stuck* operations that never report completion. A tool that started 7 minutes ago and never produced a result is not "active"; it is hung.

The fix requires distinguishing **active** in-flight work from **stuck** in-flight work using progress timestamps.

## Fix

### 1. Add `last_progress_at` to `InFlightTool`

`claudine/lib/src/stream/progress.rs`:

- Add `last_progress_at: Instant` to `InFlightTool`.
- On `record_tool_start`, set `last_progress_at = now`.
- On `record_activity` (or any event that updates `last_event_at` while the tool is in flight), also update `last_progress_at` for every in-flight tool.
- Add `stuck_tools(&self, now: Instant, threshold: Duration) -> Vec<&InFlightTool>` that returns tools where `now - last_progress_at >= threshold`.

### 2. Add `stuck_subagents` query to `LiveMetricsState`

`claudine/lib/src/stream/progress.rs`:

- Add `stuck_subagents(&self, now: Instant, threshold: Duration) -> Vec<&InFlightSubagent>` symmetric to `stuck_tools`.

### 3. Replace boolean suppression with stuck-aware logic in `evaluate_timeout_tick`

`claudine/cli/src/commands/wrap/exec/watchdog.rs`:

```rust
// Old: unconditional suppression
// let has_in_flight = !g.in_flight.is_empty() || !g.in_flight_subagents.is_empty();
// if has_in_flight { return WatchdogTickResult::Ok; }

// New: only suppress when everything in flight is still making progress
let stuck_tools = g.stuck_tools(now, budget);
let stuck_subagents = g.stuck_subagents(now, budget);
let any_stuck = !stuck_tools.is_empty() || !stuck_subagents.is_empty();
let any_active = g.in_flight.len() + g.in_flight_subagents.len() > stuck_tools.len() + stuck_subagents.len();

if any_active && !any_stuck {
    return WatchdogTickResult::Ok;
}

// If we reach here, either:
// - nothing is in flight (normal silence rule), OR
// - everything in flight is stuck (treat as silence), OR
// - a mix of active and stuck exists (treat as silence so the stuck ones get killed)
```

When `step_timeout` fires with stuck subagents, the existing `WatchdogState::outstanding_at_breach(now)` already returns them for diagnostic enrichment. When stuck tools exist, they should be appended to the message as well (new formatting helper).

### 4. Update `detect_step_timeout` in `timeouts.rs`

Apply the same stuck-aware logic to the legacy standalone helper so both paths behave identically.

### 5. Update `WatchdogTermination` message formatting

`format_step_timeout_breach_message` currently only enumerates `stuck_subagents`. Extend it to also list stuck tools (id + name) when `stuck_tools` is non-empty.

### 6. Update existing unit tests

- `evaluate_timeout_tick_silence_suppressed_by_in_flight_tool` — now needs to set `started_at = now - budget - 1s` (so the tool is **stuck**) and assert it **fires**.
- `evaluate_timeout_tick_silence_suppressed_by_in_flight_subagent` — same: set `started_at` older than `budget` and assert it **fires**.
- Add new tests for the "active in-flight still suppresses" case (fresh `started_at`, does not fire).

### 7. Un-ignore the two integration tests

Remove `#[ignore = "..."]` from:
- `watchdog_stream_idle_timeout_after_tool_call_hang`
- `watchdog_subagent_hang_terminates_and_names_stuck_ids`

Both should pass because a tool/subagent that never completes will eventually exceed `step_timeout` from its `last_progress_at`, and the breach will fire.

## Tests added

In `watchdog.rs`:
- `evaluate_timeout_tick_silence_fires_when_tool_is_stuck` — stuck tool triggers step_timeout.
- `evaluate_timeout_tick_silence_fires_when_subagent_is_stuck` — stuck subagent triggers step_timeout.
- `evaluate_timeout_tick_silence_suppressed_when_tool_is_active` — fresh tool still suppresses.
- `evaluate_timeout_tick_silence_suppressed_when_subagent_is_active` — fresh subagent still suppresses.
- `evaluate_timeout_tick_mixed_active_and_stuck_fires` — one active + one stuck still fires.

In `progress.rs`:
- `stuck_tools_returns_empty_when_all_fresh`
- `stuck_tools_returns_stuck_ones`
- `stuck_subagents_returns_stuck_ones`

## Files changed

- `claudine/lib/src/stream/progress.rs` — add `last_progress_at`, `stuck_tools`, `stuck_subagents`
- `claudine/cli/src/commands/wrap/exec/watchdog.rs` — stuck-aware `evaluate_timeout_tick`, message formatting, unit tests
- `claudine/cli/src/commands/wrap/exec/timeouts.rs` — stuck-aware `detect_step_timeout`
- `claudine/cli/tests/wrap_commands.rs` — remove `#[ignore]` from the two integration tests
