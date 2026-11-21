//! String output formatting for Markdown documents.

use crate::markdown::Markdown;
use biscuit_file::Indentation;

/// Converts a Markdown document to a string with frontmatter.
///
/// If the document has frontmatter, it will be serialized as YAML between
/// `---` delimiters, nested four spaces per level (the layout for output with
/// no destination file). The content follows after the frontmatter block.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::Markdown;
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
    as_string_with_indentation(md, Indentation::default())
}

/// [`as_string`] for a document saved to a file whose indentation preference
/// is `indentation` (see
/// [`destination_indentation`](crate::config::write::destination_indentation)).
///
/// The whole frontmatter is re-serialized, so authored comments and layout
/// are not kept. Frontmatter that cannot be serialized is written as an empty
/// block, as [`as_string`] does.
pub fn as_string_with_indentation(md: &Markdown, indentation: Indentation) -> String {
    if md.frontmatter().is_empty() {
        // No frontmatter, return just the content
        md.content().to_string()
    } else {
        // Serialize frontmatter as YAML
        let yaml = biscuit_file::to_yaml_string(md.frontmatter().as_map(), indentation)
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
        md.fm_insert("metadata", json!({"version": "1.0", "draft": false}))
            .unwrap();

        let output = as_string(&md);

        assert!(output.contains("title: Document"));
        assert!(output.contains("tags:"));
        assert!(output.contains("- rust"));
        assert!(output.contains("- markdown"));
        assert!(output.contains("metadata:"));
    }

    #[test]
    fn nested_frontmatter_is_laid_out_four_spaces_by_default() {
        let mut md = Markdown::new("Body\n".to_string());
        md.fm_insert("tags", json!(["rust", "markdown"])).unwrap();
        md.fm_insert("metadata", json!({"version": "1.0", "authors": ["a"]}))
            .unwrap();

        let output = as_string(&md);

        assert_eq!(
            output,
            "---\ntags:\n    - rust\n    - markdown\nmetadata:\n    version: '1.0'\n    authors:\n        - a\n---\nBody\n"
        );
        let reread: Markdown = output.into();
        assert_eq!(reread.frontmatter().as_map(), md.frontmatter().as_map());
    }

    #[test]
    fn an_explicit_indentation_sets_the_nesting_width() {
        let mut md = Markdown::new("Body\n".to_string());
        md.fm_insert("metadata", json!({"tags": ["a"]})).unwrap();
        let two = Indentation::spaces(std::num::NonZeroU8::new(2).unwrap());

        let output = as_string_with_indentation(&md, two);

        assert_eq!(output, "---\nmetadata:\n  tags:\n    - a\n---\nBody\n");
        let reread: Markdown = output.into();
        assert_eq!(reread.frontmatter().as_map(), md.frontmatter().as_map());
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
