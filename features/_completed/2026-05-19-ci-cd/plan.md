---
phases: 5
created: 2026-05-19
start_phase: 1
source_files_during_phase_1:
  - .github/workflows/build-integrations.yml
  - .github/workflows/messenger-desktop-tests.yml
  - .github/workflows/release-plz.yml
  - .github/workflows/sniff-performance.yml
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - justfile
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - .githooks/pre-push
  - README.md
docs_updated_during_phase_3:
  - README.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - .github/workflows/claudine-tests.yml
  - .github/workflows/darkmatter-tests.yml
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - justfile
  - .githooks/pre-push
docs_updated_during_phase_5:
  - README.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_during_phase_5: []
packages: []
---

# Execution Plan: CI/CD Infrastructure & Local Pre-Push Hook

This plan outlines the stabilization of GitHub Actions and the introduction of a local pre-push hook to improve the development loop in the `rusty-biscuit` monorepo.

## Phase 1: GitHub Actions Stabilization

Immediate resolution of Node.js 20 deprecation warnings to restore trust in CI signals.

- [ ] **Task 1.1: Patch existing workflows for Node.js 24 compatibility**
  - Add `env: FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true` to:
    - `.github/workflows/build-integrations.yml`
    - `.github/workflows/messenger-desktop-tests.yml`
    - `.github/workflows/release-plz.yml`
    - `.github/workflows/sniff-performance.yml`
- [ ] **Task 1.2: Validation Checkpoint**
  - Push changes to a branch and verify that workflows trigger without the Node.js 20 deprecation warning.

## Phase 2: Justfile Infrastructure

Standardize test orchestration at the root level to support both local hooks and CI.

- [ ] **Task 2.1: Implement root `test` recipe**
  - Add a `test *args=""` recipe to the root `justfile` that:
    - Runs `just test` in the specified area(s).
    - If no areas specified, runs tests for all areas (optional/guarded).
    - Mirrors the logic found in the `doctest` recipe for area mapping.
- [ ] **Task 2.2: Implement root `pre-push` recipe**
  - Add `pre-push *areas="claudine darkmatter"` recipe.
  - This recipe should simply delegate to `just test {{ areas }}`.
- [ ] **Task 2.3: Validation Checkpoint**
  - Run `just test claudine` and `just test darkmatter` from the root and verify they execute correctly.

## Phase 3: Local Pre-Push Hook

Introduce a developer-configurable hook for fast local feedback.

- [ ] **Task 3.1: Create hook script**
  - Create `.githooks/pre-push` (POSIX shell).
  - Implement logic to read `RUSTY_BISCUIT_PRE_PUSH` (defaulting to `warn`).
  - Implement `off` mode (exit 0 immediately).
  - Implement `warn` mode (run tests, print failures in red, exit 0).
  - Implement `strict` mode (run tests, exit non-zero on failure).
  - Hardcode the initial list of areas (`claudine`, `darkmatter`) in a single variable at the top.
- [ ] **Task 3.2: Hook Delegation**
  - The script should call `just pre-push` and capture the exit code.
- [ ] **Task 3.3: Documentation**
  - Update `README.md` with a "Local Development" section.
  - Explain how to link the hook: `ln -s ../../.githooks/pre-push .git/hooks/pre-push`.
  - Document the `RUSTY_BISCUIT_PRE_PUSH` variable.
- [ ] **Task 3.4: Validation Checkpoint**
  - Manually trigger the hook with different environment variable settings and verify behavior (especially `warn` vs `strict`).

## Phase 4: Path-Filtered Workflows

Establish authoritative gates for the core areas in CI.

- [ ] **Task 4.1: Create `claudine-tests.yml`**
  - Use `on: push: paths: ["claudine/**"]`.
  - Invoke `just test claudine` in the runner.
  - Include Node.js 24 environment variable.
- [ ] **Task 4.2: Create `darkmatter-tests.yml`**
  - Use `on: push: paths: ["darkmatter/**"]`.
  - Invoke `just test darkmatter` in the runner.
  - Include Node.js 24 environment variable.
- [ ] **Task 4.3: Validation Checkpoint**
  - Modify a file in `claudine/` and verify only the `claudine-tests.yml` workflow (and general build) triggers.

## Phase 5: Dynamic Change Detection (Optimization)

Migrate from hardcoded area lists to dynamic detection to minimize hook latency.

- [ ] **Task 5.1: Implement change detection logic**
  - Create a script or `just` recipe that uses `git diff --name-only` to identify changed files.
  - Map files to workspace areas.
- [ ] **Task 5.2: Integrate with `pre-push` hook**
  - Update `.githooks/pre-push` to use the dynamic list if no hardcoded override is provided.
- [ ] **Task 5.3: Validation Checkpoint**
  - Verify that touching only `claudine/` files results in only `claudine` tests running during the hook.
