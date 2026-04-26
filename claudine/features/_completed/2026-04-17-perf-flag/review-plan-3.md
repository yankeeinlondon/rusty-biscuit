---
ready: true
agent: claude
---

# Review 3 Implementation Plan: Performance Flag (`--perf`)

This plan addresses all findings from [review-3.md](./review-3.md).

## Findings Summary

| # | Finding | Severity | Files |
|---|---------|----------|-------|
| 1 | Documentation never landed | Must fix | `README.md`, `cli/README.md`, `docs/topics/composition.md`, `docs/cli/sequence.md` |
| 2 | Harness retries undercount launches and total time | Should fix | `cli/src/commands/wrap/mod.rs`, `cli/src/commands/wrap/exec.rs` |
| 3 | Sequence interrupts can leave `partial` unset | Should fix | `cli/src/commands/wrap/sequence.rs` |
| 4 | `set_dry_run` on `SequencePerfAccumulator` is never called | Nice to have | `cli/src/perf.rs`, `cli/src/commands/wrap/sequence.rs` |
| 5 | Duplicate `provider_api_duration` assignment in `into_report` | Nice to have | `cli/src/perf.rs` |
| 6 | `--perf` ignores `--silent` and `--quiet` | Nice to have | `cli/src/commands/wrap/mod.rs`, `cli/src/commands/wrap/composition.rs`, `cli/src/commands/wrap/sequence.rs` |
| 7 | `CompositionPerfCollector` and `WrapperPerfCollector` are near-duplicates | Nice to have | `cli/src/commands/wrap/composition.rs`, `cli/src/commands/wrap/mod.rs`, `cli/src/perf.rs` |
| 8 | `tech-design.md` documentation drift | Trivial | `tech-design.md` |

---

## Phase 1: Documentation Updates

**Goal**: Add `--perf` documentation to all four identified files.

### 1.1 `claudine/README.md`

**Location**: After the "Composition" section (around line 116), before "Getting Started".

**Change**: Add a new subsection under "Composition" (or as a sibling if more appropriate):

```markdown
### Performance Reporting

All wrapper and composition commands support an opt-in `--perf` flag that emits a detailed performance report to **stderr** after the command completes:

- `claudine {agent} --perf ...`
- `claudine compose --perf ...`
- `claudine inline-compose --perf ...`
- `claudine sequence --perf ...`

The report is divided into three sections:

1. **CLI Overhead** — time spent on arg parsing, config loading, tracing init, and environment setup.
2. **Composition Report** — when document composition occurred, shows Darkmatter pipeline timings (transclusion, interpolation, shell expansion, etc.).
3. **Agent Execution** — number of launches, first-response latency, total execution time, and provider-reported API duration when available.

For `sequence`, a single aggregated report is printed at the very end, averaging first-response latencies across all steps and summing launches and total time. The report is emitted unconditionally when `--perf` is passed, even if `--silent` or `--quiet` are also present — perf is an explicit opt-in that overrides silence settings.

> **Note:** `provider_api_duration` is only available for providers that use the structured-streaming path (e.g., Codex, Gemini, OpenCode). Legacy providers such as Goose do not report this metric.
```

### 1.2 `claudine/cli/README.md`

**Location**: In the "Wrapped provider commands" shared flags table (around line 124) and in the "Composition commands" section (around line 168).

**Change 1**: Add `--perf` to the shared wrapper flags table:

```markdown
| `--perf` | Emit a detailed performance report to stderr after execution |
```

Insert after `--dry-run` and before `-q, --quiet`.

**Change 2**: Add a short paragraph at the end of the "Composition commands" section (after line 190):

```markdown
**Performance Reporting.** All three composition commands support `--perf`, which emits a post-execution performance report to stderr. `sequence` produces a single aggregated report covering all steps; `compose` and `inline-compose` produce one report per invocation. See the main README for report layout details.
```

### 1.3 `claudine/docs/topics/composition.md`

