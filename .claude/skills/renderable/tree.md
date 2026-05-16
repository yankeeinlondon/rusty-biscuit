---
description: The canonical render tree in the renderable library — RenderNode, Document, TreeRenderable, and the Markdown/Browser tree renderers.
---

# Tree Module

The `renderable::tree` module is the library's central model: a single, owned,
target-agnostic **render tree** that sits between content sources and render
targets. Content sources fold their input into a tree; target renderers fold
the tree into concrete output.

## Core Types

- **`RenderNode`** — a node with a `kind` (`NodeKind`), a `SourceSpan`, and
  `NodeAttrs` (id / classes / data attributes).
- **`NodeKind`** — 25 block and inline variants: `Root`, `Heading`,
  `Paragraph`, `BlockQuote`, `List`, `ListItem`, `Code`, `ThematicBreak`,
  `Table`, `TableRow`, `TableCell`, `FootnoteDefinition`, `Text`, `Emphasis`,
  `Strong`, `Delete`, `Span`, `InlineCode`, `Link`, `Image`,
  `FootnoteReference`, `SoftBreak`, `HardBreak`, `Html`, `Unsupported`.
- **`Document`** — a `RenderNode` tree plus a `SourceRegistry` of origins and
  `DocumentMetadata` (`Frontmatter`).
- **Origin tracking** — `SourceSpan`, `SourceLocation`, `SourceId`,
  `SourceRegistry`, `SourceDescriptor`, `Provenance`.
- **Helpers** — `HeadingDepth` (validated 1–6), `ColumnAlign`.

All tree types are `serde`-serializable, so a tree can be persisted or
inspected as JSON.

## TreeRenderable

```rust
pub trait TreeRenderable {
    /// Renders the component to a canonical render tree.
    fn render_tree(&self) -> RenderNode;
}
```

`TreeRenderable` is the entry point to the render-tree pipeline and is
re-exported from `renderable::prelude`. A component that implements it gains
multi-target rendering: any tree renderer can fold the tree it produces.

> `TreeRenderable` **replaces** the removed placeholder `AstRenderable` trait;
> there is no longer an "AST" render target.

## Diagnostics and Validation

- **`Diagnostic`** / **`Severity`** — non-fatal issues collected during a fold
  or render.
- **`Rendered<T>`** — a render result carrying output plus diagnostics.
- **`RenderStrictness`** — `Strict` / `Warn` / `Lossy`.
- **`RenderError`** — a fatal render failure.
- **`validate`** / **`ensure_valid`** — structural-invariant checks.

## Tree Renderers

- **Markdown** — `render_markdown_node` / `render_markdown_document` with
  `MarkdownRenderOptions` and `MarkdownDialect` (`Markdown` / `MarkdownPlus`).
- **Browser** — `render_browser_node` / `render_browser_document` with
  `BrowserRenderOptions` and `RawHtmlPolicy` (`Allow` / `Escape` / `Reject`).
- **Terminal** — `render_terminal_node` / `render_terminal_document` with
  `TerminalRenderOptions` / `TerminalRenderContext`, plus the
  `TreeComponent<T>` adapter. These live in the `biscuit-terminal` crate.

## Darkmatter Fold (experimental, internal)

`darkmatter::markdown::render_tree` folds a `pulldown-cmark` 0.13 event stream
into a `Document` via `fold_markdown_to_document`. It is **experimental and
internal** — it does not change darkmatter's public `as_html` / `for_terminal`
renderers. `==mark==` / dim inline styles and HR-with-attributes folding are
intentionally deferred.
