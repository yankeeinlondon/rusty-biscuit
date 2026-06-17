---
agent: gemini
model: ""
ready: false
---

# Review: Remove command for Worktree CLI

This review covers the implementation of the `wt remove` command as specified in `worktree/features/2026-04011-remove-cmd/spec.md`.

## Summary

The `remove` command is functional and correctly handles basic worktree removal, tiered force flags, and optional branch cleanup. However, it fails to meet the project's quality bar for **test rigor** and deviates from the **safety requirements** defined in the specification.

## Findings

### 1. Lack of Integration Testing (Severity: High)

The feature has **zero Level 1 (CLI invocation) or Level 2 (Terminal rendering)** tests. 

- **Requirement**: "Worktree is removed from the filesystem and Git metadata."
  - **Status**: Untested at a functional level. Only unit tests for parsing git output exist.
- **Requirement**: "UI correctly renders uncommitted files in a tree format."
  - **Status**: Unit tests verify the generated markup string in `dirty_tree.rs`, but there is no Level 2 test verifying that this renders correctly in a real terminal (e.g., using `biscuit_test_harness`).
- **Requirement**: "Optional branch deletion works as expected."
  - **Status**: Untested. No integration test verifies that `git branch -d` is actually called or that the warning message is surfaced correctly on failure.

**Recommendation**: Implement Level 1 tests using `assert_cmd` and Level 2 tests using `biscuit_test_harness` to verify the interactive flow and rendering.

### 2. Missing Confirmation for Clean Worktrees (Severity: Medium)

The specification explicitly states:
> "If the worktree has no uncommitted changes, display a simple confirmation dialog."

The current implementation in `decide_prompt` skips all prompts for clean worktrees:
```rust
if dirty.paths.is_empty() {
    // Clean worktree: any force level skips prompt; no flag also skips.
    return false;
}
```
This reduces the safety of the tool, as a user might accidentally remove the wrong (but clean) worktree without any confirmation.

**Recommendation**: Update `decide_prompt` to return `true` if `force == 0`, even for clean worktrees, and ensure a simple confirmation message is used.

### 3. Architecture: Re-implementation of Tree Component (Severity: Low)

The spec suggested using the `FileTree` component from `darkmatter`/`biscuit-terminal`. Instead, the developer created a private `dirty_tree` module. While functional, this re-implementation:
- Increases the code surface area to maintain.
- Misses out on features or bugfixes present in the shared component.
- Diverges from the requested architecture.

**Recommendation**: Evaluate if `darkmatter::markdown::reference::file_tree::FileTree` or a more generic version can be extracted and shared, or justify why a custom implementation was necessary.

### 4. UI Polish: Prompt Message Phrasing (Severity: Low)

The prompt message for dirty worktrees with many files is:
`"- the <blue>{display_name}</blue> has {count} files which have not been committed to <b>git</b>! ..."`
This matches the spec but could be more ergonomic by stating "non-source" files if `has_source` is false, to distinguish from the source-code warning.

## Conclusion

The feature provides the core functionality but is **not ready for production** due to the critical gap in integration testing and the deviation from safety requirements.

### Status Matrix

| Requirement | Implementation | Verification Level |
|-------------|----------------|-------------------|
| Worktree Removal | Complete | None (Gap) |
| Dirtiness Detection | Complete | Level 1 (Unit) |
| Force Flag Tiering | Complete | Level 1 (Unit) |
| Safety Dialogs | **Incomplete** | None (Gap) |
| FileTree Rendering | Complete (Custom) | Level 1 (Unit) |
| Branch Cleanup | Complete | None (Gap) |
