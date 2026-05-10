---
phases: 5
created: 2026-05-09
start_phase: 1
---

# Execution Plan: Compose Pipeline Quick-Win Performance

This plan implements the performance optimizations detailed in [`spec.md`](spec.md), focusing on eliminating redundant filesystem scans (W0), improving reporting (W8, W9), and providing earlier user feedback (W1).

## Phase 1: Instrumentation and Reporting (W8 + W9)

Goal: Extend the `--perf` model to cover the full pipeline and improve report legibility to validate subsequent wins.

- [ ] **Task 1.1: Implement W8a (Phase A/B Coverage)**
    - Capture `process_start: Instant` in `main()`.
    - Extend `StartupTimings` with `pre_dispatch` and `prep_phase` durations.
    - Update subcommand entry points to record these timings.
- [ ] **Task 1.2: Implement W8b (Env-Setup Sub-stages)**
    - Add `SubstageTiming` support to `CommandPerfCollector`.
    - Add instrumentation points in `execute_composition_request_inner` for sub-stages: `target resolution`, `header env plan`, `child env build`, `mcp composition`, `argv assembly`, `system prompt`, and `stream + prompt delivery`.
- [ ] **Task 1.3: Implement W8c (Report Rendering)**
    - Update `render_perf_report` in `claudine/cli/src/perf.rs` to show new Phase A/B lines and indented sub-stages.
- [ ] **Task 1.4: Implement W9a (Section Totals)**
    - Move per-section totals to the bottom of each block.
    - Add double-underline separator (`═`) above totals.
- [ ] **Task 1.5: Implement W9b (Alignment Fixes)**
    - Compute label column width per-section to prevent overflow/abutment.
    - Right-align values for cleaner columns.
- [ ] **Task 1.6: Implement W9c (Formatting)**
    - Bold the `TOTAL:` labels using Prose markup.
- [ ] **Validation 1**: Run `c compose --perf prompts/test.md` and verify the report matches the new structure and is legible.

## Phase 2: Redundancy Elimination (W0)

Goal: Eliminate the dominant wall-clock cost identified in the trace findings.

- [ ] **Task 2.1: Enhance `CompositionPrepContext`**
    - Add `launch_workspace: LaunchWorkspaceContext` field.
    - Implement `LaunchWorkspaceContext::from_sniff_result` to reuse existing scan data.
- [ ] **Task 2.2: Thread Context through Execution**
    - Add `prep_launch_workspace: Option<LaunchWorkspaceContext>` to `CompositionExecutionRequest`.
- [ ] **Task 2.3: Optimize Header Env Plan**
    - Update `composition/mod.rs:544` to reuse `prep_launch_workspace` instead of calling `resolve_launch_workspace_context` fresh.
- [ ] **Task 2.4: Optimize Child Env Build**
    - Update `composition/mod.rs:746` to call `env::build_child_env_with_launch`.
- [ ] **Validation 2**: Run `--perf` and verify `header env plan` and `child env build` sub-stages show significant duration reduction (expected ~2s recovery).

## Phase 3: Optimization and Cleanup (W6 + W2)

Goal: Remove minor redundancies and stabilize the environment.

- [ ] **Task 3.1: Implement W6 (Terminal Memoization)**
    - Use `LazyLock<Terminal>` in `crate::log::terminal()` to avoid repeated capability detection.
- [ ] **Task 3.2: Implement W2 (Binary Path Snapshot)**
    - Extend `InstalledProviderSnapshot` with `binary_paths: BTreeMap<Provider, PathBuf>`.
    - Populate it in `build_installed_snapshot`.
    - Update `resolve_binary_path_direct` to use the snapshot lookup.
- [ ] **Validation 3**: Verify `target resolution` sub-stage remains minimal and `clippy` passes.

## Phase 4: Feedback Experience (W1)

Goal: Dramatically improve perceived latency by providing immediate feedback.

- [ ] **Task 4.1: Implement W1 (Receipt Banner)**
    - Add `Status::from_prose("→ Composing {file_ref}…")` emission in `run_compose` and `run_inline_compose` immediately after dispatch.
    - Ensure it respects `--silent` and `--quiet` flags.
- [ ] **Validation 4**: Verify the receipt banner appears instantly upon pressing Enter, well before the main execution header.

## Phase 5: Verification and Documentation

Goal: Finalize the feature and update the project records.

- [ ] **Task 5.1: Performance Baseline Capture**
    - Capture `trace-after-w0.md` using the canonical harness.
    - Capture a wrapper `--perf` control trace for comparison.
- [ ] **Task 5.2: Documentation Update**
    - Update `claudine/docs/pipeline.md` to reflect current step semantics and removed redundancies.
- [ ] **Task 5.3: Workspace-wide Checks**
    - Run all tests: `just test`.
    - Run clippy: `cargo clippy --workspace`.
- [ ] **Task 5.4: Deferred Item Re-evaluation**
    - Compare post-W0 numbers against the 50ms/3s targets.
    - Decide whether W3, W4, or W5 are still required to meet goals.
