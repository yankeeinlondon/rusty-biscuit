---
ready: false
reviewer: opencode
 date: 2026-04-21
---

# Review: Performance Flag (`--perf`) — `claudine/features/2026-04-17-perf-flag`

## Executive Summary

The `--perf` feature is **not ready for production**. Only **Phase 1** of the 6-phase execution plan has been implemented: the CLI flag surface, the raw-argv bootstrap scan, and startup-timing measurement in `main.rs` are all in place. Phases 2–6 — composition perf capture, child execution telemetry, the shared perf renderer, wrapper reporting, compose/inline-compose reporting, sequence aggregation, documentation, and integration tests — are **entirely missing**. As a result, `--perf` parses successfully but **never emits any performance report**.

---

## Critical Findings

### 1. The feature is fundamentally non-functional (Severity: Critical)

Running any supported command with `--perf` today will parse the flag, measure startup timings, and then **silently discard them**. No report is ever rendered to stderr (or anywhere else).

Evidence:

- `claudine/cli/src/perf.rs` defines `CliOverheadReport` with an `environment_setup` field, but there is **no `AgentExecutionPerf`**, **no `CommandPerfReport`**, and **no `render_perf_report()`** function.
- `claudine/cli/src/commands/wrap/mod.rs::run_provider_wrapper_inner(...)` receives `_startup_timings` (underscore-prefixed, explicitly unused) and never constructs a collector or renders a report.
- `claudine/cli/src/commands/compose.rs::run_compose_inner(...)` and `run_inline_compose_inner(...)` receive `_startup_timings` and do not use them.
- `claudine/cli/src/commands/sequence.rs::run_sequence_inner(...)` receives `_startup_timings` and does not use them.

The entire downstream perf pipeline — collection, aggregation, rendering, emission — is absent.

### 2. Composition perf data is discarded (Severity: Critical)

`claudine/lib/src/composition/prepare.rs` still throws away the Darkmatter compose report:

```rust
let (composed, _report) = source.markdown.compose_with(compose_opts)...
```

The design specifies:

- `PrepareOptions` should gain `perf_enabled: bool` — **not implemented**.
- `PreparedComposition` should gain `compose_perf: Option<ComposePerfReport>` — **not implemented**.
- `prepare_direct` and `prepare_inline` should call `ComposeOptions::with_perf(...)` and retain `report.perf` — **not implemented**.

Because of this, even if a report renderer existed, the `Composition Report` section would always be empty.

### 3. Child execution telemetry is absent (Severity: Critical)

`claudine/cli/src/commands/wrap/exec.rs` defines `ProcessResult<T>` with only `data` and `termination` fields. The design specifies extending it with `ProcessTelemetry` (total elapsed, first-response latency). None of the three child execution helpers (`run_child`, `run_child_capture`, `run_child_stream_semantic`) capture or return execution timing.

Consequences:

- `Agent Execution` section cannot report total execution time.
- First-response latency is never measured.
- Provider API duration (`StreamExecutionSummary.duration_api_ms`) is never surfaced in the perf report.

### 4. `environment_setup` is never measured (Severity: High)

`CliOverheadReport.environment_setup` exists as a field, but:

- `StartupTimings` (the struct actually threaded from `main.rs`) does **not** include it.
- No timer is started at the end of startup and stopped just before the first child launch or dry-run output.
- For wrappers, the design specifies timing from the end of `run_provider_wrapper_inner`'s early setup through binary resolution, prompt extraction, env planning, MCP setup, harness discovery, and preflight — none of this is instrumented.
- For composition, the design specifies timing around provider selection, binary resolution, env planning, MCP setup, system prompt resolution, and harness detection — none of this is instrumented.

### 5. Sequence aggregation is entirely absent (Severity: High)

The design calls for a `SequencePerfAccumulator` that aggregates:

- Phase 1 preflight/preparation into `environment_setup`
- Per-step `ComposePerfReport` via `ComposePerfReport::merge(...)`
- Per-step child execution telemetry (launches, total agent time, first-response latencies, provider API duration)
- A single final report with `first response avg` and `first response min`
- A `partial sequence metrics` note when interrupted or fail-fast stopped the run early

None of this exists. `execute_sequence` in `wrap/sequence.rs` does not reference perf at all after receiving the unused `_startup_timings` parameter.

### 6. Documentation not updated (Severity: Medium)

The design explicitly requires updating:

- `claudine/README.md`
- `claudine/cli/README.md`
- `claudine/docs/topics/composition.md`
- `claudine/docs/cli/sequence.md`

None of these files mention `--perf`. Users have no way to discover the flag or understand its behavior.

---

## Test Coverage Assessment

### Existing tests (adequate for what they cover)

`claudine/cli/src/perf.rs` has unit tests for the bootstrap scan:

- Enabled for wrapper, compose, inline-compose, sequence
- Disabled without `--perf`
- Disabled for unsupported commands (`hooks`, `logs`)
- Ignores `--perf` after `--`
- Handles empty argv

These are correct and well-scoped.

### Missing tests (Severity: Critical)

The following test categories from the tech design are **entirely absent**:

