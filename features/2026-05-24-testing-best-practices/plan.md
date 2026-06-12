---
phases: 6
created: 2026-05-24
start_phase: 1
source_files_during_phase_4:
  - biscuit-file/lib/benches/parsers.rs
  - biscuit-file/lib/Cargo.toml
  - biscuit-file/justfile
  - biscuit-hash/lib/benches/hashing.rs
  - biscuit-hash/lib/Cargo.toml
  - biscuit-hash/justfile
  - tree-hugger/lib/benches/tree_file.rs
  - tree-hugger/lib/Cargo.toml
  - tree-hugger/justfile
  - renderable/benches/render.rs
  - renderable/Cargo.toml
  - renderable/justfile
  - .github/workflows/bench-nightly.yml
  - schematic/justfile
  - schematic/define/Cargo.toml
  - schematic/definitions/Cargo.toml
  - schematic/gen/Cargo.toml
  - schematic/oauth/Cargo.toml
  - schematic/schema/Cargo.toml
  - biscuit-location/lib/Cargo.toml
  - biscuit-speaks/lib/Cargo.toml
  - homelab/lib/Cargo.toml
  - model-citizen/lib/Cargo.toml
  - playa/lib/Cargo.toml
  - queue/lib/Cargo.toml
  - research/lib/Cargo.toml
  - unchained-ai/lib/Cargo.toml
  - biscuit-test-harness/Cargo.toml
  - biscuit-browser-harness/Cargo.toml
  - tools/test-toolkit/Cargo.toml
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages:
  - biscuit-file
  - biscuit-hash
  - tree-hugger
  - renderable
  - darkmatter
  - schematic
source_files_during_phase_5:
  - biscuit-file/lib/fuzz/Cargo.toml
  - biscuit-file/lib/fuzz/rust-toolchain.toml
  - biscuit-file/lib/fuzz/fuzz_targets/pdf_extract.rs
  - biscuit-file/lib/fuzz/fuzz_targets/toml_roundtrip.rs
  - biscuit-file/lib/fuzz/fuzz_targets/yaml_roundtrip.rs
  - biscuit-file/lib/fuzz/fuzz_targets/json5_roundtrip.rs
  - biscuit-file/lib/fuzz/corpus-seed/pdf/minimal.pdf
  - biscuit-file/lib/fuzz/corpus-seed/toml/basic.toml
  - biscuit-file/lib/fuzz/corpus-seed/yaml/basic.yaml
  - biscuit-file/lib/fuzz/corpus-seed/json5/basic.json5
  - biscuit-file/justfile
  - darkmatter/lib/fuzz/Cargo.toml
  - darkmatter/lib/fuzz/rust-toolchain.toml
  - darkmatter/lib/fuzz/fuzz_targets/markdown_parser.rs
  - darkmatter/lib/fuzz/corpus-seed/markdown/basic.md
  - darkmatter/justfile
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages:
  - biscuit-file
  - darkmatter
source_files_during_phase_3:
  - justfile
  - just/devops.just
  - biscuit-hash/justfile
  - biscuit-location/justfile
  - biscuit-speaks/justfile
  - schematic/justfile
  - unchained-ai/justfile
  - playa/justfile
  - tree-hugger/justfile
  - sniff/justfile
  - model-citizen/justfile
  - research/justfile
  - queue/justfile
  - homelab/justfile
docs_updated_during_phase_3: []
docs_created_during_phase_3:
  - docs/testing-strategy.md
skills_files_updated_during_phase_3: []
packages:
  - biscuit-hash
  - biscuit-location
  - biscuit-speaks
  - schematic
  - unchained-ai
  - playa
  - tree-hugger
  - sniff
  - model-citizen
  - research
  - queue
  - homelab
source_files_during_phase_2:
  - .config/nextest.toml
  - just/devops.just
  - claudine/justfile
  - darkmatter/justfile
  - biscuit-terminal/justfile
  - biscuit-file/justfile
  - claudine/cli/Cargo.toml
  - claudine/cli/tests/level2_pty_tests.rs
  - claudine/cli/tests/level2_context_pty.rs
  - claudine/cli/tests/level2_validation_reporter_pty.rs
  - darkmatter/lib/Cargo.toml
docs_updated_during_phase_2:
  - claudine/docs/topics/testing.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
  - claudine
  - darkmatter
  - biscuit-terminal
  - biscuit-file
source_files_during_phase_1:
  - tools/test-toolkit/src/lib.rs
  - biscuit-test-harness/Cargo.toml
  - biscuit-test-harness/src/lib.rs
  - biscuit-test-harness/src/shared.rs
  - biscuit-browser-harness/Cargo.toml
  - biscuit-browser-harness/src/lib.rs
  - Cargo.toml
  - darkmatter/lib/Cargo.toml
  - darkmatter/lib/tests/browser_render.rs
  - darkmatter/lib/tests/level2_render_tree_terminal.rs
  - darkmatter/cli/Cargo.toml
  - darkmatter/cli/tests/level2_layout.rs
  - darkmatter/cli/tests/level2_errors.rs
  - biscuit-terminal/cli/Cargo.toml
  - biscuit-tui/cli/Cargo.toml
