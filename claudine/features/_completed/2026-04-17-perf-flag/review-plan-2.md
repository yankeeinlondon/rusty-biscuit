---
ready: true
agent: ${env.AGENT}
---

# Review 2 Implementation Plan: Performance Flag (`--perf`)

This plan addresses all issues identified in [review-2.md](./review-2.md).

## Issues Summary

1. **Sequence Support Missing**: No sequence-level perf aggregation structures; sequence commands ignore `startup_timings`; `execute_composition_request_inner` receives `perf_enabled: false`.
2. **Arg Parsing Timing Mismatch**: `perf_arg_parsing` is measured before `parse_cli_from(&argv)`, missing actual clap parsing overhead.
3. **Step Performance Collection**: Sequence wrapper hardcodes `perf_enabled: false`, blocking composition and agent perf collection per step.
4. **Missing Tests**: No integration tests for sequence perf reporting; no tests for `startup_timings` propagation.

---

## Phase 1: Fix Arg Parsing Timing Mismatch

**File**: `claudine/cli/src/main.rs`

**Problem**: `perf_bootstrap.started_at` is set at the end of `scan_perf_bootstrap()` (before `argv::normalize`). In `async_main`, `perf_arg_parsing` is calculated before `parse_cli_from(&argv)`, so it captures `argv::normalize` + runtime setup but **misses clap parsing**.

**Solution**: Move the arg-parsing timer to start before `argv::normalize` in `run()` and stop after `parse_cli_from` in `async_main`.

### Changes

1. **In `run()`** (line ~143-165):
   - Add `let arg_parse_start = std::time::Instant::now();` immediately before `let argv = argv::normalize(raw_argv);`.
   - Pass `arg_parse_start` into `async_main`.

2. **Update `async_main` signature**:
   ```rust
   async fn async_main(
       argv: Vec<OsString>,
       perf_bootstrap: perf::PerfBootstrap,
       arg_parse_start: std::time::Instant,
   ) -> Result<()>
   ```

3. **In `async_main`** (line ~168-174):
   - Move the `perf_arg_parsing` calculation to **after** `parse_cli_from`:
   ```rust
   let cli = parse_cli_from(&argv);
   let perf_arg_parsing = arg_parse_start.elapsed();
   ```
   - Remove the existing early `perf_arg_parsing` calculation.

4. **Rationale**: This captures the full arg-parsing pipeline: `argv::normalize` → `is_plain` check → `parse_cli_from` (including both strict and lenient clap passes for wrappers).

---

## Phase 2: Add Sequence Performance Types and Renderer

**File**: `claudine/cli/src/perf.rs`

**Problem**: `SequencePerfReport` and `SequenceStepPerf` structures are missing. The sequence orchestrator has no data model for aggregating per-step metrics.

**Solution**: Add sequence-specific perf accumulator and a dedicated renderer that produces the aggregated report format required by the spec.

### Changes

1. **Add `SequenceStepPerf`**:
   ```rust
   #[derive(Debug, Clone)]
   pub(crate) struct SequenceStepPerf {
       pub step_index: usize,
       pub step_name: String,
       pub compose_perf: Option<darkmatter::markdown::compose::ComposePerfReport>,
       pub agent_perf: Option<AgentExecutionPerf>,
   }
   ```

2. **Add `SequencePerfAccumulator`**:
   ```rust
   pub(crate) struct SequencePerfAccumulator {
       startup: StartupTimings,
       env_setup_started_at: Option<std::time::Instant>,
       env_setup_elapsed: std::time::Duration,
       steps: Vec<SequenceStepPerf>,
       dry_run: bool,
       partial: bool,
   }
   ```

