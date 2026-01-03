//! Markdown document manipulation with frontmatter support.
//!
//! This module provides a `Markdown` struct that represents a markdown document
//! with optional YAML frontmatter. It supports:
//!
//! - Parsing frontmatter from markdown content
//! - Loading from strings, files, and URLs
//! - Typed frontmatter accessors
//! - Frontmatter merging with conflict resolution strategies
//!
//! ## Examples
//!
//! ```
//! use shared::markdown::Markdown;
//!
//! let content = r#"---
//! title: Hello World
//! author: Alice
//! ---
//! # My Document
//!
//! This is the content.
//! "#;
//!
//! let md: Markdown = content.into();
//! let title: Option<String> = md.fm_get("title").unwrap();
//! assert_eq!(title, Some("Hello World".to_string()));
//! ```

mod frontmatter;
mod types;
pub mod highlighting;
pub mod cleanup;
pub mod dsl;
pub mod inline;
pub mod output;

pub use frontmatter::{Frontmatter, MergeStrategy};
pub use types::{FrontmatterMap, MarkdownError, MarkdownResult};

use std::path::Path;
use url::Url;

/// A markdown document with frontmatter support.
#[derive(Debug, Clone, PartialEq)]
pub struct Markdown {
    frontmatter: Frontmatter,
    content: String,
}

impl Markdown {
    /// Creates a new markdown document with empty frontmatter.
    pub fn new(content: String) -> Self {
        Self {
            frontmatter: Frontmatter::new(),
            content,
        }
    }

    /// Creates a markdown document with frontmatter.
    pub fn with_frontmatter(frontmatter: Frontmatter, content: String) -> Self {
        Self {
            frontmatter,
            content,
        }
    }

    /// Loads a markdown document from a URL (async).
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # use shared::markdown::Markdown;
    /// # use url::Url;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let url = Url::parse("https://example.com/doc.md")?;
    /// let md = Markdown::from_url(&url).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_url(url: &Url) -> MarkdownResult<Self> {
        let content = reqwest::get(url.as_str()).await?.text().await?;
        Ok(content.into())
    }

    /// Gets a typed value from frontmatter.
    pub fn fm_get<T: serde::de::DeserializeOwned>(&self, key: &str) -> MarkdownResult<Option<T>> {
        self.frontmatter.get(key)
    }

    /// Inserts a value into frontmatter.
    pub fn fm_insert<T: serde::Serialize>(&mut self, key: &str, value: T) -> MarkdownResult<()> {
        self.frontmatter.insert(key, value)
    }

    /// Merges external data into frontmatter using the specified strategy.
    pub fn fm_merge_with<T: serde::Serialize>(
        &mut self,
        other: T,
        strategy: MergeStrategy,
    ) -> MarkdownResult<()> {
        self.frontmatter.merge_with(other, strategy)
    }

    /// Sets default values for missing frontmatter keys.
    pub fn fm_set_defaults<T: serde::Serialize>(&mut self, defaults: T) -> MarkdownResult<()> {
        self.frontmatter.set_defaults(defaults)
    }

    /// Returns a reference to the frontmatter.
    pub fn frontmatter(&self) -> &Frontmatter {
        &self.frontmatter
    }

    /// Returns a mutable reference to the frontmatter.
    pub fn frontmatter_mut(&mut self) -> &mut Frontmatter {
        &mut self.frontmatter
    }

    /// Returns a reference to the content (without frontmatter).
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Returns a mutable reference to the content.
    pub fn content_mut(&mut self) -> &mut String {
        &mut self.content
    }

    /// Cleans up markdown content by normalizing formatting.
    ///
    /// This method performs two main operations:
    /// 1. Injects blank lines between block elements (headers, paragraphs, code blocks, etc.)
    /// 2. Aligns table columns for visual consistency
    ///
    /// The cleanup operation mutates the content in place and returns a mutable
    /// reference to self for method chaining.
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::markdown::Markdown;
    ///
    /// let content = "# Header\nParagraph\n## Subheader";
    /// let mut md: Markdown = content.into();
    /// md.cleanup();
    /// // Content now has proper spacing between elements
    /// ```
    pub fn cleanup(&mut self) -> &mut Self {
        self.content = cleanup::cleanup_content(&self.content);
        self
    }

    /// Converts the markdown document to a string representation.
    ///
    /// If the document has frontmatter, it will be serialized as YAML between
    /// `---` delimiters. The content follows after the frontmatter block.
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::markdown::Markdown;
    ///
    /// let mut md = Markdown::new("# Hello".to_string());
    /// md.fm_insert("title", "Test").unwrap();
    ///
    /// let output = md.as_string();
    /// assert!(output.contains("title: Test"));
    /// assert!(output.contains("# Hello"));
    /// ```
    pub fn as_string(&self) -> String {
        output::as_string(self)
    }

    /// Converts the markdown document to an MDAST (Markdown Abstract Syntax Tree).
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
    pub fn as_ast(&self) -> MarkdownResult<markdown::mdast::Node> {
        output::as_ast(self)
    }

    /// Converts the markdown document to HTML with syntax highlighting.
    ///
    /// This method renders the markdown content as HTML, applying syntax highlighting
    /// to code blocks and prose elements based on the provided options.
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::markdown::Markdown;
    /// use shared::markdown::output::HtmlOptions;
    ///
    /// let md = Markdown::new("# Hello\n\nWorld".to_string());
    /// let html = md.as_html(HtmlOptions::default()).unwrap();
    /// assert!(html.contains("<h1>"));
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns an error if theme loading fails or highlighting encounters issues.
    pub fn as_html(&self, options: output::HtmlOptions) -> MarkdownResult<String> {
        output::as_html(self, options)
    }
}

