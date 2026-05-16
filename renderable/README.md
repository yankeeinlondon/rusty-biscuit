# Renderable

Provides the traits and utilities to allow for type-strong, multi-target renderable components.

## Render Targets

The library recognizes five render targets:

1. **Markdown** — ergonomic text output
2. **MarkdownPlus** — Markdown with richer features via inline HTML
3. **Terminal** — escape-code styled output (trait lives in `biscuit-terminal`)
4. **Browser** — HTML/CSS output
5. **AST** — abstract syntax tree representation

This enumeration is available as [`RenderTarget`](./src/target.rs).

## Traits

Traits defined in this library:

| Trait | Purpose |
|-------|---------|
| [`MarkdownRenderable`](./src/markdown.rs) | Renders to Markdown output |
| [`BrowserRenderable`](./src/browser/renderable.rs) | Renders to HTML for browser display |
| [`AstRenderable`](./src/ast.rs) | Renders to AST representation |

> **Note:** [`TerminalRenderable`](./src/terminal.rs) (formerly `Renderable`) is defined in the `biscuit-terminal` library.

### MarkdownRenderable

```rust
pub trait MarkdownRenderable {
    fn render_markdown(&self) -> String;
    fn render_markdown_with_style(&self, _style: Option<Stylesheet>) -> String {
        self.render_markdown()
    }
}
```

### BrowserRenderable

```rust
pub trait BrowserRenderable: std::fmt::Debug + Any {
    fn render_html_fragment(&self) -> BrowserFragment<Ready>;
    fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage { ... }
    fn as_any(&self) -> &dyn Any;
}
```

### AstRenderable

```rust
pub trait AstRenderable {
    fn render_ast(&self) -> String;
}
```

## Modules

### Browser (`browser/`)

Browser-target rendering infrastructure.

- **`BrowserRenderable`** — trait for components that render to HTML
- **`BrowserFragment<Ready>`** — the universal "done" currency for composition; a fragment carries its own stylesheet, metadata, dependency links, and feature flags
- **`ComponentStylesheet`** — scoped CSS ruleset collection owned by a component; internal selectors are lowered to descendant selectors (`.component .child`)
- **`PageOptions`** — caller-supplied options for page assembly (stylesheet, CSS variables, external asset paths)
- **`RelativeAssetPath`** — filesystem path guaranteed to be relative; validates that external `<link>` and `<script>` paths stay portable

### HTML (`html/`)

Typed HTML page assembly and node construction.

- **`HtmlPage`** — fully-assembled HTML page with `<head>` state, metadata, stylesheets, and script blocks
  - `HtmlPage::from(fragment)` — promote a fragment to a standalone page
  - `HtmlPage::render()` — emit the complete HTML string
  - `set_title()` — writes the `Title` microdata key (fans out to HTML/OpenGraph/Twitter/Schema.org tags)
  - `apply_page_options()` — infallibly apply `PageOptions`
- **`tag/`** — typed HTML tag representations (`BlockTag`, `LinkTag`, `MetaTag`)
- **`attribute/`** — typed HTML attributes including ARIA and CORS
- **`script/`** — script tag handling

### Stylesheet (`stylesheet/`)

Type-safe CSS declaration builder independent of any render target.

- **`CssStyle`** — a list of `property: value` pairs; can emit CSS text, JSON, or JSON5
- **`CssRule`** — a `(selector, CssStyle)` pair
- **`Stylesheet`** — a collection of `CssRule` entries
- **`CssProp`** / **`CssValue`** — typed property and value enumerations with five categories: `Sizing`, `SizingMulti`, `Color`, `Integer`, `Raw`
- **`CssTypedProperty`** — compile-time pairing of property subsets with accepted value types
- **`StylesheetError`** — runtime validation errors for dynamic parsing

### Color (`color/`)

Cross-target color types.

- **`Color`** — the main color enum
- **`CssColor`** — CSS-compatible color values
- **`WebColor`** — web-standard colors
- **`BasicColor`** — basic terminal/ANSI colors
- **`Tailwind`** — Tailwind CSS color palette
- **`RgbColor`** / **`Octet`** — RGB representation
- **`HdrColor`** — HDR color support

### Layout (`layout.rs`)

Target-agnostic layout configuration data.

- **`Layout`** — controls margins, alignment, word-wrapping, and background color
- **`Alignment`** — horizontal alignment (`Left`, `Center`, `Right`)
- **`Margin`** — fixed (`Chars`), percentage (`Percent`), or composed (`Offset`) margin values
- **`RowFill`** — row padding strategy (`Auto`, `Fill`, `Exact`)
- **`MaxWidth`** — width constraints

### Markdown (`markdown.rs`)

The `MarkdownRenderable` trait and related types.

### AST (`ast.rs` / `ast_utils.rs`)

The `AstRenderable` trait and AST utilities.

### Target (`target.rs`)

The `RenderTarget` enum listing all recognized targets.

### Tokens (`tokens.rs`)

CSS semantic-token defaults for the `:root` block.

### Microdata (`microdata.rs`)

Metadata key definitions and microdata-to-HTML tag generation.

### Wrap Policy (`wrap_policy.rs`)

Word-wrapping strategies for text rendering.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Components                            │
│  (in biscuit-terminal, darkmatter, or downstream crates)     │
└─────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
┌───────────────┐    ┌───────────────┐    ┌───────────────┐
│  Terminal     │    │   Browser     │    │   Markdown    │
│  (biscuit-    │    │  (renderable) │    │  (renderable) │
│  terminal)    │    │               │    │               │
└───────────────┘    └───────────────┘    └───────────────┘
                              │
                              ▼
                    ┌───────────────────┐
                    │  HtmlPage         │
                    │  BrowserFragment  │
                    │  ComponentStylesheet
                    └───────────────────┘
                              │
                              ▼
                    ┌───────────────────┐
                    │  CSS / HTML output │
                    └───────────────────┘
```

## Usage

### Implementing BrowserRenderable

```rust
use renderable::browser::{BrowserRenderable, BrowserFragment, Ready};
use renderable::browser::fragment::ComposableNode;

impl BrowserRenderable for MyComponent {
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        let node = ComposableNode::text("Hello, world!");
        BrowserFragment::new(node)
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
}
```

### Rendering a Page

```rust
use renderable::browser::PageOptions;

let page = component.render_html_page(Some(PageOptions {
    stylesheet: Some(my_stylesheet),
    css_variables: Some(vec![("primary-color".into(), "#336699".into())]),
    external_stylesheet: None,
    external_code: None,
}));

let html = page.render();
```

## Dependencies

This crate is part of the `rusty-biscuit` monorepo. It has no dependencies on `biscuit-terminal` or `darkmatter` — those crates depend on it.
