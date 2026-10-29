# BrowserRenderable Trait

The `BrowserRenderable` trait provides a standardized interface for components that can render to browser-compatible output (HTML/SVG).

The trait is defined in the `renderable` crate (`renderable::browser::BrowserRenderable`) and re-exported by `biscuit_terminal::components::renderable`.

## Trait Definition

```rust
pub trait BrowserRenderable: std::fmt::Debug + Any {
    /// Produces a fully composed fragment for this component.
    fn render_html_fragment(&self) -> BrowserFragment<Ready>;

    /// Promotes this single component to a standalone page.
    fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage {
        let mut html_page = HtmlPage::from(self.render_html_fragment());
        if let Some(options) = page {
            html_page.apply_page_options(options);
        }
        html_page
    }

    fn as_any(&self) -> &dyn Any;
}
```

## Methods

### render_html_fragment

The single required method. It returns a `BrowserFragment<Ready>`, the composable "finished" form other components and pages embed. Call `.render()` on it to get the HTML string:

```rust
use biscuit_terminal::prelude::*;

let html = HorizontalRule::new().render_html_fragment().render();
```

Components that emit prebuilt markup (an SVG, third-party HTML) wrap it as a raw-HTML island inside the fragment; components built from typed nodes assemble a node tree.

### render_html_page

Has a default implementation: it builds an `HtmlPage` from `render_html_fragment()` and applies the optional `PageOptions`. It returns an `HtmlPage`, not a `String`; call `HtmlPage::render` for the final document. Implementors rarely override it.

### as_any

Returns `self` as `&dyn Any` so callers holding a `dyn BrowserRenderable` can downcast to the concrete component type.

## Implementation Guidelines

When implementing `BrowserRenderable`:

1. **Prefer the render tree**: if the component already implements `TreeRenderable`, delegate to `BrowserTreeComponent` (below) instead of hand-writing markup, so the browser output matches every other path that renders the same node
2. **Use `currentColor`**: default SVG stroke and fill to `currentColor` so the component inherits the parent element's text color
3. **Validate untrusted values**: never interpolate caller-supplied strings into HTML or CSS without validation
4. **Valid output**: ensure the generated HTML/SVG is valid and can be parsed by standard browsers
5. **Accessibility**: include appropriate ARIA attributes when relevant

## Example Implementation

`HorizontalRule` projects itself into the render tree, so its browser implementation is a delegation to the shared tree renderer:

```rust
use renderable::browser::fragment::{BrowserFragment, Ready};

use crate::components::renderable::BrowserRenderable;
use crate::render_tree::BrowserTreeComponent;

impl BrowserRenderable for HorizontalRule {
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        BrowserTreeComponent::new(self.clone()).render_html_fragment()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
```

`BrowserTreeComponent` calls the component's `render_tree()` and renders the resulting node with the browser tree renderer, including any outer `Layout` wrapper. See [HorizontalRule](horizontal-rule.md) for what that output contains.

### Variable Naming

The `HorizontalRule` browser SVG (drawn by the shared tree renderer) declares three CSS variables on the root `<svg>`:

| Variable      | Purpose                                     |
|---------------|---------------------------------------------|
| `--hr-weight` | Stroke width in pixels (derived from `RuleWeight`) |
| `--hr-color`  | Stroke/fill color (falls back to `currentColor`)   |
| `--hr-width`  | Total SVG width (falls back to `100%`)             |

Each variable is referenced via `var(--hr-…, <fallback>)` inside the SVG body so the document still renders when the inline style is stripped.

## Integration with Darkmatter

Components implementing `BrowserRenderable` can be seamlessly integrated into darkmatter's HTML rendering pipeline. The darkmatter HTML renderer automatically detects and uses the `BrowserRenderable` trait when available, falling back to other rendering strategies when not implemented.