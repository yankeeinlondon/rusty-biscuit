---
phases: 6
created: 2026-04-17
start_phase: 2
packages:
  - claudine
  - claudine-cli
source_files_during_phase_2:
  - claudine/lib/src/composition/types.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/lib/src/composition/select.rs
  - claudine/cli/src/commands/compose.rs
  - claudine/cli/src/commands/wrap/sequence.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/cli/src/perf.rs
  - claudine/cli/src/commands/wrap/exec.rs
  - claudine/lib/src/stream/logs/opencode.rs
  - claudine/cli/tests/sequence_cli.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/cli/src/perf.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/composition.rs
  - claudine/cli/tests/wrap_commands.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
---
# Execution Plan — Performance Flag (`--perf`)

Source documents:

- [`spec.md`](./spec.md)
- [`tech-design.md`](./tech-design.md)

Validated against the current implementation in:

- `claudine/cli/src/main.rs`
- `claudine/cli/src/commands/compose.rs`
- `claudine/cli/src/commands/sequence.rs`
- `claudine/cli/src/commands/wrap/{mod,composition,sequence,exec}.rs`
- `claudine/lib/src/composition/{prepare,types}.rs`
- `claudine/cli/tests/{argv_normalization,wrap_commands,wrap_direct_argv,sequence_cli}.rs`
- `darkmatter/cli/src/commands.rs`
- `darkmatter/lib/src/markdown/compose/types.rs`

The tree already has the important seams the design depends on:

- `main.rs` already owns argv normalization, clap parsing, tracing init, and config loading.
- `SharedComposeArgs` is the shared CLI surface for `compose`, `inline-compose`, and `sequence`.
- wrapper passthrough flags are already normalized in `extract_wrapper_flags_from_passthrough(...)`.
- `PreparedComposition` is already the handoff point between composition prep and execution.
- `ProcessResult<T>` already centralizes child-process outcomes in `wrap/exec.rs`.

That means this feature should land as additive perf plumbing through existing execution paths, not as a structural refactor.

## Phase Index

| Phase | Outcome | Depends on |
|---|---|---|
| 1 | CLI contract and startup timing bootstrap are in place | none |
| 2 | Composition prep preserves Darkmatter perf data when enabled | 1 |
| 3 | Child execution telemetry and shared perf renderer exist | 1 |
| 4 | Direct wrapper commands emit a final perf report | 1, 3 |
| 5 | `compose` and `inline-compose` emit final perf reports | 1, 2, 3 |
| 6 | `sequence` aggregates perf once, docs are updated, and acceptance passes | 1-5 |

## Phase 1 — Add The CLI Contract And Startup Bootstrap

1. Add a new CLI-local perf module, for example `claudine/cli/src/perf.rs`, with the startup data types the rest of the feature will consume:
   `PerfCommandKind`, `PerfBootstrap`, `CliOverheadReport`, and a small startup bundle that can be threaded into command handlers.
   Observable result: there is one owned place for perf-related CLI types instead of ad hoc tuples in `main.rs`.
2. Add a raw-argv bootstrap scan in [`claudine/cli/src/main.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/main.rs) before `argv::normalize(...)` / `parse_cli_from(...)`.
   The scan should detect only supported command surfaces and only honor `--perf` before the first literal `--`.
   Observable result: arg parsing is only timed for eligible invocations, and the disabled path remains cheap.
3. Add `perf: bool` to `SharedComposeArgs` in [`compose.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/compose.rs) so `compose`, `inline-compose`, and `sequence` share the same flag surface.
   Observable result: `claudine compose --help`, `claudine inline-compose --help`, and `claudine sequence --help` all show `--perf`.