**Location**: After the "Timing Surface" paragraph (around line 16) or at the end of the file.

**Change**: Add a new section:

```markdown
## Performance Reporting

Composition commands support an opt-in `--perf` flag that prints a detailed performance breakdown to stderr after execution completes. The report includes:

- **CLI Overhead** — arg parsing, config loading, tracing init, and environment setup.
- **Composition Report** — when `compose` or `inline-compose` (or each step of a `sequence`) triggers document preparation, the Darkmatter composition timings are shown (total time plus per-stage breakdown: interpolation, shell expansion, transclusion apply, etc.).
- **Agent Execution** — launches, first-response latency, total execution time, and provider-reported API duration when available.

For `sequence`, the report is aggregated across all steps: launches and total execution time are summed, first-response latencies are averaged (with the minimum shown in a note), and composition metrics are merged. The report appears exactly once at the end of the run, after the sequence summary.

`--perf` is emitted unconditionally when passed, even alongside `--silent` or `--quiet`, because it is an explicit opt-in.

> **Note:** `provider_api_duration` is only populated for structured-streaming providers. Legacy providers (e.g., Goose) omit this line.
```

### 1.4 `claudine/docs/cli/sequence.md`

**Location**: After the "CLI Flags" section (around line 143), before "Example: Multi-Provider Research".

**Change**: Add a new subsection:

```markdown
### Performance Reporting

`claudine sequence --perf` emits a single aggregated performance report at the end of the run, after the sequence summary. The report includes:

- **CLI Overhead** — startup timings captured at sequence entry.
- **Composition Report** — merged across all steps that performed document composition.
- **Agent Execution** — summed launches and total time, with first-response latency averaged across steps. The note line includes both average and minimum latency.

If the sequence is interrupted or stops due to `fail_fast`, the report is still rendered and includes a `partial sequence metrics` note. `--perf` overrides `--silent` and `--quiet` because it is an explicit opt-in.
```

### 1.5 `claudine/features/2026-04-17-perf-flag/tech-design.md`

**Location**: Section §6 "Agent Execution" or add a note in the "Risks and Tradeoffs" section.

**Change**: Add a note after the `provider_api_duration` paragraph (around line 383):

```markdown
> **Implementation note:** `provider_api_duration` is only populated for the structured-streaming path (Codex, Gemini, OpenCode). Legacy providers such as Goose do not provide this metric, and the line is omitted from the report in those cases.
```

---

## Phase 2: Harness Retry Aggregation

**Goal**: Fix undercounting of `launches` and loss of prior-attempt telemetry in harness retry loops.

**Files**:
- `claudine/cli/src/commands/wrap/mod.rs` (lines 2746–3268, `run_harness_loop`)
- `claudine/cli/src/commands/wrap/exec.rs` (lines 40–54, `ProcessTelemetry::into_agent_perf`)

### 2.1 Extend `ProcessTelemetry` with attempt count

**File**: `claudine/cli/src/commands/wrap/exec.rs`

**Current** (lines 33–54):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProcessTelemetry {
    pub total_elapsed: Duration,
    pub first_response_latency: Option<Duration>,
}

#[allow(dead_code)]
impl ProcessTelemetry {
    pub(crate) fn into_agent_perf(
        self,
        api_duration_ms: Option<u64>,
    ) -> crate::perf::AgentExecutionPerf {
        crate::perf::AgentExecutionPerf {
            launches: 1,
            total_elapsed: self.total_elapsed,
            first_response_latency: self.first_response_latency,
            provider_api_duration: api_duration_ms.map(Duration::from_millis),
        }
    }
}
```

**Change**: Remove the hardcoded `launches: 1`. The harness loop will aggregate.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProcessTelemetry {
    pub total_elapsed: Duration,
    pub first_response_latency: Option<Duration>,
}

#[allow(dead_code)]
impl ProcessTelemetry {
    pub(crate) fn into_agent_perf(
        self,
        api_duration_ms: Option<u64>,
    ) -> crate::perf::AgentExecutionPerf {
        crate::perf::AgentExecutionPerf {
            launches: 1,
            total_elapsed: self.total_elapsed,
            first_response_latency: self.first_response_latency,
            provider_api_duration: api_duration_ms.map(Duration::from_millis),
        }
    }
}
```

