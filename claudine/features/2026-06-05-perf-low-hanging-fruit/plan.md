---
phases: 5
created: 2026-06-05
start_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - claudine/features/2026-06-05-perf-low-hanging-fruit/plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - claudine
  - claudine-cli
---

# Execution Plan: Eliminate Redundant Repo-Root Detection in Child Env Build

## Phase 1: Confirm Scope and Baseline

- [x] Read `claudine/features/2026-06-05-perf-low-hanging-fruit/spec.md` and confirm the implementation scope is limited to the shadow-HOME root reuse described there.
- [x] Inspect `claudine/cli/src/commands/wrap/repo_home.rs`, `claudine/cli/src/commands/wrap/env.rs`, and `claudine/cli/src/commands/wrap/composition/mod.rs` to confirm current `needs_shadow_home` and `build_repo_home_env` call sites.
- [x] Inspect existing tests in `claudine/cli/src/commands/wrap/repo_home.rs` and `claudine/cli/tests/wrap_commands.rs` to identify the smallest place to add coverage.
- [x] Capture a baseline targeted test command list before edits, favoring `cargo test -p claudine-cli repo_home --lib` or the nearest existing test target if module tests are named differently.
- [x] Validation checkpoint: confirm no affected behavior requires changing `claudine/lib/src/linking/paths.rs` beyond leaving `resolve_repo_root` as the fallback API.

## Phase 2: Thread the Effective Root Through the Shadow-HOME API

- [ ] Update `repo_home::needs_shadow_home` in `claudine/cli/src/commands/wrap/repo_home.rs` to accept `effective_root: Option<&Path>`.
- [ ] Update the Codex branch of `needs_shadow_home` so it resolves the repo root from `effective_root.map(Path::to_path_buf).unwrap_or_else(|| resolve_repo_root(cwd))`.
- [ ] Update `repo_home::build_repo_home_env` in `claudine/cli/src/commands/wrap/repo_home.rs` to accept `effective_root: Option<&Path>`.
- [ ] Update `build_repo_home_env` so the measured `repo_root_detect` duration wraps only the fallback resolution path; when `effective_root` is supplied, clone that path and record only microsecond-scale local work.
- [ ] Update `RepoHomeTimings` rustdoc to describe `repo_root_detect` as fallback resolution or known-root reuse time, not as an unconditional sniff git walk.
- [ ] Update any nearby comments or rustdoc that claim the shadow-HOME hot path always re-runs repo detection.
- [ ] Validation checkpoint: search with `rg -n "needs_shadow_home|build_repo_home_env|resolve_repo_root\\(cwd\\)" claudine/cli/src/commands/wrap` and confirm the API and comments are internally consistent.

## Phase 3: Update All Call Sites

- [ ] Update `build_child_env_with_launch` in `claudine/cli/src/commands/wrap/env.rs` so `needs_shadow_home` receives `Some(launch_ctx.child_cwd.as_path())`.
- [ ] Update the `build_repo_home_env` call in `build_child_env_with_launch` so it receives `Some(launch_ctx.child_cwd.as_path())`.
- [ ] Update the MCP late shadow-HOME materialization call in `claudine/cli/src/commands/wrap/composition/mod.rs` so `build_repo_home_env` receives `Some(env_plan.child_cwd.as_path())`.
- [ ] Update any direct tests or helper call sites for `needs_shadow_home` and `build_repo_home_env` to pass either `Some(root)` or `None` explicitly.
- [ ] Parallelizable: while call sites are being updated, another implementer can prepare test fixtures for supplied-root vs. fallback behavior in `repo_home.rs`.
- [ ] Validation checkpoint: run `cargo check -p claudine-cli --color=never` and resolve all signature-update errors before adding broader tests.

## Phase 4: Add Regression Coverage

- [ ] Add a focused unit test for `needs_shadow_home` proving a supplied effective root is used for Codex repo-local prompt detection.
- [ ] Add a focused unit test for `build_repo_home_env` proving a supplied effective root materializes Codex repo prompts from that root even when `cwd` points somewhere else.
- [ ] Add a focused fallback test proving `build_repo_home_env(..., None)` still resolves from `cwd` and preserves legacy behavior.
- [ ] Add or update a regression test for the source-repo-vs-launch-repo split: when a composed source document lives outside the launch repo, Codex shadow-HOME prompt materialization uses the launch child root, not the source metadata root.
- [ ] Keep new tests L1 unless they require a real terminal; use temporary directories and local git fixtures only.
- [ ] Parallelizable: source/launch split coverage can be developed independently from the low-level `repo_home.rs` unit tests after Phase 3 compiles.
- [ ] Validation checkpoint: run the targeted tests added in this phase and confirm they fail against the old behavior or directly assert the new root-selection contract.

## Phase 5: Validate Behavior and Prepare Handoff

- [ ] Run `cargo test -p claudine-cli repo_home --lib --color=never` or the exact targeted equivalent for the updated module tests.
- [ ] Run the targeted integration test covering the source-repo-vs-launch-repo split, for example `cargo test -p claudine-cli <test_name> --test wrap_commands --color=never` if the test lands in `wrap_commands.rs`.
- [ ] Run `cargo check -p claudine -p claudine-cli --color=never` to verify both Claudine crates still compile.
- [ ] Run `just -f claudine/justfile test-cli <targeted args>` or `just -f claudine/justfile sanity` if time permits and the targeted checks are clean.
- [ ] Run a before/after-style manual perf smoke check with `claudine compose --perf --dry-run --repo <fixture-or-real-compose-file>` and confirm `child env build -> shadow home sync -> repo root detect` no longer reflects the previous 660ms-2s git walk.
- [ ] Inspect `git diff -- claudine/cli/src/commands/wrap/repo_home.rs claudine/cli/src/commands/wrap/env.rs claudine/cli/src/commands/wrap/composition/mod.rs claudine/cli/tests/wrap_commands.rs` and confirm changes are limited to the planned API, call sites, tests, and comment drift fixes.
- [ ] Validation checkpoint: acceptance criteria are satisfied when all targeted tests pass, the compile check passes, and the perf smoke check shows the redundant shadow-HOME repo detection collapsed while preserving the existing perf tree shape.
