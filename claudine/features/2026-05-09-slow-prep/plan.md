---
phases: 5
created: 2026-05-09
start_phase: 1
source_files_during_phase_1:
  - claudine/lib/src/model_catalog/service.rs
  - claudine/lib/src/model_catalog/provider_sources.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/sequence.rs
docs_updated_during_phase_1:
  - claudine/docs/topics/execution-flow.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/composition/prepare.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/composition/prep_context.rs
  - claudine/cli/src/commands/compose.rs
  - claudine/cli/src/commands/wrap/sequence.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
  - claudine
  - claudine-cli
---

# Slow Compose Prep — Execution Plan

## Phase 1: Stop Global Catalog Refresh

**Goal:** Eliminate unconditional, blocking dynamic-catalog refresh from the compose hot path.

- [ ] Add `refresh_provider_blocking(Provider)` to `ModelCatalogService` (or equivalent API).
- [ ] Implement in-process deduplication so OpenCode and Qwen share one `opencode models` subprocess per refresh scope.
- [ ] Modify eager target resolution (`wrap::composition::eagerly_resolve_target`) to skip `refresh_blocking()`.
- [ ] Move provider selection **before** any catalog refresh:
  - Explicit provider flag → select immediately, skip unrelated catalogs.
  - Frontmatter/config/provider hint → select using static data only.
  - TTY picker → select provider first, then scope refresh to the chosen provider.
- [ ] Update model resolution so dynamic refresh runs only for the selected provider and only when the model hint requires catalog validation.
- [ ] Ensure catalog refresh failure falls back to stale/static catalog (preserve existing behavior).
- [ ] **Checkpoint:** Unit tests pass:
  - `--claude` eager resolution does not call dynamic catalog refresh.
  - `--codex` eager resolution does not call dynamic catalog refresh.
  - `--opencode` refreshes only OpenCode.
  - `--qwen` refreshes only Qwen and deduplicates the underlying OpenCode fetch.
  - Non-TTY frontmatter agent selection still follows existing precedence.
  - Unknown model behavior is unchanged for providers with available catalogs.
  - Refresh failure still falls back to stale/static catalog.
- [ ] **Checkpoint:** Manual trace shows no Tokio child-pipe poller windows for `opencode models` on `--claude` runs.

## Phase 2: Share Source and Repo Context

**Goal:** Build source-root information once and reuse it across all prep phases.

- [ ] Define `CompositionPrepContext` (CLI-private struct) holding:
  - Original file reference.
  - Resolved source path.
  - Source parent directory.
  - Source repo root (if any).
  - Ambient CWD.
  - Loaded selection config for effective source repo root or CWD.
  - Optional installed-provider snapshot.
- [ ] Populate `CompositionPrepContext` immediately after `composition::resolve_composition_source` returns.
- [ ] Thread `CompositionPrepContext` into `eagerly_resolve_target` instead of rediscovering git roots.
- [ ] Thread `CompositionPrepContext` into shell preflight setup.
- [ ] Replace `prepare_direct()` / `prepare_inline()`'s internal `find_git_root_from_path()` call with a value passed via `PrepareOptions` or the prep context.
- [ ] Verify that `biscuit-file` workspace discovery (required for file-reference semantics) still runs exactly once; remove the extra `sniff::filesystem::git::detect_git` call from the compose hot path.
- [ ] **Checkpoint:** Manual trace shows at most one source repo-root discovery outside `biscuit-file`'s required resolution.
- [ ] **Checkpoint:** All existing provider-selection tests pass.

## Phase 3: Measure and Decide on Compose Pass Reuse

**Goal:** Instrument the hot path, measure after Phase 1–2 fixes, and decide whether Darkmatter composition passes are worth deduplicating.

- [ ] Add or reuse `--perf` / `tracing` spans covering:
  - File reference resolution.
  - Source repo/root discovery.
  - Selection config load.
  - Installed client detection.
  - Model catalog refresh.
  - Shell preflight discovery.
  - Final composition preparation.
  - Environment detection.
- [ ] Run the motivating `darkmatter` prompt with `--perf` and `--claude`:
  - Once with no shell directives.
  - Once with `::shell` directives.
