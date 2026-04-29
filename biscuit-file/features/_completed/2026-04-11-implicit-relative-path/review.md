---
feature: implicit-relative-path
date: 2026-04-27
ready: true
---

# Feature Review: Implicit Relative Path for FileReference

The "implicit relative path" feature has been reviewed against the specification, design, and implementation plan.

## Review Summary

The implementation is **production-ready**. It successfully introduces a new `ImplicitRelative` reference kind that resolves bare paths (e.g., `README.md`) against both the current working directory and the git repository root, while maintaining the strict CWD-only resolution for explicit relative paths (`./foo.md`).

## Findings

### 1. Functionality & Requirements
- **Requirement Coverage:** All requirements from `spec.md` are fulfilled. Bare paths are correctly distinguished from explicit relative paths during parsing.
- **Resolution Logic:** The `collect_roots` function in `resolve.rs` correctly implements the `[CWD, git_root]` search order for `ImplicitRelative` references.
- **Deduplication:** The implementation correctly avoids redundant searches when the CWD is the git root.
- **Recursive Search:** The `%` prefix correctly inherits the new search roots, as verified by integration tests.

### 2. Implementation Quality
- **Type Safety:** The addition of `ImplicitRelative` to `ReferenceKind` is handled idiomatically across the codebase.
- **Rust Edition:** The code utilizes Rust 2024 features (like let-chains) appropriately.
- **Observability:** Tracing calls (`debug!`, `trace!`) have been updated to include the new reference kind.

### 3. Test Coverage
- **Unit Tests:** Existing unit tests in `parse.rs` and `resolve.rs` were updated, and new ones were added to cover the new variant.
- **Integration Tests:** `biscuit-file/lib/tests/implicit_relative.rs` provides exhaustive coverage using real `TempDir` and `git init` scenarios.
- **Edge Cases:** Tests cover name collisions (CWD priority), resolution outside git repos, and subdirectory filtering in recursive searches.

### 4. Documentation
- **Topic Guide:** `biscuit-file/docs/topics/file-references.md` has been thoroughly updated with clear sections for Explicit vs. Implicit relative references, including updated reference tables.
- **Doctests:** Public API documentation reflects the new behavior and all doctests pass.

## Maintenance Notes
- The use of `if let ... && ...` requires Rust 2024 or the `let_chains` feature. The package is correctly configured for the 2024 edition.
- The `implicit_relative.rs` integration test uses `git2` for repo initialization, ensuring platform independence.

## Conclusion

The feature is complete, verified, and well-documented. No regressions were found in existing functionality.

**Status: READY**