4. Add `perf: bool` to `WrapperArgs` plus `perf: bool` on `ExtractedWrapperFlags` in [`wrap/mod.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/mod.rs), then teach `extract_wrapper_flags_from_passthrough_with_boundary(...)` to lift `--perf` with the same boundary behavior as `--quiet` and `--repo`.
   Observable result: `claudine codex "prompt" --perf` enables Claudine perf mode, while `claudine codex "prompt" -- --perf` leaves the token in wrapped-provider passthrough.
5. In `main.rs`, measure the four startup buckets named in the design:
   arg parsing around `argv::normalize(...)` plus `parse_cli_from(...)`,
   tracing init around `telemetry::init_tracing(...)` plus `telemetry::root_span(...)`,
   config loading around `ensure_config_exists().await?`,
   and the handoff state needed by downstream environment setup timers.
   Observable result: wrapper and composition commands can receive a populated startup timing bundle without re-measuring startup locally.
6. Thread the startup timing bundle into `run_provider_wrapper(...)`, `run_compose(...)`, `run_inline_compose(...)`, and `run_sequence(...)` without changing non-perf behavior.
   Observable result: every supported entrypoint can accumulate one end-to-end report from the same startup source.

Parallelizable work:

- Steps 1.1 and 1.3 can proceed in parallel once the perf type names are fixed.
- Steps 1.4 and 1.5 can proceed in parallel because passthrough extraction and `main.rs` timing are independent.

Validation checkpoint:

- `cargo test -p claudine-cli --test wrap_commands`
- `cargo test -p claudine-cli --test wrap_direct_argv`
- `cargo test -p claudine-cli --test argv_normalization`

## Phase 2 — Preserve Composition Perf In `claudine/lib`

1. Extend `PrepareOptions` in [`claudine/lib/src/composition/types.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/types.rs) with `perf_enabled: bool`.
   Observable result: composition preparation can opt into Darkmatter perf collection without changing unrelated call sites.
2. Extend `PreparedComposition` with `compose_perf: Option<darkmatter::markdown::compose::ComposePerfReport>`.
   Observable result: downstream CLI execution paths can render composition perf without recomposing the document.
3. Update [`prepare.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/prepare.rs) so both `prepare_direct(...)` and `prepare_inline(...)` call `ComposeOptions::with_perf(options.perf_enabled)` and retain `report.perf` instead of discarding it.
   Observable result: direct and inline preparation preserve Darkmatter pipeline timings only when `--perf` is enabled.
4. Update the command builders in `compose.rs` and `wrap/sequence.rs` to pass `perf_enabled` into `PrepareOptions`.
   Observable result: composition perf is available on `compose`, `inline-compose`, and per-step sequence prep through one existing API.
5. Add lib-level tests covering:
   perf disabled yields `compose_perf: None`,
   perf enabled yields `Some(...)`,
   and inline prep still preserves its existing closure/guardrails behavior when perf is enabled.
   Observable result: the library contract is locked before CLI rendering begins.

Parallelizable work:

- Steps 2.1 and 2.2 can land together before the prep functions are updated.
- Step 2.5 can be written in parallel with Step 2.4 once the final struct fields are fixed.

Validation checkpoint:

- `cargo test -p claudine`

## Phase 3 — Add Shared Execution Telemetry And Rendering

1. Add `AgentExecutionPerf`, `CommandPerfReport`, and `render_perf_report(...)` to the new CLI perf module.
   Shape the renderer around Claudine-owned block output, but keep stage ordering and duration formatting aligned with Darkmatter's existing compose perf presentation.
   Observable result: one shared renderer can produce the final `stderr` report for wrappers, compose, inline-compose, and sequence.
2. Add `ProcessTelemetry` to [`wrap/exec.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/exec.rs) and extend `ProcessResult<T>` to carry it.
   Observable result: all child-process helpers can return wall-clock lifetime and first-response latency together with their current payload.
3. Update the execution helpers in `wrap/exec.rs` to record:
   total elapsed from just before spawn through process completion,
   preferred first-response latency from the first semantic stdout event,
   fallback latency from the first non-filtered stdout line,
   and final fallback latency from the first non-filtered stderr line.
   Observable result: structured and legacy execution paths both produce comparable latency metrics.
