---
description: AST rendering trait in the renderable library — AstRenderable for HAST-style abstract syntax trees.
---

# AST Module

The `AstRenderable` trait for components that render to an abstract syntax tree.

## AstRenderable

```rust
/// A component capable of rendering itself to an abstract syntax tree
/// representation.
pub trait AstRenderable {
    /// Renders the component to a serialized AST string.
    fn render_ast(&self) -> String;
}
```

## Status

This is a **placeholder surface**. The trait exists so AST becomes a first-class render target alongside `TerminalRenderable`, `BrowserRenderable`, and `MarkdownRenderable`.

The single method currently returns a **serialized AST string**. A typed node model is deferred until a concrete consumer needs it.

## HAST

AST uses the [HAST](https://github.com/syntax-tree/hast) (HTML Abstract Syntax Tree) style popularized by the JavaScript/TypeScript ecosystem, now made available in Rust via the [`markdown`](https://crates.io/crates/markdown) crate.

## Future Direction

The trait will likely evolve to return a typed node tree:

```rust
// Future API (not yet implemented)
pub trait AstRenderable {
    fn render_ast(&self) -> AstNode;
}

pub enum AstNode {
    Element {
        tag_name: String,
        properties: HashMap<String, String>,
        children: Vec<AstNode>,
    },
    Text(String),
    Comment(String),
}
```

## Current Usage

For now, implement `render_ast` to return a JSON-serialized HAST node:

```rust
use renderable::ast::AstRenderable;
use serde_json;

#[derive(Debug)]
struct MyComponent { text: String }

impl AstRenderable for MyComponent {
    fn render_ast(&self) -> String {
        serde_json::json!({
            "type": "element",
            "tagName": "p",
            "children": [
                { "type": "text", "value": &self.text }
            ]
        }).to_string()
    }
}
```
