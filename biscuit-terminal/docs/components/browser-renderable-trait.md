# BrowserRenderable Trait

The `BrowserRenderable` trait provides a standardized interface for components that can render to browser-compatible output (HTML/SVG).

## Trait Definition

```rust
pub trait BrowserRenderable: std::fmt::Debug + Any {
    /// Renders the component to browser-compatible HTML/SVG.
    fn render_to_browser(&self) -> String;

    /// Renders the component to browser-compatible HTML/SVG with inline CSS variables.
    fn render_to_browser_with_inline_variables(
        &self,
        variables: &HashMap<String, String>,
    ) -> String;

    fn as_any(&self) -> &dyn Any;
}
```

## Methods

### render_to_browser

Renders the component to browser-compatible output without inline CSS variables. This method should produce valid HTML/SVG that can be embedded directly in web documents.

The output should use `currentColor` for stroke/fill colors to inherit from the parent element's text color, enabling proper theming.

### render_to_browser_with_inline_variables

Renders the component to browser-compatible output with inline CSS variables. This method is useful when the component needs to support dynamic theming or responsive sizing through CSS custom properties.

The output should include appropriate CSS variable definitions that can be overridden by parent stylesheets.

## Implementation Guidelines

When implementing `BrowserRenderable`:

1. **Use `currentColor`**: Always use `stroke="currentColor"` and `fill="currentColor"` for SVG elements to ensure proper CSS inheritance
2. **Support CSS variables**: Use CSS custom properties for dimensions, spacing, and other configurable properties
3. **Valid output**: Ensure the generated HTML/SVG is valid and can be parsed by standard browsers
4. **Accessibility**: Include appropriate ARIA attributes when relevant
5. **Performance**: Optimize SVG output to minimize file size while maintaining quality

## Example Implementation

```rust
impl BrowserRenderable for HorizontalRule {
    fn render_to_browser(&self) -> String {
        // Emit the default SVG with a style block that declares
        // --hr-weight / --hr-color / --hr-width so that
        // render_to_browser_with_inline_variables has a real override surface.
        format!(
            r#"<svg viewBox="0 0 100 10" xmlns="http://www.w3.org/2000/svg" \
  style="--hr-weight: {weight}; --hr-color: {color}; --hr-width: {width};">
  <path d="{path}" stroke="var(--hr-color, currentColor)" \
        stroke-width="var(--hr-weight, 4)" fill="none"/>
</svg>"#,
            weight = self.stroke_width(),
            color = self.color.as_deref().unwrap_or("currentColor"),
            width = self.width.as_deref().unwrap_or("100%"),
            path = self.path_data(),
        )
    }

    fn render_to_browser_with_inline_variables(
        &self,
        variables: &HashMap<String, String>,
    ) -> String {
        // Default output already contains `var(--hr-weight)` / `var(--hr-color)` /
        // `var(--hr-width)` tokens — substitute each caller-supplied override into
        // the matching `var(--…)` expression.
        let mut svg = self.render_to_browser();
        for (name, value) in variables {
            svg = svg.replace(&format!("var(--{name})"), value);
        }
        svg
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
```

### Variable Naming

The reference `HorizontalRule` implementation declares three CSS variables on the root `<svg>`:

| Variable      | Purpose                                     |
|---------------|---------------------------------------------|
| `--hr-weight` | Stroke width in pixels (derived from `RuleWeight`) |
| `--hr-color`  | Stroke/fill color (falls back to `currentColor`)   |
| `--hr-width`  | Total SVG width (falls back to `100%`)             |

Each variable is referenced via `var(--hr-…, <fallback>)` inside the SVG body so the document still renders when the inline style is stripped.

## Integration with Darkmatter

Components implementing `BrowserRenderable` can be seamlessly integrated into darkmatter's HTML rendering pipeline. The darkmatter HTML renderer automatically detects and uses the `BrowserRenderable` trait when available, falling back to other rendering strategies when not implemented.