# Performance Testing

Claudine takes a layered approach to performance testing: Criterion microbenchmarks for library hot paths, `#[ignored]` integration harnesses for end-to-end latency targets, and a `--perf` runtime flag for production observability. Tests are designed to be deterministic, non-gating in CI, and progressively tighter as the runner noise profile gets characterized.

## Philosophy

- **Benchmarks are opt-in, not gating.** Regular `cargo test` and `just test` must never pay the cost of performance harnesses. All expensive perf tests are `#[ignore]`d and invoked explicitly.
- **Measure what matters.** Each benchmark targets a documented latency threshold rooted in a spec or a user-facing SLA (e.g., shell completion p95 ≤ 100 ms).
- **Three-tier feedback.** Criterion gives statistical rigor during development. `--perf` gives real-world timing on every invocation. Integration harnesses validate end-to-end contracts in CI.
- **No false precision.** CI runners are noisy. Benchmarks in CI upload Criterion HTML reports as artifacts for human review rather than failing the build on raw numbers. Regression gates are introduced only after the runner noise profile has been characterized.

## Getting Started

### Run Criterion Benchmarks

The `claudine` library crate has a single Criterion benchmark suite targeting three hot paths:

```bash
# Run all library benchmarks
cargo bench -p claudine --bench runtime_hot_paths

# Run a specific benchmark group
cargo bench -p claudine --bench runtime_hot_paths -- protect_service
cargo bench -p claudine --bench runtime_hot_paths -- stream_parser
cargo bench -p claudine --bench runtime_hot_paths -- runtime_config
```

The suite covers:

| Group | What it measures |
|---|---|
| `protect_service` | Policy evaluation latency for bash commands and write-path checks |
| `stream_parser` | Throughput of the semantic JSON Lines parser feeding a multi-step OpenCode session |
| `runtime_config` | Full config loading → compilation → binding lookup cycle |

Results are written to `target/criterion/` with per-benchmark HTML reports.

### Run the Completion Performance Harness

The shell completion harness builds a monorepo-scale fixture (~48 packages, ~2000 markdown files) and measures p50/p95/p99 latency for three cursor slots. It is `#[ignore]`d by default.

```bash
# Build first (the harness spawns the claudine binary)
cargo build -p claudine-cli

# Run the harness
cargo test -p claudine-cli --test completion_perf -- --ignored --nocapture
```

The harness validates against spec targets:

| Region | p95 range | Behavior |
|---|---|---|
| Pass | ≤ 100 ms | Test passes |
| Warning | 100–150 ms | Printed warning; no failure |
| Cache trigger | > 150 ms | Test panics, forcing implementation of the fallback cache |

Enable detailed tracing during the run for per-phase breakdowns:

```bash
CLAUDINE_COMPLETION_PROFILE=1 \
RUST_LOG=claudine::completion=trace \
cargo test -p claudine-cli --test completion_perf -- --ignored --nocapture
```

### Run the Sequence Performance Integration Tests

These tests validate that `claudine sequence --perf` produces a correctly aggregated performance report, including startup timings propagation and partial-report behavior on fail-fast.

```bash
cargo test -p claudine-cli --test sequence_perf
```

Unlike the completion harness, these run as regular integration tests (not `#[ignore]`d) since they validate report structure, not latency numbers.

### Use the `--perf` Runtime Flag

Any wrapper, compose, inline-compose, or sequence command accepts `--perf` to emit a structured timing breakdown to stderr on completion:

```bash
claudine compose --claude --perf prompt.md
claudine sequence --goose --perf sequence.md
claudine opencode "explain this" --perf
```

The report includes:

- **CLI Overhead** — arg parsing, config loading, tracing init, environment setup
- **Source Context Timing** — invocation capture, repository observation,
  topology initialization when required, and launch-context capture; these are
  overlapping diagnostic timings, while the adjacent work note reports stable
  Git discovery and topology counts
- **Composition Report** — interpolation, shell expansion, transclusion stages
- **Agent Execution** — provider handoff, launches, first-response latency,
  total execution, and provider API time

This is the primary tool for diagnosing performance in real workflows.

### Run the Performance Review Prompt

The prompt at `.ai/prompts/performance-review.md` drives an evidence-based code review across runtime, memory, async, I/O, parsing, data structures, API architecture, and compile-time dimensions. Invoke it through claudine compose or your preferred agent.

## CI/CD Integration

### Current State

The `sniff-performance.yml` GitHub Actions workflow runs a narrow Criterion subset on PRs that touch `sniff/**`, the workspace `Cargo.toml`, or the workflow file itself. It uploads Criterion HTML reports as 14-day artifacts and does **not** fail the build on regressions — this is intentional while the runner noise profile is being characterized.

### Recommended Workflow Integration

Performance testing should be integrated into the development lifecycle at three checkpoints:

**1. PR gate (advisory)**

- Run the narrow Criterion subset on every PR touching `claudine/**`.
- Upload Criterion HTML reports as artifacts for reviewer inspection.
- Do not fail the build on raw number regressions until a stable baseline has been established on the CI runner.
- Fail the build only on structural issues: missing benchmark IDs, fixture construction failures, or harness assertion failures.

**2. Merge gate (baseline comparison)**

- After the runner noise profile has been characterized (typically after 20+ runs on the same runner type), introduce a `--fail-fast` baseline comparison using Criterion's `--save-baseline` and `--baseline` flags.
- Store a reference baseline in the repository (e.g., `benches/baselines/ci-main.json`) updated on merge to main.
- Set a regression threshold (e.g., 10% slowdown on any benchmark) that triggers a warning comment on the PR rather than a hard failure.

**3. Release gate (end-to-end harnesses)**

- Run the completion performance harness (`completion_perf`) against release candidates.
- Validate that p95 latency meets the spec target (≤ 100 ms) on the reference runner.
- Run the sequence perf integration tests to verify report structure.
- These harnesses are too expensive for every PR but should gate releases.

### Adding New Benchmarks

When adding a new Criterion benchmark to `claudine/lib/benches/runtime_hot_paths.rs`:

1. Register it in the `criterion_group!` macro at the bottom of the file.
2. Add the benchmark ID to `benches/ci-bench-ids.txt` if it should run in CI.
3. Run the `bench_ids_sync` integration test to validate the IDs file stays in sync.
4. Document the benchmark and its expected latency characteristics in this file.

When adding a new `#[ignore]`d integration harness:

1. Place it under `claudine/cli/tests/` with a `_perf` suffix.
2. Document the invocation command in the module-level doc comment.
3. Add the invocation command to this file's Getting Started section.
4. If the harness has a spec target, document the pass/warning/fail thresholds.

### Profiling Tools

For deeper investigation beyond what Criterion provides:

| Tool | Use case |
|---|---|
| `cargo flamegraph` | Identify hot functions in the stream parser or protect service |
| `instruments` (macOS) | Time profiler for CLI startup and agent wrapper overhead |
| `heaptrack` | Heap allocation profiling for config loading and composition |
| `dhat` | Fine-grained allocation attribution in benchmarks |
| `cargo bloat` | Binary size analysis for release builds |
| `tokio-console` | Async task inspection for concurrent agent execution |
| `hyperfine` | Statistical wall-clock comparison of CLI subcommands |
