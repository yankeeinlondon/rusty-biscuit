---
ready: false
---

# Review 2: Package Areas Subcommand

## Summary

The implementation of `sniff repo package-areas` successfully leverages the fast-path `detect_repo_structure` and provides the core functionality requested in the specification. However, this feature is not yet ready for production due to a critical bug in JSON output when performance reporting is enabled, a regressed test suite, and several missing validation steps that were previously identified in the first review cycle.

## Findings

### 1. JSON Output Corruption with `--perf` (Critical)
- **Issue:** When running `sniff repo package-areas --json --perf`, the command appends textual performance timings directly to `stdout` after the JSON array.
- **Impact:** This results in invalid JSON output that breaks shell pipelines (e.g., piping to `jq` fails).
- **Recommendation:** Performance data should be emitted to `stderr` when `--json` is active, or ideally, the handler should be refactored to use the standard `output::print_json` logic which correctly encapsulates performance metrics within the JSON object.

### 2. Regressed Test: `test_repo_package_areas_verbose_shows_root_dir`
- **Issue:** This test is currently failing. The implementation correctly adds a space before the parenthesis in verbose mode (`area (./dir)`), but the test asserts a form without the space (`area(./dir)`).
- **History:** This regression was explicitly predicted in the `review-plan-1.md` but was apparently not addressed during implementation.
- **Recommendation:** Update the test assertion to include the space, or replace it with the more robust tests suggested in the previous review plan.

### 3. Missing Planned Tests
- **Gap:** Several tests identified as mandatory in the previous review cycle are still missing:
    - Positional filter validation (Step 2.2 of the previous plan).
    - Negation filter validation (Step 2.3).
    - Root area (`"."` -> `"root"`) rendering validation (Step 2.4).
- **Impact:** We lack verified coverage for the specialized filtering and root-mapping logic.

### 4. Code Quality & Ergonomics
- **Logic Duplication:** `select_repo_package_areas` and `select_repo_package_areas_with_roots` in `sniff/cli/src/output/filesystem.rs` contain nearly identical filtering and collection logic. 
- **Missing Unit Tests:**
    - `make_package_area` in `sniff-lib` lacks unit tests for its path-parent derivation logic.
    - `package_area_root` in `sniff-cli` lacks unit tests for its specialized root mapping (including the `root` area sentinel).
- **Recommendation:** Refactor the selection logic so that the simpler variant calls the more comprehensive one. Move `package_area_root` to `sniff-lib` if appropriate, or at least provide local unit tests in the CLI.

### 5. Performance & Architecture
- **Success:** The implementation correctly uses `detect_repo_structure` (Tier 2/3 API), ensuring sub-50ms execution on large monorepos.
- **Consistency:** The command handler correctly integrates with the `CliPerf` and `Prose` systems, maintaining look-and-feel with the rest of the CLI.

## Recommendations

1.  **Fix JSON/Perf Integration:** Update `handle_repo_package_areas` in `sniff/cli/src/commands.rs` to emit performance data to `stderr` instead of `stdout` when in JSON mode.
2.  **Unify Selection Logic:** Refactor `output/filesystem.rs` to remove duplicated filtering code between the two `select_repo_package_area` functions.
3.  **Repair and Expand Tests:**
    - Fix the space mismatch in `test_repo_package_areas_verbose_shows_root_dir`.
    - Implement the missing tests for positional filters, negations, and the `root` area rendering.
    - Add unit tests for `make_package_area` and `package_area_root`.
