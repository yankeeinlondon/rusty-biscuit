---
description: Typed HTML page assembly in the renderable library — HtmlPage construction, fragment composition, metadata, and rendering.
---

# HTML Module

Typed HTML page assembly and node construction.

## HtmlPage

A fully-assembled HTML page with `<head>` state.

```rust
use renderable::html::HtmlPage;
use renderable::browser::fragment::BrowserFragment;

// From a single fragment
let page = HtmlPage::from(fragment);

// From multiple fragments
let page = HtmlPage::from_fragments(vec![fragment1, fragment2]);

// Render to string
let html = page.render();
// <!DOCTYPE html><html><head>...</head><body>...</body></html>
```

### Page Assembly

```rust
use renderable::html::HtmlPage;
use renderable::browser::PageOptions;
use renderable::microdata::MicrodataKey;

let mut page = HtmlPage::from(fragment);

// Title fans out to HTML / OpenGraph / Twitter / Schema.org tags
page.set_title("My Page");

// Page-level metadata wins over component metadata on key conflict
page.add_metadata(MicrodataKey::Description, "A great page");

// Add external resources
page.add_link(LinkTag::stylesheet("style.css"));
page.add_script_block("console.log('hello');");

// Apply caller options (infallible)
page.apply_page_options(page_options);
```

### Render Order

The `render()` method produces `<head>` content in this order:

1. `<meta charset="utf-8">`
2. `<meta name="viewport" ...>`
3. `<title>` — from `Title` microdata key, else first `<h1>`, else empty
4. Microdata-driven meta tags (OpenGraph, Twitter, Schema.org)
5. `<link>` tags — deduped, page-level first
6. Stylesheet — external `<link>` or inline `<style>`
7. Script — external `<script src>` or inline `<script>`

## Fragment Composition

Fragments nest via the `Component` node variant. `HtmlPage::all_fragments()` recursively collects every fragment in document order, ensuring nested component auxiliary state (stylesheets, links, metadata, features) is not lost.

## Metadata Merging

- Component metadata is collected in document order with **first-write wins** semantics
- Page-level metadata is applied last and **overwrites** any component value

## Tag Types

### BlockTag

Typed HTML block elements (`<div>`, `<p>`, `<h1>`–`<h6>`, `<section>`, etc.).

### LinkTag

`<link>` tags with deduplication support via `(rel, href)` key.

### MetaTag

`<meta>` tags for non-microdata metadata.

## Attributes

- **ARIA** — typed ARIA attributes (`role`, `aria-label`, etc.)
- **Rel** — link relationship types (`stylesheet`, `icon`, `preload`, etc.)
- **CORS** — CORS policy enumeration
