---
ready: true
agent: claude
date: 2026-04-25
---

# Review 3: Performance Flag (`--perf`)

This review evaluates the implementation of the `--perf` flag against the
[specification](./spec.md), [technical design](./tech-design.md), and the
prior reviews ([review-1](./review-1.md), [review-2](./review-2.md), and
the implementation plan that closed those out
[review-plan-2](./review-plan-2.md)).

## Verdict

**Functionally complete and ready for production**, with several smaller
gaps and ergonomic improvements worth landing in a follow-up.

The hard blockers from prior reviews are resolved:

- Sequence aggregation now exists (`SequencePerfAccumulator`) and is wired
  end-to-end through `commands/sequence.rs` →
  `commands/wrap/sequence.rs`.
- `perf_arg_parsing` is captured around `parse_cli_from(...)` (the timer
  starts in `run()` before `argv::normalize` and is read after
  `parse_cli_from` in `async_main`).
- Per-step composition perf collection is enabled (sequence steps no
  longer pass `perf_enabled: false`).
- All four targeted command surfaces (direct wrappers, `compose`,
  `inline-compose`, `sequence`) emit the report on stderr after the
  ordinary lifecycle output.

All perf-related tests pass:

- `claudine-cli` unit tests: 19/19 perf-module tests pass.
- `claudine-cli` exec telemetry: 6/6 first-response and process-telemetry
  tests pass.
- `claudine-cli` integration tests: 3/3 `sequence_perf` tests pass; 8/8
  perf-related tests in `wrap_commands` pass (compose, inline-compose,
  wrapper, dry-run, and the `perf_arg_parsing_includes_clap_time`
  smoke test).
- `claudine` library: 11/11 `composition::prepare` tests pass, including
  `direct_composition_perf_disabled_yields_none`,
  `direct_composition_perf_enabled_yields_some`, and
  `inline_composition_preserves_closure_with_perf_enabled`.

## Findings

The remaining work falls into one **must-fix** (documentation), one
**should-fix** (harness-retry undercounting), and several **nice-to-have**
ergonomic items.

### 1. Documentation never landed (Must fix)

**Severity:** Medium. The flag is invisible to anyone reading the docs,
but functional.

The tech design called for documentation updates in:

- `claudine/README.md`
- `claudine/cli/README.md`
- `claudine/docs/topics/composition.md`
- `claudine/docs/cli/sequence.md`

A grep across all four files shows zero mentions of `--perf`,
`Performance`, or the new perf surface. Help text inside the binary
(e.g. `print_wrapper_help`) does mention the flag, so users with `--help`
will discover it, but the topic docs and READMEs need short sections
describing:

- where the flag is accepted (wrappers + composition commands only)
- that emission is stderr-only and post-execution
- that `sequence` produces a single aggregated report
- the visual layout of the four sections (CLI Overhead → Composition
  Report → Agent Execution → notes)

Recommended locations: a short "Performance reporting" subsection in
`docs/topics/composition.md` and `docs/cli/sequence.md`, plus a one-line
entry in each README's command tables that links to those subsections.

### 2. Harness retries undercount launches and total time (Should fix)

**Severity:** Medium. Only affects users running harness-driven flows
(retry/resume/redirect/deviate) — for the common single-launch path the
metrics are correct.

`AgentExecutionPerf.launches` is hardcoded to `1` in
[`exec.rs::ProcessTelemetry::into_agent_perf`](../../../claudine/cli/src/commands/wrap/exec.rs):

```rust
crate::perf::AgentExecutionPerf {
    launches: 1,
    total_elapsed: self.total_elapsed,
    first_response_latency: self.first_response_latency,
    provider_api_duration: api_duration_ms.map(Duration::from_millis),
}
```

