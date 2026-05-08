# Review: Filepath Interpolation (Review #1)

**Date:** 2026-05-05  
**Scope:** Link Resolve + Link Normalization implementation across `link_resolve.rs`, `link_normalization.rs`, `types.rs`, `mod.rs`, `link_interpolation_integration.rs`

---

## 1. Gaps in Designed Functionality

### 1.1 `find_target_range` is duplicated (DRY violation)

`find_target_range` is implemented identically in both `link_resolve.rs` (line 133) and `link_normalization.rs` (line 190). The normalization variant has one extra bounds check (`if span.end > content.len()`) but otherwise the logic is the same.

**Suggestion:** Extract into a shared module (e.g., `link_utils.rs` or `resolve_utils.rs`), or add it to the `reference` subsystem since both operations already depend on it. Alternatively, add a helper method on `ReferenceRecord` or as a standalone utility in `types.rs`.

### 1.2 Warning from ENV-var substitution not tracked in `ComposeReport`

The spec calls for: *"send a warning log to STDERR using the `Status` struct in biscuit-terminal"*. The implementation emits the warning via `eprintln!` + `status.display()`, but **does not add it to `report.warnings`**. This means:

- The warning is only visible as stderr output — no programmatic way to check if an ENV substitution occurred.
- The `ComposeReport` summary does not mention it.
- Downstream consumers (CI checks, linters) cannot verify whether the final output contains path abstractions.

**Suggestion:** After emitting the stderr warning, also push a `ComposeWarning` into `report.warnings` with `stage: "link_normalization"`. Example:

```rust
report.add_warning(ComposeWarning::new(
    "link_normalization",
    format!("path {} abstracted to ${{{}}}/{}", abs_path.display(), var_name, rel.display()),
));
```

### 1.3 Empty `docs/inline/link-normalization.md`

The file exists at `docs/inline/link-normalization.md` but is 0 bytes. Per the plan (step 5.2), a doc should describe Link Normalization logic. The `docs/operations/link-normalization.md` file is well-written, but `docs/inline/` appears to be the convention for operation docs (looking at `docs/inline/interpolation.md`, `docs/inline/text-replacement.md`, etc.).

**Suggestion:** Either populate `docs/inline/link-normalization.md` or move the content there and delete `docs/operations/link-normalization.md`.

---

## 2. Broken or Incomplete Implementation

### 2.1 Media element extraction always emits `ReferenceKind::Image` / `ReferenceSyntax::HtmlImage`

In `html.rs`, the classifiers for `<video>`, `<audio>`, `<source>`, and `<iframe>` all emit `ReferenceKind::Image` and `ReferenceSyntax::HtmlImage`:

```rust
// classify_video_tag, classify_audio_tag, classify_source_tag:
kind: ReferenceKind::Image,
syntax: ReferenceSyntax::HtmlImage,

// classify_iframe_tag:
kind: ReferenceKind::Hyperlink,
syntax: ReferenceSyntax::HtmlAnchor,
```

This means:

- A `<video src="./movie.mp4">` is indistinguishable from an `<img src="./photo.jpg">` in any downstream query.
- Queries like `composed_image_references()` would return video and audio elements.
- The `ReferenceSyntax` discriminant is completely lost — the only way to tell the difference would be by re-parsing the HTML span, which defeats the purpose of typed records.

**Impact:** The extraction layer is correct from a "captures the path" perspective, but the semantic classification is misleading. This affects the **reference graph**, **link validation**, and any tooling that queries links by kind.

**Suggestion:** Add new `ReferenceKind` variants: `HtmlVideo`, `HtmlAudio`, `HtmlSource`, `HtmlIframe`. Update `classify_*` functions to use the appropriate kind. Link Resolve and Link Normalization should keep filtering `Image` + the new media kinds.

### 2.2 `<script src="...">` paths are extracted but never resolved/normalized

The `html.rs` extractors pull `src` attributes from `<script>` tags and classify them as `ReferenceKind::ScriptImport`. However:

- **Link Resolve** only filters `Hyperlink | Image | CssImport | FontImport` — `ScriptImport` is skipped.
- **Link Normalization** does the same filter.

