---
prompt: Use the specification file @./spec.md and tech design at @./tech-design.md to build a detailed plan. Save the plan to body of this document as an idiomatic Markdown document.
last_updated: 2026-03-21
---

# Inline HTML Extraction — Implementation Plan

## Overview

Add three new methods to `Markdown`: `has_inline_html()`, `inline_html_links()`, and `inline_html_image_references()`. These extract typed `Link` and `ImageRef` values from raw HTML tags embedded in Markdown content, complementing the existing Markdown-native `links()` and `image_references()` methods.

## Phase 1: Shared AST Helper

**File:** `darkmatter/lib/src/markdown/output/ast.rs`

1. Extract the `markdown::to_mdast(content, &ParseOptions::gfm())` call into a `pub(crate)` helper:

   ```rust
   pub(crate) fn parse_mdast(content: &str) -> MarkdownResult<markdown::mdast::Node>
   ```

2. Refactor `as_ast()` to call `parse_mdast()` internally.
3. Run existing `output/ast.rs` tests — all must pass unchanged.

**Why first:** Both `as_ast()` and the new `inline_html` extractor need the same MDAST parse call. Factoring this out prevents parser-option drift.

## Phase 2: `render/link.rs` Hardening

**File:** `darkmatter/lib/src/render/link.rs`

The current `parse_html_link()` function is too literal — it requires `<a ` (lowercase, single space) and `</a>` (lowercase). Before the new extractor can hand arbitrary HTML slices to it, make it robust:

1. Make opening tag detection case-insensitive: accept `<A `, `<a\t`, `<a\n`, etc.
   - Change the `starts_with("<a ")` / `starts_with("<a>")` checks to a case-insensitive, whitespace-tolerant match on the tag name `a`.
2. Make closing tag detection case-insensitive: accept `</A>`, `</a >`, etc.
   - Change the `ends_with("</a>")` check to trim trailing whitespace within the closing tag and compare case-insensitively.
3. Add unit tests:
   - `<A HREF="...">Text</A>` parses successfully
   - `<a  href="...">Text</a>` (double space) parses successfully
   - `<a\thref="...">Text</a>` (tab) parses successfully
   - Mixed case `<A href="...">text</a>` works
4. Verify existing `parse_html_link` tests still pass.

**Scope guard:** This is a targeted fix to tag-name matching only. No rewrite of attribute parsing.

## Phase 3: `inline_html.rs` — Core Extractor Module

**File:** `darkmatter/lib/src/markdown/inline_html.rs` (new)

### 3a: Internal Types

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HtmlFragmentKind {
    AnchorOpen,
    AnchorClose,
    Image,
    Other,
}

#[derive(Debug)]
struct HtmlFragment<'a> {
    span: std::ops::Range<usize>,
    value: &'a str,
    kind: HtmlFragmentKind,
}

pub(crate) struct InlineHtmlExtraction {
    pub links: Vec<Link>,
    pub images: Vec<ImageRef>,
}
```

### 3b: `has_inline_html()` — Fast Scanner

```rust
pub(crate) fn has_inline_html(content: &str) -> bool
```

Algorithm:

1. Scan the raw content byte-by-byte.
2. Track fenced code block state (triple-backtick or triple-tilde lines toggle a flag).
3. Skip inline code spans (`` ` `` ... `` ` ``) when practical.
4. On each `<` outside code regions:
   - Check if followed by an ASCII letter (opening tag) or `/` + ASCII letter (closing tag) or `!` (comment/declaration) or `?` (processing instruction).
   - Exclude autolinks: `<` followed by a URI scheme (`https://`, `http://`, `mailto:`) is not HTML.
5. Return `true` on first match; `false` if no match found.

**Key rule:** False positives acceptable, false negatives are not.

### 3c: HTML Fragment Classification

```rust
fn classify_fragment(value: &str) -> HtmlFragmentKind
```

- Case-insensitive tag name extraction from the fragment.
- `<img ...>` / `<IMG ...>` → `Image`
- `<a ...>` / `<A ...>` → `AnchorOpen`
- `</a>` / `</A>` → `AnchorClose`
- Everything else → `Other`

### 3d: MDAST Traversal and Fragment Collection

```rust
fn collect_html_fragments<'a>(
    root: &markdown::mdast::Node,
    content: &'a str,
) -> Vec<HtmlFragment<'a>>
```

- Recursively walk the MDAST tree in document order.
- For each `Node::Html` node that has a `position`, slice the original content using byte offsets.
- Classify each slice and build the ordered fragment list.

### 3e: Image Extraction

```rust
pub(crate) fn extract_inline_html_images(content: &str) -> Vec<ImageRef>
```