1. **Wrapper passthrough extraction** of `--perf` before/after `--` boundary — no tests in `wrap/mod.rs` verify that `extract_wrapper_flags_from_passthrough` lifts `--perf`.
2. **Composition prep** preserving `compose_perf` only when enabled — no tests in `claudine/lib`.
3. **Process telemetry** choosing first-response timestamps in correct precedence order — no tests in `exec.rs`.
4. **Report renderer** omitting the composition block when no composition occurred — no renderer exists to test.
5. **Integration tests** for any command surface:
   - `claudine compose --perf ...`
   - `claudine inline-compose --perf ...`
   - `claudine sequence --perf ...`
   - Direct wrapper with structured provider
   - Direct wrapper with legacy provider path
   - `--dry-run --perf`

The feature has **zero integration test coverage** proving that a perf report actually appears on stderr.

---

## Architecture / Ergonomic Observations

### What was done well

1. **Bootstrap scan design is clean.** `scan_perf_bootstrap` correctly limits itself to supported commands, respects the `--` boundary, and keeps the disabled path cheap. The completion-mode guard is correct.
2. **Startup timing measurement in `main.rs` is correct.** Arg parsing, tracing init, and config loading are measured at the right boundaries and threaded into handlers via `Option<StartupTimings>`.
3. **Flag plumbing is consistent.** `WrapperArgs.perf`, `SharedComposeArgs.perf`, and `ExtractedWrapperFlags.perf` all exist, and the passthrough extractor correctly lifts `--perf` with the same boundary behavior as `--quiet` and `--repo`.
4. **The non-interactive safety contract is respected.** The bootstrap scan does not depend on user input.

### What needs improvement

1. **`StartupTimings` should include `environment_setup` or the field should be removed from `CliOverheadReport`.** Currently the two structs are mismatched: `CliOverheadReport` has four fields but `StartupTimings` only carries three. Either `main.rs` should measure environment setup (which is impossible because environment setup happens inside the command handlers), or `StartupTimings` should be renamed/reshaped to make clear it only carries the *bootstrap* portion of CLI overhead.
   - **Recommendation:** Keep `StartupTimings` as the three bootstrap fields. Have each command handler measure `environment_setup` locally from the point it receives `StartupTimings` until just before the first child launch, then construct the full `CliOverheadReport`.

2. **`SingleCompositionOutcome` should be extended now, even if perf is optional.** The design calls for `SingleCompositionOutcome { exit_code, provider, perf: Option<AgentExecutionPerf> }`. Currently it only has `exit_code` and `provider`. Extending it would make the sequence orchestrator's aggregation path possible without refactoring the return type later.

3. **`PreparedComposition` and `PrepareOptions` need the perf fields.** These are library-level changes that block Phase 2 and everything after it. They should be the very next changes landed.

4. **Consider a `PerfCollector` struct per command type rather than ad-hoc locals.** The design mentions a `WrapperPerfCollector` and a `SequencePerfAccumulator`. Having explicit collector types keeps aggregation logic testable and prevents the command handlers from accumulating a sprawl of `Option<Duration>` locals.

---

## Recommended Implementation Order

This is essentially the original execution plan, because none of Phases 2–6 have started:

1. **Phase 2 (Library):** Add `perf_enabled` to `PrepareOptions`, `compose_perf` to `PreparedComposition`, and update `prepare_direct`/`prepare_inline` to retain `report.perf`. Add unit tests.
2. **Phase 3 (CLI telemetry + renderer):** Add `ProcessTelemetry`, extend `ProcessResult<T>`, instrument `run_child`/`run_child_capture`/`run_child_stream_semantic` for total elapsed and first-response latency. Add `AgentExecutionPerf`, `CommandPerfReport`, and `render_perf_report()` to `perf.rs`. Add unit tests for renderer and latency precedence.
3. **Phase 4 (Wrapper):** Build a `WrapperPerfCollector`, measure `environment_setup` in `run_provider_wrapper_inner`, collect `ProcessTelemetry` from exec helpers, and render the report after the wrapper completion path.
4. **Phase 5 (Compose + Inline-compose):** Extend `CompositionExecutionRequest` and `SingleCompositionOutcome` to carry perf, measure composition environment setup, collect child telemetry, and render after the summary/closure paths.
5. **Phase 6 (Sequence + docs + tests):** Build `SequencePerfAccumulator`, instrument Phase 1 and Phase 2 separately, render one aggregated report after the sequence summary. Update READMEs and CLI docs. Add integration tests for all four command surfaces plus dry-run.

---

## Release Gate Status

| Criterion | Status |
|---|---|
| `--perf` available on direct wrappers, compose, inline-compose, sequence | ✅ Flag exists on all surfaces |
| Report emitted to `stderr` only when `--perf` enabled | ❌ No report is ever emitted |
| `CLI Overhead` includes arg parsing, config loading, tracing init, environment setup | ⚠️ Arg/config/tracing measured; `environment_setup` never measured |
| `Composition Report` appears when composition occurred | ❌ `prepare.rs` discards Darkmatter perf data |
| `Agent Execution` reports total execution and first-response latency | ❌ No child execution telemetry exists |
| `sequence` emits one aggregated report at end | ❌ Sequence has no perf aggregation |
| stdout matches non-perf invocation | ❌ Cannot verify — no perf output path exists |
| Documentation updated | ❌ No docs mention `--perf` |
| Strong unit and integration test coverage | ⚠️ Bootstrap scan only; zero integration coverage |

**Verdict: `ready: false`.**
