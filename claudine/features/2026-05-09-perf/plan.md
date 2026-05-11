---
phases: 5
created: 2026-05-09
start_phase: 1
source_files_during_phase_1:
  - claudine/cli/src/perf.rs
  - claudine/cli/src/main.rs
  - claudine/cli/src/commands/compose.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/composition/launch_workspace.rs
  - claudine/lib/src/composition/mod.rs
  - claudine/lib/src/composition/types.rs
  - claudine/cli/src/commands/wrap/composition/prep_context.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/env.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/compose.rs
  - claudine/cli/src/commands/wrap/sequence.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/cli/src/log.rs
  - claudine/lib/src/composition/types.rs
  - claudine/lib/src/composition/select.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/compose.rs
  - claudine/cli/src/commands/wrap/sequence.rs
  - claudine/cli/src/commands/wrap/composition/prep_context.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/cli/src/commands/compose.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/docs/pipeline.md
docs_updated_during_phase_5:
  - claudine/docs/pipeline.md
docs_created_during_phase_5:
  - claudine/features/2026-05-09-perf/trace-after-w0.md
skills_files_updated_during_phase_5: []
packages:
  - claudine
---

# Execution Plan: Compose Pipeline Quick-Win Performance

This plan implements the performance optimizations detailed in [`spec.md`](spec.md), focusing on eliminating redundant filesystem scans (W0), improving reporting (W8, W9), and providing earlier user feedback (W1).

## Phase 1: Instrumentation and Reporting (W8 + W9)

Goal: Extend the `--perf` model to cover the full pipeline and improve report legibility to validate subsequent wins.

- [x] **Task 1.1: Implement W8a (Phase A/B Coverage)**
    - Capture `process_start: Instant` in `main()`.
    - Extend `StartupTimings` with `pre_dispatch` and `prep_phase` durations.
    - Update subcommand entry points to record these timings.
- [x] **Task 1.2: Implement W8b (Env-Setup Sub-stages)**
    - Add `SubstageTiming` support to `CommandPerfCollector`.
    - Add instrumentation points in `execute_composition_request_inner` for sub-stages: `target resolution`, `header env plan`, `child env build`, `mcp composition`, `argv assembly`, `system prompt`, and `stream + prompt delivery`.
- [x] **Task 1.3: Implement W8c (Report Rendering)**
    - Update `render_perf_report` in `claudine/cli/src/perf.rs` to show new Phase A/B lines and indented sub-stages.
- [x] **Task 1.4: Implement W9a (Section Totals)**
    - Move per-section totals to the bottom of each block.
    - Add double-underline separator (`═`) above totals.
- [x] **Task 1.5: Implement W9b (Alignment Fixes)**
    - Compute label column width per-section to prevent overflow/abutment.
    - Right-align values for cleaner columns.
- [x] **Task 1.6: Implement W9c (Formatting)**
    - Bold the `TOTAL:` labels using Prose markup.
- [x] **Validation 1**: Tests pass (`cargo test --manifest-path cli/Cargo.toml perf`) and clippy is clean (`cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`).

## Phase 2: Redundancy Elimination (W0)

Goal: Eliminate the dominant wall-clock cost identified in the trace findings.

- [x] **Task 2.1: Enhance `CompositionPrepContext`**
    - Add `launch_workspace: LaunchWorkspaceContext` field.
    - Build `launch_workspace` from shared sniff result using `env::launch_workspace_context_from_repo_info`.
- [x] **Task 2.2: Thread Context through Execution**
    - Add `prep_launch_workspace: Option<LaunchWorkspaceContext>` to `CompositionExecutionRequest`.
- [x] **Task 2.3: Optimize Header Env Plan**
    - Update `composition/mod.rs` to reuse `request.prep_launch_workspace` instead of calling `resolve_launch_workspace_context` fresh.
- [x] **Task 2.4: Optimize Child Env Build**
    - Update `composition/mod.rs` to call `env::build_child_env_with_launch` with the pre-computed context.
- [x] **Validation 2**: Tests pass (`just test`) and clippy is clean (`just lint`).

## Phase 3: Optimization and Cleanup (W6 + W2)

Goal: Remove minor redundancies and stabilize the environment.

- [x] **Task 3.1: Implement W6 (Terminal Memoization)**
    - Use `LazyLock<Terminal>` in `crate::log::terminal()` to avoid repeated capability detection.
- [x] **Task 3.2: Implement W2 (Binary Path Snapshot)**
    - Extend `InstalledProviderSnapshot` with `binary_paths: BTreeMap<Provider, PathBuf>`.
    - Populate it in `build_installed_snapshot`.
    - Update `resolve_binary_path_direct` to use the snapshot lookup.
- [x] **Validation 3**: Verify `target resolution` sub-stage remains minimal and `clippy` passes.

## Phase 4: Feedback Experience (W1)

Goal: Dramatically improve perceived latency by providing immediate feedback.

- [x] **Task 4.1: Implement W1 (Receipt Banner)**
    - Add `Status::from_prose("→ Composing {file_ref}…")` emission in `run_compose` and `run_inline_compose` immediately after dispatch.
    - Ensure it respects `--silent` and `--quiet` flags.
- [x] **Validation 4**: Verify the receipt banner appears instantly upon pressing Enter, well before the main execution header.

## Phase 5: Verification and Documentation

Goal: Finalize the feature and update the project records.

- [x] **Task 5.1: Performance Baseline Capture**
    - Capture `trace-after-w0.md` using the canonical harness.
    - Capture a wrapper `--perf` control trace for comparison.
- [x] **Task 5.2: Documentation Update**
    - Update `claudine/docs/pipeline.md` to reflect current step semantics and removed redundancies.
- [x] **Task 5.3: Workspace-wide Checks**
    - Run all tests: `just test`.
    - Run clippy: `cargo clippy --workspace`.
- [x] **Task 5.4: Deferred Item Re-evaluation**
    - Compare post-W0 numbers against the 50ms/3s targets.
    - Decide whether W3, W4, or W5 are still required to meet goals.
