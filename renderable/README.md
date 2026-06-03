# Renderable

Provides the traits and utilities to allow for type-strong, multi-target renderable components.

## Render Targets

The library recognizes these render targets:

1. **Markdown** — ergonomic text output
2. **MarkdownPlus** — Markdown with richer features via inline HTML
3. **Terminal** — escape-code styled output (trait lives in `biscuit-terminal`)
4. **Browser** — HTML/CSS output

This enumeration is available as [`RenderTarget`](./src/target.rs).

## The Render Tree

The library's central model is the **render tree** ([`tree`](./src/tree/mod.rs)
module): a single, owned, target-agnostic representation that sits between
content sources and render targets. Content sources fold their input into a
tree, and target renderers fold the tree into concrete output.

- **[`RenderNode`](./src/tree/node.rs)** — a node with a `kind` ([`NodeKind`],
  25 block and inline variants such as `Heading`, `Paragraph`, `List`, `Table`,
  `Code`, `Strong`, `Link`, `Image`), a [`SourceSpan`] recording where it came
  from, and [`NodeAttrs`] (id / classes / data attributes).
- **[`Document`](./src/tree/document.rs)** — a `RenderNode` tree plus a
  [`SourceRegistry`] of origins and [`DocumentMetadata`] (frontmatter).
- All tree types are `serde`-serializable, so a tree can be persisted or
  inspected as JSON.
- **Diagnostics and validation** — [`validate`]/[`ensure_valid`] check
  structural invariants; renderers return a [`Rendered<T>`] carrying output
  plus non-fatal [`Diagnostic`]s, governed by [`RenderStrictness`]
  (`Strict` / `Warn` / `Lossy`).

## Traits

Traits defined in this library:

| Trait | Purpose |
|-------|---------|
| [`TreeRenderable`](./src/tree/mod.rs) | Renders a component to a canonical [`RenderNode`] tree |
| [`MarkdownRenderable`](./src/markdown.rs) | Renders directly to Markdown output |
| [`BrowserRenderable`](./src/browser/renderable.rs) | Renders to HTML for browser display |

> **Note:** [`TerminalRenderable`](./src/terminal.rs) (formerly `Renderable`) is defined in the `biscuit-terminal` library.

A component that implements `TreeRenderable` gains multi-target rendering for
free: any of the tree renderers below can fold the tree it produces.

### TreeRenderable

```rust
pub trait TreeRenderable {
    /// Renders the component to a canonical render tree.
    fn render_tree(&self) -> RenderNode;
}
```

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

### Layout (`layout/`)

Target-agnostic layout configuration for block-level components.

- **`Layout`** — margins, alignment, max-width, and word-wrapping
- **`TargetValue<T>`** — a value that is universal or specified per render target
- **`Length`** — a layout length: `Zero`, `Ch`, `Percent`, or target-native `Css`
- **`Margin`** — a four-sided box, each side a `TargetValue<Length>`
- **`Alignment`** — horizontal alignment (`Left`, `Center`, `Right`)
- **`LayoutError`** — invalid percentage, non-universal unit, or empty per-target map

### Style (`style.rs`)

The target-agnostic **appearance** primitive — the sibling of `Layout`.
`Layout` decides *where the box sits*; `Style` decides *what the box looks
like*. Components declare a `Style`; the tree renderers apply it (the
Terminal renderer first). A component never hand-writes ANSI or CSS.

- **`Style`** — foreground `color`, `background`, `emphasis`, `border`, and
  `fill`. Only `color` and `emphasis` inherit through the render tree;
  `background`, `border`, and `fill` are box-painting properties that stay
  explicit on the painting node
- **`TextEmphasis`** / **`UnderlineStyle`** / **`EmphasisLayer`** — shared
  text weight and decoration leaves (bold, dim, italic, underline,
  strikethrough, blink); reused by `biscuit-terminal`'s `Prose`
