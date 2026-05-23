# Render-Tree Fold (Experimental / Internal)

> **Status:** experimental, internal. The render-tree fold does **not** change
> any public darkmatter behavior. The existing `as_html` / `for_terminal`
> renderers and the [rendering pipeline](./darkmatter-rendering-pipeline.md)
> are unchanged. Nothing in this document is part of darkmatter's stable API.

## What it is

The [`renderable`](../../renderable/README.md) crate defines a canonical,
owned **render tree** — a single target-agnostic representation
(`renderable::tree::Document` / `RenderNode`) that sits between content
sources and render targets.

The `darkmatter::markdown::render_tree` module is darkmatter's home for the
**events → tree fold**: the conversion of a
[`pulldown-cmark`](https://crates.io/crates/pulldown-cmark) 0.13 `Event`
stream into a `renderable::tree::Document`.

It contains two pieces:

- **`inventory`** — a verified, compile-checked catalog of every
  `pulldown-cmark` 0.13 `Event` / `Tag` / `TagEnd` variant and the disposition
  the fold applies to each. An exhaustive-match test fails to build if
  `pulldown-cmark` changes its enums.
- **`fold_markdown_to_document(source, input) -> (Document, Vec<Diagnostic>)`** —
  the fold entry point. It walks the event stream and builds a `Document`,
  returning any non-fatal diagnostics alongside it.

## Coverage

The fold covers the common Milestone 1 event set — headings, paragraphs,
block quotes, lists, code blocks, thematic breaks, tables, inline emphasis /
strong / strikethrough, links, images, and inline code — plus footnotes,
grouped raw-HTML blocks, and native superscript / subscript.

## Deferred work

Two darkmatter inline conveniences are **intentionally deferred** to a
follow-up feature:

- `==mark==` and dim inline styles.
- Horizontal rules with attribute blocks.

Both are produced by darkmatter's `InlineStyleProcessor` / `RuleProcessor` —
iterator adapters that discard source byte offsets. Folding them without
losing each node's `SourceLocation` needs a separate design decision; see the
`inventory` module documentation for the full rationale. Frontmatter wiring is
likewise deferred.
