---
description: Expert knowledge for the `renderable` library which provides traits and utilities for multi-target rendering (Terminal, Markdown, Browser) in the rusty-biscuit monorepo.
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
| [Layout Module](./layout.md)              | Target-agnostic layout: `Layout`, `TargetValue`, `Length`, `Margin`, `Alignment`                               |
| [Markdown Module](./markdown.md)          | `MarkdownRenderable` trait and style-aware Markdown output                                                     |
| [Tree Module](./tree.md)                  | Render tree: `RenderNode`, `Document`, `TreeRenderable`, component projection, render hints, `CodeRenderer`, tree renderers |
| [Design Decisions](./design-decisions.md) | Key architecture decisions, naming conventions, migration notes                                                |

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
