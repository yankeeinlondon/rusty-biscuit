---
prompt: Your task is to review the Markdown docs at @darkmatter/docs/structs/Markdown.md as well as the source code for context and then read the specification file at @./spec.md and build a high definition technical design for this implementation. Your technical design should be saved to the body of this document as idiomatic Markdown content.
last_updated: 2026-03-21
---

# Inline HTML Tech Design

This document defines the implementation-ready technical design for the `inline-html` feature in Darkmatter. It is derived from:

- `darkmatter/features/2026-03-21. inline-html/spec.md`
- `darkmatter/docs/structs/Markdown.md`
- the current `Markdown` implementation in `darkmatter/lib/src/markdown/mod.rs`
- the current MDAST export path in `darkmatter/lib/src/markdown/output/ast.rs`
- the existing HTML parsing support in `darkmatter/lib/src/render/link.rs` and `darkmatter/lib/src/render/image_ref.rs`

The design goal is to add HTML-aware companion APIs to `Markdown` without changing the behavior of the existing Markdown-native `links()` and `image_references()` methods.

## Purpose

Darkmatter already exposes two extraction helpers on `Markdown`:

- `links()` for Markdown links such as `[Docs](https://example.com)`
- `image_references()` for Markdown images such as `![Alt](./img.png)`

The spec asks for the HTML equivalents:

- `inline_html_links()` for anchor tags such as `<a href="https://example.com">Docs</a>`
- `inline_html_image_references()` for image tags such as `<img src="./img.png" alt="Alt" />`
- `has_inline_html()` as a cheap fast-path so callers can avoid the heavier extraction path when the document clearly contains no raw HTML

The implementation should fit the current `Markdown` architecture:

1. `Markdown` remains the central document facade
2. `Link` and `ImageRef` remain the typed output types
3. parsing should reuse existing crates and existing typed HTML parsers where possible
4. extraction stays best-effort and non-fallible, matching the current `links()` and `image_references()` contract

## Goals

1. Add `Markdown::has_inline_html() -> bool`.
2. Add `Markdown::inline_html_links() -> Vec<Link>`.
3. Add `Markdown::inline_html_image_references() -> Vec<ImageRef>`.
4. Preserve document order in returned results.
5. Reuse the existing `Link::try_from(&str)` and `ImageRef::try_from(&str)` HTML parsing logic instead of introducing a second typed parser stack.
6. Avoid regex-only parsing for HTML extraction.
7. Keep the existing `links()` and `image_references()` behavior unchanged.
8. Keep renderer safety unchanged: this feature extracts metadata, it does not make raw HTML renderable.

## Non-Goals

1. Changing `links()` so it also returns HTML anchors.
2. Changing `image_references()` so it also returns HTML `<img>` tags.
3. Adding a general DOM parser dependency for arbitrary HTML traversal.
4. Making raw HTML pass through `as_html()` or `as_terminal()` unescaped.
5. Parsing arbitrary HTML elements beyond the two requested resource types:
   - `<a ...>...</a>`
   - `<img ...>`
6. Adding memoization or caching to `Markdown` itself in v1.

## Current Baseline

The existing implementation gives us most of the pieces already:

1. `Markdown::links()` in `darkmatter/lib/src/markdown/mod.rs` uses `pulldown-cmark` event scanning to extract Markdown-native links.
2. `Markdown::image_references()` in the same file does the equivalent for Markdown-native images.
3. `Markdown::as_ast()` in `darkmatter/lib/src/markdown/output/ast.rs` already converts content to MDAST using `markdown::ParseOptions::gfm()`.
4. `Link::try_from(&str)` already supports parsing a full HTML anchor fragment.
5. `ImageRef::try_from(&str)` already supports parsing a full HTML image fragment.
6. `as_html()` currently escapes raw HTML events in `darkmatter/lib/src/markdown/output/html.rs`, which is the correct current safety posture.

There are two important observations from the current parser behavior:

1. Neither `pulldown-cmark` nor the `markdown` crate converts raw HTML into a DOM-like tree for us.
2. They do, however, surface raw HTML as discrete HTML fragments in document order.

