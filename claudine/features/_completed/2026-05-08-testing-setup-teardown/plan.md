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
source_files_during_phase_3: []
docs_updated_during_phase_3:
  - claudine/features/2026-05-08-testing-setup-teardown/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/lib/Cargo.toml
  - claudine/lib/tests/canonical_dispatch.rs
docs_updated_during_phase_4:
  - claudine/features/2026-05-08-testing-setup-teardown/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/lib/src/messaging/send.rs
docs_updated_during_phase_5:
  - claudine/docs/topics/testing.md
  - claudine/features/2026-05-08-testing-setup-teardown/plan.md
  - docs/dependencies.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/rust-testing/SKILL.md
source_files_during_phase_6:
  - claudine/cli/src/commands/compose.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/composition/prep_context.rs
  - claudine/cli/src/commands/wrap/sequence.rs
  - claudine/cli/src/output/mod.rs
  - claudine/cli/tests/wrap_commands.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/parser.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/alias.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
  - darkmatter/lib/src/markdown/compose/toc_linking/mod.rs
  - darkmatter/lib/src/markdown/compose/toc_linking/render.rs
  - darkmatter/lib/tests/ternary_integration.rs
  - schematic/gen/tests/artifact_drift.rs
  - schematic/schema/src/artificial_analysis.rs
  - schematic/schema/src/prelude.rs
  - schematic/schema/tests/artificial_analysis_client.rs
docs_updated_during_phase_6:
  - claudine/features/2026-05-08-testing-setup-teardown/plan.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages:
  - claudine
  - claudine-cli
  - darkmatter
  - messenger
  - schematic-gen
  - schematic-schema
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

- [x] Add root `.config/nextest.toml` with the default and CI profiles from the spec.
- [x] Verify `just/devops.just` already prefers nextest when installed and does not need wrapping logic for timing output.
- [x] Add or update root-level documentation near the test command description only if the repository already documents test runner behavior there.
- [x] Validation checkpoint: attempt `cargo nextest run -p test-toolkit` with the default profile, then run the available fallback validation.

### Phase 3 Implementation Notes

- Updated the existing root `.config/nextest.toml` instead of replacing it, preserving the repository's `retries = 3` behavior for both default and CI profiles.
- Added the default profile slow-test threshold from the spec: `slow-timeout = { period = "5s", terminate-after = 3 }`.
- Added the CI profile slow-test threshold and JUnit output from the spec: `slow-timeout = { period = "10s", terminate-after = 2 }` and `junit = { path = "test-results.xml" }`.
- Verified `just/devops.just` already checks `cargo nextest --version` and runs `cargo nextest run -p <pkg>` when available, so no recipe changes were needed.
- No root-level testing documentation currently describes nextest runner configuration, so no documentation change was needed beyond this plan update.
- `cargo nextest run -p test-toolkit` could not validate the config in this environment because `cargo-nextest` is not installed. Fallback validation passed with `cargo test -p test-toolkit`; package-area validation passed with `just test` and `just lint` from `claudine/`.

## Phase 4: Adopt the Infrastructure in Claudine First

- [x] Add `rstest = "0.25"` as a dev-dependency to the Claudine package manifests that will use new or touched tests in this feature.
- [x] Add `test-toolkit` as a path dev-dependency for the same Claudine package manifests, using the correct relative path to `tools/test-toolkit`.
- [x] Replace one concrete Claudine hand-rolled env guard pattern with `test_toolkit::EnvGuard`, keeping the test behavior unchanged.
- [x] Convert only the touched tests to `#[rstest]` where fixture injection or the new guard makes the test clearer; leave unrelated existing tests unchanged.
- [x] Add at least one focused Claudine test or fixture use that demonstrates the intended `rstest` setup pattern without creating artificial coverage.
- [x] Use `trace_phase!` only in a test or fixture where the phase boundary is meaningful and observable.
- [x] Validation checkpoint: run the narrow affected Claudine test target with nextest or cargo test and confirm the migrated tests pass.

### Phase 4 Implementation Notes

- Added `rstest = "0.25"` and `test-toolkit = { path = "../../tools/test-toolkit" }` to `claudine/lib/Cargo.toml`, which is the only Claudine package manifest touched by this phase.
- Replaced the local `PlayaDryRunGuard` in `claudine/lib/tests/canonical_dispatch.rs` with a `playa_dry_run` `#[fixture]` returning `test_toolkit::EnvGuard`.
- Converted only `dispatch_sound_effect_action` to the new pattern: `#[rstest]`, `#[tokio::test]`, and `#[serial_test::serial]` stacked directly on the async test.
- Wrapped the dry-run fixture setup in `trace_phase!("setup_playa_dry_run", ...)` so the setup boundary is observable without holding a tracing span across async awaits.
- Focused validation passed with `cargo test -p claudine --test canonical_dispatch dispatch_sound_effect_action`.