4. Add a small adapter that converts `ProcessTelemetry` plus any available `StreamExecutionSummary.duration_api_ms` into the shared `AgentExecutionPerf` model.
   Observable result: provider-reported API duration stays optional and separate from observed wall-clock duration.
5. Add focused unit tests for:
   first-response precedence ordering,
   no-output fallback behavior,
   and renderer behavior with and without a composition block.
   Observable result: the two trickiest behaviors are proven before command-level integration starts.

Parallelizable work:

- Steps 3.1 and 3.2 can proceed in parallel.
- Step 3.5 can be written once the telemetry struct fields are stable, independent of wrapper integration.

Validation checkpoint:

- `cargo test -p claudine-cli`

## Phase 4 — Wire Direct Wrapper Reporting

1. Add a wrapper-level collector in [`wrap/mod.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/mod.rs) that is only constructed when `args.perf || extracted.perf` is true.
   Observable result: the ordinary wrapper path does not pay for extra aggregation when `--perf` is absent.
2. Measure wrapper environment setup from the end of startup timing until just before the first child launch or dry-run emission.
   Use the real startup work already present in `run_provider_wrapper_inner(...)`: startup detection, binary resolution, prompt extraction, env planning, MCP setup, harness discovery, and preflight.
   Observable result: `CLI Overhead` reports an `environment setup` duration that reflects wrapper-specific setup rather than child runtime.
3. Convert the `ProcessTelemetry` returned by `exec::run_child(...)`, `run_child_capture(...)`, and `run_child_stream_semantic(...)` into `AgentExecutionPerf`.
   Observable result: direct wrappers report launch count, first response, total execution, and provider API duration when available.
4. Render the final report to `stderr` only after the existing wrapper completion/error lifecycle is done.
   Observable result: stdout remains pipe-safe and current wrapper summaries still appear before perf output.
5. Ensure `--dry-run --perf` still renders a report with an explicit skipped or dry-run agent section.
   Observable result: dry runs validate the CLI surface without launching a provider.
6. Add or extend wrapper tests for:
   passthrough extraction of `--perf`,
   help text exposing the flag,
   and at least one wrapper execution path confirming stderr-only perf output.
   Observable result: direct wrapper perf behavior is covered at both parsing and runtime levels.

Parallelizable work:

- Steps 4.1 and 4.6 can proceed in parallel after the CLI flag shape is settled.
- Steps 4.2 and 4.3 can proceed in parallel because setup timing and child telemetry integration touch different parts of the wrapper pipeline.

Validation checkpoint:

- `cargo test -p claudine-cli --test wrap_commands`
- `cargo test -p claudine-cli --test wrap_direct_argv`

## Phase 5 — Wire `compose` And `inline-compose`

1. Extend `CompositionExecutionRequest` and the compose command entrypoints to carry the startup timing bundle plus perf enablement into the wrapper-grade composition path.
   Observable result: `compose` and `inline-compose` have the same end-to-end timing inputs as direct wrappers.
2. Extend `SingleCompositionOutcome` in [`wrap/composition.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/composition.rs) to return execution perf metadata alongside exit code and provider.
   Observable result: the composition executor becomes the single source of child execution perf for both standalone commands and sequence.
3. Measure composition-command environment setup in `execute_composition_request_inner(...)` around provider selection, binary resolution, env planning, MCP setup, system prompt resolution, harness detection, and request construction.
   Do not include Darkmatter document composition here; that must stay in the composition block from `PreparedComposition.compose_perf`.
   Observable result: the final report cleanly separates setup overhead from document composition and agent runtime.
4. Render a single final perf report for `claudine compose --perf ...` after the existing compose summary path completes.
   Observable result: stderr shows `Performance`, `CLI Overhead`, `Composition Report`, and `Agent Execution` in one block, while stdout remains unchanged from the non-perf invocation.
5. Render a single final perf report for `claudine inline-compose --perf ...` after inline closure and the deferred summary path complete.
   Observable result: inline perf output appears once, after file mutation / closure messaging, not before.
