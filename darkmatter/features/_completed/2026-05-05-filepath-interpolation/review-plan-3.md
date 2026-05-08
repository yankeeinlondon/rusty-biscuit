# Implementation Plan: Review 3 Fixes

## Overview

This plan addresses the 4 low-priority findings from `review-3.md` for the filepath interpolation feature in darkmatter.

**Estimated effort:** Small (all fixes are localized, no API changes)

---

## Phase 1: Core `find_target_range` Fixes

### 1.1 Fix Wrong Attribute Replacement (Issue 1)

**Problem:** `find_target_range` returns the first occurrence of the target string within the span, which can match a non-target attribute if it contains the same path and appears first.

**Solution:** Enhance `find_target_range` to be attribute-aware for HTML records.

**Implementation:**
- In `darkmatter/lib/src/markdown/compose/mod.rs`, update `find_target_range` to accept the `ReferenceSyntax` (already available on `record.origin.syntax`)
- Map HTML syntax kinds to their attribute names:
  - `href`: `HtmlAnchor`, `HtmlLinkTag`
  - `src`: `HtmlImage`, `HtmlVideoTag`, `HtmlAudioTag`, `HtmlSourceTag`, `HtmlIframeTag`, `HtmlScriptTag`
- When the syntax is HTML-based, search for the pattern `attr="target"` or `attr='target'` (with optional whitespace around `=`) within the span instead of just the raw target string
- For Markdown syntax (`MarkdownLink`, `MarkdownImage`), keep the existing behavior (search for the target within `()`)
- Fall back to current behavior if attribute-specific search fails

**Key file:** `darkmatter/lib/src/markdown/compose/mod.rs` (lines 144-190)

**Test:** Add unit test in `link_resolve.rs`:
```rust
#[test]
fn test_link_resolve_wrong_attribute_not_replaced() {
    // <img alt="logo.png" src="logo.png"> should replace src, not alt
}
```

### 1.2 Fix HTML Entity Decoding Mismatch (Issue 2)

**Problem:** `extract_attribute` decodes HTML entities (e.g., `&amp;` → `&`), but `find_target_range` searches for the decoded string in the raw HTML.

**Solution:** Try both decoded and HTML-encoded forms in `find_target_range`.

**Implementation:**
- In `find_target_range`, after attempting to find `raw_target`, also attempt to find `html_escape::encode_html_entities(raw_target)`
- The `html_escape` crate is already a dependency (used in `extract_attribute`)
- Try the decoded form first (handles normal cases), then the encoded form (handles entity cases)

**Key file:** `darkmatter/lib/src/markdown/compose/mod.rs` (lines 144-190)

**Test:** Add unit test in `link_resolve.rs`:
```rust
#[test]
fn test_link_resolve_html_entity_in_attribute() {
    // <a href="foo&amp;bar.md"> should resolve correctly
}
```

---

## Phase 2: Warning Message and Missing Test Coverage

### 2.1 Add Prose Markup to ENV-var Warning (Issue 3)

**Problem:** The ENV-var warning message is plain text instead of including `<blue>` and `<b>` prose markup as specified.

**Solution:** Update the warning string in `link_normalization.rs`.

**Implementation:**
- In `darkmatter/lib/src/markdown/compose/link_normalization.rs` (lines 172-176), change:
  ```rust
  let msg = format!(
      "the path <blue>{}</blue> was found to be an offset of the <b>{}</b> environment variable and will use this abstraction.",
      abs_path.display(),
      var_name
  );
  ```
- The CLI already renders warnings through `Status::from_prose`, so the markup will be processed correctly

**Key file:** `darkmatter/lib/src/markdown/compose/link_normalization.rs` (lines 172-176)

**Test:** Update the existing `test_normalize_links_env_var` test to assert that the warning message contains `<blue>` and `<b>` tags.

### 2.2 Add Nested `<source>` Test (Issue 4)

**Problem:** No test exists for `<source>` tags nested inside `<video>` or `<audio>`.

**Solution:** Add a unit test that verifies nested `<source>` tags are correctly resolved.

**Implementation:**
- Add test in `darkmatter/lib/src/markdown/compose/link_resolve.rs`:
  ```rust
  #[test]
  fn test_link_resolve_nested_source_in_video() {
      // <video><source src="./movie.mp4"></video> should resolve the source
  }
  ```

**Key file:** `darkmatter/lib/src/markdown/compose/link_resolve.rs`

---

## Phase 3: Validation

### 3.1 Run Tests

Execute the following test commands to ensure all fixes work:

```bash
cargo test -p darkmatter --lib -- link_resolve link_normalization
cargo test -p darkmatter --test link_interpolation_integration
cargo test -p darkmatter-cli --test cli -- test_compose_link_relative_same_repo test_compose_link_transcluded_child test_compose_env_var_substitution_one_warning test_compose_html_spaced_attributes
```

### 3.2 Lint and Type Check

```bash
just lint darkmatter
```

Or if no area justfile:

```bash
cargo clippy -p darkmatter --all-targets -- -D warnings
cargo clippy -p darkmatter-cli --all-targets -- -D warnings
```

### 3.3 Verify No Regression

- Ensure existing tests still pass
- Ensure no new compiler warnings
- Ensure no debug/trace output to stdout during tests (already addressed in review-2)

---

## Summary of Changes

| File | Lines | Change |
|------|-------|--------|
| `darkmatter/lib/src/markdown/compose/mod.rs` | 144-190 | Enhance `find_target_range` with attribute-aware matching and HTML entity fallback |
| `darkmatter/lib/src/markdown/compose/link_normalization.rs` | 172-176 | Add `<blue>`/`<b>` markup to ENV-var warning |
| `darkmatter/lib/src/markdown/compose/link_resolve.rs` | Tests | Add 3 new unit tests for wrong-attribute, HTML entity, and nested source |

## Test Strategy

- **Level 1 unit tests** for all 4 fixes (adequate per review classification)
- No CLI-level tests needed (findings are edge cases in core logic)
- Update existing `test_normalize_links_env_var` to verify markup tags
- All tests use tempfile for isolated filesystem operations
- No additional dependencies required