3. **Implement `SequencePerfAccumulator`**:
   - `new(startup: StartupTimings) -> Self`: starts env_setup timer.
   - `mark_env_setup_complete(&mut self)`: captures env setup duration.
   - `add_step(&mut self, step: SequenceStepPerf)`: appends a step result.
   - `set_dry_run(&mut self)`: marks as dry run.
   - `set_partial(&mut self)`: marks as partial (interrupted / fail-fast).
   - `into_report(self, total_elapsed: Duration) -> CommandPerfReport`: aggregates and renders.

4. **Aggregation logic in `into_report`**:
   - **Composition**: Merge all step `compose_perf` values using `ComposePerfReport::merge`. Only include the composition block if at least one step has compose perf.
   - **Agent Execution**:
     - `launches`: sum of all step `agent_perf.launches`.
     - `total_elapsed`: sum of all step `agent_perf.total_elapsed`.
     - `first_response_latency`: compute `avg` and `min` across all steps that have latency. Store both in `notes` (e.g., `first response avg: 1.2s, min: 0.8s`). For the `first_response_latency` field itself, use the **average** so the standard renderer shows a meaningful number.
     - `provider_api_duration`: sum of all step `provider_api_duration`.
   - **Notes**: Add `"partial sequence metrics"` if `self.partial` is true. Add `"Agent execution skipped (dry run)"` if `self.dry_run`.

5. **Reuse existing `render_perf_report`**: `SequencePerfAccumulator::into_report` produces a standard `CommandPerfReport` with `title: "Sequence"`. The existing renderer handles the visual layout; no new renderer needed.

---

## Phase 3: Wire Sequence Command to Accept and Pass Performance Data

**Files**:
- `claudine/cli/src/commands/sequence.rs`
- `claudine/cli/src/commands/wrap/sequence.rs`

**Problem**: `run_sequence_inner` discards `startup_timings` (`_startup_timings`). `execute_sequence` does not accept perf parameters. The call to `execute_composition_request_inner` passes `perf_enabled: false`, so even when the user passes `--perf`, steps do not collect agent execution perf.

### Changes in `commands/sequence.rs`

1. **In `run_sequence_inner`**:
   - Rename `_startup_timings` to `startup_timings`.
   - Pass `shared.perf` and `startup_timings` into `execute_sequence`.

2. **Update `execute_sequence` call** (line ~103):
   ```rust
   super::wrap::sequence::execute_sequence(
       &source,
       plan,
       &shared,
       set_overrides,
       execution_options,
       verbose,
       shared.perf,
       startup_timings,
   )
   ```

### Changes in `commands/wrap/sequence.rs`

1. **Update `execute_sequence` signature**:
   ```rust
   pub(crate) fn execute_sequence(
       source: &ResolvedCompositionSource,
       plan: SequencePlan,
       shared: &SharedComposeArgs,
       user_set_overrides: Option<serde_json::Value>,
       execution_options: SequenceExecutionOptions,
       verbose: u8,
       perf_enabled: bool,
       startup_timings: Option<crate::perf::StartupTimings>,
   ) -> Result<i32>
   ```

2. **Build perf accumulator when enabled**:
   - After extracting `silent`, create the accumulator:
   ```rust
   let mut perf_accumulator = if perf_enabled {
       startup_timings.map(|timings| {
           crate::perf::SequencePerfAccumulator::new(timings)
       })
   } else {
       None
   };
   ```

3. **Capture env setup time**:
   - After Phase 1 pre-flight loop completes (before the "Preflight: shell commands approved" message), call `perf_accumulator.as_mut().map(|a| a.mark_env_setup_complete());`.

4. **Enable per-step perf collection**:
   - Change line ~288 from:
     ```rust
     let step_result = super::composition::execute_composition_request_inner(
         request, verbose, None, false
     );
     ```
   to:
     ```rust
     let step_result = super::composition::execute_composition_request_inner(
         request, verbose, None, perf_enabled
     );
     ```
   - Rationale: `startup_timings` is `None` for individual steps because the CLI overhead was already measured at sequence entry. Each step still collects its own `compose_perf` (already prepared) and `agent_perf` (from child execution).