6. Add integration tests covering:
   `compose --perf`,
   `inline-compose --perf`,
   identical stdout between perf and non-perf runs,
   and stderr containing the expected section headings.
   Observable result: the perf flag is proven on the two composition commands independently from sequence.

Parallelizable work:

- Steps 5.1 and 5.2 can proceed in parallel once the shared perf types are stable.
- Steps 5.4 and 5.5 can proceed in parallel after Step 5.3 has defined the aggregation contract.

Validation checkpoint:

- `cargo test -p claudine-cli --test argv_normalization`
- `cargo test -p claudine-cli --test wrap_commands`

## Phase 6 — Aggregate `sequence`, Update Docs, And Close The Feature

1. Add a `SequencePerfAccumulator` in either `claudine/cli/src/perf.rs` or [`wrap/sequence.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/sequence.rs) to collect:
   aggregated environment setup duration,
   merged `ComposePerfReport`,
   launch count,
   total agent execution,
   first-response latency samples,
   and summed provider API duration.
   Observable result: `sequence` can report one final rollup without reusing the single-run composition report shape incorrectly.
2. Instrument the two existing sequence phases separately:
   Phase 1 preflight/preparation contributes to `environment setup` and merges per-step `compose_perf`,
   while Phase 2 execution merges per-step `ProcessTelemetry` / `AgentExecutionPerf`.
   Observable result: the report reflects the real structure of `execute_sequence(...)` instead of collapsing prep and execution into one timer.
3. Render exactly one final sequence perf report after the existing sequence summary, including aggregated first-response metrics such as average and minimum, and add a `partial sequence metrics` note when interrupted or stopped early.
   Observable result: sequence never prints per-step perf blocks and still reports useful partial data on fail-fast or Ctrl+C exits.
4. Update public docs in the same change set:
   [`claudine/README.md`](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/README.md),
   [`claudine/cli/README.md`](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/README.md),
   [`claudine/docs/topics/composition.md`](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/docs/topics/composition.md),
   and [`claudine/docs/cli/sequence.md`](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/docs/cli/sequence.md).
   Observable result: user-facing docs explicitly say `--perf` is stderr-only, supported on wrapper and composition commands, and aggregated for `sequence`.
5. Add the final integration and acceptance coverage:
   one `sequence --perf` run that proves exactly one final report,
   one dry-run case that proves stdout is still clean,
   and one wrapper legacy-path case that proves total execution still reports even without semantic first-response events.
   Observable result: the design's edge cases are covered where regressions are most likely.
6. Run the full package validation and a short manual smoke pass:
   `cargo test -p claudine`
   `cargo test -p claudine-cli`
   `cd claudine && just test`
   plus manual checks for `claudine codex "prompt" --perf`, `claudine compose --perf @file.md --dry-run`, `claudine inline-compose --perf @file.md --dry-run`, and `claudine sequence --perf @file.md --dry-run`.
   Observable result: both automated and binary-level acceptance criteria pass before the feature is considered complete.

Parallelizable work:

- Steps 6.1 and 6.4 can proceed in parallel after compose/inline perf integration is stable.
- Step 6.5 can be developed in parallel with Step 6.4 once the sequence renderer contract is fixed.

Validation checkpoint:

- `cargo test -p claudine`
- `cargo test -p claudine-cli`
- `cd claudine && just test`

## Release Gate

Do not consider the feature complete until all of the following are true:

- `--perf` is available on direct wrappers, `compose`, `inline-compose`, and `sequence`, but not on unrelated admin commands.
- the report is emitted to `stderr` only and only when `--perf` is enabled.
- `CLI Overhead` includes arg parsing, config loading, tracing init, and environment setup.
- `Composition Report` appears only when document composition occurred and is sourced from Darkmatter's `ComposePerfReport`.
- `Agent Execution` reports total execution and best-effort first-response latency, with provider API duration shown only when available.
- `sequence` emits exactly one aggregated report at the end and marks interrupted / fail-fast runs as partial when appropriate.
- stdout matches the corresponding non-perf invocation for the same command.
