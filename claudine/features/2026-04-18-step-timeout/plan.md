---
phases: 5
created: 2026-04-18
start_phase: 3
source_files_during_phase_1:
  - claudine/lib/src/harness/model.rs
  - claudine/lib/src/harness/timeout.rs
  - claudine/lib/src/harness/parse.rs
  - claudine/lib/src/harness/audit.rs
  - claudine/lib/src/harness/validate.rs
  - claudine/lib/src/composition/preflight.rs
  - claudine/lib/src/composition/types.rs
  - claudine/cli/src/commands/compose.rs
  - claudine/cli/src/commands/wrap/sequence.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase1: []
source_files_during_phase_2:
  - claudine/lib/src/stream/logs/opencode.rs
  - claudine/cli/src/commands/wrap/exec.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/composition.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase2: []
packages:
  - claudine
  - claudine-cli
---

# Step Timeout — Execution Plan

## Overview

Implement `step_timeout` — a silence-detection timeout that resets on every stream event, independent of the existing wall-clock `timeout`.

## Phase 1: Data Model and Parsing

### Steps

1. **Add `step_timeout` field to `HarnessPlan`** (`claudine/lib/src/harness/model.rs`)
   - Add `pub step_timeout: Option<std::time::Duration>` beside `timeout`
   - Update all constructor sites to set `step_timeout: None`

2. **Add `format_duration` helper** (`claudine/lib/src/harness/timeout.rs`)
   - Render `Duration` as `{n}s` / `{n}m` user-facing string for error messages

3. **Extend `parse_harness_plan`** (`claudine/lib/src/harness/parse.rs`)
   - Parse `step_timeout` from frontmatter using `parse_timeout()` (same syntax as `timeout`)
   - Validate `step_timeout <= timeout` when both are present; return `HarnessError::InvalidTimeout` on violation
   - Add validation: non-string yields `HarnessError::InvalidFrontmatter`

4. **Update `HARNESS_KEYS` constant** (`claudine/lib/src/harness/parse.rs`)
   - Add `"step_timeout"` to the slice

5. **Update `has_harness_properties`** (`claudine/lib/src/harness/parse.rs`)
   - Ensure harness loop activates when only `step_timeout` is present

6. **Add `step_timeout` to `CompositionExecutionRequest`** (`claudine/lib/src/composition/types.rs`)
   - Add `pub step_timeout: Option<u64>` beside `timeout`
   - Update all call sites to set `step_timeout: None`

7. **Add unit tests**
   - `parse_harness_plan_extracts_step_timeout`
   - `parse_harness_plan_rejects_non_string_step_timeout`
   - `parse_harness_plan_rejects_step_timeout_exceeding_timeout`
   - `parse_harness_plan_accepts_step_timeout_without_timeout`
   - `parse_harness_plan_accepts_timeout_without_step_timeout`
   - `has_harness_properties_returns_true_for_step_timeout_only`
   - `harness_plan_default_step_timeout_is_none`

### Validation Checkpoint
- All Phase 1 unit tests pass
- `cargo test -p claudine-lib -- harness` passes

---

## Phase 2: Enforcement in Wait Loop

### Steps

8. **Add `EarlyTermination::StepTimeout` variant** (`claudine/lib/src/stream/logs/opencode.rs`)
   - Add `StepTimeout { message: String }` to `EarlyTermination` enum
   - Add rustdoc noting it maps to `ProcessTermination::TimedOut`

9. **Add `detect_step_timeout` helper** (`claudine/cli/src/commands/wrap/exec.rs`)
   - Read `last_event_at` from `LiveMetrics`
   - Compare silence duration against configured `step_timeout`
   - Return `Some(EarlyTermination::StepTimeout { message })` when exceeded
   - Return `None` if `last_event_at` is `None` (first-event grace)

10. **Fold wall-clock `timeout` into `wait_with_signal_and_early_termination`** (`claudine/cli/src/commands/wrap/exec.rs`)
    - Add `wall_clock_timeout: Option<Duration>` parameter to the function
    - Add `step_timeout: Option<Duration>` parameter to the function
    - Capture `loop_start = Instant::now()` at top of function
    - Add wall-clock check in poll loop: if `loop_start.elapsed() >= budget`, set `EarlyTermination::WallClockTimeout` pseudo-variant, send SIGTERM
    - Add step-timeout check in poll loop: call `detect_step_timeout`, set variant, send SIGTERM
    - Resolve ambiguity: wall-clock takes precedence (checked first); step-silence branch sees `early_termination.is_some()` and skips

11. **Update `early_termination_process_outcome`** (`claudine/cli/src/commands/wrap/exec.rs`)
    - Map `EarlyTermination::StepTimeout` → `ProcessTermination::TimedOut`

12. **Mirror changes in `#[cfg(not(unix))]` variant** (`claudine/cli/src/commands/wrap/exec.rs`)
    - Same two new parameters, same two new clauses
    - Windows uses `child.kill()` for SIGTERM analog

13. **Add `step_timeout` to `AttemptLaunch`** (`claudine/cli/src/commands/wrap/mod.rs`)
    - Add `pub step_timeout: Option<u64>` field

14. **Replace `launch_timeout_secs` with `resolve_launch_timeouts`** (`claudine/cli/src/commands/wrap/mod.rs`)
    - Return `LaunchTimeouts { timeout, step_timeout }` struct
    - CLI flag overrides frontmatter; `or_else` with `plan_timeout.as_secs()`

15. **Wire `step_timeout` through `build_harness_launch` and `run_harness_loop`** (`claudine/cli/src/commands/wrap/mod.rs`)
    - `build_harness_launch` gains `cli_step_timeout` and `plan_step_timeout` parameters
    - `run_harness_loop` gains `cli_step_timeout: Option<u64>`
    - Add `step_timeout_secs` to `info_span!("harness_launch_plan", ...)`