**No change to `ProcessTelemetry` itself** — instead, aggregation happens in the harness loop by constructing `AgentExecutionPerf` manually rather than via `into_agent_perf`. We deprecate `into_agent_perf` for harness use but keep it for single-launch paths (compose, inline-compose, wrapper direct).

### 2.2 Aggregate perf inside `run_harness_loop`

**File**: `claudine/cli/src/commands/wrap/mod.rs`

**Current** (lines 2759, 3087–3088):
```rust
let mut last_perf: Option<crate::perf::AgentExecutionPerf> = None;
// ...
let (outcome, perf) = attempt_result?;
last_perf = perf;  // overwrites on retry
```

**Change**: Replace `last_perf` with an accumulator.

At line 2759, change:
```rust
let mut last_perf: Option<crate::perf::AgentExecutionPerf> = None;
```
to:
```rust
let mut harness_perf: Option<crate::perf::AgentExecutionPerf> = None;
let mut harness_attempts: usize = 0;
```

At lines 3087–3088, change:
```rust
let (outcome, perf) = attempt_result?;
last_perf = perf;
```
to:
```rust
let (outcome, perf) = attempt_result?;
if let Some(p) = perf {
    harness_attempts += 1;
    match harness_perf.as_mut() {
        Some(acc) => {
            acc.launches += p.launches;
            acc.total_elapsed += p.total_elapsed;
            if acc.first_response_latency.is_none() && p.first_response_latency.is_some() {
                acc.first_response_latency = p.first_response_latency;
            }
            if let Some(api) = p.provider_api_duration {
                acc.provider_api_duration = Some(
                    acc.provider_api_duration.unwrap_or(Duration::ZERO) + api,
                );
            }
        }
        None => {
            harness_perf = Some(p);
        }
    }
}
```

Then replace all subsequent uses of `last_perf` with `harness_perf` in the same function. There are two return points that use `last_perf`:

1. Line 3096: `return Ok((outcome.exit_code, last_perf));`
   Change to: `return Ok((outcome.exit_code, harness_perf));`

2. Line 3226: `return Ok((outcome.exit_code, last_perf));`
   Change to: `return Ok((outcome.exit_code, harness_perf));`

### 2.3 Add test for harness retry aggregation

**File**: `claudine/cli/tests/wrap_commands.rs` (or new file `claudine/cli/tests/harness_perf.rs`)

Add an integration test that drives a 2-attempt harness retry with `--perf` and asserts:
- `launches: 2` (or the summed count).
- `total execution:` reflects the sum of both attempts.

This requires constructing a harness prompt that fails on attempt 1 and succeeds on attempt 2 (e.g., a pre-check that is fixed by a handler). If the existing test fixtures don't support this, add a unit test in `claudine/cli/src/commands/wrap/mod.rs` instead:

```rust
#[cfg(test)]
mod harness_perf_tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn harness_perf_aggregates_across_attempts() {
        let mut harness_perf: Option<crate::perf::AgentExecutionPerf> = None;
        let mut harness_attempts: usize = 0;

        let attempt1 = crate::perf::AgentExecutionPerf {
            launches: 1,
            total_elapsed: Duration::from_secs(1),
            first_response_latency: Some(Duration::from_millis(500)),
            provider_api_duration: Some(Duration::from_millis(800)),
        };
        let attempt2 = crate::perf::AgentExecutionPerf {
            launches: 1,
            total_elapsed: Duration::from_secs(2),
            first_response_latency: Some(Duration::from_millis(700)),
            provider_api_duration: Some(Duration::from_millis(900)),
        };

        for p in [attempt1, attempt2] {
            harness_attempts += 1;
            match harness_perf.as_mut() {
                Some(acc) => {
                    acc.launches += p.launches;
                    acc.total_elapsed += p.total_elapsed;
                    if acc.first_response_latency.is_none() && p.first_response_latency.is_some() {
                        acc.first_response_latency = p.first_response_latency;
                    }
                    if let Some(api) = p.provider_api_duration {
                        acc.provider_api_duration = Some(
                            acc.provider_api_duration.unwrap_or(Duration::ZERO) + api,
                        );
                    }
                }
                None => harness_perf = Some(p),
            }
        }

        let perf = harness_perf.unwrap();
        assert_eq!(perf.launches, 2);
        assert_eq!(perf.total_elapsed, Duration::from_secs(3));
        assert_eq!(perf.first_response_latency, Some(Duration::from_millis(500)));
        assert_eq!(perf.provider_api_duration, Some(Duration::from_millis(1700)));
        assert_eq!(harness_attempts, 2);
    }
}
```

---

## Phase 3: Sequence Interrupt and Accumulator Cleanup

**Goal**: Fix between-step interrupt `partial` flag, wire or remove `set_dry_run`, and deduplicate `provider_api_duration` assignment.

**File**: `claudine/cli/src/commands/wrap/sequence.rs`

### 3.1 Fix between-step interrupt path

**Current** (lines 238–242):
```rust
if interrupted.load(Ordering::SeqCst) {
    interrupt_observed = true;
    break;
}
```

**Change** (add `set_partial()` call):
```rust
if interrupted.load(Ordering::SeqCst) {
    interrupt_observed = true;
    if let Some(ref mut acc) = perf_accumulator {
        acc.set_partial();
    }
    break;
}
```

### 3.2 Fix preflight interrupt path

**Current** (lines 143–145):
```rust
if interrupted.load(Ordering::SeqCst) {
    return Ok(SEQUENCE_INTERRUPT_EXIT_CODE);
}
```

**Change**: Render a partial perf report before returning.

Replace:
```rust
if interrupted.load(Ordering::SeqCst) {
    return Ok(SEQUENCE_INTERRUPT_EXIT_CODE);
}
```

With:
```rust
if interrupted.load(Ordering::SeqCst) {
    if let Some(mut acc) = perf_accumulator {
        acc.mark_env_setup_complete();
        acc.set_partial();
        let total = sequence_start.elapsed();
        let report = acc.into_report(total);
        eprint!("{}", crate::perf::render_perf_report(&report));
    }
    return Ok(SEQUENCE_INTERRUPT_EXIT_CODE);
}
```

### 3.3 Wire up or remove `set_dry_run`

**Decision**: Wire it up for symmetry with the composition path.

**File**: `claudine/cli/src/commands/wrap/sequence.rs`

**Change**: After building the accumulator (around line 51), when `shared.dry_run` is true, call `set_dry_run()`:

```rust
let mut perf_accumulator = if perf_enabled {
    startup_timings.map(|timings| {
        let mut acc = crate::perf::SequencePerfAccumulator::new(timings);
        if shared.dry_run {
            acc.set_dry_run();
        }
        acc
    })
} else {
    None
};
```

Then remove `#[allow(dead_code)]` from `set_dry_run` in `claudine/cli/src/perf.rs` (line 148).

### 3.4 Consolidate duplicate `provider_api_duration` assignment

**File**: `claudine/cli/src/perf.rs`

**Current** (lines 171–225 in `SequencePerfAccumulator::into_report`):

There are two assignments of `provider_api_duration`:
1. Lines 192–196 inside the `None` arm when creating the first `AgentExecutionPerf`.
2. Lines 219–223 inside the `!first_response_latencies.is_empty()` block.

**Change**: Compute `provider_api_total_opt` once near the top of the agent aggregation block and assign once.

Replace the entire agent aggregation section (lines 171–225) with:

```rust
// Aggregate agent execution perf across all steps
let mut agent: Option<AgentExecutionPerf> = None;
let mut first_response_latencies: Vec<Duration> = Vec::new();
let mut provider_api_total = Duration::ZERO;

for step in &self.steps {
    if let Some(ref step_agent) = step.agent_perf {
        first_response_latencies.extend(step_agent.first_response_latency);
        if let Some(api) = step_agent.provider_api_duration {
            provider_api_total += api;
        }
        match agent {
            Some(ref mut a) => {
                a.launches += step_agent.launches;
                a.total_elapsed += step_agent.total_elapsed;
            }
            None => {
                agent = Some(AgentExecutionPerf {
                    launches: step_agent.launches,
                    total_elapsed: step_agent.total_elapsed,
                    first_response_latency: None,
                    provider_api_duration: None,
                });
            }
        }
    }
}

let provider_api_total_opt = if provider_api_total > Duration::ZERO {
    Some(provider_api_total)
} else {
    None
};

// Compute average and min first-response latency across all steps
let mut notes = Vec::new();
if !first_response_latencies.is_empty() {
    let total_latency: Duration = first_response_latencies.iter().sum();
    let avg = total_latency / first_response_latencies.len() as u32;
    let min = *first_response_latencies
        .iter()
        .min()
        .expect("non-empty checked above");
    notes.push(format!(
        "first response avg: {}, min: {}",
        fmt_duration(avg),
        fmt_duration(min)
    ));
    if let Some(ref mut a) = agent {
        a.first_response_latency = Some(avg);
    }
}

if let Some(ref mut a) = agent {
    a.provider_api_duration = provider_api_total_opt;
}
```

### 3.5 Add tests

**File**: `claudine/cli/src/perf.rs` (existing `#[cfg(test)]` module)

Add a test for dry-run note:
```rust
#[test]
fn sequence_perf_accumulator_dry_run_note() {
    let startup = StartupTimings {
        arg_parsing: Duration::ZERO,
        tracing_init: Duration::ZERO,
        config_loading: Duration::ZERO,
    };
    let mut acc = SequencePerfAccumulator::new(startup);
    acc.mark_env_setup_complete();
    acc.set_dry_run();
    let report = acc.into_report(Duration::from_secs(1));
    let notes = report.notes.join(", ");
    assert!(
        notes.contains("Agent execution skipped (dry run)"),
        "missing dry-run note: {notes}"
    );
    assert!(report.agent.is_none(), "agent should be None for dry run");
}
```

Add a test that verifies `provider_api_duration` is still correct after the consolidation (the existing `sequence_perf_accumulator_aggregates_agent_perf` already covers this; keep it and confirm it passes).

---

## Phase 4: Deduplicate Perf Collectors

**Goal**: Merge `WrapperPerfCollector` and `CompositionPerfCollector` into a single `CommandPerfCollector` in `crate::perf`.

**Files**:
- `claudine/cli/src/perf.rs`
- `claudine/cli/src/commands/wrap/mod.rs`
- `claudine/cli/src/commands/wrap/composition.rs`

### 4.1 Add `CommandPerfCollector` to `perf.rs`

**File**: `claudine/cli/src/perf.rs`

Insert after `SequencePerfAccumulator` (after line 248):