docs_updated_during_phase_1:
  - biscuit-test-harness/README.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - test-toolkit
  - biscuit-test-harness
  - biscuit-browser-harness
  - darkmatter
  - darkmatter-cli
  - biscuit-terminal-cli
  - biscuit-tui-cli
source_files_during_phase_6:
  - .claude/skills/rust-testing/SKILL.md
  - docs/testing-strategy.md
  - prompts/snippets/test-rigor.md
  - CLAUDE.md
  - .github/workflows/sanity.yml
  - .github/workflows/test.yml
  - .github/workflows/fuzz-nightly.yml
  - .github/workflows/coverage.yml
  - docs/dependencies.md
  - darkmatter/lib/tests/level2_render_tree_terminal.rs
  - darkmatter/cli/tests/level2_layout.rs
  - darkmatter/justfile
docs_updated_during_phase_6:
  - docs/testing-strategy.md
  - docs/dependencies.md
  - CLAUDE.md
  - prompts/snippets/test-rigor.md
docs_created_during_phase_6:
  - .github/workflows/sanity.yml
  - .github/workflows/test.yml
  - .github/workflows/fuzz-nightly.yml
  - .github/workflows/coverage.yml
skills_files_updated_during_phase_6:
  - .claude/skills/rust-testing/SKILL.md
packages:
  - darkmatter
---

# Execution Plan: Testing Best Practices

This plan implements the testing infrastructure, taxonomies, and consistency improvements defined in the [Functional Specification](./spec.md).

## Phase 1 — Foundation Crates

Establish the shared infrastructure for test levels, terminal harnesses, and browser testing.

- [ ] **Task 1.1: Extend `tools/test-toolkit` with Level Enforcement**
    - Add `Level` enum (L1, L2, L3) to `tools/test-toolkit`.
    - Implement `require_level!(level, check)` macro supporting the `BISCUIT_TEST_LEVEL` and `BISCUIT_TEST_LEVEL_REQUIRED` environment variables.
    - Standardize `RUN_LEVEL3` integration.
- [ ] **Task 1.2: Enhance `biscuit-test-harness` with Shared Utilities**
    - Implement `harness::shared::SharedHarness<T>` to encapsulate the Mutex/atexit cleanup pattern.
    - Update `biscuit-test-harness/README.md` to document the preference for `tmux` as the default L2 backend (D5).
- [ ] **Task 1.3: Create `biscuit-browser-harness` Crate**
    - Initialize `biscuit-browser-harness` as a new workspace member.
    - Define `BrowserHarness` trait (`spawn`, `render_html`, `computed_style`, `screenshot`).
    - Extract and adapt the `ChromeHarness` implementation from `darkmatter`.
    - Implement `available()` probe and `BISCUIT_BROWSER_REQUIRED` enforcement.
- [ ] **Task 1.4: Migrate Initial Harness Consumers**
    - Migrate `darkmatter` browser tests to use `biscuit-browser-harness`.
    - Update `darkmatter`, `biscuit-terminal`, `biscuit-tui`, and `claudine` to use the new `test-toolkit` level helpers and `SharedHarness`.
- [ ] **Task 1.5: Validation Checkpoint — Foundation**
    - Verify that `BISCUIT_TEST_LEVEL=1` successfully skips L2 tests in `claudine` and `darkmatter`.
    - Verify that `BISCUIT_BROWSER_REQUIRED=1` panics in `darkmatter` if Chrome is absent.

## Phase 2 — Shared Just Recipes & Sanity Tier

Standardize the `just` interface across the repository and implement the fast sanity check.

- [x] **Task 2.1: Implement Shared Lifecycle Recipes**
    - Create `just/lifecycle.just` (or update `just/devops.just`) with canonical recipe templates: `_sanity`, `_test`, `_test_l2`, `_test_l3`, `_test_browser`, `_test_real`, `_lint`, `_bench`, `_coverage`, `_doctest`, `_fuzz`, `_all`.
    - Implement `_check_canonical` recipe to validate presence of required recipes.
- [x] **Task 2.2: Configure Nextest Filtersets**
    - Update `.config/nextest.toml` to define filtersets: `set:level2`, `set:level3`, `set:browser`, `set:real`, `set:slow`.
    - `set:slow` should aggregate all other "slow" categories.
