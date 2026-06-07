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
  `NodeAttrs` (`id` / `classes` plus typed sparse fields — see
  [Render Hints](#render-hints)).
- **`NodeKind`** — 27 block and inline variants: `Root`, `Heading`,
  `Section`, `Paragraph`, `BlockQuote`, `List`, `ListItem`, `Code`,
  `ThematicBreak`, `Table`, `TableRow`, `TableCell`, `FootnoteDefinition`,
  `Text`, `Emphasis`, `Strong`, `Delete`, `Span`, `InlineCode`, `Link`,
  `Image`, `FootnoteReference`, `SoftBreak`, `HardBreak`, `Html`,
  `Extended`, `Unsupported`. `Extended { token, children, payload }` is the
  target-agnostic inline-extension node: renderers dispatch on `token`. The
  built-in `mark` and `dim` tokens lower per target — `mark` to `<mark>`
  (Browser, recovering semantic fidelity), reverse video / SGR 7 (Terminal),
  and `==children==` (Markdown); `dim` to `<span style="opacity:0.6">`
  (Browser), `<dim>` / SGR 2 (Terminal), and `⌄children⌄` (Markdown). A token a
  renderer does not recognize falls back to a neutral default that preserves
  `children` (a `<span class="extended-{token}">` in Browser, plain inline
  content in Markdown and Terminal).
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

    /// Optional layout for this component, seeded on its root node by the
    /// tree renderers. Defaults to `None`.
    fn tree_layout(&self) -> Option<renderable::layout::Layout> { None }
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
  (default `None`). Lives in `biscuit-terminal`. For migrated
  `biscuit-terminal` components, the override should delegate to the same
  private projection helper used by `TreeRenderable::render_tree`; otherwise a
  nested `RenderableTerminalContent::Component` can degrade to an
  ANSI-stripped text fallback.

The **projection layer** (`biscuit-terminal`'s `render_tree::projection`)
converts `RenderableTerminalContent` into tree nodes:
`to_tree_nodes(&self, &mut TreeProjectionContext) -> ProjectionResult`.
`TreeProjectionContext` carries `strictness`, `max_depth`, `current_depth`
(recursion-guarded); `ProjectionResult` carries `nodes` + `diagnostics`.

## Render Hints

First-class presentation lives in **typed sparse fields** on `NodeAttrs`, not in
the `data` bag — so a renderer reads a hint with no serde round-trip, clone, or
key formatting. The render-tree JSON these fields serialize to is **same-version**
serde output (for debug, inspection, and persistence); it is *not* a promised
cross-version durable format. The fields:

- **`layout: Option<Box<Layout>>`** — margins, alignment, max-width, word wrap.
  Set via `NodeAttrs::set_layout`; read with `NodeAttrs::layout` (clone) or
  `layout_ref` (borrowed, hot path). Permitted on block-level nodes only. See
  `layout.md`.
- **`style: Option<Box<Style>>`** — color, background, emphasis, border. Set via
  `NodeAttrs::set_style`; read with `NodeAttrs::style` / `style_ref`. Permitted
  on block nodes and inline `Span` nodes. `Style::inherited_from` cascades the
  text-appearance fields (`color`, `emphasis`) only. See `style.md`.
- **`sequence_join: Option<SequenceJoin>`** — child-sequence join policy
  (`Root` nodes only).
- **`list_marker_policy: ListMarkerPolicy`** — list marker presentation; the
  default policy serializes to nothing.
- **`component: Option<Box<ComponentHints>>`** — the per-kind hint group (below).
- **`text_layout: Option<Box<TextLayoutHints>>`** — unresolved, width-dependent
  text intent (`width`, `max_width`, `alignment`, `overflow`). Permitted on
  `Link`, `Image`, and `ListItem` nodes only. The producer states *what* it
  wants; the renderer resolves percentages, measures rendered content, and
  applies alignment/overflow during its single fold — the tree never stores
  pre-truncated text, and a link keeps its children / an image keeps its `alt`.
  `width` pads shorter content per `alignment`; `max_width` only truncates
  content that exceeds the cap. Consumed by the terminal renderer; the browser
  and Markdown folds ignore it. Set via `set_text_layout` / read with
  `text_layout` / `text_layout_ref`.
- **`browser: Option<Box<BrowserAttrs>>`** — typed, validated browser-target
  attributes (see [Browser Attributes](#browser-attributes)). Set via
  `set_browser` / read with `browser` / `browser_ref`.
- **`data: BTreeMap<String, serde_json::Value>`** — **extension namespaces only**
  (e.g. `darkmatter.hr.*`). `set_hint` / `get_hint` /
  `remove_hint` and the `HintNamespace` wrapper remain for package-local
  extension data. Stale `renderable.*` keys in `data` are a **validation error**
  (see [Validation](#diagnostics-and-validation)) — they are not migrated into
  the typed fields.

### `ComponentHints`

A node carries at most one `ComponentHints`, matched to its `NodeKind`. Its
variants wrap the per-component hint structs, so only nodes of the matching kind
pay for the box:

- **`List(ListRenderHints)`** — bullet, hanging indent, child indent.
- **`Code(CodeRenderHints)`** — header row, language label, highlight flag.
- **`Progress(ProgressHints)`** — value, bar width, glyphs, brackets, colors.
- **`Columns(ColumnsHints)`** / **`ColumnWidthKind`** — gap, left width,
  `left_count`, stack threshold (two-column layout, carried on a `BlockQuote`).
- **`Task(TaskHints)`** / **`TaskState`** — richer Todo state that degrades to a
  GFM checkbox in Markdown.
- **`Table(TableHints)`** — grouped table hints: `columns` (a
  `BTreeMap<usize, TableColumnHints>` keyed per column index), `terminal`
  (`TableTerminalHints` striping / cursor preference), and `title`. The
  per-column, terminal, and title setters are co-resident — setting one does not
  clobber the others.
- **`TableCell(TableCellHints)`** — per-cell typed value and alignment.

The accessor names and signatures (`set_list_hints` / `list_hints`,
`set_table_column_hints(i, …)` / `table_column_hints(i)`, etc.) are unchanged
from the bag era — only their bodies switched to typed-field reads, plus
`*_ref` borrowed variants (`list_hints_ref`, `code_hints_ref`, `style_ref`,
`layout_ref`) for renderer hot paths.

## Browser Attributes

`BrowserAttrs` (the typed `NodeAttrs::browser` field) carries every attribute
the browser renderer emits *before* the fold begins, so the browser paths never
patch completed HTML:

- **`link: Option<LinkBrowserAttrs>`** — valid only on a `Link`. Holds a typed
  `target` (`LinkTarget`: the four standard keywords plus `Named`), `rel`
  (`Vec<LinkRelation>` emitted as a space-separated token list), and `download`.
- **`image: Option<ImageBrowserAttrs>`** — valid only on an `Image`. Holds
  `loading` (`ImageLoading`) and `decoding` (`ImageDecoding`).
- **`inline_style: Option<CssStyle>`** — a *validated* `CssStyle`, not a raw
  string, so unparsed CSS cannot be injected. A property set here replaces the
  same property derived from the node's `Style`.
- **`data_attrs: BTreeMap<DataAttrName, String>`** and
  **`aria_attrs: BTreeMap<AriaAttrName, String>`** — first-class typed `data-*`
  / `aria-*` attributes in deterministic key order. `DataAttrName` /
  `AriaAttrName` reject empty, uppercase, or unsafe names (and `data-*` reserves
  the `xml` prefix) with `BrowserAttrNameError`, at construction *and* on serde
  deserialize. The fixed `data-` / `aria-` prefix makes it impossible to inject
  an arbitrary attribute (`onclick`, `style`, `href`, `src`) through the map.

These are distinct from the opaque `NodeAttrs::data` extension bag. Both browser
writers (fragment and streaming) fold them into identical output: classes, the
single merged `style` attribute, typed link/image attributes, and stable
`data-*` / `aria-*` ordering and escaping. The Markdown renderer never reads
`browser`, so both dialects drop browser-only attrs.

## Inheritance Resolver — `InheritedStyle`

`InheritedStyle` is the single resolver every render fold threads to push
text appearance down the tree, replacing each renderer's bespoke
color/emphasis threading. The inheritance rule lives in exactly one place:

```rust
let root = InheritedStyle::root();
// `enter` returns the child context to thread into this node's children plus
// the full effective Style to apply to the node itself.
let (child_ctx, effective) = root.enter(node.attrs.style_ref());
```

Only `Style::color` and `Style::emphasis` inherit; the box-painting fields
(`background`, `border`) and geometry never do. The node's own box-painting is
layered onto its `effective` style but is cleared from the `child_ctx` carried
into descendants. `effective()` exposes the accumulated text appearance for a
renderer that must merge a derived style (e.g. a heading's intrinsic emphasis)
before folding a subtree. The terminal fold in `biscuit-terminal`
(`render_tree::render`) threads an `InheritedStyle` rather than reconstructing
the push-down by hand.

## Performance Gate

The typed fields exist so that **a fold round-trips zero renderable-owned hints
through `data`**. A structural test gate enforces this: a namespace-partitioned
counter (`HINT_ACCESSES`) bumps on every `set_hint` / `get_hint` / `remove_hint`,
and the gate folds a styled corpus per target and asserts the `renderable.*` slot
stayed at zero (the extension slot is non-zero, proving the counter works). Any
accessor that reaches back into the bag for a first-class hint fails the gate.

The gate covers **every** fold: `renderable`'s own test folds the corpus through
the Markdown, browser-fragment, and browser-streaming renderers; `biscuit-terminal`
folds it through the terminal renderer (`tests/perf_gate.rs`). The counter is
active under `cfg(test)` and under renderable's test-only `hint-access-counter`
feature, which `biscuit-terminal` enables from `[dev-dependencies]` so its
out-of-crate test can observe the counter. The feature is never enabled in a
release build, so the instrumentation carries no runtime cost in production.

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
- **`validate`** / **`ensure_valid`** — structural-invariant checks. These
  enforce typed-field placement (`Layout` block-only, `sequence_join` Root-only,
  `task_hints` ListItem-only, `table_title` Table-only, `text_layout` only on
  `Link` / `Image` / `ListItem`, a `browser.link` group only on a `Link` and a
  `browser.image` group only on an `Image`) and reject any stale `renderable.*`
  key left in `data` — first-class hints must be typed fields, while other
  namespaces (`darkmatter.*`) are allowed.

## Tree Renderers

- **Markdown** — `render_markdown_node` / `render_markdown_document` with
  `MarkdownRenderOptions` and `MarkdownDialect` (`Markdown` / `MarkdownPlus`).
- **Browser** — `render_browser_node` / `render_browser_document` with
  `BrowserRenderOptions` and `RawHtmlPolicy` (`Allow` / `Escape` / `Reject`).
  `render_browser_document_html(doc, opts)` is the direct `Document` → final
  HTML `String` path: it streams the tree into one buffer (no fragment per
  node) and emits bytes identical to
  `render_browser_document(..)?.output.render()`. Use it when you only need the
  final string; use `render_browser_document` when you need an `HtmlPage` /
  `BrowserFragment<Ready>` for composition.
- **Terminal** — `render_terminal_node` / `render_terminal_document` with
  `TerminalRenderOptions` / `TerminalRenderContext`. Heading/section, lists,
  and tables render **natively** (no delegation back to bespoke components).
  Adapters: `TreeComponent<T>` (terminal) and `BrowserTreeComponent<T>`
  (browser). These live in the `biscuit-terminal` crate — see the
  biscuit-terminal skill's *Render Tree* topic.

## Darkmatter Fold (production)

`darkmatter::markdown::render_tree` folds a `pulldown-cmark` 0.13 event stream
into a `Document` via `fold_markdown_to_document`, then bakes component policy,
alpha-bearing `PaintColor`, text layout, browser attrs, and HR defaults onto the
nodes during construction through a `TreeBuildContext` (the context-aware fold
entry points). This is the **production** path: `Markdown::as_html` /
`as_terminal` and `DarkmatterPage::render` / `render_to_browser` build a complete
tree this way and run exactly **one target fold** over it — there is no
post-fold decoration pass, no `darkmatter.li` / `darkmatter.style` hint, and no
HTML rewriting. See the `darkmatter` skill for the build-context details.
