---
phases: 6
created: 2026-05-09
start_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - claudine/features/2026-05-08-testing-setup-teardown/plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - Cargo.toml
  - tools/test-toolkit/Cargo.toml
  - tools/test-toolkit/src/lib.rs
docs_updated_during_phase_2:
  - claudine/features/2026-05-08-testing-setup-teardown/plan.md
  - docs/dependencies.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
  - claudine
  - test-toolkit
---

# Testing Setup/Teardown Execution Plan

## Phase 1: Baseline Inventory and Decisions

- [x] Confirm the workspace package list with `cargo metadata --no-deps --format-version 1` and record whether `tools/test-toolkit` already exists.
- [x] Inspect root `Cargo.toml`, `just/devops.just`, and existing `.config/` contents to confirm where new workspace members, dev-dependencies, and nextest config belong.
- [x] Inspect representative Claudine tests for existing setup/teardown patterns, especially hand-rolled env guards such as `PlayaDryRunGuard` in `canonical_dispatch.rs`.
- [x] Decide the standard `serial_test` composition style for `rstest` tests and document the chosen convention in the implementation notes before touching tests.
- [x] Validation checkpoint: identify the exact files that will be added or changed, and confirm no existing tests need bulk migration.

### Phase 1 Implementation Notes

- Workspace inventory: `cargo metadata --no-deps --format-version 1` reports 56 workspace members. `sniff repo` reports 57 Rust packages because it also shows the excluded `schematic/schema` package. `tools/test-toolkit` does not exist yet.
- Root `Cargo.toml`: new shared test infrastructure belongs in the workspace `members` list as `tools/test-toolkit`. `schematic/schema` remains excluded and is unrelated to this feature.
- `just/devops.just`: `_test` already prefers `cargo nextest run -p <pkg>` when nextest is installed, falling back to `cargo test -p <pkg>`. No timing wrapper logic is needed.
- Existing `.config/nextest.toml`: present already, but it only configures `retries = 3` for `default` and `ci`. Phase 3 should preserve or intentionally reconcile that retry behavior while adding the spec's slow-test and JUnit settings.
- Claudine package manifests: `claudine/lib/Cargo.toml` and `claudine/cli/Cargo.toml` both already use `serial_test = "3"` as a dev-dependency. Neither has `rstest` or `test-toolkit` yet. Phase 4 should add those only to the package manifests whose tests are actually touched.
- Representative env-guard patterns: `claudine/lib/tests/canonical_dispatch.rs` has `PlayaDryRunGuard` for `PLAYA_DRY_RUN=1`; `claudine/cli/src/commands/wrap/composition/mod.rs`, `claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs`, and `claudine/cli/src/commands/wrap/exec/timeouts.rs` have similar local `EnvGuard` / `TestEnvGuard` patterns. Phase 4 should migrate one concrete Claudine pattern first, with `canonical_dispatch.rs` as the most focused target.
- Standard `rstest` + `serial_test` convention: use `#[rstest]` on new or newly migrated tests, and stack `#[serial_test::serial]` directly on the same test when global env or other process-global state requires serialization. Prefer the fully qualified `#[serial_test::serial]` attribute in migrated tests, avoid `rstest_reuse` for now, and keep `#[rstest]` visually first. Async tests should use the `rstest` async-compatible form required by the final selected implementation and should not be migrated unless the fixture or guard change makes the test clearer.
- Planned file scope for later phases: `Cargo.toml`, `tools/test-toolkit/Cargo.toml`, `tools/test-toolkit/src/lib.rs`, `.config/nextest.toml`, the touched Claudine manifest(s), one focused Claudine test file such as `claudine/lib/tests/canonical_dispatch.rs`, `.claude/skills/rust-testing/SKILL.md`, and any dependency/testing documentation required by the added crates. No existing tests need bulk migration.

## Phase 2: Add the Shared `test-toolkit` Crate

- [x] Create the `tools/test-toolkit` package with a minimal library crate and add it as a workspace member.
- [x] Add crate metadata, dependencies, and dev-dependencies needed for the initial scope: `tracing` for phase spans and test dependencies for validating env restoration.
- [x] Implement `trace_phase!` as an exported macro that creates an INFO `tracing` span, enters it for the wrapped block, and returns the block result.
- [x] Implement `EnvGuard` with constructors for setting an environment variable and removing it, preserving whether the variable was previously unset or previously set.
- [x] Ensure `EnvGuard` restores/removes variables in `Drop` and exposes enough API for tests without encouraging manual teardown calls.
- [x] Add unit tests for `trace_phase!` expression return behavior, `EnvGuard` restore behavior, `EnvGuard` removal behavior, and nested guard behavior.
- [x] Validation checkpoint: run `cargo test -p test-toolkit` or `cargo nextest run -p test-toolkit` and confirm the new crate is isolated and passing.

