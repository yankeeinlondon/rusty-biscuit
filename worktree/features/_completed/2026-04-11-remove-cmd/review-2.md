---
agent: gemini
model: ""
ready: false
---

# Review: Worktree Remove Command

## Summary

The `wt remove` command implementation correctly implements the core logic described in the specification, including uncommitted file detection via the `sniff` library, the branch cleanup flag, and the multi-level force behavior (`-f` vs `-ff`). The code is well-structured, leveraging a clean split between library logic and CLI presentation.

However, the feature is **not yet ready for production** due to a significant gap in test rigor. Specifically, the hierarchical tree rendering and interactive prompts—both key user-facing requirements—lack the Level 2 verification required for "user-observable behaviour" in this monorepo.

## Findings

### High Severity: Missing Level 2 Testing for UI Rendering

The specification requires that uncommitted changes be presented using a hierarchical tree format and that specific confirmation messages (with styling) be shown. 

- **Issue:** The existing integration tests (`worktree/cli/tests/remove.rs`) are Level 1 only. They exclusively test the "bypass" paths where `-f` or `-ff` is used to skip prompts. There is no verification that the `FileTree` (or the custom `dirty_tree` implementation) renders correctly in a real terminal, nor that the `inquire` prompts display the correct styled messages.
- **Requirement:** Add Level 2 tests using `biscuit_test_harness` (e.g., `TmuxHarness` or `WezTermHarness`) to capture terminal output and verify the rendering of the dirty file tree and the confirmation dialogs.
- **Reference:** See "Test Rigor" section in the review instructions. "Spec requires `^X` badges with specific colors + Level-1 unit tests on style only = needs Level-2 capture verifying real-terminal rendering."

### Medium Severity: Departure from Spec Component (FileTree)

- **Issue:** The specification explicitly mentions using the `FileTree` component from `darkmatter`/`biscuit-terminal`. The implementation instead uses a custom `dirty_tree` module.
- **Note:** While `dirty_tree.rs` contains a reasonable justification for this (simplification for static git-status paths), using a custom implementation instead of a shared component increases the maintenance surface area. If `FileTree` truly was too complex, the decision is acceptable, but the Level 2 testing requirement becomes even more critical to ensure this custom rendering matches the quality of the standard components.

### Low Severity: Error Message for Main Worktree

- **Observation:** In `remove.rs:run`, the error message for removing the main checkout is printed to `stderr` and then a `WorktreeError` is returned. This results in double-printing if the caller also handles the error by printing it.
- **Recommendation:** Rely on the `WorktreeError` to carry the message and let the top-level error handler manage the display, or use a specific error variant that indicates the message has already been presented.

## Success Criteria Verification

| Criterion | Status | Notes |
|-----------|--------|-------|
| Worktree removed from filesystem/git | ✅ | Verified by Level 1 functional tests. |
| UI correctly renders uncommitted files in tree format | ⚠️ | Logic implemented and unit-tested for markup, but terminal rendering unverified (Level 2 gap). |
| `sniff` library leveraged for file categorization | ✅ | Verified in `worktree/lib/src/worktree.rs`. |
| Optional branch deletion works | ✅ | Verified by Level 1 functional tests, including soft-delete safety. |

## Ergonomics and Performance

- **Parallelism:** `list_worktrees` in `lib/src/worktree.rs` uses `std::thread::scope` for parallel status checks. This is excellent for performance in large repositories.
- **Bypass Logic:** The `-f` bypass logic (`< 10 files && no source code`) is implemented exactly as specified and is well-covered by unit tests.
- **CLI Arg Action:** Using `clap::ArgAction::Count` for `force` is the idiomatic way to handle `-f` vs `-ff`.

## Recommendations

1. **Implement Level 2 Tests:** Create at least two Level 2 tests:
    - One that spawns `wt remove` on a dirty worktree (with source files) and verifies the captured terminal text contains the tree characters (`├──`, `└──`) and the specific "source code files" warning message.
    - One that verifies the styling (red for source, yellow for non-source) via escape code assertion or computed style checks if using a more advanced harness.
2. **Review `dirty_tree` vs `FileTree`:** Ensure that `dirty_tree` isn't reinventing too much that `FileTree` already provides, specifically around terminal width handling or special character support.
3. **Consolidate Error Handling:** Clean up the manual `eprintln!` in `remove.rs` to avoid redundant error reporting.

## Conclusion

**Ready for Production:** No (False)

The implementation logic is sound, but the lack of terminal-level verification for the primary UX component (the tree rendering) fails to meet the monorepo's rigor standards for user-observable features.