For example, a source string such as:

```md
Before <a href="https://example.com">Click</a> and <img src="./x.png" alt="X" />
```

is surfaced as separate pieces:

- opening `<a ...>` HTML fragment
- text node `Click`
- closing `</a>` HTML fragment
- standalone `<img ...>` HTML fragment

That means the extraction problem is not HTML parsing in the large. It is:

1. finding the correct raw HTML fragment boundaries in Markdown content
2. slicing the exact original source for each candidate
3. handing those slices to the existing typed parsers

## Primary Recommendation

Use a hybrid design:

1. `has_inline_html()` should use a lightweight source scanner with no AST allocation.
2. `inline_html_links()` and `inline_html_image_references()` should use the `markdown` crate's MDAST as the primary extraction path.
3. The extractor should use MDAST source positions to slice exact HTML fragments out of the original markdown source.
4. Those slices should then be parsed by the existing `Link` and `ImageRef` HTML parsers.
5. If MDAST parsing fails, the non-fallible public APIs should fall back to a `pulldown-cmark` event scanner rather than returning an error.

This is the best fit for the current codebase.

### Why not regex-only parsing

Regex-only extraction is the wrong tool here because it would have to solve all of the following at once:

1. skip code spans and fenced code blocks
2. handle quoted attribute values containing `>`
3. distinguish HTML tags from autolinks such as `<https://example.com>`
4. reconstruct multi-fragment anchors correctly

That is exactly the kind of parsing drift Darkmatter should avoid.

### Why MDAST is the right primary extractor

The spec explicitly asked for the `markdown` crate and MDAST to be considered first. That is the right default here, but for a specific reason:

MDAST gives us stable byte offsets.

That matters because it lets us take the exact source slice from the original markdown string instead of trying to reserialize an anchor or image candidate from parsed tokens. For `<img>`, this is convenient. For `<a>...</a>`, it is the difference between a brittle reconstruction pass and a straightforward span-based extraction pass.

### Why keep `Link` and `ImageRef` as the typed parsers

Darkmatter already knows how to parse:

- HTML anchor attributes into `Link`
- HTML image attributes into `ImageRef`

Those parsers already understand existing metadata conventions:

- classes
- inline styles
- titles
- typed image attributes such as `loading`, `decoding`, `fetchpriority`, `sizes`, and `srcset`
- `data-*` metadata

Reusing them keeps behavior aligned across the library and avoids parallel parsing logic.

## Public API

Add these methods to `impl Markdown` in `darkmatter/lib/src/markdown/mod.rs`:

```rust
impl Markdown {
    pub fn has_inline_html(&self) -> bool;

    pub fn inline_html_links(&self) -> Vec<Link>;

    pub fn inline_html_image_references(&self) -> Vec<ImageRef>;
}
```

Contract:

1. These methods inspect `self.content()` only, never frontmatter.
2. They are complementary to the existing Markdown-native methods.
3. They return results in source order.
4. They do not deduplicate.
5. Malformed candidates are skipped rather than returned as errors.

No new public error type should be added. This keeps the API consistent with `links()` and `image_references()`.

## Naming Clarification

The method names use `inline_html_*` to distinguish the source syntax from Markdown-native links and images.

They should still accept HTML tags that happen to occupy an entire line by themselves. In other words, this naming is about syntax family, not paragraph placement.

That means all of the following should be eligible:

```md
Before <a href="https://example.com">Docs</a> after
```

```md
<img src="./diagram.png" alt="Diagram" />
```

```md
<a href="https://example.com">
  Multi-line link
</a>
```

## Proposed Internal Layout

Recommended file changes:

```txt
darkmatter/lib/src/markdown/
├── inline_html.rs         # new internal extractor module
├── mod.rs                 # public Markdown methods
└── output/ast.rs          # shared crate-private mdast helper
```

Additional targeted hardening:

```txt
darkmatter/lib/src/render/link.rs
```

Recommended responsibilities:

- `markdown/mod.rs`
    - public method surface only
