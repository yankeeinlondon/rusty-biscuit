//! AST (Abstract Syntax Tree) output for Markdown documents.

use crate::markdown::{Markdown, MarkdownError, MarkdownResult};
use markdown::ParseOptions;

/// Converts a Markdown document to an MDAST (Markdown Abstract Syntax Tree).
///
/// The AST representation allows programmatic manipulation of the markdown
/// structure. This uses the `markdown` crate's MDAST implementation with
/// GitHub Flavored Markdown (GFM) extensions enabled.
///
/// ## Returns
///
/// Returns a `markdown::mdast::Node` on success, which is the root node of
/// the AST. The node can be serialized to JSON or manipulated programmatically.
///
/// ## Errors
///
/// Returns `MarkdownError::AstParse` if the content cannot be parsed into an AST.
///
/// ## Examples
///
/// ```
/// use shared::markdown::Markdown;
///
/// let md = Markdown::new("# Hello\n\nWorld".to_string());
/// let ast = md.as_ast().unwrap();
///
/// // AST can be serialized to JSON
/// let json = serde_json::to_string_pretty(&ast).unwrap();
/// assert!(json.contains("heading"));
/// ```
pub fn as_ast(md: &Markdown) -> MarkdownResult<markdown::mdast::Node> {
    // Use GFM options for full markdown feature support
    let options = ParseOptions::gfm();

    // Parse content to MDAST
    markdown::to_mdast(md.content(), &options).map_err(|e| MarkdownError::AstParse(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use markdown::mdast::Node;

    #[test]
    fn test_as_ast_basic() {
        let md = Markdown::new("# Hello World".to_string());
        let ast = as_ast(&md).unwrap();

        // Should be a Root node
        assert!(matches!(ast, Node::Root(_)));
    }

    #[test]
    fn test_as_ast_serializable() {
        let md = Markdown::new("# Header\n\nParagraph text.".to_string());
        let ast = as_ast(&md).unwrap();

        // Should be serializable to JSON
        let json = serde_json::to_string(&ast).unwrap();
        assert!(!json.is_empty());
    }

    #[test]
    fn test_as_ast_complex_structure() {
        let content = r#"# Title

## Subtitle

- Item 1
- Item 2

```rust
fn main() {}
```

[Link](https://example.com)"#;

        let md = Markdown::new(content.to_string());
        let ast = as_ast(&md).unwrap();

        // Verify it's a root node with children
        if let Node::Root(root) = ast {
            assert!(root.children.len() > 1);
        } else {
            panic!("Expected Root node");
        }
    }

    #[test]
    fn test_as_ast_empty_content() {
        let md = Markdown::new(String::new());
        let ast = as_ast(&md).unwrap();

        // Should still return a valid root node
        assert!(matches!(ast, Node::Root(_)));
    }

    #[test]
    fn test_as_ast_with_frontmatter() {
        let content = r#"---
title: Test
---
# Document"#;

        let md: Markdown = content.into();
        let ast = as_ast(&md).unwrap();

        // AST should only contain content, not frontmatter
        let json = serde_json::to_string(&ast).unwrap();
        assert!(json.contains("heading"));
        // Frontmatter should not appear in AST
        assert!(!json.contains("title: Test"));
    }

    #[test]
    fn test_as_ast_gfm_features() {
        // Test GitHub Flavored Markdown features
        let content = r#"# GFM Test

| Column 1 | Column 2 |
|----------|----------|
| Cell 1   | Cell 2   |

~~strikethrough~~

- [ ] Task 1
- [x] Task 2"#;

        let md = Markdown::new(content.to_string());
        let ast = as_ast(&md).unwrap();

        let json = serde_json::to_string(&ast).unwrap();

        // Should recognize table
        assert!(json.contains("table") || json.contains("Table"));
    }

    #[test]
    fn test_as_ast_preserves_structure() {
        let content = "# H1\n## H2\n### H3\n\nParagraph.";
        let md = Markdown::new(content.to_string());
        let ast = as_ast(&md).unwrap();

        if let Node::Root(root) = ast {
            // Should have multiple children representing different heading levels
            assert!(root.children.len() >= 4);
        } else {
            panic!("Expected Root node");
        }
    }

    #[test]
    fn test_as_ast_code_blocks() {
        let content = r#"```rust
fn main() {
    println!("Hello");
}
```"#;

        let md = Markdown::new(content.to_string());
        let ast = as_ast(&md).unwrap();

        let json = serde_json::to_string(&ast).unwrap();
        assert!(json.contains("code"));
        assert!(json.contains("rust"));
    }
}
