---
phases: 4
created: 2026-06-07
start_phase: 1
source_files_during_phase_1:
  - justfile
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - .github/workflows/test.yml
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - .github/workflows/test.yml
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - .github/workflows/test.yml
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages: []
---

# Execution Plan: Multi-OS Matrix Testing (Linux, Windows, WSL1)

This plan outlines the implementation of a targeted 3-OS matrix CI pipeline for the `rusty-biscuit` monorepo. It leverages git change-detection, filters out platform-unsupported areas, and uses highly-optimized WSL guest filesystem execution to keep builds extremely fast and cost-conscious.

---

## Phase 1: Local & CI Change Detection Enhancements

Ensure `just` recipes are the single source of truth for identifying changed package areas, supporting both local Git ranges and CI headless environments.

- [ ] **Task 1.1: Extend `just changed-areas` to accept a custom revision range**
  - Modify `changed-areas` in the root `justfile` to accept an optional range argument.
  - Usage: `just changed-areas [range]`.
  - If `range` is provided, compare `git diff --name-only "$range"`. Otherwise, fallback to the current `$upstream..HEAD` comparison.
- [ ] **Task 1.2: Add a CI change-detection helper `ci-changed-areas`**
  - Create a new recipe `ci-changed-areas [before_commit]` or script to format changed package areas for GitHub Actions.
  - Requirements:
    1. Determine the target revision range (e.g. comparing the push head with the previous pushed commit `${{ github.event.before }}` or `HEAD~1` if single-commit / new branch).
    2. Identify changed directories and match them against the curated `areas` list.
    3. **Apply Platform Exclusions**: Safely filter out platform-unsupported areas such as `homelab`.
    4. **Global Change Fallback**: If global config files (e.g., `Cargo.toml`, `Cargo.lock`, root `justfile`, `.github/workflows/test.yml`) are modified, return all supported areas.
    5. **JSON Output**: Output the list of areas as a JSON-compatible string array (e.g., `["biscuit-file", "biscuit-terminal"]`) so GHA can directly parse it into a dynamic matrix.
- [ ] **Task 1.3: Validation Checkpoint**
  - Run `just changed-areas HEAD~1` locally and verify the correct areas are output.
  - Run `just ci-changed-areas` locally simulating various ranges and verify that modifying `homelab/` is excluded, while modifying `Cargo.toml` returns the full list of supported areas in a JSON array.

---

## Phase 2: Main Branch 3-OS Matrix Configuration

Update the primary CI gate to trigger the 3-OS matrix on `main` pushes/merges, while retaining fast Linux-only runs on feature branches.

- [ ] **Task 2.1: Implement Branch Gating**
  - Modify `.github/workflows/test.yml` to split or configure jobs according to branch targets.
  - If target branch is **NOT** `main` (feature branches):
    - Run the quick, cheap single-OS validation on `ubuntu-latest` (Linux) only.
    - Validate `just check-canonical` and run `just all <changed-areas>`.
- [ ] **Task 2.2: Add `detect-changes` setup job for `main`**
  - Add a lightweight `detect-changes` job running on `ubuntu-latest` that:
    - Checks out the repository.
    - Runs `just ci-changed-areas "${{ github.event.before }}"` to compute the target areas.
    - Saves the JSON array of package areas as a GHA job output.
- [ ] **Task 2.3: Configure 3-OS Test Matrix**
  - Define the main `test` job dependent on `detect-changes`.
  - Use a matrix layout mapping the target environments:
    - **Linux**: `os: ubuntu-latest`
    - **Windows**: `os: windows-latest`
    - **WSL1**: `os: windows-latest` (explicitly tagged or configured as WSL)
  - Bind the matrix `area` to `${{ needs.detect-changes.outputs.areas }}`.
- [ ] **Task 2.4: Cache Optimization**
  - Ensure `Swatinem/rust-cache` is configured with distinct keys reflecting each environment (including a separate key suffix for WSL1) to prevent cache thrashing.

---

## Phase 3: WSL1 Setup and Isolation Strategy

Establish a high-performance WSL1 guest test environment inside the Windows runner, eliminating standard GHA filesystem bottlenecks.

- [ ] **Task 3.1: WSL1 Distribution Provisioning**
  - Integrate a verified GHA WSL setup action (such as `ubuntu/wsl`) inside the `wsl1` matrix runner job.
  - Configure it to boot an Ubuntu distribution and verify WSL1 translation layer compatibility.
- [ ] **Task 3.2: Workspace Synchronization to WSL Guest Filesystem**
  - To avoid the slow, translation-heavy Windows mount `/mnt/c`, synchronize or clone the workspace repository directly into the native Linux guest filesystem:
    - Create a directory `/home/runner/rusty-biscuit` inside the WSL distribution.
    - Copy the checked-out codebase from the GHA workspace to the guest path:
      ```bash
      wsl -d Ubuntu -- mkdir -p /home/runner/rusty-biscuit
      wsl -d Ubuntu -- cp -r /mnt/c/actions-runner/_work/rusty-biscuit/rusty-biscuit/. /home/runner/rusty-biscuit/
      ```
- [ ] **Task 3.3: Toolchain Setup inside WSL guest**
  - Execute a setup script purely within the WSL environment:
    - Install Rust via `rustup` inside the WSL distribution.
    - Install the `just` recipe orchestrator using `cargo install just --locked` (or curl standard release).
- [ ] **Task 3.4: Isolated Test Execution**
  - Run the tests targeting the selected area inside WSL:
    ```bash
    wsl -d Ubuntu -- cd /home/runner/rusty-biscuit && source $HOME/.cargo/env && just test <area>
    ```

---

## Phase 4: Integration Verification and Acceptance Gate

Verify execution hermeticity and performance limits to finalize the feature.

- [ ] **Task 4.1: Feature Branch Validation**
  - Push changes to a feature branch.
  - Confirm that the CI triggers **only** the `ubuntu-latest` job, compiling and testing only the changed package areas very quickly.
- [ ] **Task 4.2: Merging / Push to `main` Validation**
  - Merge the branch into `main` and push.
  - Verify that the `detect-changes` job executes correctly and computes the dynamic list of changed areas.
  - Confirm that the `ubuntu-latest`, `windows-latest` (native), and `windows-latest` (WSL1) test runners are triggered in parallel for only those specific package areas.
  - Verify that the WSL1 compilation does not time out and finishes with comparable speed.
- [ ] **Task 4.3: Global Change Sanity Check**
  - Push a change directly to `main` modifying the root `Cargo.toml`.
  - Confirm that the change detection recognizes this as a global change, automatically falls back to testing all supported areas, and executes the entire suite across all 3 environments.