5. **Collect step perf from outcomes**:
   - In the `Ok(outcome) if outcome.exit_code == 0` branch (and other `Ok(outcome)` branches):
     ```rust
     if let Some(ref mut acc) = perf_accumulator {
         acc.add_step(crate::perf::SequenceStepPerf {
             step_index,
             step_name: step.name.clone(),
             compose_perf: step_ctx.prepared.compose_perf.clone(),
             agent_perf: outcome.agent_perf,
         });
     }
     ```
   - Note: `step_ctx.prepared.compose_perf` contains the Darkmatter compose perf captured during Phase 1 preparation (because `prepare_options.perf_enabled: shared.perf` was already set correctly on line 178).

6. **Handle interruption / fail-fast partial metrics**:
   - In the interrupt and early-break paths, call `perf_accumulator.as_mut().map(|a| a.set_partial());`.

7. **Render final aggregated report**:
   - After the final sequence summary (after line ~422), add:
   ```rust
   if let Some(acc) = perf_accumulator {
       let total = sequence_start.elapsed(); // need to add this timer at function start
       let report = acc.into_report(total);
       eprint!("{}", crate::perf::render_perf_report(&report));
   }
   ```
   - Add `let sequence_start = std::time::Instant::now();` at the very beginning of `execute_sequence`.

---

## Phase 4: Add Comprehensive Tests

### 4a. Unit Tests for Sequence Perf Aggregation

**File**: `claudine/cli/src/perf.rs` (in the existing `#[cfg(test)]` module)

Add tests:

1. `sequence_perf_accumulator_empty`:
   - Create accumulator, mark env setup complete, convert to report.
   - Assert report has `title == "Sequence"`, `composition: None`, `agent: None`.

2. `sequence_perf_accumulator_merges_composition`:
   - Add two steps with `ComposePerfReport` values.
   - Assert merged report composition totals equal sum of inputs.
   - Assert metrics are aggregated by stage (e.g., `Interpolation` elapsed sums).

3. `sequence_perf_accumulator_aggregates_agent_perf`:
   - Add two steps with `AgentExecutionPerf` (launches=1, total=1s, first_response=500ms, api=800ms each).
   - Assert `agent.launches == 2`, `agent.total_elapsed == 2s`.
   - Assert `agent.first_response_latency` is the average (750ms).
   - Assert report notes contain `"first response avg:"` and `"min:"`.

4. `sequence_perf_accumulator_partial_note`:
   - Create accumulator, call `set_partial()`, convert.
   - Assert notes contain `"partial sequence metrics"`.

### 4b. Integration Test for Sequence `--perf`

**File**: `claudine/cli/tests/sequence_perf.rs` (new file)

Test: `sequence_perf_renders_single_aggregated_report`

Setup:
- Create a temp workspace with a fake provider binary (e.g., `goose`).
- Write a markdown file with a 2-step sequence.
- Run `claudine sequence --perf --goose <file>`.

Assertions:
- Exit code is 0.
- `stderr` contains exactly **one** `Performance` block.
- `stderr` contains `CLI Overhead`.
- `stderr` contains `Agent Execution`.
- `stderr` contains `launches: 2` (or however many steps ran).
- `stderr` does **not** contain per-step `Composition Report` sections (only the aggregated one, or none if no composition occurred).
- `stderr` contains `Sequence finished` before the perf block.

Test: `sequence_perf_with_fail_fast_still_renders_partial_report`

Setup:
- 3-step sequence where step 2 fails.
- `--perf --fail-fast true`.

Assertions:
- Exit code is non-zero.
- `stderr` contains exactly one `Performance` block.
- `stderr` contains `partial sequence metrics` note.
- `stderr` contains `launches: 1` (only first step ran).

### 4c. Test for `startup_timings` Propagation

**File**: `claudine/cli/tests/sequence_perf.rs` (or extend existing)

Test: `sequence_perf_propagates_startup_timings`