- **`PerMode<T>`** — a value that is `Universal` or `Adaptive { light, dark }`,
  resolved against the terminal/page `ColorMode`; composes with `TargetValue`
  as `TargetValue<PerMode<Color>>`
- **`Border`** — `color`, `weight` (`BorderWeight`), `line_style`
  (`BorderLineStyle`), `sides` (`BorderSides`), and `radius`
- **`Fill`** — painted-band behavior: `color`, `intensity` (`FillIntensity`),
  `band` (`FillBand`), and `inset`

`Style` rides on render-tree nodes via `NodeAttrs::set_style` / `style` (the
`renderable.style` hint namespace) and may attach to block nodes *and* inline
`Span` nodes. `Style`, `PerMode`, `Border`, `Fill`, and the emphasis leaves
all derive `serde` with `snake_case` casing. The Markdown renderer ignores
`Style` entirely, so Markdown output is unaffected by appearance.

### Darkmatter `style:` frontmatter

Darkmatter also has a document-level `style:` frontmatter pipeline built on
renderable primitives. This is separate from the render-tree `Style` attribute:
frontmatter is parsed by `darkmatter::style` and applied to `DarkmatterPage`
before terminal/HTML rendering.

Implemented wiring currently covers sub-specs 1 through 5 from
[`features/2026-05-23-style-property`](./features/2026-05-23-style-property/):
schema/parser, page layout, table/image/block-quote layout, `ul`/`ol`/`li`
layout, and page/component `color` / `bg-color`. The active phase is
`ACTIVE_STYLE_WIRING_SUB_SPEC = 5`; HR migration and bespoke knobs such as
`page.stylesheet`, `page.meta`, code theme defaults, and local link/image
style remain planned.

CLI precedence is field-level: invocation flags win over frontmatter. Use
`md --strict-style` to fail on unknown or deprecated `style:` keys while still
allowing valid future-phase keys to report as `KnownButInactive`.

### Markdown (`markdown.rs`)

The `MarkdownRenderable` trait and related types.

### Tree (`tree/`)

The canonical render tree and its target renderers.

- **`RenderNode`** / **`NodeKind`** / **`NodeAttrs`** — the owned node model
- **`Document`** / **`DocumentMetadata`** / **`Frontmatter`** — a tree plus its
  source registry and metadata
- **`SourceSpan`** / **`SourceLocation`** / **`SourceRegistry`** /
  **`SourceDescriptor`** / **`Provenance`** — origin tracking for each node
- **`TreeRenderable`** — the trait a component implements to produce a tree
- **`Diagnostic`** / **`Severity`** / **`Rendered<T>`** / **`RenderStrictness`**
  / **`RenderError`** — diagnostics and the strictness policy; `validate` /
  `ensure_valid` enforce structural invariants
- **Markdown tree renderer** — `render_markdown_node` / `render_markdown_document`
  with `MarkdownRenderOptions` and `MarkdownDialect` (`Markdown` /
  `MarkdownPlus`)
- **Browser tree renderer** — `render_browser_node` / `render_browser_document`
  with `BrowserRenderOptions` and `RawHtmlPolicy` (`Allow` / `Escape` /
  `Reject`). `render_browser_document_html` is the direct
  `Document` → final HTML `String` path: it streams the whole tree into one
  buffer (skipping a fragment per node) and emits bytes identical to
  `render_browser_document(..)?.output.render()`. Use it when you already own a
  `Document` and only need the final string; use `render_browser_document` when
  you need an `HtmlPage` or `BrowserFragment<Ready>` for further composition.

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

## Migrating a Component to the Render Tree IR

See [`docs/migrate-component-to-ir.md`](./docs/migrate-component-to-ir.md) for
the canonical recipe — both the flip-from-bespoke (Variant A) and
born-on-the-tree (Variant B) paths, escape-hatch rules, and the
documentation-update obligations a migration carries with it.

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
