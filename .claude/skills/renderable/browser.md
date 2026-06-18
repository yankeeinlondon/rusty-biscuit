---
description: Browser-target rendering in the renderable library — BrowserRenderable trait, BrowserFragment, HtmlPage, ComponentStylesheet, and page assembly.
---

# Browser Module

Browser-target rendering infrastructure.

## BrowserRenderable

The core trait for components that render to HTML.

```rust
pub trait BrowserRenderable: std::fmt::Debug + Any {
    /// Produces a fully-composed BrowserFragment<Ready>.
    fn render_html_fragment(&self) -> BrowserFragment<Ready>;

    /// Promotes this component to a standalone HtmlPage.
    fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage { ... }

    /// Enables downcasting to the concrete component type.
    fn as_any(&self) -> &dyn Any;
}
```

- `render_html_fragment` is the single required method. It returns `BrowserFragment<Ready>` — the universal "done" currency for composition.
- `render_html_page` has a default impl that builds an `HtmlPage` from the fragment and applies optional `PageOptions`.
- No legacy string-producing surface exists (see [design decisions](./design-decisions.md)).

## BrowserFragment<Ready>

A fragment carries everything needed for HTML composition:

- **Composable node tree** — the actual HTML structure
- **Component stylesheet** — scoped CSS rules
- **Dependency links** — external resources (`<link>` tags)
- **Metadata** — microdata key/value pairs
- **Features** — page-level capability flags (e.g. needs Mermaid JS)

```rust
use renderable::browser::fragment::{BrowserFragment, ComposableNode, Ready};

let node = ComposableNode::text("Hello, world!");
let fragment = BrowserFragment::new(node)
    .with_title("My Component")
    .with_stylesheet(component_sheet);
```

## ComponentStylesheet

Scoped CSS owned by a component. Internal selectors are lowered to descendant selectors.

```rust
use renderable::browser::ComponentStylesheet;
use renderable::stylesheet::{CssStyle, CssSizingProp, CssSizing};

let sheet = ComponentStylesheet::new("my-component")
    .add("header", CssStyle::new()
        .add(CssSizingProp::Width, CssSizing::Percent(100.0)));

// as_stylesheet() produces:
// .my-component .header { width: 100%; }
```

## PageOptions

Caller-supplied options for page assembly. Infallible — external asset paths are validated at construction time via `RelativeAssetPath`.

```rust
use renderable::browser::{PageOptions, RelativeAssetPath};

let options = PageOptions {
    stylesheet: Some(page_stylesheet),
    css_variables: Some(vec![("primary".into(), "#336699".into())]),
    external_stylesheet: Some(RelativeAssetPath::new("styles.css")?),
    external_code: Some(RelativeAssetPath::new("app.js")?),
};
```

## RelativeAssetPath

Guarantees that external `<link href>` and `<script src>` paths are relative, keeping emitted HTML portable across hosting locations.

```rust
use renderable::browser::RelativeAssetPath;

let path = RelativeAssetPath::new("assets/style.css")?; // ok
let bad = RelativeAssetPath::new("/absolute/path.css");  // Err(AbsoluteAssetPath)
```

## Implementing the Trait

```rust
use renderable::browser::{BrowserRenderable, BrowserFragment, Ready};
use renderable::browser::fragment::ComposableNode;
use std::any::Any;

#[derive(Debug)]
struct MyComponent { text: String }

impl BrowserRenderable for MyComponent {
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        let node = ComposableNode::text(&self.text);
        BrowserFragment::new(node)
    }

    fn as_any(&self) -> &dyn Any { self }
}
```
