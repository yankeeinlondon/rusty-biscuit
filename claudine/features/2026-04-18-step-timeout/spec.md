# Step Timeout

## Problem

The existing `timeout` frontmatter property sets a wall-clock deadline measured from child-process spawn. A long-running agentic session may legitimately take 10+ minutes across many tool calls, subagent spawns, and permission confirmations — but if the provider goes silent mid-session (stuck on a network call, hung subagent, deadlocked tool), the user has no way to detect and recover from that specific failure mode without also killing healthy-but-slow sessions.

The two timeouts address orthogonal concerns:

| Property | Scope | Starts When | Resets | Catches |
|---|---|---|---|---|
| `timeout` | Entire composition attempt | Child process spawn | Never (per attempt) | Sessions that exceed a hard budget |
| `step_timeout` | Inter-step silence | Last stream event | Every stream event | Provider that goes silent mid-session |

## Frontmatter Declaration

```yaml
step_timeout: 2m
```

Same syntax and parser as `timeout`: `{number}{unit}` where unit is `s`/`sec`/`seconds`, `m`/`min`/`minutes`, or `h`/`hr`/`hours`.

Both properties are optional and independent:

```yaml
timeout: 10m       # hard wall-clock budget
step_timeout: 2m   # silence detector
```

- `timeout` without `step_timeout`: the session must complete within the budget regardless of activity.
- `step_timeout` without `timeout`: the session may run indefinitely as long as it keeps producing stream events.
- Neither: no timeout enforcement (current default behavior).
- Both: whichever deadline fires first kills the process.

## What Counts As A Step

A "step" is any `SemanticEvent` that indicates the provider is actively working. The `step_timeout` deadline resets whenever the stream layer records activity via `LiveMetricsState::last_event_at`.

Concretely, the following events reset the step-timeout deadline:

| Event | `SemanticEvent` Variant | Why |
|---|---|---|
| Assistant text delta | `OutputText` | Model is generating |
| Thinking/reasoning delta | `Reasoning` | Model is reasoning |
| Tool call starts | `ToolCall` | Agent invoked a tool |
| Tool call ends | `ToolResult` | Tool returned a result |
| Subagent starts | `SubagentStart` | Agent spawned a subagent |
| Subagent stops | `SubagentStop` | Subagent completed |
| Permission request | `PermissionRequest` | Agent is waiting for user approval |
| Turn boundary | `TurnStart`, `TurnComplete` | Multi-turn session progress |
| File change | `FileChange` | Agent is writing files |
| Plan update | `PlanUpdate` | Agent updated its plan |

Events that do **not** reset the deadline:

- `SessionStart` — fires once at startup before any real work
- `Info`, `Warning`, `Error` — diagnostic metadata, not work progress
- `ProviderExtension` — unknown provider-specific data; ignored for safety

This matches the existing `LiveMetricsState::record_activity()` and `record_tool_start()`/`record_tool_end()`/`record_subagent_start()`/`record_subagent_stop()` methods that already maintain `last_event_at`.

## Relationship To Existing `last_event_at`

The stream layer already tracks `last_event_at: Option<Instant>` in `LiveMetricsState`. The heartbeat uses it for stall warnings; the OpenCode hang detector uses it for early termination. `step_timeout` reuses this same field as its silence indicator — no new tracking state is needed in the metrics layer.

The difference is enforcement: stall warnings are informational; `step_timeout` is fatal (kills the process).

## Enforcement Point

`step_timeout` is enforced in the same wait loop that currently handles `timeout`, early-termination signals, and SIGINT forwarding.

### Current Flow (in `exec.rs`)

```
run_child_stream_semantic()
  └─ wait_with_signal_and_early_termination()
       ├─ poll child.try_wait()
       ├─ check early_termination_rx
       ├─ check detect_opencode_hang_termination()
       └─ sleep 75ms
```

### Proposed Flow

```
run_child_stream_semantic()
  └─ wait_with_signal_and_early_termination()
       ├─ poll child.try_wait()
       ├─ check early_termination_rx
       ├─ check detect_opencode_hang_termination()
       ├─ check detect_step_timeout()          ← NEW
       └─ sleep 75ms
```

The step-timeout check is a new clause inside the existing 75ms poll loop. It reads `last_event_at` from `LiveMetrics` and compares the silence duration against the configured `step_timeout`.

### In `run_child_capture()` (non-streaming path)