- `markdown/inline_html.rs`
    - fast HTML detection
    - AST traversal
    - fallback pulldown traversal
    - span slicing
    - link display normalization
- `markdown/output/ast.rs`
    - shared `ParseOptions::gfm()` helper reused by `as_ast()` and the inline HTML extractor
- `render/link.rs`
    - make HTML tag-name matching for `<a>` case-insensitive and more tag-aware

## Shared AST Helper

Today `darkmatter/lib/src/markdown/output/ast.rs` owns the `markdown::to_mdast(..., ParseOptions::gfm())` call directly inside `as_ast()`.

That should be factored into a crate-private helper such as:

```rust
pub(crate) fn parse_mdast(content: &str) -> MarkdownResult<markdown::mdast::Node>
```

Then:

- `Markdown::as_ast()` continues to call that helper
- `inline_html.rs` reuses the same helper

This avoids parser-option drift between the public AST export path and the new extractor.

## Detection Design

### `has_inline_html()`

`has_inline_html()` should be fast, allocation-light, and conservative.

Recommended behavior:

1. scan the raw content once
2. skip fenced code blocks
3. skip inline code spans when practical
4. when a `<` is encountered outside code regions, test whether it starts a plausible HTML construct
5. return `true` on the first positive match

Recommended matching rule:

Treat a candidate as HTML when it matches one of:

1. opening or closing tag syntax with an ASCII tag name:
   - `<a ...>`
   - `</a>`
   - `<img ...>`
   - `<div>`
2. comments or declarations:
   - `<!-- ... -->`
   - `<!doctype html>`
   - `<?xml ...?>`

Do not treat these as HTML:

1. autolinks such as `<https://example.com>`
2. angle-bracketed email links
3. plain mathematical comparisons like `x < y`

The detector does not need to be a full Markdown parser. The important rule is:

- false positives are acceptable
- false negatives are not

That makes it safe as a gate in front of the heavier extraction path.

### Why not reuse compose parsing utilities

`compose::parse_utils::find_code_regions()` already exists, but this feature lives on the core `Markdown` type rather than the compose pipeline.

The design should not make `Markdown` depend on compose internals just to answer a lightweight detection query.

If shared code-region scanning becomes necessary later, it should be extracted into a neutral markdown utility module rather than reaching "up" into compose.

## Extraction Design

The primary extractor should:

1. parse the markdown body to MDAST
2. flatten the relevant nodes into document order with byte offsets
3. classify HTML fragments
4. build exact source spans for `<a>` and `<img>` candidates
5. parse those spans into `Link` and `ImageRef`

Recommended internal types:

```rust
enum HtmlFragmentKind {
    AnchorOpen,
    AnchorClose,
    Image,
    Other,
}

struct HtmlFragment<'a> {
    span: std::ops::Range<usize>,
    value: &'a str,
    kind: HtmlFragmentKind,
}

struct InlineHtmlExtraction {
    links: Vec<Link>,
    images: Vec<ImageRef>,
}
```

The extractor should remain private. The public surface stays the three `Markdown` methods only.

### HTML fragment classification

Classification must be tag-aware and case-insensitive.

Recommended rules:

1. `<img ...>` and `<IMG ...>` classify as `Image`
2. `<a ...>` and `<A ...>` classify as `AnchorOpen`
3. `</a>` and `</A>` classify as `AnchorClose`
4. everything else is `Other`

The classifier only needs to understand tag name boundaries and whether the fragment is a closing tag. It does not need to parse attributes.

## Image Extraction Algorithm

`<img>` extraction is the simple path.

Algorithm:

1. if `has_inline_html()` is false, return `Vec::new()`
2. build MDAST
3. traverse nodes in document order
4. for each HTML fragment classified as `Image`:
   - slice the exact source text using the node's byte range
   - call `ImageRef::try_from(fragment)`
   - if parsing succeeds, push the result
   - if parsing fails, skip and optionally `trace!`

This reuses all existing image attribute logic, including:

- plain `src`
- `srcset`
- `alt`
- `title`
- typed loading and fetch attributes
- width and height values
- `data-*`

## Link Extraction Algorithm

Anchor extraction is the only non-trivial part because raw HTML anchors are represented as multiple fragments.

Recommended algorithm:

1. if `has_inline_html()` is false, return `Vec::new()`
2. build MDAST
3. flatten relevant inline nodes into a source-ordered sequence with byte ranges
4. scan that sequence with a small state machine

State machine:

1. on `AnchorOpen`, record the opening span
2. continue advancing until the matching `AnchorClose`
3. when found, create a full source slice from:
   - `open_span.start`
   - to `close_span.end`
4. pass that full slice to `Link::try_from`
5. normalize the extracted display text
6. push the resulting `Link`

Malformed cases:

1. open without close: skip
2. close without open: ignore
3. nested anchors: treat as malformed and skip the nested region

Nested anchors are invalid HTML anyway, so best-effort skip behavior is acceptable and easier to reason about than trying to salvage a broken tree.

## Link Display Normalization

`Link::try_from("<a ...>...</a>")` already parses anchor metadata correctly, but its current HTML branch treats the inner HTML literally.

That is good enough for:

```html
<a href="https://example.com">Click</a>
```

but it is not ideal for:

```html
<a href="https://example.com"><strong>Important</strong> docs</a>
```

because the caller wants the display text, not raw nested HTML tags.

The extractor should therefore normalize the parsed display before returning the final `Link`.

Recommended helper:

```rust
fn normalize_inline_html_link_display(raw: &str) -> String
```

Behavior:

1. decode HTML entities
2. translate `<br>` and `<br/>` into newlines
3. translate `<code>` / `</code>` into backticks so code-like display remains recognizable
4. strip all other HTML tags from the display
5. trim only the whitespace introduced by tag removal when needed, not the author's meaningful internal spacing

Recommended construction flow:

1. parse the full fragment with `Link::try_from`
2. compute normalized display text from `parsed.display()`
3. return `parsed.with_display(normalized_display)`

This keeps all existing metadata parsing in one place while fixing the only part the current HTML parser does not model well for extraction.

## Why MDAST Beats `pulldown-cmark` For The Primary Path

`pulldown-cmark` can see raw HTML, but it does not give us the exact same extraction ergonomics:

1. `<a ...>` and `</a>` arrive as separate events
2. reconstructing the exact full fragment requires manual buffering
3. there are no source offsets available on the normal event stream

MDAST gives us:

1. the same raw HTML visibility
2. stable source positions
3. the ability to slice the exact original substring for typed parsing

That is why the AST path should be primary even though both parsers are available.

## Fallback Path

The public API is intentionally non-fallible, so MDAST parse failure should not bubble up.

Recommended fallback:

1. run a lightweight `pulldown-cmark` event scan
2. reconstruct anchors from:
   - `InlineHtml("<a ...>")`
   - intervening text and soft/hard breaks
   - `InlineHtml("</a>")`
3. parse `<img ...>` from a single `InlineHtml(...)` event

This fallback does not need to be as elegant as the primary path. Its job is resilience, not feature expansion.

If both primary and fallback parsing fail for a candidate, the candidate should be skipped.

## `render/link.rs` Hardening

`ImageRef` HTML parsing is already tag-name tolerant enough for this feature, but `Link` HTML parsing is currently too literal.

Before this feature ships, `darkmatter/lib/src/render/link.rs` should be hardened so HTML anchor parsing:

1. matches tag names case-insensitively
2. recognizes `<a>` even when whitespace after the tag name is not a plain single space
3. recognizes `</a>` case-insensitively

This should remain a targeted fix, not a rewrite.

The extractor should not lower-case source fragments before handing them to `Link::try_from`. The parser should become robust enough to accept normal HTML spelling variations directly.

## Semantics

The new APIs should follow these semantic rules:

