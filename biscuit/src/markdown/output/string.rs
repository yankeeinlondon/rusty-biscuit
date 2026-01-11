//! String output formatting for Markdown documents.

use crate::markdown::Markdown;

/// Converts a Markdown document to a string with frontmatter.
///
/// If the document has frontmatter, it will be serialized as YAML between
/// `---` delimiters. The content follows after the frontmatter block.
///
/// ## Examples
///
/// ```
/// use shared::markdown::Markdown;
/// use serde_json::json;
///
/// let mut md = Markdown::new("# Hello".to_string());
/// md.fm_insert("title", "Test").unwrap();
///
/// let output = md.as_string();
/// assert!(output.contains("title: Test"));
/// assert!(output.contains("# Hello"));
/// ```
pub fn as_string(md: &Markdown) -> String {
    if md.frontmatter().is_empty() {
        // No frontmatter, return just the content
        md.content().to_string()
    } else {
        // Serialize frontmatter as YAML
        let yaml = serde_yaml::to_string(md.frontmatter().as_map())
            .unwrap_or_else(|_| String::new());

        // Build the full document with frontmatter delimiters
        let mut output = String::new();
        output.push_str("---\n");
        output.push_str(&yaml);

        // Only add closing delimiter if there's actual YAML content
        if !yaml.trim().is_empty() {
            output.push_str("---\n");
        }

        output.push_str(md.content());
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_as_string_no_frontmatter() {
        let md = Markdown::new("# Hello World\n\nContent here.".to_string());
        let output = as_string(&md);

        assert_eq!(output, "# Hello World\n\nContent here.");
        assert!(!output.contains("---"));
    }

    #[test]
    fn test_as_string_with_frontmatter() {
        let mut md = Markdown::new("# Document\n\nText.".to_string());
        md.fm_insert("title", "Test").unwrap();
        md.fm_insert("author", "Alice").unwrap();

        let output = as_string(&md);

        assert!(output.starts_with("---\n"));
        assert!(output.contains("title: Test"));
        assert!(output.contains("author: Alice"));
        assert!(output.contains("# Document"));
    }

    #[test]
    fn test_as_string_preserves_content() {
        let content = "# Header\n\n- List item 1\n- List item 2\n\n```rust\nfn main() {}\n```";
        let md = Markdown::new(content.to_string());
        let output = as_string(&md);

        assert_eq!(output, content);
    }

    #[test]
    fn test_as_string_complex_frontmatter() {
        let mut md = Markdown::new("Content".to_string());
        md.fm_insert("title", "Document").unwrap();
        md.fm_insert("tags", json!(["rust", "markdown"])).unwrap();
        md.fm_insert("metadata", json!({"version": "1.0", "draft": false})).unwrap();

        let output = as_string(&md);

        assert!(output.contains("title: Document"));
        assert!(output.contains("tags:"));
        assert!(output.contains("- rust"));
        assert!(output.contains("- markdown"));
        assert!(output.contains("metadata:"));
    }

    #[test]
    fn test_as_string_roundtrip() {
        let original = r#"---
title: Test
author: Bob
---
# My Document

This is some content."#;

        let md: Markdown = original.into();
        let output = as_string(&md);

        // Parse the output again
        let md2: Markdown = output.into();

        let title: Option<String> = md2.fm_get("title").unwrap();
        let author: Option<String> = md2.fm_get("author").unwrap();

        assert_eq!(title, Some("Test".to_string()));
        assert_eq!(author, Some("Bob".to_string()));
        assert!(md2.content().contains("# My Document"));
    }
}