- [ ] Compare shell preflight discovery vs. final preparation timing.
- [ ] **Decision gate:**
  - If two-pass Darkmatter composition is now material (>50 ms or >10 % of remaining prep time):
    - Add reuse path between `collect_shell_commands()` and final `prepare_*()` when the composed document/report is compatible.
    - Document equivalence conditions (same interpolation state, same approved-command set).
  - If still sub-millisecond:
    - Leave two-pass behavior in place and document the measured reason in the feature notes.
- [ ] **Checkpoint:** Decision recorded in the feature directory (`decision.md` or inline in `plan.md`).

## Phase 4: Environment Detection Follow-Up

**Goal:** Move environment detection out of the critical pre-launch path if Phase 3 shows it matters.

- [ ] Review whether `events::detect_environment_fast` is still on the critical path after Phase 1–2.
- [ ] If it is:
  - Make `EnvironmentContext` creation lazy or minimally scoped.
  - Spawn provider with a minimal context; enrich with OS/hardware/repo details asynchronously or after launch.
  - Preserve dispatch metadata correctness for lifecycle and stream events.
- [ ] If it is not material:
  - Add perf spans for future visibility and defer deeper work.
- [ ] **Checkpoint:** Manual trace shows environment detection is either off the critical path or explicitly instrumented.

## Phase 5: Integration Validation and Acceptance

**Goal:** Verify the complete fix meets acceptance criteria and no regressions are introduced.

- [ ] Implement CLI-level test: `claudine compose fast.md --claude --dry-run` completes prep without executing `opencode models` (use a test double on `PATH` that fails if invoked).
- [ ] Implement CLI-level test: `claudine inline-compose fast.md --claude --dry-run` has the same guarantee.
- [ ] Verify shell-preflight tests still pass (prompt with `::shell` still triggers approval/discovery).
- [ ] Verify Ctrl+C during prep exits 130 and emits the clean user-interrupt notice.
- [ ] Run manual verification:
  ```sh
  RUST_LOG=trace claudine compose prompts/implement-phase.md \
    plan="features/2026-05-08-expression-syntax/plan.md" \
    -y --claude total_phases=6
  ```
- [ ] **Checkpoint:** Trace shows:
  - No `opencode models` subprocess windows for unselected providers.
  - At most one source repo-root discovery outside required file-reference resolution.
  - Provider launch or dry-run output reached in **under 1 second** on the same repo, targeting **under 500 ms**.
- [ ] **Checkpoint:** All existing provider-selection and shell-preflight tests pass.
- [ ] **Checkpoint:** Acceptance criteria satisfied (see `spec.md`).

## Parallelizable Work

- **Phase 1 and Phase 2 are mostly sequential** (Phase 2 depends on the `CompositionPrepContext` interface, which can be designed in parallel but should be implemented after Phase 1 to avoid merge conflicts in `eagerly_resolve_target`).
- **Phase 3 measurement** can begin as soon as Phase 1 instrumentation spans are in place, even before Phase 2 finishes.
- **Phase 4** can be designed in parallel with Phase 3 but should only be implemented after Phase 3 decision gate.
- **Phase 5 integration tests** can be drafted during Phase 1 (test harness, test double on `PATH`) and finalized once Phase 1–2 behavior is stable.

## Validation Checkpoints Summary

| Phase | Checkpoint | How Verified |
|-------|-----------|-------------|
| 1 | No dynamic refresh for `--claude` / `--codex` | Unit tests + manual trace |
| 1 | Provider-scoped refresh works for OpenCode/Qwen | Unit tests |
| 2 | Single source-root discovery | Manual trace |
| 2 | All existing tests pass | `cargo test` |
| 3 | Decision on composition-pass reuse | `decision.md` + perf spans |
| 4 | Environment detection off critical path or instrumented | Manual trace |
| 5 | CLI-level dry-run tests pass | Test suite |
| 5 | Ctrl+C behavior preserved | Manual + existing tests |
| 5 | Under 1 s prep time (target 500 ms) | Manual trace on same repo |
