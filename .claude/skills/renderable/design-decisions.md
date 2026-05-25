---
description: Key design decisions and architecture of the renderable library — trait design, naming conventions, and migration notes.
---

# Design Decisions

## 1. No Legacy String Surface for BrowserRenderable

`render_html_fragment()` returns `BrowserFragment<Ready>`, not `String`. Callers compose fragments and call `HtmlPage::render()` for the final string. This enables:

- Stylesheet composition across nested components
- Metadata merging
- Dependency link deduplication
- Feature flag rollup

## 2. Typestate Pattern

`BrowserFragment<Ready>` is the universal "done" currency. Other states may be added in the future (e.g., `Building`, `Validated`).

## 3. PageOptions is Infallible

External asset path validation happens at `RelativeAssetPath::new()`, not during page assembly. Once a caller holds a `RelativeAssetPath`, `PageOptions` can be applied without `Result`.

## 4. Microdata-Driven Metadata

`HtmlPage::set_title()` writes the `Title` microdata key, which fans out to HTML/OpenGraph/Twitter/Schema.org tags. One code path, no dedicated `title` field.

Metadata merging rules:
- Component metadata: **first-write wins** in document order
- Page metadata: **overwrites** component values

## 5. Component Stylesheets are Scoped

`ComponentStylesheet` lowers internal selectors to descendant selectors under the component wrapper class:

```
ComponentStylesheet::new("table")
    .add("header", style)
    // produces: .table .header { ... }
```

## 6. Naming Scheme A

- `CssStyle` — a declaration block (`property: value` pairs)
- `CssRule` — a `(selector, CssStyle)` pair
- `Stylesheet` — a collection of `CssRule` entries
- `HtmlClassDefinition` / `ClassDefinition` — left alone (list of class names, not a stylesheet)

## 7. Layout Data is Target-Agnostic

The `Layout` struct lives in `renderable` and holds pure data (margins, alignment, etc.). Terminal-specific width application lives in `biscuit-terminal` as the `LayoutTerminalExt` extension trait.

## 8. Markdown and MarkdownPlus Share a Trait

Both targets use `MarkdownRenderable`. The difference is in output style — MarkdownPlus injects more inline HTML for richer features.

## Migration Notes

| Old | New | Location |
|-----|-----|----------|
| `Renderable` | `TerminalRenderable` | `biscuit-terminal` |
| `RenderableContent` | `RenderableTerminalContent` | `biscuit-terminal` |
| `BrowserRenderable` (old location) | `BrowserRenderable` | `renderable::browser` |
| `Stylesheet` (from darkmatter) | `CssStyle` / `Stylesheet` | `renderable::stylesheet` |
| `Layout` (from biscuit-terminal) | `Layout` | `renderable::layout` |
| `Color` (from biscuit-terminal) | `Color` | `renderable::color` |
