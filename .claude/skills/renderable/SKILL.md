---
name: renderable
description: Expert knowledge for the renderable library, which provides traits and utilities for multi-target rendering (Terminal, Markdown, Browser) from a shared render-tree IR in the rusty-biscuit monorepo. Use when working in the renderable package area, implementing multi-target rendering for a type, adding or targeting a render target (Terminal/Markdown/Browser), working with the render tree (TreeRenderable, RenderNode, Document), or adding the renderable dependency.
---
# `renderable` Library

Provides traits and utilities for type-strong, multi-target renderable components.

## The Render Tree

The library's central model is the **render tree** (`renderable::tree`): a
single, owned, target-agnostic representation that sits between content
sources and render targets. A `Document` holds a `RenderNode` tree plus a
`SourceRegistry` and metadata. Components implement `TreeRenderable`
(`fn render_tree(&self) -> RenderNode`) to produce a tree, and the Markdown,
Browser, and Terminal **tree renderers** fold that tree into output.

> `TreeRenderable` **replaces** the removed placeholder `AstRenderable` trait —
> there is no longer an "AST" render target. See [Tree Module](./tree.md).

## Start Here

- Use `TreeRenderable` when a component should render to multiple targets from
  one structural projection.
- Use `Layout` for block positioning and wrapping.
- Use `Style` for target-agnostic appearance carried on `RenderNode` attrs.
- Use `CssStyle` / `Stylesheet` for browser CSS declaration blocks and scoped
  component stylesheets.
- Use `RenderTarget` when a value must resolve differently for Markdown,
  MarkdownPlus, Browser, or Terminal.

## Progressive Disclosure

Open only the topic file needed for the task:

| Topic | File |
|-------|------|
| Render tree, diagnostics, validation, renderers | `tree.md` |
| Target-agnostic layout | `layout.md` |
| Target-agnostic appearance | `style.md` |
| Browser rendering and fragments | `browser.md` |
| HTML page assembly | `html.md` |
| CSS builder | `stylesheet.md` |
| Color model | `color.md` |
| Markdown trait/rendering | `markdown.md` |
| Architecture decisions | `design-decisions.md` |

For terminal folding, switch to the `biscuit-terminal` skill. For Markdown
parsing/composition or `style:` frontmatter, switch to the `darkmatter` skill.

## Render Targets

| Target       | Trait                | Crate              |
|--------------|----------------------|--------------------|
| Terminal     | `TerminalRenderable` | `biscuit-terminal` |
| Markdown     | `MarkdownRenderable` | `renderable`       |
| MarkdownPlus | `MarkdownRenderable` | `renderable`       |
| Browser      | `BrowserRenderable`  | `renderable`       |

> **MarkdownPlus** is Markdown with richer features via inline HTML.

A component can also implement `TreeRenderable` instead of (or alongside) the
per-target traits; the tree renderers then cover Markdown, Browser, and
Terminal from the single tree it produces.

## Topics

| Topic                                     | Description                                                                                                    |
|-------------------------------------------|----------------------------------------------------------------------------------------------------------------|
| [Browser Module](./browser.md)            | `BrowserRenderable` trait, `BrowserFragment<Ready>`, `ComponentStylesheet`, `PageOptions`, `RelativeAssetPath` |
| [HTML Module](./html.md)                  | `HtmlPage` assembly, fragment composition, metadata merging, tag types                                         |
| [Stylesheet Module](./stylesheet.md)      | Type-safe CSS builder: `CssStyle`, `CssRule`, `Stylesheet`, typed properties and values                        |
| [Color Module](./color.md)                | Cross-target color system: `Color`, `CssColor`, `WebColor`, `BasicColor`, `Tailwind`, RGB, HDR                 |
| [Layout Module](./layout.md)              | Target-agnostic layout: `Layout`, `TargetValue`, `Length`, `Edges`, `Width`, `Alignment`                       |
| [Style Module](./style.md)                | Target-agnostic appearance: `Style`, `PerMode`, `TextEmphasis`, `Border`, `Background`                          |
| [Markdown Module](./markdown.md)          | `MarkdownRenderable` trait and style-aware Markdown output                                                     |
| [Tree Module](./tree.md)                  | Render tree: `RenderNode`, `Document`, `TreeRenderable`, component projection, render hints, `CodeRenderer`, tree renderers |
| [Migrating a Component to the IR](../../../renderable/docs/migrate-component-to-ir.md) | Canonical recipe — flip-from-bespoke (Variant A) and born-on-the-tree (Variant B), escape-hatch rules, doc-update obligations |
| [Design Decisions](./design-decisions.md) | Key architecture decisions, naming conventions, migration notes                                                |

## Darkmatter `style:` Frontmatter

Darkmatter's document-level `style:` frontmatter is implemented in
`darkmatter::style`, but it intentionally uses renderable primitives:
`Length`, `Alignment`, and color-backed values. Active wiring covers
sub-specs 1 through 7 from
`renderable/features/2026-05-23-style-property/`: schema/parser, page layout,
table/image/block-quote layout, `ul`/`ol`/`li` layout, page/component
`color` / `bg-color`, `style.hr.*`, and the bespoke knobs
`page.stylesheet`, `page.meta`, `page.code.theme`, hyperlink style, and
local hyperlink/image style.

This pipeline is separate from `renderable::style::Style`. Frontmatter applies
policy to `DarkmatterPage`; `Style` is the render-tree appearance attribute
carried by `NodeAttrs`.

## Quick Start

### Implementing BrowserRenderable

```rust
use renderable::browser::{BrowserRenderable, BrowserFragment, Ready};
use renderable::browser::fragment::ComposableNode;
use std::any::Any;

#[derive(Debug)]
struct MyComponent { text: String }

impl BrowserRenderable for MyComponent {
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        BrowserFragment::new(ComposableNode::text(&self.text))
    }
    fn as_any(&self) -> &dyn Any { self }
}
```