While `<script src>` is not listed in the spec's "links" list, the extraction system already does the work. If CSS `@import` references (extracted from `<style>` blocks) are handled, it seems inconsistent that `<script src>` paths are left as raw strings. 

**Decision point:** If the intent is to intentionally exclude script sources, document that decision. If they should be included, add `ScriptImport` to both filters and extractors.

### 2.3 `find_target_range` fallback can match incorrect occurrences

Both `find_target_range` implementations search for the raw target string within the span. The fallback (`if let Some(idx) = outer_text.find(raw_target)`) will match the **first** occurrence of the substring, which may not be the attribute value if the path appears elsewhere in the tag text.

**Example:** A link like `[README.md](/path/README.md)` — if the file path happens to share a substring with surrounding text, the wrong position could be targeted.

**Suggestion:** Tighten the fallback to require the target to be preceded by `(` or `=` or `'` or `"`.

### 2.4 `base_file` canonicalization may panic

In `link_normalization.rs` line ~140:

```rust
let base_file = match &source {
    ComposeSource::File(path) => Some(std::fs::canonicalize(path).unwrap_or(path.clone())),
    _ => None,
};
```

If the source file doesn't exist, `canonicalize` will fail and `unwrap_or` returns the original path, which may not be absolute. This path is then used for `find_git_repo_root` and `compute_relative_path`. The repo-root walk expects a real directory tree.

**Suggestion:** Log a warning and skip normalization when the source file cannot be canonicalized (i.e., the file is missing or the path is synthetic).

---

## 3. Test Coverage Gaps

### 3.1 No tests for CSS/Font import path resolution

`<link rel="stylesheet" href="style.css">` and `<link rel="preload" href="font.woff2">` are extracted as `CssImport` / `FontImport`, and these kinds are included in the filter in both `link_resolve.rs` and `link_normalization.rs`. But there are no dedicated tests verifying these paths get resolved/normalized.

**Add:**
- `test_link_resolve_css_import` — verify `<link rel="stylesheet" href="./style.css">` is resolved to absolute path
- `test_link_resolve_font_import` — verify `<link href="font.woff2">` is resolved
- `test_link_normalize_css_import` — verify CSS import paths are normalized back to relative
- `test_link_normalize_font_import` — verify font import paths are normalized

### 3.2 No tests for edge cases in `find_target_range`

**Add:**
- Single-quoted attributes: `[link]('./path.md')`
- Mixed quoting across operations: resolve with double-quoted, normalize with single-quoted
- Targets containing parentheses: `[link](path/with (parens).md)` — this breaks the `(target)` pattern
- Targets appearing multiple times in the same span

### 3.3 No tests for non-existent target files

`resolve_absolute` in `link_resolve.rs` tries `canonicalize` first, then falls back to `Some(joined)` (non-canonicalized). No test covers the fallback path or verifies behavior when the target file doesn't exist.

**Add:**
- `test_link_resolve_nonexistent_target` — link to a file that doesn't exist; verify it still resolves to the joined path

### 3.4 No tests for links in transcluded documents

The integration test `test_child_no_normalization` exists but only tests that Link Resolve runs on children. It doesn't verify:
- That child links are correctly resolved with the child's context-aware base (not the parent's)
- That child `CssImport`/`FontImport` links are also resolved

### 3.5 No tests for same-repo with deep directory nesting

The `test_normalize_links_same_repo` test uses a shallow 2-level structure. The `diff_paths` / `compute_relative_path` logic should be tested with deeper nesting (3+ levels) and same-directory cases (`./file.md`).

**Add:**
- `test_normalize_links_same_dir` — source and target in the same directory → should produce `./file.md`
- `test_normalize_links_deep_nesting` — 3+ levels of directories

### 3.6 No tests for ENV-var longest-match selection

The spec says: *"choose the most specific ENV variable from the matched ENV's (the one which has the longest path)"*. There's no test that verifies the longest-match logic when multiple whitelisted vars overlap.

**Add:**
- `test_env_var_longest_match` — set `PROJECT_ROOT=/a/b/c` and `DOCS_BASE=/a/b` for a path `/a/b/c/x/y`; expect `PROJECT_ROOT` wins

### 3.7 Home-dir test uses real `$HOME`