### Phase 2 Implementation Notes

- Added the `test-toolkit` package at `tools/test-toolkit` and registered it as a workspace member.
- `trace_phase!` wraps a block in an INFO-level tracing span and returns the block result, so it can be used for fixture setup expressions as well as assertion blocks.
- `EnvGuard` preserves the prior env state and restores it in `Drop`. Its constructors are `unsafe` because Rust 2024 treats process environment mutation as unsafe global state; callers must serialize env access with `#[serial_test::serial]` or an equivalent strategy.
- Validation completed with `cargo test -p test-toolkit` and `cargo clippy -p test-toolkit --all-targets -- -D warnings`.

## Phase 3: Configure Monorepo Test Timing

- [ ] Add root `.config/nextest.toml` with the default and CI profiles from the spec.
- [ ] Verify `just/devops.just` already prefers nextest when installed and does not need wrapping logic for timing output.
- [ ] Add or update root-level documentation near the test command description only if the repository already documents test runner behavior there.
- [ ] Validation checkpoint: run `cargo nextest run -p test-toolkit` with the default profile and confirm nextest accepts `.config/nextest.toml`.

## Phase 4: Adopt the Infrastructure in Claudine First

- [ ] Add `rstest = "0.25"` as a dev-dependency to the Claudine package manifests that will use new or touched tests in this feature.
- [ ] Add `test-toolkit` as a path dev-dependency for the same Claudine package manifests, using the correct relative path to `tools/test-toolkit`.
- [ ] Replace one concrete Claudine hand-rolled env guard pattern with `test_toolkit::EnvGuard`, keeping the test behavior unchanged.
- [ ] Convert only the touched tests to `#[rstest]` where fixture injection or the new guard makes the test clearer; leave unrelated existing tests unchanged.
- [ ] Add at least one focused Claudine test or fixture use that demonstrates the intended `rstest` setup pattern without creating artificial coverage.
- [ ] Use `trace_phase!` only in a test or fixture where the phase boundary is meaningful and observable.
- [ ] Validation checkpoint: run the narrow affected Claudine test target with nextest or cargo test and confirm the migrated tests pass.

## Phase 5: Documentation and Drift Maintenance

- [ ] Update `.claude/skills/rust-testing/SKILL.md` to make `rstest`, `trace_phase!`, `EnvGuard`, and nextest timing the local testing convention.
- [ ] Document the chosen `serial_test` plus `rstest` composition style in the skill so future migrations are consistent.
- [ ] Update Claudine or monorepo README/testing docs if public test workflow guidance changes because of `.config/nextest.toml` or `test-toolkit`.
- [ ] Update `docs/dependencies.md` and any affected per-area dependency docs if this repo tracks added crates there.
- [ ] Validation checkpoint: review docs for consistency with the spec's migration policy: no bulk migration, new and modified tests prefer `#[rstest]`.

## Phase 6: Full Validation and Handoff

- [ ] Run `cargo fmt` for the workspace.
- [ ] Run `cargo test -p test-toolkit` or `cargo nextest run -p test-toolkit` after formatting.
- [ ] Run the affected Claudine package tests with nextest where available.
- [ ] Run the repository's applicable lint command for touched packages, such as `just lint claudine` if supported or the package-specific clippy command.
- [ ] Inspect `git diff --stat` and `git diff` to confirm the implementation stayed within the planned scope.
- [ ] Validation checkpoint: record the commands run, their results, and any skipped validations with reasons.

## Parallelization Notes

- [ ] Phase 1 must complete before implementation starts because it determines exact manifests, test files, and the `serial_test` convention.
- [ ] Phase 2 and Phase 3 can proceed in parallel after Phase 1 because `test-toolkit` implementation and nextest configuration are independent.
- [ ] Phase 4 depends on Phase 2 because Claudine tests need the `test-toolkit` API before adopting it.
- [ ] Phase 5 can partially run in parallel with Phase 4 after the public API names are stable, but final docs should wait until the code shape is confirmed.
- [ ] Phase 6 depends on all implementation and documentation phases.
