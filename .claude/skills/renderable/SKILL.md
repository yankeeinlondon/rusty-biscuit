---
name: renderable
description: Expert knowledge for the renderable library, which provides traits and utilities for multi-target rendering (Terminal, Markdown, Browser) from a shared render-tree IR in the rusty-biscuit monorepo. Use when working in the renderable package area, implementing multi-target rendering for a type, adding or targeting a render target (Terminal/Markdown/Browser), working with the render tree (TreeRenderable, RenderNode, Document), or adding the renderable dependency.
hash: c0aac8e9eca375bd-d33aeebcc9cd75ea
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
| [Component catalog & IR state](../../../renderable/docs/components.md) | Per-component target support and IR State column (the migration is complete; the prescriptive flip-guide was retired as obsolete) |
| [Design Decisions](./design-decisions.md) | Key architecture decisions, naming conventions, migration notes                                                |

## Darkmatter `style:` Frontmatter

Darkmatter's document-level `style:` frontmatter is implemented in
`darkmatter::style`, but it intentionally uses renderable primitives:
`Length`, `Alignment`, and color-backed values. Active wiring covers
sub-specs 1 through 8, and sub-spec 9 (the
[Style Everywhere](../../../renderable/features/2026-06-30-style-everywhere/matrix.md)
surface) expands every `PageComponent` bucket with `margin`, `padding`,
`border`, `emphasis`, `word-wrap`, and explicit `width` mode keywords (`auto`,
`fit-content`, lengths). See the darkmatter `style:` reference for the full
per-component property list.

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
    - the IR migration is **complete**: every IR-aware component shares one private projection helper between `TreeRenderable::render_tree` and `TerminalRenderable::render_tree_node`, and nested `RenderableTerminalContent::Component` values project structurally instead of falling back to ANSI-stripped text
    - the CSS Box Architecture closeout (`renderable/features/_completed/2026-06-06-tree-closeout`) classified every bespoke biscuit-terminal component with a durable disposition (see its `component-assessment.md`); all are **retained** as intentional terminal specializations and none blocks the architecture. `FileSystem`'s terminal `render` flip stays deferred (the target-agnostic projection emits Unicode icons and cannot reproduce the bespoke Nerd Font terminal icons), but `FileSystem` is off the darkmatter production path, so it is not a blocker
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
    - The render pipeline runs entirely on the render tree; the CSS Box Architecture program (`_completed/2026-06-04-css-box-architecture`) is **complete**.
    - Tree-cutover status (`_completed/2026-06-02-tree-cutover`, completed by the tree-features cutover `_completed/2026-06-06-tree-features` and signed off by `renderable/features/_completed/2026-06-06-tree-closeout`): the document pipeline is fully flipped on **every** target, and production rendering is now **one context-aware fold followed by one target fold** — there is no post-fold decoration pass. The one documented exception is `DarkmatterPage`, retained as the constrained **Option A** slim page frame: a viewport-level assembler (page width/margin/padding, full-page background, max-width centering, code-theme contrast, browser page-wrapper metadata + stylesheet assembly) that operates on the folded output, carrying no component policy and mutating no component content. **Terminal:** `Markdown::as_terminal` and `DarkmatterPage::render` route through `render_tree::render_tree_terminal` / `render_tree_terminal_with_context`. Component policy, page/component colors (alpha-bearing `PaintColor`), hyperlink/image text-layout, and HR defaults are baked onto nodes during construction by the `render_tree::build_context` (`TreeBuildContext`) fold; the terminal renderer resolves them. Hyperlink-label width/alignment/truncation, the `▉ IMAGE[alt]` placeholder (alt shaped *inside* the brackets), and the marker-lift + block-aligned list-item body are driven by typed `NodeAttrs::text_layout` (the old `render_tree::decorate` pass, `darkmatter.li`, and `darkmatter.style` hints are deleted). **Browser:** `Markdown::as_html` / `DarkmatterPage::render_to_browser` route through `render_tree::render_tree_html`; structured link/image directives (`class`/`target`/`data-*`/`prompt`) and per-link/per-image inline CSS are typed `NodeAttrs::browser` attrs lowered to `<a>`/`<img>` attributes by the browser fold (a validated `inline_style` *replaces* the derived `Style` declaration for the same property — no opacity sentinels or post-render HTML rewrites). A malformed fenced code-block directive is still a fatal `MarkdownError::InvalidLineRange` via the `validate_code_directives` preflight the HTML entry points run over the folded tree. The `Prose` collapse is landed and darkmatter's `YamlBlock` is `tree render only`: its terminal `render` and browser `render_html_fragment` fold the projected `Code` node through the shared tree renderers wired with `TerminalCodeRenderer`.


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