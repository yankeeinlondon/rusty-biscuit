---
prompt: |-
    Review the specification file at @./spec.md and the technical design at @./tech-design.md and then provide a detailed review for the implementation.

    - look for functionality which was missed out on in the implementation but documented in the specification or tech design document
    - make sure all new functionality has good test coverage
    - try to identify opportunities to make the current implementation more idiomatic or ergonomic
    - if there are any low risk performance improvements that could be made without impacting the intended functionality suggest these

    Write all your review suggestions to the body of this document as a standard-based and idiomatic Markdown document.
last_updated: 2026-03-21
---

# Inline HTML Implementation Review

This review compares the implemented feature against the requirements in [spec.md](./spec.md) and [tech-design.md](./tech-design.md), with a focus on missed functionality, test coverage, idiomatic Rust usage, and low-risk performance opportunities.

At a high level, the public API shape, parser hardening, documentation updates, and renderer-safety posture all line up well with the design. The main remaining functional gap is in how anchor display text is normalized, and the main remaining quality gap is extractor-level regression coverage for some metadata paths the design explicitly called out.

## Findings

### 1. High: `inline_html_links()` does not yet preserve visible display semantics when the anchor body contains Markdown formatting

The spec says the new HTML helpers should have the same functional goals as the existing Markdown-native extractors, and the design treats `inline_html_links()` as a companion to `links()`. The current primary extraction path in [darkmatter/lib/src/markdown/inline_html.rs](../../lib/src/markdown/inline_html.rs) at lines 93-153 parses the full `<a>...</a>` fragment into a `Link`, then rewrites the display using `normalize_inline_html_link_display()` at lines 237-264. That normalizer only strips HTML tags, decodes entities, and trims the result.

That means anchor bodies such as:

```html
<a href="https://example.com">**Bold** and `code`</a>
```

will keep the raw Markdown markers in the returned display text instead of producing the visible text semantics that `Markdown::links()` already provides for Markdown-native links. The unconditional `.trim()` at line 264 is also a behavior difference from `links()`, because it can remove meaningful author-supplied leading or trailing whitespace.

The fallback path in the same file at lines 267-323 is a useful clue here: it rebuilds display text from parsed `pulldown-cmark` events (`Text`, `Code`, breaks) rather than doing post-hoc string stripping. The primary and fallback paths therefore do not currently agree on the display contract.

Recommended change:

- Derive display text from the parsed content between the opening and closing anchor spans, rather than from `Link::display()` plus string stripping.
- If the span-based architecture should stay in place, reparse the inner anchor content and collect visible inline events the same way `Markdown::links()` does, then apply HTML-tag normalization only to raw HTML fragments nested inside the anchor body.

Recommended regression tests:

- `<a href="https://example.com">**Bold**</a>` should return `Bold`
- `<a href="https://example.com">`code`</a>` should return `` `code` ``
- `<a href="https://example.com"> leading and trailing </a>` should preserve the intended display contract explicitly

### 2. Medium: extractor-level coverage is still thin for some metadata paths the design explicitly promised

The design calls out preservation of `prompt` and richer typed image metadata through the new extraction APIs. The current extractor tests in [darkmatter/lib/src/markdown/inline_html.rs](../../lib/src/markdown/inline_html.rs) at lines 616-680 cover a good baseline, but they stop short of a few important integration paths:

- The link extractor tests do not verify `data-prompt` even though [darkmatter/lib/src/render/link.rs](../../lib/src/render/link.rs) at lines 814-825 maps it into `Link::prompt()`.
- The image extractor tests do not exercise `decoding`, `fetchpriority`, `sizes`, `width`, `height`, or image `data-*`, even though those are part of the typed `ImageRef` surface in [darkmatter/lib/src/render/image_ref.rs](../../lib/src/render/image_ref.rs) at lines 382-418.
- There is no regression test proving the new methods inspect markdown content only and do not accidentally pick up HTML-looking text from frontmatter.

None of those look like implementation bugs today, but they are exactly the kind of integration behavior that tends to regress later because the parsing logic is split between `Markdown`, the inline HTML extractor, and the typed `Link` / `ImageRef` parsers.

Recommended test additions:

- `inline_html_links()` round-trip for `data-prompt`
- `inline_html_image_references()` regression covering `fetchpriority`, `decoding`, `sizes`, `width`, `height`, and one `data-*` attribute
- a `Markdown` value with HTML-looking frontmatter and no HTML in the body, asserting all three new methods behave as content-only APIs

## Additional Suggestions

### Idiomatic and ergonomic improvements

- The current implementation already has the right internal split overall, but the display-normalization logic would be more idiomatic if it operated on parsed inline events instead of raw string surgery. That would make the primary extractor closer in behavior to the existing `links()` implementation and easier to reason about.
- If this area grows further, consider factoring the display-text extraction logic into a small shared helper used by both `Markdown::links()` and `inline_html_links()`. That would reduce semantic drift between the Markdown-native and HTML-native code paths.

### Low-risk performance improvements

- [darkmatter/lib/src/markdown/inline_html.rs](../../lib/src/markdown/inline_html.rs) lines 55-90 currently build a full `InlineHtmlExtraction` for both links and images on every call. As a result, `inline_html_links()` computes images it never returns, `inline_html_image_references()` computes links it never returns, and callers that invoke both public methods parse the same document twice. A small refactor to split fragment collection from result extraction would remove that wasted work without changing behavior.
- The tag classifiers in the same file at lines 171-190 and 471-484 allocate temporary `String`s just to compare tag names. Rewriting those comparisons to operate on borrowed slices or iterator windows would be a low-risk micro-optimization in a hot path.

## Validation

I ran `just test` from `darkmatter/`.

- The library test suite itself passed: `1487 passed`, `0 failed`, `2 ignored`.
- The overall `just test` command still exited non-zero during the doc-test phase with rustdoc `E0463` crate-resolution failures such as `can't find crate for biscuit_terminal`, `globset`, and `serde_json`.

That doc-test failure does not look specific to the inline HTML implementation, but it does mean the package-level test command is not currently green end-to-end.

## Overall Assessment

The feature is close to the design target. The public API, AST-based extraction approach, fallback strategy, parser hardening, and documentation updates all look directionally correct. I did not find any larger spec or tech-design misses beyond the anchor display-semantics issue above.

If the display normalization is tightened up and the missing extractor-level regressions are added, this implementation will be in much better shape for long-term maintenance.