`test_normalize_links_home_dir` resolves against the real user's home directory. This is fine for a smoke test, but could break in CI environments where home directory permissions or paths differ. Consider a temp-dir-based test with a mockable home directory.

---

## 4. Ergonomic & Performance Suggestions

### 4.1 `compute_relative_path` canonicalizes twice

`compute_relative_path` calls `std::fs::canonicalize` on both arguments, and the caller (`normalize_links`) also calls `canonicalize` on `abs_path` before passing it in. This means each path is canonicalized potentially 2-3 times per link.

**Suggestion:** Canonicalize once in the caller and pass already-canonicalized paths to `compute_relative_path`. The function signature can be updated to `compute_relative_path(from: &Path, to: &Path) -> PathBuf` where both are assumed already canonical.

### 4.2 String allocations in `find_target_range`

`find_target_range` builds formatted strings for each search pattern:

```rust
let search_patterns = [
    format!("\"{}\"", raw_target),
    format!("'{}'", raw_target),
    format!("({})", raw_target),
];
```

For links with long paths (e.g., deeply nested absolute paths), this allocates 3 strings per call.

**Suggestion:** Use simpler substring search with `str::find` on the raw target plus delimiter, avoiding format allocations. Or build the patterns once and reuse.

### 4.3 Content copy-on-write unnecessarily aggressive

Both `link_resolve.rs` and `link_normalization.rs` always create `let mut new_content = content.to_string()` even when there are zero links to process. The early-return `if to_resolve.is_empty() / if to_normalize.is_empty()` does prevent this in the common case, but a pathological document with no links still hits the early return only after all 9 extractors have run.

**Minor suggestion:** Consider an early-exit check before running extractors — scan for `[(` and `href="` / `src="` patterns to skip parsing entirely when no link-like syntax is present. This could save MDAST parsing overhead on simple documents.

### 4.4 `diff_paths` could be `impl`-level

`diff_paths` is a standalone function that could be a method on `Path` or `PathBuf`. This is a minor ergonomics point but aligns with the existing codebase style (e.g., `diff_paths` is conceptually `to_can.diff(from_dir)`).

### 4.5 Report counters could use `sum()` with iterators

In `link_resolve.rs`:

```rust
if applied_count > 0 {
    report.link_resolves_applied += applied_count;
}
```

This could be simplified to always add: `report.link_resolves_applied += applied_count;` since adding 0 is a no-op. Same for `link_normalization.rs`. Minor point.

### 4.6 `find_git_repo_root` is duplicated

There are now two implementations of `find_git_repo_root`: one in `mod.rs` (the `find_git_root_from` helper at line ~172) and one in `link_normalization.rs`. They have slightly different loop bodies but equivalent logic.

**Suggestion:** Consolidate into a shared utility (reuse `find_git_root_from` from `mod.rs` or extract to a dedicated module).

---

## Summary Table

| Category | Issue | Severity | Effort |
|----------|-------|----------|--------|
| **Gap** | `find_target_range` duplicated across 2 files | Low | Small |
| **Gap** | ENV warning not tracked in `ComposeReport` | Medium | Small |
| **Gap** | `docs/inline/link-normalization.md` is empty | Low | Trivial |
| **Broken** | Media elements classified as `ReferenceKind::Image` | High | Medium |
| **Broken** | `<script src>` extracted but excluded from pipeline | Low (by design) | Small |
| **Incomplete** | `find_target_range` fallback can match wrong occurrence | Medium | Small |
| **Incomplete** | Missing source file handling in normalization | Low | Small |
| **Tests** | No CSS/Font import tests | Medium | Medium |
| **Tests** | No `find_target_range` edge case tests | Medium | Small |
| **Tests** | No non-existent target tests | Low | Small |
| **Tests** | No deep-nesting / same-dir tests | Low | Small |
| **Tests** | No ENV longest-match tests | Medium | Small |
| **Perf** | Double canonicalization of paths | Low | Small |
| **Perf** | String allocations in pattern search | Low | Small |
| **Ergo** | `diff_paths` as standalone vs method | Low | Trivial |
| **Ergo** | `find_git_root_from` duplicated | Low | Small |
