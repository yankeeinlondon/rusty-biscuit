//! Output formatting for Markdown documents.
//!
//! This module provides methods to convert Markdown documents to different output formats:
//! - String representation with frontmatter
//! - Terminal output with ANSI escape codes
//! - HTML with syntax highlighting
//! - MDAST (Markdown Abstract Syntax Tree) for programmatic manipulation
//!
//! ## Examples
//!
//! ```
//! use darkmatter_lib::markdown::Markdown;
//!
//! let content = r#"---
//! title: Hello
//! ---
//! # Document
//! "#;
//!
//! let md: Markdown = content.into();
//! let output = md.as_string();
//! assert!(output.contains("title: Hello"));
//! ```

mod ast;
pub mod html;
mod string;
pub mod terminal;

pub use ast::as_ast;
pub use html::{HtmlOptions, as_html};
pub use string::as_string;
pub use terminal::{
    ColorDepth, ImageRenderer, ItalicMode, MermaidMode, TerminalOptions, for_terminal,
    write_terminal,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::Markdown;

    #[test]
    fn test_roundtrip_with_frontmatter() {
        let original = r#"---
title: Test
author: Alice
---
# Hello World

This is content."#;

        let md: Markdown = original.into();
        let output = as_string(&md);

        // Should contain frontmatter
        assert!(output.contains("title: Test"));
        assert!(output.contains("author: Alice"));
        assert!(output.contains("# Hello World"));
    }

    #[test]
    fn test_roundtrip_without_frontmatter() {
        let original = "# Plain Document\n\nNo frontmatter.";
        let md: Markdown = original.into();
        let output = as_string(&md);

        // Should not have frontmatter delimiters
        assert!(!output.starts_with("---"));
        assert!(output.contains("# Plain Document"));
    }
}