This is an integration-level smoke test that verifies the entire chain:
- The bootstrap scan detects `--perf` for sequence.
- `main.rs` passes `startup_timings` to `run_sequence`.
- The final report includes `arg parsing`, `config loading`, and `tracing init` lines under `CLI Overhead`.

Assertion:
- `stderr` contains `arg parsing:` and `config loading:` and `tracing init:`.

### 4d. Unit Test for Arg Parsing Timing Fix

**File**: `claudine/cli/src/main.rs` (add `#[cfg(test)]` module if not present, or extend existing)

This is trickier to unit-test because `parse_cli_from` uses `std::env::args_os`. An alternative:

Test in `claudine/cli/tests/`:

Test: `perf_arg_parsing_includes_clap_time`

Setup:
- Run `claudine --perf compose <file>` with a valid compose file.
- Capture stderr.

Assertion:
- `arg parsing:` line shows a non-zero duration (e.g., not `0µs`).
- This is a smoke test; exact timing is environment-dependent.

A simpler unit test for the timer wiring:
**File**: `claudine/cli/src/main.rs`

Add a small test that verifies `async_main` computes `perf_arg_parsing` after `parse_cli_from` by mocking:
- Not easily mockable. Rely on integration tests instead.

---

## Phase 5: Remove Lint Warnings and Verify

### Lint Check

Run:
```bash
cargo clippy -p claudine-cli -- -D warnings
```

**Expected issues to watch for**:
- `#[allow(dead_code)]` on `PerfBootstrap` and `CliOverheadReport` may now be removable if sequence usage makes them fully used. Check and remove unnecessary `#[allow(dead_code)]` attributes.
- The `perf` module still has `#[allow(dead_code)]` on several structs. After sequence wiring, verify which ones are truly unused and keep only necessary allowances.

### Compile Check

```bash
cargo check -p claudine-cli
```

### Test Run

```bash
cargo test -p claudine-cli
```

All existing tests must pass. New tests must pass.

### Integration Test Run

```bash
cargo test -p claudine-cli --test sequence_perf
```

---

## Recommended Implementation Order

1. **Phase 1** (arg parsing fix) — small, independent, unblocks correct timing for all commands.
2. **Phase 2** (sequence perf types) — adds data model; can be compiled and unit-tested in isolation.
3. **Phase 3** (wire sequence command + wrap/sequence.rs) — connects types to execution; the core functional fix.
4. **Phase 4** (tests) — validate the fixes.
5. **Phase 5** (lint + verify) — clean up and confirm green build.

---

## Files Modified Summary

| File | Changes |
|------|---------|
| `claudine/cli/src/main.rs` | Move arg-parsing timer; pass `arg_parse_start` to `async_main` |
| `claudine/cli/src/perf.rs` | Add `SequenceStepPerf`, `SequencePerfAccumulator`, aggregation logic, unit tests |
| `claudine/cli/src/commands/sequence.rs` | Pass `perf_enabled` and `startup_timings` to `execute_sequence` |
| `claudine/cli/src/commands/wrap/sequence.rs` | Accept perf params; build accumulator; enable step perf; collect outcomes; render final report |
| `claudine/cli/tests/sequence_perf.rs` | New integration tests for sequence perf reporting |

---

## Verification Checklist

- [ ] `cargo clippy -p claudine-cli -- -D warnings` passes with zero warnings.
- [ ] `cargo test -p claudine-cli` passes (all existing + new tests).
- [ ] `claudine sequence --perf <file>` emits exactly one `Performance` block at the end.
- [ ] `claudine sequence --perf <file>` aggregated report includes `CLI Overhead`, `Agent Execution`, and merged `Composition Report` when applicable.
- [ ] `claudine compose --perf <file>` still works (regression test).
- [ ] `claudine codex "prompt" --perf` still works (regression test).
- [ ] Arg parsing duration is non-zero and includes clap overhead.
