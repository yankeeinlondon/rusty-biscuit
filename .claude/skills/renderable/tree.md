---
description: The canonical render tree in the renderable library — RenderNode, Document, TreeRenderable, component projection, render hints, and the Markdown/Browser/Terminal tree renderers.
---

# Tree Module

The `renderable::tree` module is the library's central model: a single, owned,
target-agnostic **render tree** that sits between content sources and render
targets. Content sources fold their input into a tree; target renderers fold
the tree into concrete output.

## Core Types

- **`RenderNode`** — a node with a `kind` (`NodeKind`), a `SourceSpan`, and
  `NodeAttrs` (id / classes / `data` map).
- **`NodeKind`** — 26 block and inline variants: `Root`, `Heading`,
  `Section`, `Paragraph`, `BlockQuote`, `List`, `ListItem`, `Code`,
  `ThematicBreak`, `Table`, `TableRow`, `TableCell`, `FootnoteDefinition`,
  `Text`, `Emphasis`, `Strong`, `Delete`, `Span`, `InlineCode`, `Link`,
  `Image`, `FootnoteReference`, `SoftBreak`, `HardBreak`, `Html`,
  `Unsupported`.
- **`Document`** — a `RenderNode` tree plus a `SourceRegistry` of origins and
  `DocumentMetadata` (`Frontmatter`).
- **Origin tracking** — `SourceSpan`, `SourceLocation`, `SourceId`,
  `SourceRegistry`, `SourceDescriptor`, `Provenance`. Runtime-projected nodes
  use `SourceSpan::synthetic()`.
- **Helpers** — `HeadingDepth` (validated 1–6), `ColumnAlign`.

All tree types are `serde`-serializable, so a tree can be persisted or
inspected as JSON.

### `NodeKind::Section`

`Section { depth, heading, children }` is a heading paired with its body.
`heading` holds phrasing content; `children` holds block-level content.
Distinct from `Heading`, which is just the heading line. Validation requires
`Section.heading` to be phrasing-only and `Section.children` to be
block-level; a `Section` may nest inside `Root`, `BlockQuote`, `Section`, or
`ListItem`.

## TreeRenderable

```rust
pub trait TreeRenderable {
    /// Renders the component to a canonical render tree.
    fn render_tree(&self) -> RenderNode;

    /// Optional layout hints for tree rendering. Defaults to `None`.
    fn tree_layout_hints(&self) -> Option<LayoutHints> { None }
}
```

`TreeRenderable` is the entry point to the render-tree pipeline and is
re-exported from `renderable::prelude`. A component that implements it gains
multi-target rendering: any tree renderer can fold the tree it produces.

> `TreeRenderable` **replaces** the removed placeholder `AstRenderable` trait;
> there is no longer an "AST" render target.

## Component Projection

Two complementary entry points produce trees from existing components:

- **`TreeRenderable::render_tree()`** — a component authored tree-first.
- **`TerminalRenderable::render_tree_node(&self) -> Option<RenderNode>`** — an
  existing terminal component opts into tree rendering by overriding this
  (default `None`). Lives in `biscuit-terminal`. The Group 1 components
  (`Section`, `OrderedList`, `UnorderedList`, `Progress`, `TwoColumn`,
  `Table`, darkmatter's `YamlBlock`) all implement it.

The **projection layer** (`biscuit-terminal`'s `render_tree::projection`)
converts `RenderableTerminalContent` into tree nodes:
`to_tree_nodes(&self, &mut TreeProjectionContext) -> ProjectionResult`.
`TreeProjectionContext` carries `strictness`, `max_depth`, `current_depth`
(recursion-guarded); `ProjectionResult` carries `nodes` + `diagnostics`.

## Render Hints

Presentational hints ride on `NodeAttrs.data` under namespaced keys via
`HintNamespace` (`LAYOUT`, `LIST`, `TABLE`, `CODE`, `TERMINAL`,
`WIDGET_PROGRESS`, `WIDGET_COLUMNS`). `NodeAttrs::set_hint` / `get_hint` /
`remove_hint` are the low-level accessors; typed helper structs wrap them:

- **`LayoutHints`** — margins.
- **`ListRenderHints`** — bullet, hanging indent, child indent.
- **`CodeRenderHints`** — header row, language label, highlight flag.
- **`ProgressHints`** — value, bar width, glyphs, brackets.
- **`ColumnsHints`** / **`ColumnWidthKind`** — gap, left width, `left_count`,
  stack threshold (two-column layout, carried on a `BlockQuote` node).
- **`TableColumnHints`** / **`TableCellHints`** / **`TableTerminalHints`** —
  per-column width/conditional/drop metadata, per-cell typed value and
  alignment, terminal striping and cursor preference.

`HintNamespace` is a transparent `&'static str` wrapper, so other crates can
define their own namespace roots.

## CodeRenderer

`CodeRenderer` is an optional hook for custom code-block rendering (e.g.
syntax highlighting). The terminal and browser tree renderers consult it for
`NodeKind::Code` nodes; `Some` output is used verbatim, `None` falls back to
the built-in plain renderer.

```rust
pub trait CodeRenderer {
    fn render_terminal_code(&self, lang: Option<&str>, value: &str,
                            attrs: &NodeAttrs, width: u32) -> Option<String>;
    fn render_browser_code(&self, lang: Option<&str>, value: &str,
                           attrs: &NodeAttrs) -> Option<BrowserFragment<Ready>>;
}
```

> `render_terminal_code` takes a primitive `width: u32` rather than
> `TerminalRenderContext` because `renderable` cannot depend on
> `biscuit-terminal`. Widening it to carry color depth / color mode is
> specified in `renderable/features/2026-05-16-color-decisions/` as a
> precursor to the darkmatter tree migration.

## Diagnostics and Validation

- **`Diagnostic`** / **`DiagnosticKind`** / **`Severity`** — non-fatal issues
  collected during a fold, projection, or render.
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
  `TerminalRenderOptions` / `TerminalRenderContext`. Heading/section, lists,
  and tables render **natively** (no delegation back to bespoke components).
  Adapters: `TreeComponent<T>` (terminal) and `BrowserTreeComponent<T>`
  (browser). These live in the `biscuit-terminal` crate — see the
  biscuit-terminal skill's *Render Tree* topic.

## Darkmatter Fold (experimental, internal)

`darkmatter::markdown::render_tree` folds a `pulldown-cmark` 0.13 event stream
into a `Document` via `fold_markdown_to_document`. It is **experimental and
internal** — it does not change darkmatter's public `as_html` / `for_terminal`
renderers. A full darkmatter migration onto the tree renderer is upcoming
(`renderable/features/2026-05-16-color-decisions/` is its first precursor).