```rust
/// Generic perf collector for single-shot commands (wrapper, compose, inline-compose).
///
/// Holds startup timings, environment-setup duration, optional composition perf,
/// and optional agent execution perf. Produces a [`CommandPerfReport`] on completion.
#[derive(Debug)]
pub(crate) struct CommandPerfCollector {
    title: &'static str,
    startup: StartupTimings,
    env_setup_started_at: Option<std::time::Instant>,
    env_setup_elapsed: Duration,
    agent_perf: Option<AgentExecutionPerf>,
    composition_perf: Option<darkmatter::markdown::compose::ComposePerfReport>,
    dry_run: bool,
}

impl CommandPerfCollector {
    /// Start a new collector with the given title and startup timings.
    ///
    /// The environment-setup timer begins immediately.
    pub fn new(title: &'static str, startup: StartupTimings) -> Self {
        Self {
            title,
            startup,
            env_setup_started_at: Some(std::time::Instant::now()),
            env_setup_elapsed: Duration::ZERO,
            agent_perf: None,
            composition_perf: None,
            dry_run: false,
        }
    }

    /// Start a new collector that also holds composition perf from the outset.
    pub fn new_with_composition(
        title: &'static str,
        startup: StartupTimings,
        composition_perf: Option<darkmatter::markdown::compose::ComposePerfReport>,
    ) -> Self {
        Self {
            title,
            startup,
            env_setup_started_at: Some(std::time::Instant::now()),
            env_setup_elapsed: Duration::ZERO,
            agent_perf: None,
            composition_perf,
            dry_run: false,
        }
    }

    /// Capture the elapsed time since construction as environment setup.
    pub fn mark_env_setup_complete(&mut self) {
        if let Some(started) = self.env_setup_started_at.take() {
            self.env_setup_elapsed = started.elapsed();
        }
    }

    /// Set the agent execution perf.
    pub fn set_agent_perf(&mut self, perf: AgentExecutionPerf) {
        self.agent_perf = Some(perf);
    }

    /// Mark this run as a dry run (skips agent execution in the report).
    pub fn set_dry_run(&mut self) {
        self.dry_run = true;
        self.mark_env_setup_complete();
    }

    /// Consume the collector and build the final [`CommandPerfReport`].
    pub fn into_report(self, total_elapsed: Duration) -> CommandPerfReport {
        CommandPerfReport {
            title: self.title,
            total_elapsed,
            cli: CliOverheadReport {
                arg_parsing: self.startup.arg_parsing,
                config_loading: self.startup.config_loading,
                tracing_init: self.startup.tracing_init,
                environment_setup: self.env_setup_elapsed,
            },
            composition: self.composition_perf,
            agent: if self.dry_run { None } else { self.agent_perf },
            notes: if self.dry_run {
                vec!["Agent execution skipped (dry run)".into()]
            } else {
                vec![]
            },
        }
    }
}
```

### 4.2 Replace `WrapperPerfCollector` in `wrap/mod.rs`

**File**: `claudine/cli/src/commands/wrap/mod.rs`

**Current** (lines 3670–3727): Remove the entire `WrapperPerfCollector` struct and impl.

**Change**: Replace with a type alias (or remove entirely and update call sites):

```rust
/// Wrapper perf collector is now the generic `CommandPerfCollector` with title "Wrapper".
pub(crate) type WrapperPerfCollector = crate::perf::CommandPerfCollector;
```

Or, better, remove the type alias and update all call sites to use `crate::perf::CommandPerfCollector::new("Wrapper", startup)` directly.

**Call site updates** (search for `WrapperPerfCollector::new` in `wrap/mod.rs`):
- Where `WrapperPerfCollector::new(startup)` is called, change to `crate::perf::CommandPerfCollector::new("Wrapper", startup)`.
- Where `wrapper_perf_collector.set_agent_perf(...)` is called, keep as-is (method names match).
- Where `wrapper_perf_collector.set_dry_run()` is called, keep as-is.
- Where `wrapper_perf_collector.into_report(total)` is called, keep as-is.

### 4.3 Replace `CompositionPerfCollector` in `wrap/composition.rs`

**File**: `claudine/cli/src/commands/wrap/composition.rs`

**Current** (lines 94–153): Remove the entire `CompositionPerfCollector` struct and impl.

**Change**: Replace with a type alias or update call sites.

```rust
/// Composition perf collector is now the generic `CommandPerfCollector` with title "Composition".
pub(crate) type CompositionPerfCollector = crate::perf::CommandPerfCollector;
```