16. **Add non-streaming warning hook** (`claudine/cli/src/commands/wrap/mod.rs`)
    - In `execute_harness_attempt`, when `use_structured == false` and `step_timeout.is_some()`, emit warning and zero the field

17. **Add unit tests**
    - `detect_step_timeout_fires_after_silence_exceeds_budget`
    - `detect_step_timeout_returns_none_when_recent`
    - `detect_step_timeout_returns_none_when_last_event_at_is_none`
    - `early_termination_process_outcome_maps_step_timeout_to_timed_out`

### Validation Checkpoint
- All Phase 2 unit tests pass
- `cargo test -p claudine-cli -- exec` passes
- `wall_clock_timeout_fires_at_budget_for_streamed_run` regression test passes

---

## Phase 3: CLI Flag and Composition Wiring

### Steps

18. **Add `--step-timeout` to `COMPOSITION_FLAGS_WITH_VALUE`** (`claudine/cli/src/argv.rs`)
    - Add `"--step-timeout"` entry to the list

19. **Add `--step-timeout` to `WrapArgs`** (`claudine/cli/src/commands/wrap/mod.rs`)
    - Parallel to `--timeout`: `#[arg(long = "step-timeout", value_name = "DURATION")]`
    - Parse with `parse_timeout` helper, store as `Option<String>`

20. **Add `--step-timeout` to `ComposeArgs` and `SequenceArgs`** (`claudine/cli/src/commands/wrap/composition.rs`)
    - Same structure as `WrapArgs`

21. **Add interactive-mode rejection**
    - In `WrapArgs`: reject `--step-timeout` + `--interactive`
    - In `ComposeArgs` / `SequenceArgs`: same guard

22. **Wire flag into `CompositionExecutionRequest`** (`claudine/cli/src/commands/wrap/mod.rs`)
    - Parse `step_timeout` string value → `Duration::as_secs()` → `CompositionExecutionRequest::step_timeout`

23. **Add unit tests**
    - `composition_flags_with_value_matches_clap_surface` (sentinel — should now pass)

### Validation Checkpoint
- `cargo build -p claudine-cli` succeeds
- `--step-timeout` appears in `claudine compose --help`

---

## Phase 4: Sequence Support

### Steps

24. **Verify sequence step overlay passes `step_timeout` through** (`claudine/lib/src/composition/sequence.rs`)
    - Overlay state map flows to `set` overlay without key filtering

25. **Add integration test** (`claudine/cli/tests/`)
    - `sequence_per_step_step_timeout_override`: document-level `step_timeout: 2m`, step-level `step_timeout: 10s`, second step stalls at 10s

### Validation Checkpoint
- Sequence integration test passes

---

## Phase 5: Documentation

### Steps

26. **Update `composition.md`**
    - Document `step_timeout` frontmatter property
    - Explain semantics: resets on `SemanticEvent`, streaming-only, precedence rules

27. **Update `validations-and-handlers.md`**
    - Add `step_timeout` to the timeout section
    - Note handler behavior (`handle_timeout` matches both)

28. **Update `non-interactive-sessions.md`**
    - Add `--step-timeout` CLI flag documentation

### Validation Checkpoint
- All doc files updated with no placeholder text

---

## Dependencies

```
Phase 1 ──────────────────────────────────────────────────────────────
  Steps 1-7: Pure library changes, no dependencies

Phase 2 ──────────────────────────────────────────────────────────────
  Steps 8-17: Depend on Phase 1 (data model must exist)
  Steps 10-12: Depend on Step 8 (EarlyTermination variant)
  Steps 14-15: Depend on Steps 1, 13 (AttemptLaunch struct)

Phase 3 ──────────────────────────────────────────────────────────────
  Steps 18-23: Depend on Phase 1 (CompositionExecutionRequest) and Phase 2 (AttemptLaunch)

Phase 4 ──────────────────────────────────────────────────────────────
  Steps 24-25: Depend on Phases 1-3

Phase 5 ──────────────────────────────────────────────────────────────
  Steps 26-28: Depend on all implementation phases
```

## Parallelization Notes

- **Within Phase 1**: Steps 1-6 are independent and can run in parallel
- **Within Phase 2**: Steps 8-9 (EarlyTermination variant + detect function) can parallelize with Steps 13-15 (AttemptLaunch + launch_timeouts), then all converge at Steps 10-12 (wait loop wiring)
- **Phase 3 Steps 18-23**: Steps 19-21 and Steps 22-23 can be parallelized

## Integration Test Summary

| # | Test | Validates |
|---|---|---|
| 13 | `step_timeout_kills_silent_provider` | Silent provider killed at budget; `TimedOut`; `handle_timeout` runs |
| 14 | `step_timeout_tolerates_active_provider` | Active provider (OutputText every 3s) survives 5s budget |
| 15 | `step_timeout_fires_before_wall_clock_timeout` | `timeout: 10s`, `step_timeout: 3s`; stall at 4s → step wins |
| 16 | `wall_clock_timeout_fires_before_step_timeout` | `timeout: 3s`, `step_timeout: 10s`; active provider → wall-clock wins |
| 17 | `cli_step_timeout_overrides_frontmatter` | Flag `--step-timeout 10s` wins over frontmatter `step_timeout: 5m` |
| 18 | `cli_step_timeout_rejects_interactive` | `--step-timeout 5m --interactive` errors |
| 19 | `handle_timeout_runs_for_step_timeout` | Handler fires for step timeout |
| 20 | `step_timeout_ignored_in_capture_mode` | Capture mode warns and ignores |
| 21 | `sequence_per_step_step_timeout_override` | Per-step override kills only that step |
