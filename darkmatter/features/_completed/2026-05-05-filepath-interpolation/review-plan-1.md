# Implementation Plan: Filepath Interpolation Fixes (Review #1)

This plan addresses all the feedback from `review-1.md` for the filepath-interpolation feature in the `darkmatter` package. The work is divided into four iterative phases to ensure high confidence and progressive validation.

## Phase 1: Shared Utilities & Data Structures
**Goal:** Resolve DRY violations and correct the extraction/classification layer for media and script elements.

1. **`ReferenceKind` & `ReferenceSyntax` Updates:**
   - In `lib/src/markdown/reference/types.rs`, add new variants to `ReferenceKind`: `HtmlVideo`, `HtmlAudio`, `HtmlSource`, `HtmlIframe` (or semantically equivalent names).
   - Add corresponding variants to `ReferenceSyntax` (e.g., `HtmlVideoTag`).
2. **Extractor Updates:**
   - In `lib/src/markdown/reference/html.rs`, update the `classify_video_tag`, `classify_audio_tag`, `classify_source_tag`, and `classify_iframe_tag` functions to emit the new `ReferenceKind` and `ReferenceSyntax` variants instead of defaulting to `Image` and `Hyperlink`.
3. **Consolidate `find_target_range`:**
   - Extract the duplicated `find_target_range` logic from `link_resolve.rs` and `link_normalization.rs`.
   - Make it a shared utility function (e.g., expose it as `pub(crate)` in `link_resolve.rs` or move to a common utils module).
4. **Consolidate Git Root Discovery:**
   - Expose the `find_git_root_from` function in `lib/src/markdown/compose/mod.rs` as `pub(crate)` and use it inside `link_normalization.rs` instead of duplicating the logic.

**Validation:**
- Run `cargo check -p darkmatter`.
- Run unit tests in `html.rs` and `types.rs` to ensure correct extraction logic.

## Phase 2: Core Logic Fixes (Resolve & Normalization)
**Goal:** Address logical flaws, missing warnings, and apply performance improvements to the pipeline operations.

1. **Include Script Imports:**
   - In both `link_resolve.rs` and `link_normalization.rs`, add `ReferenceKind::ScriptImport` (along with the new media kinds) to the filter list so they are processed properly.
2. **Enhance `find_target_range` Fallback:**
   - Tighten the fallback substring matching logic. Instead of matching the first occurrence of the raw target string, require the target to be preceded by a relevant delimiter character (`(`, `=`, `"`, `'`).
   - Optimize the function to avoid allocating format strings in a loop.
3. **Fix Canonicalization Panic:**
   - In `link_normalization.rs`, handle the case where `std::fs::canonicalize(path)` fails for `base_file` gracefully. Log a warning and skip normalization instead of unwrapping.
4. **Optimize Path Canonicalization:**
   - Modify `compute_relative_path` to accept already-canonicalized paths (`from: &Path`, `to: &Path`) to avoid double-canonicalization during the loop.
5. **Track ENV Warnings in Report:**
   - In `link_normalization.rs`, when an ENV-var substitution occurs, call `report.add_warning(ComposeWarning::new("link_normalization", ...))` so the warning is properly tracked programmatically.
6. **Ergonomic Cleanups:**
   - Implement the `applied_count` addition directly (`report.link_resolves_applied += applied_count;`) without the `if applied_count > 0` wrapper.
   - Introduce an early exit check (e.g., scanning for `href=` or `]({`) to avoid cloning the content if no link-like patterns exist.

**Validation:**
- Run `cargo test -p darkmatter`.
- Verify the existing unit tests in `link_resolve.rs` and `link_normalization.rs` pass with the updated logic.

## Phase 3: Test Coverage Expansion
**Goal:** Address all missing test cases identified in the review.

1. **Add CSS/Font Import Tests:**
   - In `link_resolve.rs` and `link_normalization.rs`, add dedicated tests for `<link rel="stylesheet">` and `<link rel="preload" as="font">` paths.
2. **Add Target Range Edge Case Tests:**
   - Test single-quoted attributes, mixed quoting, targets with parentheses (e.g., `[link](path/with (parens).md)`), and targets appearing multiple times in the same span.
3. **Add Non-Existent Target Tests:**
   - Ensure the fallback path for resolving non-existent targets is tested.
4. **Add Nested/Dir Tests:**
   - In `link_normalization.rs`, add tests for deep directory nesting (3+ levels) and same-directory (`./file.md`) cases.
5. **Add ENV Longest-Match Tests:**
   - Test that the most specific environment variable is chosen when multiple whitelisted variables match the path.
6. **Add Integration Tests:**
   - Update `lib/tests/link_interpolation_integration.rs` to verify that links within transcluded children are resolved correctly based on the child's base path, not the parent's.

**Validation:**
- Run `cargo test -p darkmatter`. Ensure all new tests pass and adequately cover the edge cases.

## Phase 4: Ergonomics & Documentation
**Goal:** Fix the remaining documentation issues and ensure the codebase meets standards.

1. **Consolidate Documentation:**
   - Move the content from `docs/operations/link-normalization.md` into `docs/inline/link-normalization.md`.
   - Delete the now-redundant `docs/operations/link-normalization.md` file.
2. **Lint & Formatting:**
   - Run `cargo clippy -p darkmatter` to fix any remaining warnings introduced by these changes.
   - Run `cargo fmt -p darkmatter`.

**Validation:**
- Confirm the documentation is properly located and populated.
- Ensure zero lint warnings or errors exist in the `darkmatter` package.