**Call site updates** (search for `CompositionPerfCollector::new` in `wrap/composition.rs`):
- Where `CompositionPerfCollector::new(startup, composition_perf)` is called, change to `crate::perf::CommandPerfCollector::new_with_composition("Composition", startup, composition_perf)`.
- All other method calls (`mark_env_setup_complete`, `set_agent_perf`, `set_dry_run`, `into_report`) remain the same.

### 4.4 Add unit test for `CommandPerfCollector`

**File**: `claudine/cli/src/perf.rs` (in the `#[cfg(test)]` module)

```rust
#[test]
fn command_perf_collector_full_report() {
    let startup = StartupTimings {
        arg_parsing: Duration::from_millis(1),
        tracing_init: Duration::from_millis(2),
        config_loading: Duration::from_millis(3),
    };
    let mut collector = CommandPerfCollector::new("Test", startup);
    collector.mark_env_setup_complete();
    collector.set_agent_perf(AgentExecutionPerf {
        launches: 1,
        total_elapsed: Duration::from_secs(1),
        first_response_latency: Some(Duration::from_millis(100)),
        provider_api_duration: Some(Duration::from_millis(200)),
    });
    let report = collector.into_report(Duration::from_secs(2));
    assert_eq!(report.title, "Test");
    assert!(report.agent.is_some());
    assert_eq!(report.agent.unwrap().launches, 1);
}

#[test]
fn command_perf_collector_dry_run() {
    let startup = StartupTimings {
        arg_parsing: Duration::ZERO,
        tracing_init: Duration::ZERO,
        config_loading: Duration::ZERO,
    };
    let mut collector = CommandPerfCollector::new("Test", startup);
    collector.set_dry_run();
    let report = collector.into_report(Duration::from_secs(1));
    assert!(report.agent.is_none());
    assert!(report.notes.iter().any(|n| n.contains("dry run")));
}

#[test]
fn command_perf_collector_with_composition() {
    let startup = StartupTimings {
        arg_parsing: Duration::ZERO,
        tracing_init: Duration::ZERO,
        config_loading: Duration::ZERO,
    };
    let compose = darkmatter::markdown::compose::ComposePerfReport {
        total: Duration::from_millis(100),
        metrics: vec![],
    };
    let collector = CommandPerfCollector::new_with_composition("Test", startup, Some(compose));
    let report = collector.into_report(Duration::from_secs(1));
    assert!(report.composition.is_some());
}
```

---

## Phase 5: Add Intentional-Behavior Comments for `--perf` vs `--silent`/`--quiet`

**Goal**: Document that `--perf` emission is intentional even when `--silent` or `--quiet` are present.

**Files**:
- `claudine/cli/src/commands/wrap/mod.rs`
- `claudine/cli/src/commands/wrap/composition.rs`
- `claudine/cli/src/commands/wrap/sequence.rs`

### 5.1 Add comments at each perf emission point

**File**: `claudine/cli/src/commands/wrap/mod.rs`

Find the perf emission block (search for `if let Some(collector) = perf_collector`). Add a comment above it:

```rust
// `--perf` is an explicit opt-in and overrides `--silent`/`--quiet`.
// The perf report is always emitted to stderr when requested.
if let Some(collector) = perf_collector {
    let total = wrapper_start.elapsed();
    let report = collector.into_report(total);
    eprint!("{}", crate::perf::render_perf_report(&report));
}
```

**File**: `claudine/cli/src/commands/wrap/composition.rs`

Find the perf emission block. Add the same comment.

**File**: `claudine/cli/src/commands/wrap/sequence.rs`

Find the perf emission block (around line 467). Add the same comment.

---

## Phase 6: Lint Cleanup and Verification

### 6.1 Remove stale `#[allow(dead_code)]` attributes

After Phase 3 and Phase 4, review all `#[allow(dead_code)]` in `claudine/cli/src/perf.rs`:

