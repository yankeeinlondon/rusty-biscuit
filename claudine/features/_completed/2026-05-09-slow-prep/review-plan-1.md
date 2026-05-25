---
phases: 5
created: 2026-05-09
start_phase: 1
source_files_during_phase_3:
  - claudine/lib/src/model_catalog/service.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
  - claudine
---

# Execution Plan: Slow Compose Prep — Review Gap Closure

This plan closes the functional gaps and missing coverage identified in `review-1.md`. All tasks reference concrete source locations and observable outcomes.

---

## Phase 1: Fix CWD Fallback Selection Config

**Goal:** Restore the legacy behavior where `selection_config` loads from CWD when the prompt file is outside any git repository.

**Depends on:** Nothing (can start immediately).
**Parallelizable with:** Phase 2.

- [x] Update `CompositionPrepContext::new()` in `claudine/cli/src/commands/wrap/composition/prep_context.rs:91` to compute `effective_root` as `source_repo_root.or(Some(&cwd))` and pass it to `load_selection_config()`.
- [x] Audit `claudine/cli/src/commands/wrap/composition/prep_context.rs:117` and `claudine/cli/src/commands/wrap/composition/mod.rs:1605` for the same pattern and apply the same fix.
- [x] Add unit test: `CompositionPrepContext` loads CWD config when `source_repo_root` is `None`.
- [x] Add Level 1 CLI integration test: invoke `claudine compose` with a prompt file outside git, CWD inside a configured repo, and assert non-TTY resolution uses the CWD favorite-agent/model override.
- [x] **Validation checkpoint:** Run existing provider-selection tests; confirm no regressions.

---

## Phase 2: Fix Dynamic Refresh Gating for Env Var Overrides

**Goal:** Prevent dynamic catalog refresh from blocking when provider env vars or `MODEL` override a frontmatter model hint.

**Depends on:** Nothing (can start immediately).
**Parallelizable with:** Phase 1.

- [x] Update `refresh_for_model_validation()` in `claudine/cli/src/commands/wrap/composition/mod.rs:399` to accept the fully resolved model source (CLI arg, env var, or frontmatter) rather than only `cli_model` and `hints.model`.
- [x] Update `resolve_model_with_hints()` in `claudine/lib/src/composition/select.rs:370` and `:380` to communicate whether the final model came from an env var override back to the refresh gate.
- [x] Add unit test: `OPENCODE_MODEL=fast` with frontmatter `model: slow` skips `refresh_provider_blocking(OpenCode)` when `--opencode` is explicit.
- [x] Add unit test: generic `MODEL=fast` with frontmatter `model: slow` skips refresh for the resolved provider.
- [x] Add unit test: provider-specific env var (e.g., `CLAUDE_MODEL`) with frontmatter `model: slow` skips refresh for that provider.
- [x] Add unit test: frontmatter `model: slow` without env override still refreshes the selected dynamic provider when required.
- [x] **Validation checkpoint:** Run catalog service tests and compose preflight tests; confirm no regressions.

---

## Phase 3: Fix Global Refresh OpenCode/Qwen Dedup

**Goal:** Ensure `refresh_all()` shares the underlying `opencode models` result between OpenCode and Qwen, consistent with `refresh_provider()`.

**Depends on:** Nothing (can start immediately).
**Parallelizable with:** Phase 1 and Phase 2.

- [x] Refactor `refresh_all()` in `claudine/lib/src/model_catalog/service.rs:227` and `:241` to iterate providers and call `refresh_provider()` instead of the undeduplicated `refresh()` / `fetch_provider_catalog()` path.
- [x] Add unit test: `refresh_all()` runs `opencode models` at most once when both OpenCode and Qwen are refreshed.
- [x] Add unit test: `refresh_all()` still refreshes Claude and Codex static catalogs correctly.
- [x] **Validation checkpoint:** Run model catalog service tests; confirm dedup coverage passes.

---

## Phase 4: Implement Missing CLI Acceptance Tests

**Goal:** Protect the primary acceptance criteria with CLI-level tests that fail if `opencode models` is invoked on `--claude` or `--codex` paths.

**Depends on:** Phase 1, Phase 2, Phase 3 (the implementation under test must be correct before tests are finalized).
**Parallelizable with:** Phase 5 planning.

- [x] Create a failing `opencode` test double binary that exits 1 and place it on a temporary `PATH`.
- [x] Add CLI integration test: `claudine compose fast.md --claude --dry-run` with the test double on `PATH` completes successfully (proving `opencode models` was not called).
- [x] Add CLI integration test: `claudine inline-compose fast.md --claude --dry-run` with the test double on `PATH` completes successfully.
- [x] Add CLI integration test: `claudine compose fast.md --opencode --dry-run` with the test double on `PATH` fails or handles the expected refresh call correctly.
- [x] Add CLI integration test: `claudine compose fast.md --codex --dry-run` with the test double on `PATH` completes successfully.
- [x] **Validation checkpoint:** Run new acceptance tests alongside existing suite; confirm they fail without Phase 1–3 fixes and pass with them.

---

## Phase 5: Ctrl+C Verification and Final Acceptance

**Goal:** Verify the Ctrl+C user-interrupt behavior during prep and complete the manual acceptance run.

**Depends on:** Phase 1, Phase 2, Phase 3 (prep path must be stable).
**Parallelizable with:** Phase 4 test authoring.

- [x] Document the verification strategy decision: either (a) implement Level 3 OS keyboard injection test, or (b) provide explicit written justification in `review-1.md` for treating Ctrl+C as signal behavior and implement an in-process SIGINT injection test.
- [ ] If Level 3 is chosen: implement OS-level keyboard injection test that sends Ctrl+C during `claudine compose` prep and asserts exit code 130 plus the clean INFO notice in stderr/stdout. *(Not chosen — signal-level path was selected and implemented.)*
- [x] If signal-level is chosen: implement in-process `SIGINT` delivery test that asserts the observed interrupt flag propagates and the command exits 130 with the clean notice.
- [ ] Run manual verification command:
  ```sh
  RUST_LOG=trace claudine compose prompts/implement-phase.md \
    plan="features/2026-05-08-expression-syntax/plan.md" \
    -y --claude total_phases=6
  ```
- [ ] Confirm trace shows: no Tokio child-pipe windows for `opencode models`, no dynamic refresh for unselected providers, at most one source repo-root discovery outside `biscuit-file` resolution, and provider launch reached in under 1 second.
- [ ] **Validation checkpoint:** All acceptance criteria from `spec.md` are met:
  - `--claude` run reaches provider launch in under 1 second (target under 500 ms).
  - Explicit Claude/Codex compose and inline-compose do not execute `opencode models`.
  - OpenCode/Qwen model validation still works when selected.
  - Existing provider-selection and shell-preflight tests pass.
  - Ctrl+C during prep exits 130 with the INFO notice.

---

## Risk Register

| Risk | Mitigation |
|------|------------|
| Phase 1 fix changes path-resolution semantics for repo-scoped configs | Keep `biscuit-file` as authority for file reference; only change config load root fallback |
| Phase 2 env-var precedence logic conflicts with TTY picker | Gate refresh only after full resolution; TTY picker still refreshes selected provider post-choice |
| Phase 3 `refresh_all()` refactor breaks existing callers | Add unit tests for all providers before refactor; maintain `refresh_all()` public API |
| Phase 4 test double pollutes host `PATH` | Use isolated temporary directories and `std::env::remove_var` in test teardown |
| Phase 5 Ctrl+C test is flaky in CI | Use `serial_test` or process-level isolation; skip if PTY is unavailable |