- [x] **Task 2.3: Migrate Core Package Justfiles**
    - Migrate `claudine`, `darkmatter`, `biscuit-terminal`, and `biscuit-file` to use the shared lifecycle recipes.
    - Ensure `sanity` recipe uses the `set:slow` exclusion filter.
- [x] **Task 2.4: Apply Nextest Filter Contract to Core Packages**
    - Rename tests or modules in `claudine`, `darkmatter`, `biscuit-terminal`, and `biscuit-file` to include stable identifiers (e.g., `level2_`, `browser_`).
- [x] **Task 2.5: Validation Checkpoint — Sanity**
    - Run `just sanity` in `darkmatter` and verify it completes in <15s (excluding build).
    - Verify `just all` runs the tiers in the correct order (D7).

## Phase 3 — Full Package Migration & Validator

Roll out the consistency changes to the entire monorepo area list.

- [x] **Task 3.1: Bulk Migration of Package Justfiles**
    - Update all remaining package areas in the root `justfile`'s curated `areas` list to the canonical 12-recipe set.
    - Implement explicit no-ops for non-applicable recipes with explanatory comments (D6).
- [x] **Task 3.2: Workspace-Wide Validation**
    - Run `just _check_canonical` against all curated areas.
    - Verify that the root `just sanity` orchestrator correctly iterates through all areas.
- [x] **Task 3.3: Update Metadata for Non-Crate Areas**
    - Ensure documentation in `docs/testing-strategy.md` explains which areas are excluded from the canonical set and why (per Scope Clarification for Topic 7).

## Phase 4 — Benchmarking Standardization

Establish a uniform benchmarking story with baseline tracking.

- [ ] **Task 4.1: Add New Criterion Benchmarks**
    - Implement initial benchmarks for `biscuit-file` (parsers), `biscuit-hash`, `tree-hugger`, and `renderable`.
- [ ] **Task 4.2: Implement Bench Opt-out Convention**
    - Add `[package.metadata.benchmarks] required = false` to pure data crates or crates without hot paths.
- [ ] **Task 4.3: Wire Bencher.dev for Darkmatter**
    - Configure GitHub Actions to run darkmatter benchmarks and push results to Bencher.dev (requires CI secret setup).
- [ ] **Task 4.4: Validation Checkpoint — Benchmarking**
    - Verify `just bench` works in newly benchmarked packages.
    - Confirm Bencher.dev receives data from the darkmatter workflow.

## Phase 5 — Fuzz Infrastructure

Add adversarial input testing to high-risk parser crates.

- [ ] **Task 5.1: Setup `biscuit-file` Fuzzing**
    - Create `biscuit-file/fuzz/` with targets for PDF extraction and JSON5/YAML/TOML round-tripping.
    - Pin nightly toolchain in `biscuit-file/fuzz/rust-toolchain.toml`.
    - Add initial `corpus-seed/` contents.
- [ ] **Task 5.2: Setup `darkmatter` Fuzzing**
    - Create `darkmatter/fuzz/` with markdown parser target.
    - Pin nightly toolchain and add `corpus-seed/`.
- [ ] **Task 5.3: Validation Checkpoint — Fuzzing**
    - Run `cargo +nightly fuzz run <target> -- -runs=100` locally for each target to ensure basic functionality.

## Phase 6 — Documentation & CI Workflows

Finalize the documentation and automate the testing tiers in CI.

- [ ] **Task 6.1: Create Agent-Facing Testing Skill**
    - Author `.claude/skills/rust-testing/SKILL.md` (≤200 lines).
    - Include the test-selection decision tree (D11).
- [ ] **Task 6.2: Create Human-Facing Testing Strategy**
    - Author `docs/testing-strategy.md` with deep dives into rationale, patterns, and fuzzing playbooks.
- [ ] **Task 6.3: Update Prompts and Global Docs**
    - Update `prompts/snippets/test-rigor.md` to reference the new skill and `require_level!`.
    - Add testing skill pointer to `CLAUDE.md`.
- [ ] **Task 6.4: Implement Automated CI Workflows**
    - `sanity.yml`: PR gate, `just sanity` across workspace.
    - `test.yml`: PR gate, `just all` across workspace (tier-skipped).
    - `fuzz-nightly.yml`: Nightly scheduled run for all fuzz targets.
    - `bench-nightly.yml`: Nightly scheduled bench run (Darkmatter -> Bencher.dev).
    - `coverage.yml`: Artifact-generating coverage run.
- [ ] **Task 6.5: Final Repository Sweep**
    - Update `docs/dependencies.md` for new `biscuit-browser-harness`.
    - Remove or deprecate old per-package level variables (e.g., `DARKMATTER_LEVEL2_REQUIRED`).
- [ ] **Task 6.6: Final Validation Checkpoint**
    - Verify all CI workflows trigger correctly on a dummy PR.
    - Verify the Testing Skill is correctly loaded and followed by an agent.
