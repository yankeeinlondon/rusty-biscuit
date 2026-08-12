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

A component overrides it to opt into tree rendering. IR-aware components should
implement `TreeRenderable`, `MarkdownRenderable`, `BrowserRenderable`, and
`TerminalRenderable::render_tree_node`, with `render_tree_node` delegating to
the same private projection helper as `TreeRenderable::render_tree`.

The structural projection gap for nested components is closed:
`BlockQuote`, `StatusBlock`, and `FileSystem` provide `render_tree_node`
overrides, and containers project migrated children structurally instead of
falling back to ANSI-stripped text. The connector-list per-item `Style` lowering
gap is closed too (`render_tree_connector_list` applies each list item's
`Paragraph` `Style`), so `FileSystem`'s terminal styling is at parity.
`FileSystem::render` still uses its bespoke terminal path: that flip stays
**deferred as an accepted specialization** — the target-agnostic projection emits
portable Unicode icons and cannot reproduce the bespoke Nerd Font terminal icons.
`FileSystem` is off the darkmatter production path, so this is not a blocker (CSS
Box Architecture closeout disposition; see its `component-assessment.md`).

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
  The fallback logs with the stable `TerminalRenderable::type_name()` hook so
  missing projections are observable in logs and CI.

## Native Rendering

The terminal tree renderer renders headings/sections, ordered/unordered
lists, and tables **natively** — it no longer delegates back to the bespoke
`Section`, `OrderedList`/`UnorderedList`, or `Table` components. Progress
widgets render from the typed `ComponentHints::Progress` group on a `Paragraph`;
two-column layouts render from `ComponentHints::Columns` on a `BlockQuote`
(typed `NodeAttrs::component` fields, not `data`-bag hints). Tables use a
two-pass pre-scan / emit renderer that reuses the table module's width-planning
utilities (but not `Table::render()`).

### List-item inline coalescing (wrap invariant)

A list item's children shape differs by CommonMark list *kind*: a **loose**
item wraps its content in a `Paragraph` (`[Paragraph([Text, InlineCode, …])]`),
but a **tight** item carries a **flat run of inline siblings**
(`[Text, InlineCode, Text, …]`) with no `Paragraph`. `render_list_item` must
**coalesce a maximal run of consecutive inline-kind children into one inline
render** before wrapping — wrapping is **per-list-item**, not per-inline-node.
If only the first inline child is sent through the prefix/hanging-indent wrap
path, every following sibling (the code span, the trailing text) falls through
to the block branch and renders on its own **unwrapped** line — the span lands
alone and the trailing text overflows the width. Regression test:
`render_tree_tight_list_item_coalesces_inline_run_and_wraps`. Note this is
purely a render-tree concern: darkmatter's `Markdown::as_terminal` (and thus
every Markdown surface, e.g. claudine's prompt reporting) folds through here, so
a "tight bullet with `code`/**bold**/link wraps wrong" bug is fixed in this
renderer, not upstream.

### Nested-list width narrowing (overflow invariant)

A nested list is a **block child** of a list item: it renders first, then the
parent `indent_block`s the whole thing by `indent_children`. So the nested
content must be rendered at the width left **after** that indent —
`render_list_item` narrows via `render_in_width(child, available_width −
indent_children)`. If a nested block instead wraps against the full terminal
width (the bug was `render_list_text` reading `term.width()` with no
depth-narrowing), every line runs `indent_children` cells over the terminal
width once indented, and the **terminal hard-wraps the overflow** — e.g. a lone
trailing `;` or bracket dropped onto its own visual line. The renderer's own
output looks ~1–2 cells too wide per line; the visible artifact is the terminal
folding it. Sweep widths and assert no rendered line exceeds the width. Regression
test: `render_tree_nested_list_item_wraps_within_width`.

## Layout

The terminal tree renderer applies a block node's `renderable::layout::Layout`
(read from `NodeAttrs::layout()`). It resolves margins, `padding`, and the
`width` modes to whole cells against the available width via the shared
`resolve_cells` helper (`Ch(n)`→`n`, `Percent(p)`→`round(width*p/100)`,
`Zero`/`Css`/absent→`0`, resolving for `RenderTarget::Terminal`). The
content-box width comes from `layout.width`: `Auto` fills
`available − margin − padding − border`, `Fixed(tv)` resolves `tv` clamped to
that cap, and `FitContent` renders once at the cap then re-renders at the
measured widest line. `padding` is resolved into cells **once** against the
parent available width (so a `%` padding has a single basis) and threaded into
the paint step; the content renders at exactly the content-box width and
`padding` + `border` are painted **around** it (a `Fixed(n)` box keeps all `n`
content columns — the border is not carved out of them). The resolved
content-box width is passed to the paint layer as a **floor**: a background band
or bordered interior fills the full resolved width even when the text is
narrower, so a `Fixed`/`Auto` box paints all its columns rather than shrinking to
the widest line (`FitContent` already measured that line, so the floor is a
no-op there; a left-only border with no background stays ragged). The box is then
**block-placed within `available − margin`** for every width mode (`margin:auto`
semantics): a sub-available `Fixed` / `FitContent` / `max_width`-capped box is
centered/right-offset as a unit, while a box that fills the area centers its
visible content. Top/bottom margins emit as blank rows. The `padding` box is
painted by `paint_text` with `Style.background`; the margin stays transparent —
padding cells are still reserved even with an empty `Style`. Drawn borders
reserve only their glyph cells (one per vertical edge) — no implicit interior
gap, so `Layout.padding` is the single source of inner spacing. `max_width` caps
the content box and the capped box is block-placed (symmetric with the browser).
Migrated components seed their own `Layout` onto the projected node, so layout
flows through whether a component is rendered via the tree or composed bespoke.

`LayoutTerminalExt` (`utils::layout`) — `apply_layout` / `apply_block_layout` /
`available_width` — is the bespoke (non-tree) path's terminal layout
application, now reading the same `Layout` type. The legacy `RowFill` /
`MaxWidth` / `Margin` enum and the `row_fill_strategy` builder are removed.

## Text Layout

The terminal renderer is the only target that consumes
`NodeAttrs::text_layout` (typed `TextLayoutHints`: `width`, `max_width`,
`alignment`, `overflow`). `Writer::apply_text_layout` resolves `width` /
`max_width` to cells against the render width, pads per `alignment`, and
truncates overflow with `…` via the ANSI-aware `word_wrap::truncate`. It is
wired into three node shapes without mutating the tree:

- **`Link`** — the link label/display text is shaped to the resolved field
  (the structured link children stay intact for OSC 8 / fallback).
- **`Image`** — the `▉ IMAGE[alt]` placeholder is shaped with the alt text
  *inside* the brackets; the source `alt` stays intact on the node.
- **`ListItem`** — the marker is lifted out and the item body is block-aligned
  and padded within the resolved field; the marker stays structurally separate
  from body placement.

`width` establishes an exact field width and pads shorter content; `max_width`
only truncates content that exceeds the cap. Resolution happens during the fold,
so rendering one tree at different widths never mutates it. The browser and
Markdown folds ignore `text_layout`.

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