`step_timeout` has no effect in the capture path. There is no stream to observe, so there is no `last_event_at` to track. The wall-clock `timeout` still applies.

### In `run_child()` (passthrough path)

Same as capture: no stream events, no step timeout. Only `timeout` applies.

## Termination Behavior

When `step_timeout` fires:

1. Claudine sends `SIGTERM` to the child's process group.
2. After a 5-second grace period, `SIGKILL`.
3. The attempt's `ProcessTermination` is set to `TimedOut` — same as the existing `timeout` behavior.
4. The `FailureEvent::Timeout` handler resolution path runs, matching `handle_timeout` declarations in frontmatter.

This means `step_timeout` and `timeout` produce the same downstream failure classification. Handler authors do not need to distinguish between the two.

### Ambiguity Resolution

When both `timeout` and `step_timeout` are configured, it is possible (though unlikely) that both fire in the same 75ms poll cycle. In that case:

- `timeout` takes precedence (it is the harder constraint).
- The `TimedOut` termination is reported once; there is no double-kill.

## Data Model Changes

### `HarnessPlan` (library)

```rust
pub struct HarnessPlan {
    pub source_path: PathBuf,
    pub timeout: Option<std::time::Duration>,
    pub step_timeout: Option<std::time::Duration>,  // NEW
    pub pre_checks: Vec<ValidationRule>,
    pub post_checks: Vec<ValidationRule>,
    pub handlers: HandlerTable,
    pub programmatic_handler: Option<ApprovedRuntimeCommand>,
}
```

### `parse_harness_plan` (library)

Parse the new `step_timeout` key from effective frontmatter using the same `parse_timeout()` function:

```rust
let step_timeout = if let Some(v) = obj.get("step_timeout") {
    let raw = v.as_str().ok_or(...)?;
    Some(parse_timeout(raw, source_path)?)
} else {
    None
};
```

### `CompositionExecutionRequest` (library)

```rust
pub struct CompositionExecutionRequest {
    // ...existing fields...
    pub timeout: Option<u64>,
    pub step_timeout: Option<u64>,  // NEW
}
```

### `AttemptLaunch` (CLI)

```rust
struct AttemptLaunch {
    args: Vec<String>,
    env: HashMap<OsString, OsString>,
    stdin_seed: Option<String>,
    timeout: Option<u64>,
    step_timeout: Option<u64>,  // NEW
}
```

### `launch_timeout_secs` → `resolve_launch_timeouts`

The existing `launch_timeout_secs(cli_timeout, plan_timeout) -> u64` function is replaced by a struct-returning helper:

```rust
struct LaunchTimeouts {
    timeout: Option<u64>,
    step_timeout: Option<u64>,
}

fn resolve_launch_timeouts(
    cli_timeout: Option<u64>,
    cli_step_timeout: Option<u64>,
    plan_timeout: Option<std::time::Duration>,
    plan_step_timeout: Option<std::time::Duration>,
) -> LaunchTimeouts {
    LaunchTimeouts {
        timeout: cli_timeout.or_else(|| plan_timeout.map(|d| d.as_secs())),
        step_timeout: cli_step_timeout.or_else(|| plan_step_timeout.map(|d| d.as_secs())),
    }
}
```

### `HARNESS_KEYS` constant

```rust
const HARNESS_KEYS: &[&str] = &[
    "pre_checks", "post_checks", "timeout", "step_timeout", "handle",
];
```

### `has_harness_properties`

Must also check for `"step_timeout"` so the harness loop is activated when only `step_timeout` is present (with no `timeout` or checks).

## CLI Flag

```
--step-timeout <DURATION>
```

Same syntax as the frontmatter value (`30s`, `5m`, `1h`). Parsed with the same `parse_timeout()` function and stored as seconds in `CompositionExecutionRequest::step_timeout`.

Precedence mirrors `--timeout`: CLI flag overrides frontmatter. Explicit frontmatter is used when no CLI flag is given.

### Interactive Mode Restriction

`--step-timeout` is restricted to non-interactive mode, same as `--timeout`. Using `--step-timeout 5m --interactive` is a hard error.

## Handler Interaction

`step_timeout` fires as `FailureEvent::Timeout` — the same event as the existing wall-clock `timeout`. This is intentional:

- The user typically wants the same recovery strategy regardless of which timeout fired.
- Handler authors write one `handle_timeout` block.
- If finer distinction is needed in the future, a `FailureEvent::StepTimeout` variant can be introduced without breaking existing handlers.