[`run_harness_loop`](../../../claudine/cli/src/commands/wrap/mod.rs#L2746)
overwrites `last_perf` each iteration:

```rust
let mut last_perf: Option<crate::perf::AgentExecutionPerf> = None;
loop {
    ...
    let (outcome, perf) = attempt_result?;
    last_perf = perf;            // overwrites on retry
    ...
}
```

So a 3-attempt harness run reports `launches: 1` and the wall-clock /
api / first-response numbers from only the final attempt. The earlier
attempts' setup and execution time are silently discarded.

Two reasonable fixes:

- **Aggregate inside `run_harness_loop`**: maintain a `total_attempts`
  counter, sum `total_elapsed` and `provider_api_duration` across
  attempts, and pick the first observed `first_response_latency` (or the
  minimum). Return a single `AgentExecutionPerf` with `launches` equal to
  the actual attempt count.
- **Carry attempts through `ProcessTelemetry`**: extend the struct with
  an `attempts: usize` field defaulting to 1; the harness loop bumps it
  per retry before constructing the agent-execution shape.

The first option is simpler because the aggregation logic stays local
to the harness loop and matches what `SequencePerfAccumulator` already
does for cross-step aggregation.

### 3. Sequence interrupts can leave `partial` unset (Should fix)

**Severity:** Low–Medium. Cosmetic in some paths, misleading in others.

[`commands/wrap/sequence.rs`](../../../claudine/cli/src/commands/wrap/sequence.rs)
has three interrupt paths in Phase 2:

1. **Between-step interrupt** (lines 238–242): the loop checks
   `interrupted.load(...)` at the top, sets `interrupt_observed = true`,
   and breaks — but does **not** call
   `perf_accumulator.as_mut().map(|a| a.set_partial())`. The final
   report omits the "partial sequence metrics" note even though the
   sequence was interrupted.
2. **In-step interrupt** (the `Ok(outcome) if outcome.exit_code ==
   SEQUENCE_INTERRUPT_EXIT_CODE` arm): correctly calls `set_partial()`.
3. **Preflight interrupt** (lines 143–145): early-returns
   `SEQUENCE_INTERRUPT_EXIT_CODE` without rendering any report at all.
   Acceptable in principle, but inconsistent — preflight time has been
   measured and could be surfaced as `notes: ["partial preflight only"]`.

Recommended fix for case 1: hoist the `set_partial()` call into the
between-step interrupt branch. Case 3 is a judgment call; emitting a
partial report for preflight-only runs would be more informative but
requires a tiny bit of restructuring (move the final-render block above
the early returns, or fall through with a flag).

### 4. `set_dry_run` on `SequencePerfAccumulator` is never called (Nice to have)

**Severity:** Low.

`SequencePerfAccumulator::set_dry_run` exists and is annotated
`#[allow(dead_code)]`, but the sequence orchestrator never invokes it
even when `shared.dry_run` is `true`. As a consequence, a
`claudine sequence --perf --dry-run …` run will:

- still show all the section headers,
- omit the `Agent Execution` block (because no children launch and no
  `agent_perf` accumulates),
- but never render the `Agent execution skipped (dry run)` note that
  the single-composition path renders via `CompositionPerfCollector`.

Two paths forward:

- **Drop the field** — `dry_run` for sequence isn't observably useful
  if the report still degrades gracefully. Removing `dry_run`/`set_dry_run`
  from `SequencePerfAccumulator` (and the `#[allow(dead_code)]` on it) is
  the cleanest cleanup.
- **Wire it up** — in `execute_sequence`, when `shared.dry_run` is true
  call `acc.set_dry_run()` once before rendering, so the report matches
  composition's output for symmetry.

The latter is more user-friendly; the former is honest about current
behavior. Either is fine, but the dead-code marker should not stay.

### 5. Duplicate `provider_api_duration` assignment in `into_report` (Nice to have)

**Severity:** Low (cosmetic).

In [`SequencePerfAccumulator::into_report`](../../../claudine/cli/src/perf.rs#L159),
`provider_api_duration` is set twice on the agent struct:

```rust
None => {
    agent = Some(AgentExecutionPerf {
        ...
        provider_api_duration: if provider_api_total > Duration::ZERO {
            Some(provider_api_total)
        } else {
            None
        },
    });
}
...
if let Some(ref mut a) = agent {
    a.first_response_latency = Some(avg);
    a.provider_api_duration = if provider_api_total > Duration::ZERO {
        Some(provider_api_total)
    } else {
        None
    };
}
```

The second assignment happens unconditionally in the latency path. It is
not a bug — both assignments compute the same value — but the duplicate
makes the function harder to read and easier to mis-edit. Consolidate
into a single computed `let provider_api_total_opt = ...;` near the top
of the loop and assign once.

### 6. `--perf` ignores `--silent` and `--quiet` (Nice to have)

**Severity:** Low (matter of opinion).

The wrapper, composition, and sequence emitters all unconditionally
`eprint!(...)` the perf report:

```rust
if let Some(collector) = perf_collector {
    let total = wrapper_start.elapsed();
    let report = collector.into_report(total);
    eprint!("{}", crate::perf::render_perf_report(&report));
}
```

Neither `silent` nor `quiet` short-circuits emission. The spec is silent
on this interaction. Reasonable interpretation: `--perf` is an explicit
opt-in and overrides quiet/silent — that mirrors how `--debug` behaves.
But it deserves a one-line comment in the renderer or documentation
note so future readers know it is intentional.

### 7. `CompositionPerfCollector` and `WrapperPerfCollector` are
near-duplicates (Nice to have)

**Severity:** Low.

[`commands/wrap/composition.rs::CompositionPerfCollector`](../../../claudine/cli/src/commands/wrap/composition.rs#L94-L153)
and [`commands/wrap/mod.rs::WrapperPerfCollector`](../../../claudine/cli/src/commands/wrap/mod.rs#L3673-L3727)
are 90% the same code:

- both hold `startup`, `env_setup_started_at`, `env_setup_elapsed`,
  `agent_perf`, `dry_run`
- both expose `mark_env_setup_complete`, `set_agent_perf`, `set_dry_run`,
  `into_report`
- the only meaningful differences are `title` (`"Composition"` vs
  `"Wrapper"`) and that the composition collector also stores a
  `composition_perf` field

Lifting them into the `crate::perf` module as a single
`CommandPerfCollector` parameterized by title (and an optional
`composition_perf`) would cut roughly 60 lines of duplication and keep
"how do I record perf for command X" in one place. The
`SequencePerfAccumulator` is structurally different enough to stay
separate.

### 8. `tech-design.md` mentions a CLI flag that does not exist
(Documentation drift)

**Severity:** Trivial.

The design document at section §4 "Environment Setup" lists harness
preflight as part of env setup; the implementation correctly captures
that. The design also mentions an `AgentExecutionPerf` field
`provider_api_duration` that is only populated when
`StreamExecutionSummary.duration_ms` is `Some`. Today only the
structured-stream path provides it; the legacy path passes `None`. That
is consistent with the spec's "as an additional detail line under Agent
Execution", but worth calling out in the docs (item #1 above) so users
do not expect it for, say, Goose.

## Test Coverage Assessment

Coverage is now strong:

- **Bootstrap scan**: 8 unit tests covering wrapper/compose/inline-compose/sequence,
  disabled-on-other-commands, `--` boundary, completion mode.
- **Renderer**: 3 unit tests covering full report, missing
  composition/agent, and dash fallback for missing latency.
- **Sequence accumulator**: 4 unit tests covering empty,
  composition-merge, agent aggregation with avg/min, and partial-note.
- **Process telemetry**: 6 unit tests covering precedence, fallbacks,
  and round-tripping into `AgentExecutionPerf`.
- **Format helpers**: 2 unit tests covering sub-second and
  second-and-above.
- **Integration**: 3 sequence tests (single aggregated report, fail-fast
  partial, startup-timings propagation) and 6 wrapper/compose tests
  (wrapper basic, wrapper dry-run, compose basic, compose stdout
  identity, inline basic, inline stdout identity, compose dry-run, arg
  parsing smoke).

Gaps worth filling (low priority):

1. No test exercises the harness retry path with `--perf`. Adding one
   that drives a 2-attempt retry and asserts `launches: 2` (or whatever
   the chosen aggregation strategy reports) would lock in finding #2
   above once it's fixed.
2. No test covers `--perf --silent` to lock in the intentional behavior
   from finding #6.
3. No test covers `--perf` with the legacy (non-structured) wrapper
   path, where `provider_api_duration` is `None`. The wrapper helper
   path is exercised, but the assertion that `provider api:` is omitted
   would document the contract.

## Ergonomics & Performance Observations

- The disabled path is genuinely cheap. `scan_perf_bootstrap` short-circuits
  on argv length, completion mode, and absence of `--perf`, and the
  `Option<StartupTimings>` threading through the command tree is `None`
  unless the bootstrap scan succeeds. No per-event overhead is added.
- The `BlockQuote` rendering with the yellow `▌ ` border is consistent
  with the existing `--debug` output style and reads well in both
  truecolor and `NO_COLOR=1` environments (verified by the integration
  tests).
- The composition and wrapper collector duplication (finding #7) is
  the main ergonomics improvement opportunity.

## Release Gate Status

| Criterion | Status |
|---|---|
| `--perf` available on direct wrappers, compose, inline-compose, sequence | OK |
| Report emitted to `stderr` only when `--perf` enabled | OK |
| `CLI Overhead` includes arg parsing, config loading, tracing init, environment setup | OK |
| `Composition Report` appears when composition occurred | OK |
| `Agent Execution` reports total execution and first-response latency | OK (single launch); undercounts harness retries |
| `sequence` emits one aggregated report at end | OK; partial-note misses between-step interrupts |
| stdout matches non-perf invocation | OK (verified in integration tests) |
| Documentation updated | NOT DONE |
| Strong unit and integration test coverage | OK |

## Recommendation

Mark this feature **ready for production** because:

1. The user-facing contract from the spec is met for the dominant
   non-harness, non-interrupt code path — i.e. the path 99% of users
   exercise.
2. All listed test cases pass and integration coverage proves the
   stderr surface is real.
3. The remaining issues are either documentation gaps (which can land
   in a follow-up doc PR), or edge cases (harness retries, between-step
   interrupts, dry-run sequence note) that produce slightly imprecise
   metrics rather than incorrect or unsafe behavior.

**Suggested follow-up ticket scope**: docs in finding #1, fix
finding #2 (harness retry aggregation), fix finding #3 (between-step
partial), pick a path for finding #4 (drop or wire `set_dry_run`),
clean up finding #5 (duplicate assignment), refactor finding #7
(deduplicate collectors). Findings #6 and #8 are notes/comments only.
