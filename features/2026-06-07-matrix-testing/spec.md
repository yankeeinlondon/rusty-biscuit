---
status: ready for planning and implementation
reviewed: true
---

# Multi-OS Matrix Testing Specification

**Status:** Reviewed and ready for planning and implementation. This specification details the plan to expand the monorepo's testing strategy to validate package changes across multiple host environments (Linux, Windows, and WSL1).

---

## Context: What we have today

The monorepo contains **72 workspace packages** (point-in-time; source of truth is `cargo metadata --no-deps --format-version 1`) grouped under **21 curated package areas** defined in `.github/ci/areas.json`.

Currently, the primary CI validation (`.github/workflows/test.yml`) runs exclusively on `ubuntu-latest` (Linux). On every pull request or push, it executes `just check-canonical` followed by `just all` to build and test all 21 curated areas on Linux. While extremely fast and efficient, this setup fails to catch platform-specific regressions on Windows or WSL1 (e.g. file-path translation differences in `biscuit-file`, terminal manipulation differences in `biscuit-terminal`, or system-discovery bugs in `sniff`).

We also have a local git hook (`.githooks/pre-push`) that uses a dynamic change detection recipe (`just changed-areas`) to restrict pre-push validation only to modified package areas.

---

## Decisions

### D1: Workspace Scope and Change Detection in CI (Question 1)
- **Outcome:** **CI-Specific Change Detection with Targeted Exclusions**
- **Details:** 
  - We will extend the existing dynamic change-detection heuristic (`just changed-areas` or a companion recipe like `just ci-changed-areas`) to support comparing a commit range in a CI environment (e.g. comparing the push/merge head against the base branch/commit).
  - To prevent compilation and execution failures on platform-incompatible packages, we will define a static list of **excluded/unsupported package areas** (e.g., `homelab` which targets physical IoT hardware, or others that do not support cross-platform compilation).
  - The CI workflow will dynamically format the list of changed, supported package areas as a JSON list and pass it as a dynamic matrix input to the test jobs.

### D2: WSL1 CI Environment & Execution Strategy (Question 2)
- **Outcome:** **Isolated Toolchain on Native WSL Guest Filesystem**
- **Details:**
  - WSL1 test execution will run on a `windows-latest` GitHub Actions runner.
  - To provision the WSL1 environment, we will use a mature setup action (such as `ubuntu/wsl`) to initialize an Ubuntu distribution.
  - To bypass the notorious `/mnt/c` performance bottleneck of WSL1's translation layer (which makes compiling Rust projects extremely slow and prone to CI timeouts), the repository will be checked out, cloned, or copied directly into the native WSL guest filesystem (e.g. `/home/runner/rusty-biscuit`).
  - The Linux Rust toolchain (`rustup`) and the `just` utility will be installed and executed purely within the WSL guest environment to compile and test the target package areas.

### D3: CI Triggering and OS Coverage Matrix (Question 3)
- **Outcome:** **Fast Linux on Feature Branches, 3-OS Matrix on `main`**
- **Details:**
  - **Feature Branches:** Pushes to feature branches or non-main branches will continue to trigger a fast, low-cost sanity/validation run on Linux (`ubuntu-latest`) only, verifying `just check-canonical` and the changed package areas.
  - **Main Branch:** Pushes or merges directly to the `main` branch will trigger the comprehensive 3-OS matrix run covering:
    1. **Linux** (`ubuntu-latest`)
    2. **Windows** (`windows-latest` native)
    3. **WSL1** (`windows-latest` with WSL guest environment)
  - **macOS Exclusion:** Since the primary developer develops and thoroughly tests on macOS locally before pushing, macOS is explicitly omitted from the CI matrix to minimize GitHub Actions credit consumption and maintain fast execution times.
  - **Matrix Targeting:** On `main`, only the dynamically detected changed package areas (as determined in D1) will be tested across the 3-OS matrix. If a global file (such as root `Cargo.toml` or root `justfile`) is modified, the entire list of supported areas will be tested.

---

## Implementation Details

### Git Change Detection inside GHA
The CI workflow will determine changed package areas by querying git:
```bash
# In CI, find changed files between the previous pushed state and HEAD
git diff --name-only ${{ github.event.before }} HEAD
```
If a push represents a new branch or `github.event.before` is empty/all-zeros, it will default to comparing against `HEAD~1` or `origin/main`.

### GHA Matrix Integration Example
The pipeline will be structured in two stages:
1. **`detect-changes` Job:**
   - Runs on `ubuntu-latest`.
   - Computes changed areas using `just ci-changed-areas`.
   - Filters out excluded areas.
   - Outputs a JSON string containing the array of areas to test, e.g., `["biscuit-terminal", "biscuit-file"]`.
2. **`test` Job:**
   - Depends on `detect-changes`.
   - Defines a matrix over `os` (`[ubuntu-latest, windows-latest, wsl1]`) and the output `areas`.
   - Runs tests specifically for those areas using `just test <area>`.

### WSL1 Setup Commands
On the WSL1 runner, the workflow steps will look like:
```yaml
- name: Setup WSL and Ubuntu
  uses: ubuntu/wsl@v1 # or similar verified WSL setup action

- name: Setup guest environment and clone repo
  shell: wsl-bash {0}
  run: |
    # Clone current checkout to the home directory inside WSL guest
    git clone "${GITHUB_WORKSPACE}" /home/runner/rusty-biscuit
    cd /home/runner/rusty-biscuit
    # Install Rust
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
    # Install just
    cargo install just --locked
    # Execute tests
    just test <area>
```

---

## Acceptance Criteria

1. **Deterministic Change Isolation:**
   - Modifying files in `biscuit-file/` and pushing to `main` must only trigger compilation and tests for `biscuit-file` (and its dependents, if applicable) on Linux, Windows, and WSL1.
   - Modifying a non-code file (e.g. `docs/design.md`) must not trigger the 3-OS test suite.
2. **Hermetic WSL1 Verification:**
   - The WSL1 workflow must run within the native WSL guest filesystem, and compile and run the tests successfully without hitting GHA job timeouts.
3. **Branch-Level Guardrails:**
   - Pushes to any branch other than `main` must not spin up Windows or WSL1 runners, ensuring fast iterations for the developer.
   - Pushes/merges to `main` must pass all 3 operating systems before being considered green.
