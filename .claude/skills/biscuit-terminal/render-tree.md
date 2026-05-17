# Render Tree (terminal)

The `biscuit_terminal::render_tree` module is the **terminal renderer for the
canonical `renderable` render tree**. It folds a `renderable::tree::RenderNode`
(or a whole `Document`) into a terminal string, and bridges biscuit-terminal's
components into the tree pipeline.

See the `renderable` skill's *Tree Module* topic for the tree model itself
(`RenderNode`, `NodeKind`, `TreeRenderable`, render hints, `CodeRenderer`).

## Entry Points

- **`render_terminal_node(node, opts) -> Result<Rendered<String>, RenderError>`**
  — fold a node to a terminal string plus diagnostics.
- **`render_terminal_document(doc, opts)`** — the same for a whole `Document`.
- **`TreeComponent<T: TreeRenderable>`** — adapts any `TreeRenderable` into a
  `TerminalRenderable`, so a tree-first component composes with the bespoke
  component ecosystem.
- **`BrowserTreeComponent<T: TreeRenderable>`** — the browser-side equivalent;
  `new` / `with_strictness`, `render_html`, and implements `BrowserRenderable`.

## Options & Context

- **`TerminalRenderOptions`** — `new(&Terminal, RenderStrictness)`; carries the
  render `context`, strictness, and an optional `code_renderer`
  (`Option<Rc<dyn CodeRenderer>>`).
- **`TerminalRenderContext`** — the terminal-capability snapshot the renderer
  consults: `width`, `available_width`, `current_indent`, `color_depth`,
  `color_mode`, `hyperlinks`, `image_support`, `supports_unicode`, `layout`,
  `terminal`, `active_layout`. Built via `from_terminal` / `fallback`.
  Fork helpers — `for_child(indent_delta, width_delta)`, `with_width`,
  `with_layout` — narrow width and accumulate indent for nested content.
  (`active_layout` is currently a carried-but-unread field; the renderer
  applies layout from `NodeAttrs::layout()` directly — see *Layout* below.)

## Strictness model

The node is validated first; an error-severity finding is an immediate
`RenderError::InvalidTree`. Warning-severity findings (including `Unsupported`
nodes) escalate to an error under `Strict`, fold into `Rendered::diagnostics`
under `Warn`, and are dropped under `Lossy`.

## Component Projection

`TerminalRenderable` gains an optional method:

```rust
fn render_tree_node(&self) -> Option<RenderNode> { None }
```

A component overrides it to opt into tree rendering. Group 1 components that
implement it: `Section`, `OrderedList`, `UnorderedList`, `Progress`,
`TwoColumn`, `Table` (and darkmatter's `YamlBlock`).

The **projection layer** (`render_tree::projection`) converts
`RenderableTerminalContent` into tree nodes:

```rust
RenderableTerminalContent::to_tree_nodes(
    &self, ctx: &mut TreeProjectionContext,
) -> ProjectionResult
```

- `TreeProjectionContext { strictness, max_depth, current_depth }` — recursion
  is depth-guarded; overflow yields a diagnostic.
- `ProjectionResult { nodes, diagnostics }`.
- A `String` projects to a `Text` node; a `Component` calls its
  `render_tree_node()`. A component returning `None` becomes an `Unsupported`
  node (Strict) or an ANSI-stripped fallback + diagnostic (Warn / Lossy).

## Native Rendering

The terminal tree renderer renders headings/sections, ordered/unordered
lists, and tables **natively** — it no longer delegates back to the bespoke
`Section`, `OrderedList`/`UnorderedList`, or `Table` components. Progress
widgets render from `renderable.widget.progress.*` hints on a `Paragraph`;
two-column layouts render from `renderable.widget.columns.*` hints on a
`BlockQuote`. Tables use a two-pass pre-scan / emit renderer that reuses the
table module's width-planning utilities (but not `Table::render()`).

## Layout

The terminal tree renderer applies a block node's `renderable::layout::Layout`
(read from `NodeAttrs::layout()`). It resolves each margin to whole cells
against the available width via the shared `resolve_cells` helper
(`Ch(n)`→`n`, `Percent(p)`→`round(width*p/100)`, `Zero`/`Css`/absent→`0`,
resolving for `RenderTarget::Terminal`), narrows the child render width by
left+right margins, prefixes each line, block-aligns the component as a unit,
and emits top/bottom margins as blank rows. `max_width` is **not** applied
(Browser-only). The Group 1 components seed their own `Layout` onto the
projected node, so layout flows through whether a component is rendered via
the tree or composed bespoke.

`LayoutTerminalExt` (`utils::layout`) — `apply_layout` / `apply_block_layout` /
`available_width` — is the bespoke (non-tree) path's terminal layout
application, now reading the same `Layout` type. The legacy `RowFill` /
`MaxWidth` / `Margin` enum and the `row_fill_strategy` builder are removed.

## Code-Render Hook

`TerminalRenderOptions.code_renderer` and `TreeComponent::with_code_renderer`
carry an `Option<Rc<dyn CodeRenderer>>` (trait defined in `renderable`). For a
`NodeKind::Code` node the renderer consults the hook with `lang`, `value`,
`attrs`, and `available_width`; a `Some` result is used verbatim, `None` falls
back to the built-in plain code panel.

## Quick Start

```rust
use renderable::tree::RenderNode;
use biscuit_terminal::render_tree::{render_terminal_node, TerminalRenderOptions};

let tree = RenderNode::root(vec![RenderNode::paragraph(vec![
    RenderNode::text("Hello, terminal"),
])]);
let rendered = render_terminal_node(&tree, &TerminalRenderOptions::default())?;
assert!(rendered.output.contains("Hello, terminal"));
```
