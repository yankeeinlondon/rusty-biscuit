# Mermaid Diagrams

## Two Approaches and a Hybrid

When looking to support [Mermaid](https://mermaid.js.org) diagrams as part of the Markdown pipelining process there are two broad stroke means to do this:

1. Integrate with `Mermaid.js`
2. Render Diagram to an Image


## Research

- [Mermaid Theming](./mermaid-theming.md)
- [Mermaid to Image](./mermaid-to-image.md)

## Implementation Approach

### Step 1

Create a **mermaid** module in the shared library and add a `Mermaid` struct:

```rust
pub struct MermaidTheme {
  background: Color;
  // ... other named colors
}


pub struct Mermaid {
  /// the Mermaid instructions found in
  instructions: String,
  /// theme for light and dark mode
  theme: (MermaidTheme, MermaidTheme),
  title: Option<String>,
  footer: Option<String>,
  hash: u32,
  html_header: &'static str
}
```

The **Mermaid** struct should:

- implement the `From` trait to that instructions can be brought in from any string like type (aka, extends Into<String>)
- implements the `Default` trait which provides a simple flow diagram example for the instructions.
- implements the following functions as part of a builder pattern:
    - `withTheme(theme: (MermaidTheme, MermaidTheme)) -> this`
    - `withTitle<T: Into<String>>(title: T) -> this`
    - `withFooter<T: Into<String>>(footer: T) -> this`
    - `useSyntectTheme(theme: Theme) -> this`
- implements the following functions for rendering
    - `renderForHtml() -> (String, String)`

        - Produces two HTML strings:
            1. The string content for the HTML page's header. This adds the inline Javascript as well as the Style definitions.
                - all styles should leverage CSS variable abstractions
            2. The string content for the HTML which will render the mermaid diagram

    - `renderForTerminal()`

        - read the [mermaid-to-image]("./mermaid-to-image.md") documents for more details on the approach we take

- the `MermaidTheme` struct should:
    - implement `TryFrom` trait and take a `String`, `&str`, or json-serde `Value` and treat it as JSON object to create the theme.
- we should statically define a set of themes inside a `Lazy_Static {}` block

> **Note:** the header content we inject into ANY page which uses the `Mermaid` struct will be the same:
>
> - it will have all the themes embedded as CSS variables
> - the CSS classes which the Mermaid renderer will use

### Step 2

While the `Mermaid` crate creates the "capability"