1. Call `has_inline_html(content)` — early return `Vec::new()` if false.
2. Parse MDAST via `parse_mdast(content)`.
3. If MDAST parse fails, fall back to pulldown-cmark scanner (Phase 4).
4. Collect HTML fragments.
5. For each `Image` fragment:
   - Call `ImageRef::try_from(fragment.value)`.
   - Push on success, skip on failure (optionally `trace!`).
6. Return images in source order.

### 3f: Link Extraction with State Machine

```rust
pub(crate) fn extract_inline_html_links(content: &str) -> Vec<Link>
```

1. Call `has_inline_html(content)` — early return `Vec::new()` if false.
2. Parse MDAST via `parse_mdast(content)`.
3. If MDAST parse fails, fall back to pulldown-cmark scanner (Phase 4).
4. Collect HTML fragments and flatten inline text nodes (needed to reconstruct `<a>...</a>` spans).
5. State machine:
   - On `AnchorOpen`: record `open_span.start`.
   - Advance until `AnchorClose`.
   - On `AnchorClose`: slice `content[open_span.start..close_span.end]`.
   - Pass full slice to `Link::try_from(slice)`.
   - Apply display normalization (Phase 3g).
   - Push result.
6. Malformed cases (open without close, close without open, nested anchors): skip.

### 3g: Display Normalization Helper

```rust
fn normalize_inline_html_link_display(raw: &str) -> String
```

- Decode HTML entities (`&amp;` → `&`, `&lt;` → `<`, etc.).
- Translate `<br>` / `<br/>` → newline.
- Translate `<code>` / `</code>` → backtick.
- Strip all other HTML tags.
- Preserve meaningful author whitespace; trim only tag-removal artifacts.

Applied after `Link::try_from` succeeds:

```rust
let normalized = normalize_inline_html_link_display(parsed.display());
parsed.with_display(normalized)
```

## Phase 4: Pulldown-cmark Fallback Path

**File:** `darkmatter/lib/src/markdown/inline_html.rs` (same module)

```rust
fn fallback_extract_links(content: &str) -> Vec<Link>
fn fallback_extract_images(content: &str) -> Vec<ImageRef>
```

Used only when MDAST parsing fails (the public API is non-fallible).

Algorithm:

1. Parse with `pulldown-cmark`.
2. For images: look for `Event::InlineHtml(...)` events where the value starts with `<img` (case-insensitive). Pass to `ImageRef::try_from`.
3. For links: buffer `Event::InlineHtml("<a ...>")` as open, collect intervening `Text`/`SoftBreak`/`HardBreak` events, close on `Event::InlineHtml("</a>")`. Reconstruct the full HTML string and pass to `Link::try_from`.

This path does not need to be as robust as the primary path — its purpose is resilience.

## Phase 5: Public API on `Markdown`

**File:** `darkmatter/lib/src/markdown/mod.rs`

1. Add `mod inline_html;` to the module declarations.
2. Add the three public methods:

```rust
impl Markdown {
    /// Returns `true` if the document content contains any inline HTML.
    ///
    /// This is a fast, allocation-light check that can be used as a gate
    /// before the heavier extraction methods. False positives are possible
    /// (e.g., HTML comments will return `true`), but false negatives are not.
    pub fn has_inline_html(&self) -> bool {
        inline_html::has_inline_html(&self.content)
    }

    /// Extracts typed links from HTML `<a>` tags in the document content.
    ///
    /// Complements `links()`, which extracts Markdown-native links only.
    /// Results are returned in source order without deduplication.
    /// Malformed or unterminated anchors are silently skipped.
    pub fn inline_html_links(&self) -> Vec<Link> {
        inline_html::extract_inline_html_links(&self.content)
    }

    /// Extracts typed image references from HTML `<img>` tags in the document content.
    ///
    /// Complements `image_references()`, which extracts Markdown-native images only.
    /// Results are returned in source order without deduplication.
    /// Malformed image tags are silently skipped.
    pub fn inline_html_image_references(&self) -> Vec<ImageRef> {
        inline_html::extract_inline_html_images(&self.content)
    }
}
```

## Phase 6: Unit Tests

**File:** `darkmatter/lib/src/markdown/inline_html.rs` (inline `#[cfg(test)]` module)

### `has_inline_html()` tests

| # | Input | Expected |
|---|-------|----------|
| 1 | `"# Hello\n\nJust text."` | `false` |
| 2 | `"[link](https://x.com)"` | `false` |
| 3 | `"![img](./a.png)"` | `false` |
| 4 | `"Before <a href=\"https://x.com\">X</a> after"` | `true` |
| 5 | `"An <img src=\"./a.png\" />"` | `true` |
| 6 | `"<https://example.com>"` (autolink) | `false` |
| 7 | Fenced code block containing `<a>` | `false` |
| 8 | Inline code span containing `<img>` | `false` (best effort) |
| 9 | `"<!-- comment -->"` | `true` |
| 10 | `"<div class=\"note\">Hello</div>"` | `true` |