- `PerfBootstrap` — used in `main.rs` and tests. Can remove `#[allow(dead_code)]` if used.
- `CliOverheadReport` — used in `CommandPerfReport`. Can remove if fully used.
- `StartupTimings` — used widely. Can remove.
- `AgentExecutionPerf` — used widely. Can remove.
- `CommandPerfReport` — used in renderer and tests. Can remove.
- `SequenceStepPerf` — used in accumulator. Can remove.
- `SequencePerfAccumulator` — used in sequence.rs. Can remove.
- `CommandPerfCollector` — new in Phase 4, used in mod.rs and composition.rs. No need for `#[allow(dead_code)]`.

Also check `claudine/cli/src/commands/wrap/exec.rs`:
- `ProcessTelemetry` — used. Can remove `#[allow(dead_code)]` on the struct if it's used.
- `ProcessResult` — used. Can remove `#[allow(dead_code)]` on the `telemetry` field.

### 6.2 Run lint and tests

```bash
cargo clippy -p claudine-cli -- -D warnings
cargo check -p claudine-cli
cargo test -p claudine-cli
```

### 6.3 Integration test smoke checks

Run the existing integration tests:
```bash
cargo test -p claudine-cli --test sequence_perf
cargo test -p claudine-cli --test wrap_commands
```

Verify:
- All 19 perf-module unit tests pass.
- All 6 exec telemetry tests pass.
- All 3 sequence integration tests pass.
- All 8 wrap command integration tests pass.

### 6.4 Verification checklist

- [ ] `cargo clippy -p claudine-cli -- -D warnings` passes with zero warnings.
- [ ] `cargo test -p claudine-cli` passes (all existing + new tests).
- [ ] Documentation files mention `--perf` and its behavior.
- [ ] Harness retry test asserts `launches == 2` after two attempts.
- [ ] Sequence between-step interrupt test asserts `partial sequence metrics` note.
- [ ] Sequence dry-run test asserts `Agent execution skipped (dry run)` note.
- [ ] `CommandPerfCollector` replaces both `WrapperPerfCollector` and `CompositionPerfCollector` with no behavioral change.
- [ ] `provider_api_duration` is assigned exactly once in `SequencePerfAccumulator::into_report`.

---

## Files Modified Summary

| File | Changes |
|------|---------|
| `claudine/README.md` | Add "Performance Reporting" subsection under Composition |
| `claudine/cli/README.md` | Add `--perf` to wrapper flags table; add perf note to composition section |
| `claudine/docs/topics/composition.md` | Add "Performance Reporting" section |
| `claudine/docs/cli/sequence.md` | Add "Performance Reporting" subsection |
| `claudine/features/2026-04-17-perf-flag/tech-design.md` | Add note about `provider_api_duration` availability |
| `claudine/cli/src/perf.rs` | Add `CommandPerfCollector`; consolidate `provider_api_duration`; add unit tests; remove stale `#[allow(dead_code)]` |
| `claudine/cli/src/commands/wrap/mod.rs` | Replace `WrapperPerfCollector` with `CommandPerfCollector`; aggregate harness perf across retries; add comment about `--perf` overriding silence |
| `claudine/cli/src/commands/wrap/composition.rs` | Replace `CompositionPerfCollector` with `CommandPerfCollector`; add comment about `--perf` overriding silence |
| `claudine/cli/src/commands/wrap/sequence.rs` | Fix between-step and preflight interrupt `partial` handling; wire `set_dry_run`; add comment about `--perf` overriding silence |
| `claudine/cli/src/commands/wrap/exec.rs` | Verify `ProcessTelemetry`/`ProcessResult` dead-code allowances are still needed or remove them |

---

## Recommended Implementation Order

1. **Phase 1** (docs) — independent, can land anytime.
2. **Phase 3** (sequence fixes) — small, targeted, high user impact.
3. **Phase 2** (harness retry aggregation) — medium complexity, affects correctness for harness users.
4. **Phase 4** (collector deduplication) — refactor, best done after Phases 2 and 3 so the new collector is shaped correctly.
5. **Phase 5** (comments) — trivial, can be batched with any phase.
6. **Phase 6** (lint + verify) — final cleanup.
