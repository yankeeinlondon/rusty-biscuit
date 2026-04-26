---
ready: true
agent: ${env.AGENT}
---

# Review 2: Performance Flag (`--perf`)

This review evaluates the implementation of the `--perf` flag against the [specification](./spec.md) and [technical design](./tech-design.md).

## Summary

The implementation of `--perf` is **not ready** for production. While the core reporting for direct wrapper commands and single composition runs is functional and well-architected, the **entire sequence aggregation requirement was skipped**.

## Critical Gaps

### 1. Sequence Support is Missing
The technical design explicitly requires `claudine sequence` to aggregate performance metrics across multiple steps.
- **Missing Structures:** `SequencePerfReport` and `SequenceStepPerf` (defined in the tech design) do not exist in `perf.rs`.
- **Logic Gap:** `claudine/cli/src/commands/sequence.rs` ignores `startup_timings` and does not pass `perf_enabled: true` to its composition steps.
- **Aggregation Gap:** `claudine/cli/src/commands/wrap/sequence.rs` does not collect `agent_perf` from step outcomes and does not render a final aggregated report.

### 2. Arg Parsing Timing Mismatch
- **Tech Design:** "Measured in `main.rs` from just before `argv::normalize(...)` until just after `parse_cli_from(...)`."
- **Implementation:** `perf_arg_parsing` is calculated at the very start of `async_main`, *before* `parse_cli_from(&argv)` is called. This misses the actual clap parsing overhead, which is a significant part of the metric.

## Functional Issues

- **Step Performance Collection:** In `claudine/cli/src/commands/wrap/sequence.rs`, the call to `execute_composition_request_inner` explicitly sets `perf_enabled: false`. This means even if a user passes `--perf` to a sequence, the individual steps do not collect `composition_perf` from Darkmatter.

## Testing Coverage

- **Bootstrap Scan:** Excellent coverage in `perf.rs`.
- **First Response Latency:** Good unit tests in `exec.rs`.
- **Missing:** No integration tests for sequence performance reporting (as the feature is missing).
- **Missing:** No tests verifying that `startup_timings` are correctly propagated through the command hierarchy.

## Ergonomics & Performance Observations

- **Bootstrap Scan:** The `scan_perf_bootstrap` implementation is highly ergonomic and performant, successfully avoiding a full clap parse for telemetry initialization.
- **Telemetry Capture:** `ProcessTelemetry` and `resolve_first_response` in `exec.rs` are well-implemented and provide high-signal latency data.

## Recommendations

1. **Implement `SequencePerfReport`:** Add the missing structures to `perf.rs` and implement the `render_sequence_perf_report` logic.
2. **Wire Sequence Aggregation:** Update `run_sequence` and `execute_sequence` to accept `StartupTimings`, enable performance collection for steps, and aggregate the results into a final report.
3. **Correct Arg Parsing Metric:** Move the `perf_arg_parsing` calculation in `main.rs` to occur *after* `parse_cli_from`.
4. **Add Sequence Perf Tests:** Add a test case in `claudine/cli/tests` (or similar) that runs a multi-step sequence with `--perf` and verifies the aggregated report format.