impl From<String> for Markdown {
    fn from(content: String) -> Self {
        match frontmatter::parse_frontmatter(&content) {
            Ok((frontmatter, remaining_content)) => Self::with_frontmatter(frontmatter, remaining_content),
            Err(_) => Self::new(content),
        }
    }
}

impl From<&str> for Markdown {
    fn from(content: &str) -> Self {
        content.to_string().into()
    }
}

impl TryFrom<&Path> for Markdown {
    type Error = MarkdownError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let content = std::fs::read_to_string(path)?;
        Ok(content.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_markdown_from_string() {
        let content = r#"---
title: Test
---
# Hello"#;

        let md: Markdown = content.to_string().into();
        let title: Option<String> = md.fm_get("title").unwrap();
        assert_eq!(title, Some("Test".to_string()));
        assert!(md.content().contains("# Hello"));
    }

    #[test]
    fn test_markdown_from_str() {
        let content = "# Plain content";
        let md: Markdown = content.into();
        assert!(md.frontmatter().is_empty());
        assert_eq!(md.content(), "# Plain content");
    }

    #[test]
    fn test_markdown_from_path() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "---").unwrap();
        writeln!(file, "title: File Test").unwrap();
        writeln!(file, "---").unwrap();
        writeln!(file, "# Content").unwrap();

        let md = Markdown::try_from(file.path()).unwrap();
        let title: Option<String> = md.fm_get("title").unwrap();
        assert_eq!(title, Some("File Test".to_string()));
    }

    #[test]
    fn test_markdown_fm_merge() {
        let content = "---\ntitle: Original\n---\n# Test";
        let mut md: Markdown = content.into();

        let new_data = json!({"author": "Alice"});
        md.fm_merge_with(&new_data, MergeStrategy::ErrorOnConflict)
            .unwrap();

        let author: Option<String> = md.fm_get("author").unwrap();
        assert_eq!(author, Some("Alice".to_string()));
    }

    #[test]
    fn test_markdown_fm_defaults() {
        let content = "---\ntitle: Test\n---\n# Content";
        let mut md: Markdown = content.into();

        let defaults = json!({"title": "Default", "author": "Anonymous"});
        md.fm_set_defaults(&defaults).unwrap();

        let title: Option<String> = md.fm_get("title").unwrap();
        let author: Option<String> = md.fm_get("author").unwrap();
        assert_eq!(title, Some("Test".to_string()));
        assert_eq!(author, Some("Anonymous".to_string()));
    }

    #[test]
    fn test_markdown_content_access() {
        let content = "---\ntitle: Test\n---\n# Hello\nWorld";
        let mut md: Markdown = content.into();

        assert!(md.content().contains("# Hello"));

        *md.content_mut() = "New content".to_string();
        assert_eq!(md.content(), "New content");
    }

    #[test]
    fn test_markdown_frontmatter_mut() {
        let content = "# No frontmatter";
        let mut md: Markdown = content.into();

        md.frontmatter_mut().insert("title", json!("Added")).unwrap();
        let title: Option<String> = md.fm_get("title").unwrap();
        assert_eq!(title, Some("Added".to_string()));
    }

    #[test]
    fn test_cleanup_basic_spacing() {
        let content = "# Header\nParagraph";
        let mut md: Markdown = content.into();
        md.cleanup();

        let cleaned = md.content();
        assert!(cleaned.contains("Header"));
        assert!(cleaned.contains("Paragraph"));
    }

    #[test]
    fn test_cleanup_returns_self() {
        let content = "# Test";
        let mut md: Markdown = content.into();
        let result = md.cleanup();

        // Should return mutable reference for chaining
        result.fm_insert("title", "Test").unwrap();
        let title: Option<String> = md.fm_get("title").unwrap();
        assert_eq!(title, Some("Test".to_string()));
    }

    #[test]
    fn test_cleanup_preserves_frontmatter() {
        let content = "---\ntitle: Test\n---\n# Header\nContent";
        let mut md: Markdown = content.into();
        md.cleanup();

        let title: Option<String> = md.fm_get("title").unwrap();
        assert_eq!(title, Some("Test".to_string()));
        assert!(md.content().contains("Header"));
    }

    #[test]
    fn test_cleanup_method_chaining() {
        let content = "# Test";
        let mut md: Markdown = content.into();

        md.cleanup()
            .fm_insert("author", "Alice")
            .unwrap();

        let author: Option<String> = md.fm_get("author").unwrap();
        assert_eq!(author, Some("Alice".to_string()));
    }
}
