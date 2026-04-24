---
ready: false
---

# Review 1: Package Areas Subcommand

## Summary

The implementation of `sniff repo package-areas` is functional and correctly follows the fast-path performance requirements. However, there are several bugs in the output formatting (especially in verbose mode) and a few minor gaps in test coverage that should be addressed before this is ready for production.

## Findings

### 1. Verbose Output Space Mismatch
- **Issue:** The specification requires a space before the parenthesis in verbose mode: `{package-area} ({dir})`.
- **Implementation:** The current implementation renders it without a space: `{package-area}({dir})`.
- **Impact:** Minor visual inconsistency with the specification. This also exists in the `sniff repo packages` command, but `package-areas` specifically called it out in the spec.

### 2. "root" Area Directory Bug (Critical)
- **Issue:** For packages at the repository root, the area is named `"root"`. In verbose mode, this renders as `root(./root)`.
- **Details:** There is typically no `./root` directory in the repository. Top-level packages reside at `./`.
- **Recommendation:** Special-case the `"root"` area in `render_repo_package_areas_formatted` to display as `./` or omit the directory annotation if it's confusing.

### 3. Redundant Directory Prefix and Plan Divergence
- **Issue:** `render_repo_package_areas_formatted` hardcodes the `./` prefix and assumes the directory name exactly matches the area name.
- **Details:** While usually true in this monorepo, the `Package` struct already contains the correct relative path information.
- **Divergence from Plan:** The original execution plan (`plan.md` step 2.1) included more robust logic for calculating `area_root` from `pkg.relative` and specifically included the space in the format string: `format!("{area} (<dim><i>./{root}</i></dim>)")`. The implementation seems to have been over-simplified, losing both the robustness and the correct formatting.
- **Inconsistency:** `sniff repo packages` uses `pkg.relative` which is more accurate than just prepending `./` to the area name.

### 4. Test Coverage Gaps
- **Missing Tests:**
    - Positional filtering: No test verifies that `sniff repo package-areas {filter}` works.
    - Root area: No test verifies how the special `"root"` area is rendered.
    - Exact matching: No test verifies that `--package-area {exact-name}` works as intended.

### 5. Filter Logic Inconsistency
- **Observation:** Multiple filters use `OR` logic (`any`). While consistent with `sniff repo packages`, a combination of inclusion and exclusion filters (e.g., `pkg-a !test`) might produce unexpected results because of the `OR` behavior. This is an inherited pattern but worth noting for future ergonomic improvements.

## Recommendations

1.  **Fix Verbose Formatting:** Update `render_repo_package_areas_formatted` in `sniff/cli/src/output/filesystem.rs` to include the space before the parenthesis.
2.  **Fix Root Directory Mapping:** In verbose mode, if the area is `"root"`, render the directory as `./` instead of `./root`.
3.  **Enhance Tests:** Add test cases to `sniff/cli/tests/cli.rs` for positional filters and the `"root"` area handling.