### `inline_html_image_references()` tests

| # | Scenario | Assertion |
|---|----------|-----------|
| 1 | Basic `<img src="./a.png" alt="A" />` | Returns 1 `ImageRef` with correct `src` and `alt` |
| 2 | Multiple images in order | Returns all in source order |
| 3 | `title` attribute preserved | `image.title()` matches |
| 4 | `srcset` attribute preserved | `image.srcset()` matches |
| 5 | `loading="lazy"` preserved | Typed attribute set correctly |
| 6 | Malformed `<img>` (missing src) | Skipped, no error |
| 7 | `<img>` inside fenced code block | Not extracted |
| 8 | `<IMG SRC="x.png">` (uppercase) | Extracted correctly |
| 9 | Mixed: MD image + HTML image | Only HTML image returned |

### `inline_html_links()` tests

| # | Scenario | Assertion |
|---|----------|-----------|
| 1 | Basic `<a href="https://x.com">Click</a>` | Returns 1 `Link` with correct href/display |
| 2 | Multiple anchors in order | Returns all in source order |
| 3 | Attributes: `class`, `style`, `target`, `title`, `data-*` | All preserved on `Link` |
| 4 | Nested `<strong>` in display | Display normalized to plain text |
| 5 | Nested `<em>` in display | Display normalized |
| 6 | Nested `<code>` in display | Display contains backticks |
| 7 | Nested `<br>` in display | Display contains newline |
| 8 | Unterminated anchor (no `</a>`) | Skipped, no error |
| 9 | Inside fenced code block | Not extracted |
| 10 | `<A HREF="...">Text</A>` (uppercase) | Extracted correctly |
| 11 | `[link](...)` in same doc | Not returned by `inline_html_links()` |
| 12 | Multi-line anchor | Extracted correctly |

### Fallback path tests

- Unit-test `fallback_extract_links` and `fallback_extract_images` directly with simple inputs.

## Phase 7: Documentation Updates

1. **`darkmatter/docs/structs/Markdown.md`**
   - Add entries for `has_inline_html()`, `inline_html_links()`, `inline_html_image_references()`.
   - Clarify that `links()` and `image_references()` remain Markdown-syntax only.

2. **`darkmatter/docs/structs/Link.md`**
   - Note that HTML `<a>` tags can now be extracted from `Markdown` via `inline_html_links()`.

3. **`darkmatter/docs/structs/ImageRef.md`**
   - Note that HTML `<img>` tags can now be extracted from `Markdown` via `inline_html_image_references()`.

## Execution Order and Dependencies

```
Phase 1 ─── Phase 2 ─── Phase 3a ─── Phase 3b ─── Phase 3c
                              │            │
                              ▼            ▼
                          Phase 3d ─── Phase 3e
                              │            │
                              ▼            ▼
                          Phase 3f ─── Phase 3g
                              │
                              ▼
                          Phase 4 ──── Phase 5 ──── Phase 6 ──── Phase 7
```

- **Phase 1 and 2** are independent of each other and can be done in parallel.
- **Phase 3a–3g** are sequential within the module but 3e (images) and 3f (links) can be developed in parallel once 3d is complete.
- **Phase 4** depends on 3e/3f for the function signatures.
- **Phase 5** depends on Phase 3 + 4.
- **Phase 6** should be written alongside each phase (TDD).
- **Phase 7** is done last once the API is stable.

## Files Changed

| File | Change |
|------|--------|
| `darkmatter/lib/src/markdown/output/ast.rs` | Add `pub(crate) fn parse_mdast()`, refactor `as_ast()` |
| `darkmatter/lib/src/render/link.rs` | Case-insensitive tag matching in `parse_html_link()` |
| `darkmatter/lib/src/markdown/inline_html.rs` | **New file** — extractor, scanner, fallback, tests |
| `darkmatter/lib/src/markdown/mod.rs` | Add `mod inline_html;` and 3 public methods |
| `darkmatter/docs/structs/Markdown.md` | Document new methods |
| `darkmatter/docs/structs/Link.md` | Note HTML anchor extraction |
| `darkmatter/docs/structs/ImageRef.md` | Note HTML image extraction |

## Invariants

- `links()` behavior is unchanged.
- `image_references()` behavior is unchanged.
- `as_ast()` behavior is unchanged (same parse options via shared helper).
- `as_html()` still escapes raw HTML — no rendering behavior change.
- All new methods are non-fallible; malformed inputs are silently skipped.
- Results are always in source order, never deduplicated.