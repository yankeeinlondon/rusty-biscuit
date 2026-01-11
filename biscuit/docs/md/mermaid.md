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
  /// xxHash using XXH32 variant (use `xxhash-rust` crate)
  /// utility is to act as a good cache key for the instructions
  /// content; the instructions should have all blank lines removed and
  /// then the hash is produced. This will ensure that regardless of the
  /// spacing that the instruction have to help users understand, they
  /// will  have NO IMPACT on the hash value
  hash: u32,
  /// a reference to the static content used in the HEAD section
  /// of the HTML to load in the JS library as well as
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

While the `Mermaid` crate creates the "capability" to render mermaid we haven't actually wired it up to anything yet.

Our first target will be the `md` CLI.

- we will add the CLI switch `--mermaid` will will render mermaid diagrams to the terminal (by default the mermaid diagrams will NOT be rendered but instead just be shows as a YAML document)
- any rendering to HTML with the `--html` or `--show-html` switches will render HTML which automatically renders the mermaid diagrams using the

## Accessibility Considerations

### Screen Reader Support

- **ARIA attributes**: Rendered SVGs must include `role="img"` and `aria-labelledby` pointing to a title element
- **Alt text generation**: The `Mermaid` struct should generate descriptive alt text from:
  1. Explicit `title` field if provided
  2. Diagram type detection (e.g., "Flowchart diagram", "Sequence diagram")
  3. Fallback: "Mermaid diagram"

### High Contrast Support

- Provide a `neutral` theme variant optimized for high contrast displays
- Ensure minimum 4.5:1 contrast ratio between text and background colors
- Border colors should be distinguishable from fill colors

### Terminal Rendering

- When rendering to terminal as images, provide text fallback option via `--mermaid-alt` flag
- Fallback displays simplified ASCII representation or the raw mermaid source in a code block

### Focus Management

- For HTML rendering with dynamic theme switching, preserve focus state during re-renders
- Avoid triggering unnecessary screen reader announcements on theme change

### Implementation Notes

```rust
impl Mermaid {
    /// Generate accessible alt text for the diagram
    fn alt_text(&self) -> String {
        if let Some(ref title) = self.title {
            return title.clone();
        }

        // Detect diagram type from first line of instructions
        let diagram_type = self.instructions
            .lines()
            .next()
            .map(|line| {
                if line.starts_with("flowchart") || line.starts_with("graph") {
                    "Flowchart"
                } else if line.starts_with("sequenceDiagram") {
                    "Sequence diagram"
                } else if line.starts_with("classDiagram") {
                    "Class diagram"
                } else if line.starts_with("stateDiagram") {
                    "State diagram"
                } else if line.starts_with("erDiagram") {
                    "Entity relationship diagram"
                } else if line.starts_with("pie") {
                    "Pie chart"
                } else if line.starts_with("gantt") {
                    "Gantt chart"
                } else {
                    "Mermaid diagram"
                }
            })
            .unwrap_or("Mermaid diagram");

        diagram_type.to_string()
    }
}
```