1. source order is preserved
2. no deduplication is performed
3. malformed HTML candidates are skipped
4. HTML inside fenced code blocks is ignored
5. HTML inside inline code spans is ignored by `has_inline_html()` when practical and always ignored by the AST-based extractor
6. Markdown links and Markdown images are not returned by the new HTML-specific methods
7. raw HTML comments and declarations may cause `has_inline_html()` to return `true`, but they do not produce `Link` or `ImageRef` outputs

## No Rendering Behavior Change

This feature is extraction-only.

It must not change the current output safety model in `darkmatter/lib/src/markdown/output/html.rs`, which intentionally escapes:

- `Event::Html(...)`
- `Event::InlineHtml(...)`

That current behavior is correct and should remain in place. Extracting typed metadata from raw HTML does not imply that raw HTML is now trusted for rendering.

## Testing Strategy

Add focused unit tests alongside the new extractor module and any small parser hardening.

### `has_inline_html()` tests

1. plain markdown returns `false`
2. Markdown links and images return `false`
3. `<a href="...">` returns `true`
4. `<img ...>` returns `true`
5. `<https://example.com>` returns `false`
6. fenced code block containing `<a>` returns `false`
7. inline code span containing `<img>` returns `false` when the lightweight scanner can determine that safely
8. HTML comment returns `true`

### `inline_html_image_references()` tests

1. extracts a basic `<img src="./a.png" alt="A" />`
2. extracts multiple images in order
3. preserves `title`
4. preserves `srcset`
5. preserves typed attributes such as `loading="lazy"`
6. ignores malformed `<img>` fragments
7. ignores HTML-looking text inside code spans and fenced code blocks
8. handles uppercase tag names

### `inline_html_links()` tests

1. extracts a basic `<a href="https://example.com">Click</a>`
2. extracts multiple anchors in order
3. preserves `class`, `style`, `target`, `title`, `prompt`, and `data-*`
4. normalizes nested display formatting:
   - `<strong>`
   - `<em>`
   - `<code>`
   - `<br>`
5. ignores malformed or unterminated anchors
6. ignores HTML-looking text inside code spans and fenced code blocks
7. handles uppercase `<A>` and `</A>`
8. does not return Markdown-native `[link](...)` entries

### Fallback-path tests

If the fallback is implemented as a separable helper, unit-test it directly instead of trying to induce an MDAST parser failure from public API tests.

## Documentation Updates

When the implementation lands, the same change should update:

1. `darkmatter/docs/structs/Markdown.md`
   - add the three new methods
   - clarify that `links()` and `image_references()` remain Markdown-only
2. `darkmatter/docs/structs/Link.md`
   - mention that HTML anchors can now be extracted from `Markdown`
3. `darkmatter/docs/structs/ImageRef.md`
   - mention that HTML `<img>` tags can now be extracted from `Markdown`

This keeps the documentation in sync with the API surface and avoids drift.

## Blast Radius

The expected implementation blast radius is:

- `darkmatter/lib/src/markdown/mod.rs`
- `darkmatter/lib/src/markdown/inline_html.rs`
- `darkmatter/lib/src/markdown/output/ast.rs`
- `darkmatter/lib/src/render/link.rs`
- `darkmatter/docs/structs/Markdown.md`
- optionally:
    - `darkmatter/docs/structs/Link.md`
    - `darkmatter/docs/structs/ImageRef.md`

## Implementation Sequence

Recommended sequence:

1. factor a shared crate-private MDAST parse helper out of `output/ast.rs`
2. add `markdown/inline_html.rs`
3. add `has_inline_html()`, `inline_html_links()`, and `inline_html_image_references()` to `Markdown`
4. harden HTML tag-name parsing in `render/link.rs`
5. add focused unit tests
6. update the struct documentation

## Final Recommendation

Implement the feature as a non-fallible, source-order-preserving companion API on `Markdown`.

The key design choice is:

- use a cheap scanner for `has_inline_html()`
- use MDAST plus exact source spans for primary extraction
- reuse `Link` and `ImageRef` as the canonical typed parsers

That approach fits the current codebase cleanly, keeps the public API small, avoids HTML parsing duplication, and preserves Darkmatter's existing safety and separation of concerns.
