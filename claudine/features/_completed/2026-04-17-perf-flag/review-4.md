---
ready: true
agent: ${env.AGENT}
---

# Feature Review: Performance Flag (`--perf`)

This review evaluates the implementation of the `--perf` flag across Claudine's wrapper and composition surfaces, based on the [specification](./spec.md) and [technical design](./tech-design.md).

## Overview

The implementation successfully introduces a global performance monitoring capability that captures metrics across all major execution phases:
1.  **CLI Overhead:** Arg parsing, config loading, tracing initialization, and environment setup.
2.  **Composition:** Darkmatter-level prompt interpolation and expansion metrics.
3.  **Agent Execution:** Launch counts, first-response latency, and total provider execution time.

## Functional Gaps

No significant functional gaps were identified. The implementation fulfills all requirements:
- Early detection of the `--perf` flag via `PerfBootstrap` ensures that startup timings are captured accurately even before formal argument parsing.
- Tracing spans now include duration metadata, providing a detailed view of the execution pipeline.
- Aggregated reporting for sequences prevents stderr clutter while providing a holistic view of multi-step performance.

## Implementation Quality

- **Surgical Integration:** The use of `CommandPerfCollector` and `SequencePerfAccumulator` in the CLI layer provides a clean separation between timing logic and command execution.
- **Protocol Hardening:** The `ProcessTelemetry` logic in `exec.rs` correctly resolves "first response" latency by considering both semantic events and raw stdout/stderr streams, ensuring accurate metrics across different provider types.
- **Bootstrap Logic:** The `scan_perf_bootstrap` logic correctly identifies the `--perf` flag in raw `argv` while respecting the `--` separator, avoiding false positives from passthrough arguments.

## Ergonomics & Performance

- **Aesthetic Rendering:** The perf report uses `biscuit-terminal` components (`BlockQuote`, `Prose`) to deliver a high-signal, visually distinct summary on stderr. The use of humanized units (µs, ms, s) via `fmt_duration` is appropriate for the scale of the metrics captured.
- **Sequence Aggregation:** The aggregation logic for sequences (averaging first-response latency while summing total execution) is a sensible ergonomic choice for summarizing multi-agent runs.
- **Zero Impact when Disabled:** When `--perf` is not present, the performance tracking overhead is negligible, as most timing operations and collector initializations are guarded.

## Test Coverage

Test coverage is excellent:
- **Unit Tests:** `perf.rs` and `prepare.rs` include exhaustive tests for bootstrap detection, duration formatting, and report construction.
- **Integration Tests:** `sequence_perf.rs` and `wrap_commands.rs` (via `perf_*` tests) verify end-to-end behavior, including startup timing propagation and partial report generation during fail-fast scenarios.

## Conclusion

The feature is robust, well-tested, and adheres strictly to the technical design. It provides significant value for diagnosing performance bottlenecks in complex composition workflows.

**Status: Ready for Production**