### Rendering a Page

```rust
use renderable::browser::PageOptions;

let page = component.render_html_page(Some(PageOptions {
    stylesheet: Some(my_stylesheet),
    css_variables: Some(vec![("primary".into(), "#336699".into())]),
    ..Default::default()
}));

let html = page.render();
```

### Building a Stylesheet

```rust
use renderable::stylesheet::{
    CssStyle, CssSizingProp, CssSizing, CssColorProp, CssColor
};

let style = CssStyle::new()
    .add(CssSizingProp::Width, CssSizing::px(320.0))
    .add(CssColorProp::Color, CssColor::rgb(0x33, 0x66, 0x99));
```

## Architecture

```text
┌─────────────────────────────────────────────┐
│  Components (biscuit-terminal, darkmatter)  │
│  + content sources (darkmatter Markdown)    │
└─────────────────────────────────────────────┘
                │  TreeRenderable / fold
                ▼
      ┌─────────────────────────┐
      │  Render Tree            │
      │  Document / RenderNode  │
      └─────────────────────────┘
                │
    ┌───────────┼───────────┐
    ▼           ▼           ▼
┌────────┐ ┌────────┐ ┌────────┐
│Terminal│ │ Browser│ │Markdown│
│renderer│ │renderer│ │renderer│
└────────┘ └────────┘ └────────┘
```

## Relationship to other Packages

The **renderable** library can and should be used by any renderable components which need to render to Markdown, the terminal, or the browser from a shared render-tree IR. There is no AST render target; use `TreeRenderable` and the `renderable::tree` module for target-agnostic structure. The packages which already play a big role in this ecosystem are:

- `biscuit-terminal` 
    - provides all sorts of utilities for discovering features of a given terminal as well as how to render to a terminal (with good fallbacks)
    - because it is so concentrated on terminal features, the `TerminalRenderable` trait resides in **biscuit-terminal** instead of **renderable**
    - current IR migration goal: every IR-aware component should share one private projection helper between `TreeRenderable::render_tree` and `TerminalRenderable::render_tree_node`; nested `RenderableTerminalContent::Component` values should project structurally instead of falling back to ANSI-stripped text
    - Stage 3 closed the remaining structural-projection gap for `BlockQuote`, `StatusBlock`, and `FileSystem`, deferred the `FileSystem::render` terminal flip, and retired or documented each remaining `render_bespoke` compatibility hook
    - tree-cutover Phase 4 closed the connector-list per-item `Style` lowering gap (`render_tree_connector_list` now applies each list item's `Paragraph` `Style`), so `FileSystem`'s terminal styling is at parity; the terminal `render` flip itself stays deferred because the target-agnostic projection emits Unicode icons and cannot reproduce the bespoke Nerd Font terminal icons. `FileSystem` is not rendered by the darkmatter document pipeline, so it is not a cutover blocker
    - **biscuit-terminal** also provides these important components:
        - `Prose`
        - `Table`
        - `BlockQuote`
        - `TwoColumn`
        - `UnorderedList` / `OrderedList`
        - and several more
- `darkmatter`
    - The **darkmatter** library provides two important pipelines:
        - a **composition** pipeline that uses Darkmatter's DSL to transform a graph of documents into valid Markdown content
        - a **render** pipeline that is able to render markdown content into:
            - Markdown (2 variants)
            - HTML
            - Terminal
    - The render pipeline currently provides strong capability but is not _yet_ implementing all of the "renderable" traits it should.
    - Tree-cutover status (`renderable/features/2026-06-02-tree-cutover/`): the document pipeline is fully flipped on **every** target. **Terminal:** `Markdown::as_terminal`, the default-layout `DarkmatterPage::render`, *and* the per-component-layout path (`as_terminal_with_layout(Some(ctx))`) all route through the render-tree terminal renderer (`render_tree::render_tree_terminal` / `render_tree_terminal_with_layout`). The decorated path's hyperlink-label width/alignment/truncation, `▉ IMAGE[alt]` placeholder, and right-aligned list-item body are implemented on the tree via the `render_tree::decorate` pass plus the `image_placeholder` flag and the `darkmatter.li` alignment hint. **Browser:** `Markdown::as_html` / `DarkmatterPage::render_to_browser` route through `render_tree::render_tree_html`; structured link directives (`class`/`target`/`data-*`/`prompt`) and per-link inline CSS are lowered to `<a>` attributes by `render_tree::entrypoints`, and a malformed fenced code-block directive is still a fatal `MarkdownError::InvalidLineRange` (via the `validate_code_directives` preflight `as_html` runs). The `Prose` collapse is landed and darkmatter's `YamlBlock` is now `tree render only` (Phase 4): its terminal `render` and browser `render_html_fragment` fold the projected `Code` node through the shared tree renderers wired with `TerminalCodeRenderer`.


## Migration from Pre-Renderable

**15 May 2026**

This library was born out of a desire to start centralizing traits and utilities designed to
promote the development of "renderable components" which can render to multiple targets.

| Old                       | New                         | Location                 |
|---------------------------|-----------------------------|--------------------------|
| `Renderable`              | `TerminalRenderable`        | `biscuit-terminal`       |
| `RenderableContent`       | `RenderableTerminalContent` | `biscuit-terminal`       |
| `BrowserRenderable`       | `BrowserRenderable`         | `renderable::browser`    |
| `Stylesheet` (darkmatter) | `CssStyle` / `Stylesheet`   | `renderable::stylesheet` |
| `Layout`                  | `Layout`                    | `renderable::layout`     |
| `Color`                   | `Color`                     | `renderable::color`      |