## Phase 5: Documentation and Drift Maintenance

- [x] Update `.claude/skills/rust-testing/SKILL.md` to make `rstest`, `trace_phase!`, `EnvGuard`, and nextest timing the local testing convention.
- [x] Document the chosen `serial_test` plus `rstest` composition style in the skill so future migrations are consistent.
- [x] Update Claudine or monorepo README/testing docs if public test workflow guidance changes because of `.config/nextest.toml` or `test-toolkit`.
- [x] Update `docs/dependencies.md` and any affected per-area dependency docs if this repo tracks added crates there.
- [x] Validation checkpoint: review docs for consistency with the spec's migration policy: no bulk migration, new and modified tests prefer `#[rstest]`.

### Phase 5 Implementation Notes

- Updated `.claude/skills/rust-testing/SKILL.md` with the local testing convention: new and modified tests prefer `#[rstest]`, process-global state requires `#[serial_test::serial]`, `test_toolkit::EnvGuard` owns environment restoration, and `trace_phase!` is reserved for meaningful setup/body/teardown spans.
- Updated `claudine/docs/topics/testing.md` so Claudine's public package-area workflow points at `just test` and `just lint`, explains nextest slow-test timing, and records the `rstest` / `serial_test` / `test-toolkit` migration policy.
- Updated `docs/dependencies.md` to include `rstest = "0.25"` in the development testing dependencies. No per-area Claudine dependency document exists.
- Fixed `claudine/lib/src/messaging/send.rs` so the existing optional desktop notification body path builds a `messenger::MessageBody::Plain`, which was required for the package-area lint check to compile.
- Reviewed the docs for the no-bulk-migration policy and kept the guidance scoped to new and modified tests.
- Validation passed with `CARGO_TARGET_DIR=/tmp/claudine-phase5-target just lint` and `CARGO_TARGET_DIR=/tmp/claudine-phase5-target just test` from `claudine/`. The isolated target directory avoided contention with another Cargo process using the shared workspace `target/`.

## Phase 6: Full Validation and Handoff

- [x] Run `cargo fmt` for the workspace.
- [x] Run `cargo test -p test-toolkit` or `cargo nextest run -p test-toolkit` after formatting.
- [x] Run the affected Claudine package tests with nextest where available.
- [x] Run the repository's applicable lint command for touched packages, such as `just lint claudine` if supported or the package-specific clippy command.
- [x] Inspect `git diff --stat` and `git diff` to confirm the implementation stayed within the planned scope.
- [x] Validation checkpoint: record the commands run, their results, and any skipped validations with reasons.

### Phase 6 Implementation Notes

- `cargo nextest --version` failed because `cargo-nextest` is not installed in this environment, so nextest-specific validation used the repository's fallback cargo paths.
- `cargo fmt --all -- --check` initially found formatting drift. `cargo fmt --all` was run, then `cargo fmt --all -- --check` passed.
- Workspace formatting touched Claudine CLI, Darkmatter, and Schematic source files that were already in the broader dirty worktree context. The Phase 6 frontmatter records the source files updated by formatting.
- `cargo test -p test-toolkit` passed: 5 unit tests, 0 doctests.
- `CARGO_TARGET_DIR=/tmp/claudine-phase6-target just lint` passed from `claudine/`.
- `CARGO_TARGET_DIR=/tmp/claudine-phase6-target just test` passed from `claudine/`, covering both `claudine` and `claudine-cli`. The PTY/performance ignored tests remained ignored by the package-area recipe.
- `git diff --stat` and `git diff --name-only` were reviewed after validation. No commits or staging were performed.

## Parallelization Notes

- [ ] Phase 1 must complete before implementation starts because it determines exact manifests, test files, and the `serial_test` convention.
- [ ] Phase 2 and Phase 3 can proceed in parallel after Phase 1 because `test-toolkit` implementation and nextest configuration are independent.
- [ ] Phase 4 depends on Phase 2 because Claudine tests need the `test-toolkit` API before adopting it.
- [ ] Phase 5 can partially run in parallel with Phase 4 after the public API names are stable, but final docs should wait until the code shape is confirmed.
- [ ] Phase 6 depends on all implementation and documentation phases.
