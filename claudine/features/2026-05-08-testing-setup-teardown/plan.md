---
phases: 6
created: 2026-05-09
start_phase: 1
---

# Testing Setup/Teardown Execution Plan

## Phase 1: Baseline Inventory and Decisions

- [ ] Confirm the workspace package list with `cargo metadata --no-deps --format-version 1` and record whether `tools/test-toolkit` already exists.
- [ ] Inspect root `Cargo.toml`, `just/devops.just`, and existing `.config/` contents to confirm where new workspace members, dev-dependencies, and nextest config belong.
- [ ] Inspect representative Claudine tests for existing setup/teardown patterns, especially hand-rolled env guards such as `PlayaDryRunGuard` in `canonical_dispatch.rs`.
- [ ] Decide the standard `serial_test` composition style for `rstest` tests and document the chosen convention in the implementation notes before touching tests.
- [ ] Validation checkpoint: identify the exact files that will be added or changed, and confirm no existing tests need bulk migration.

## Phase 2: Add the Shared `test-toolkit` Crate

- [ ] Create the `tools/test-toolkit` package with a minimal library crate and add it as a workspace member.
- [ ] Add crate metadata, dependencies, and dev-dependencies needed for the initial scope: `tracing` for phase spans and test dependencies for validating env restoration.
- [ ] Implement `trace_phase!` as an exported macro that creates an INFO `tracing` span, enters it for the wrapped block, and returns the block result.
- [ ] Implement `EnvGuard` with constructors for setting an environment variable and removing it, preserving whether the variable was previously unset or previously set.
- [ ] Ensure `EnvGuard` restores/removes variables in `Drop` and exposes enough API for tests without encouraging manual teardown calls.
- [ ] Add unit tests for `trace_phase!` expression return behavior, `EnvGuard` restore behavior, `EnvGuard` removal behavior, and nested guard behavior.
- [ ] Validation checkpoint: run `cargo test -p test-toolkit` or `cargo nextest run -p test-toolkit` and confirm the new crate is isolated and passing.

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
