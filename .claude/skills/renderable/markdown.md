---
description: Markdown rendering trait in the renderable library — MarkdownRenderable and style-aware Markdown output.
---

# Markdown Module

The `MarkdownRenderable` trait for components that render to Markdown output.

## MarkdownRenderable

```rust
pub trait MarkdownRenderable {
    /// Renders the component as a Markdown string.
    fn render_markdown(&self) -> String;

    /// Renders with an optional Stylesheet for style-aware output.
    /// Default ignores the stylesheet and delegates to render_markdown.
    fn render_markdown_with_style(&self, _style: Option<Stylesheet>) -> String {
        self.render_markdown()
    }
}
```

## Plain Markdown

Components that lower cleanly to ergonomic Markdown implement `render_markdown`:

```rust
impl MarkdownRenderable for MyComponent {
    fn render_markdown(&self) -> String {
        format!("## {}\n\n{}", self.title, self.body)
    }
}
```

## Style-Aware Markdown

Components that need richer styling can consume a `Stylesheet` and project Markdown-addressable rules into the output:

```rust
impl MarkdownRenderable for StyledComponent {
    fn render_markdown(&self) -> String {
        "# Hello\n\nBasic content".to_string()
    }

    fn render_markdown_with_style(&self, style: Option<Stylesheet>) -> String {
        let mut output = self.render_markdown();
        
        if let Some(sheet) = style {
            // Extract Markdown-addressable rules and inject
            // as inline HTML style attributes or classes
            output.push_str(&format!("\n\n<style>{}</style>", sheet.to_css()));
        }
        
        output
    }
}
```

## Markdown vs MarkdownPlus

- **Markdown** — standard CommonMark/GFM, most ergonomic for authors
- **MarkdownPlus** — same trait, but output includes more inline HTML for richer features

A component can detect which target is requested and adjust output accordingly:

```rust
impl MarkdownRenderable for MyComponent {
    fn render_markdown(&self) -> String {
        self.render_to_markdown(false)
    }

    fn render_markdown_with_style(&self, style: Option<Stylesheet>) -> String {
        let mut md = self.render_to_markdown(true); // MarkdownPlus mode
        
        if let Some(sheet) = style {
            // Inject stylesheet as inline HTML
        }
        
        md
    }
}
```