## Interaction With Existing Stall Detection

The existing stall-warning system (`HeartbeatPolicy`, `maybe_emit_stall_warning`) is informational only — it prints a warning to stderr but does not kill the process. `step_timeout` is a separate, fatal enforcement layer.

However, the two share the same `last_event_at` field, so:

- A stall warning fires when silence exceeds `stall_threshold` (typically 30s).
- `step_timeout` fires when silence exceeds the configured `step_timeout` (e.g., 2m).
- The stall warning always fires first (shorter threshold), giving the user a heads-up before the kill.

## Sequence Composition

In sequence runs, each step gets its own `step_timeout` (just like `timeout`). The step-timeout deadline is per-attempt, not shared across steps.

## Sequence Step-Level Override

Sequence step objects may carry their own `step_timeout`:

```yaml
sequence:
  - name: quick-check
    step_timeout: 30s
  - name: deep-analysis
    step_timeout: 5m
```

Step-level `step_timeout` overrides the document-level value for that step only. This mirrors how per-step overrides work for other frontmatter properties via the `set` overlay.

## Validation

- `step_timeout` must be positive and non-zero (same as `timeout`).
- `step_timeout` must not exceed `timeout` when both are set. If it does, parse returns `HarnessError::InvalidTimeout` with a message like `"step_timeout (10m) must not exceed timeout (5m)"`.
- `step_timeout` requires structured streaming (`use_structured = true`). If specified in capture or passthrough mode, Claudine emits a warning and ignores it (does not error).

## Test Plan

### Unit Tests (library)

1. `parse_timeout` accepts `step_timeout` string values (already covered by existing tests).
2. `parse_harness_plan` extracts `step_timeout` from frontmatter.
3. `parse_harness_plan` rejects `step_timeout` when it exceeds `timeout`.
4. `has_harness_properties` returns `true` when only `step_timeout` is present.
5. `HarnessPlan::step_timeout` defaults to `None`.

### Integration Tests (CLI)

6. A composition with `step_timeout: 5s` kills a provider that produces no output for 5+ seconds.
7. A composition with `step_timeout: 5s` does **not** kill a provider that keeps emitting `OutputText` events every 3 seconds, even if the total run exceeds 5s.
8. A composition with both `timeout: 10s` and `step_timeout: 3s` where the provider goes silent at 4s fires `step_timeout` first.
9. A composition with both `timeout: 3s` and `step_timeout: 10s` where the provider is actively producing events fires `timeout` at 3s despite ongoing activity.
10. `--step-timeout` CLI flag overrides frontmatter `step_timeout`.
11. `--step-timeout` with `--interactive` is a hard error.
12. `handle_timeout` handler fires for both `timeout` and `step_timeout` terminations.
13. In capture/passthrough mode, `step_timeout` is ignored with a warning.

## Implementation Phases

### Phase 1: Data Model And Parsing

- Add `step_timeout` to `HarnessPlan`.
- Extend `parse_harness_plan` to parse `step_timeout`.
- Extend `HARNESS_KEYS` and `has_harness_properties`.
- Add validation: `step_timeout <= timeout` when both present.
- Add `step_timeout` to `CompositionExecutionRequest`.

### Phase 2: Enforcement In Wait Loop

- Add `step_timeout` to `AttemptLaunch`.
- Add `step_timeout` check to `wait_with_signal_and_early_termination` (both `#[cfg(unix)]` and `#[cfg(not(unix))]` variants).
- Wire `step_timeout` through `build_harness_launch` and `run_harness_loop`.
- Return `ProcessTermination::TimedOut` when step timeout fires.

### Phase 3: CLI Flag And Composition Wiring

- Add `--step-timeout` clap argument to `compose`, `inline-compose`, and non-interactive wrapper subcommands.
- Validate `--step-timeout` + `--interactive` is rejected.
- Wire flag value into `CompositionExecutionRequest::step_timeout`.
- Add interactive-mode validation.

### Phase 4: Sequence Support

- Allow `step_timeout` in sequence step overlay objects.
- Apply per-step `step_timeout` override during sequence execution.

### Phase 5: Docs

- Update `composition.md` to document `step_timeout`.
- Update `validations-and-handlers.md` to add `step_timeout` to the timeout section.
- Update `non-interactive-sessions.md`.
