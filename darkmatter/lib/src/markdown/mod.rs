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
//! use darkmatter::markdown::Markdown;
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

pub mod cleanup;
pub mod delta;
pub mod dsl;
mod frontmatter;
pub mod fs;
pub mod hash;
pub mod highlighting;
pub mod inline;
pub mod normalize;
pub mod output;
pub mod toc;
pub mod transform;
mod types;

pub use delta::{
    BrokenLink, ChangeAction, CodeBlockChange, ContentChange, DeltaStatistics, DocumentChange,
    FrontmatterChange, MarkdownDelta, MovedSection, SectionId, SectionPath,
};
pub use frontmatter::{Frontmatter, MergeStrategy};
pub use normalize::{
    HeadingAdjustment, HeadingLevel, NormalizationError, NormalizationReport, StructureIssue,
    StructureIssueKind, StructureValidation, ViolationCorrection,
};
pub use toc::{CodeBlockInfo, InternalLinkInfo, MarkdownToc, MarkdownTocNode};
pub use types::{FrontmatterMap, MarkdownError, MarkdownResult};

use std::path::Path;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use url::Url;

use crate::render::{ImageRef, Link};

/// A markdown document with frontmatter support.
#[derive(Debug, Clone, PartialEq)]
pub struct Markdown {
    frontmatter: Frontmatter,
    content: String,
}

impl Markdown {
    /// Creates a new markdown document with empty frontmatter.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            frontmatter: Frontmatter::new(),
            content: content.into(),
        }
    }

    /// Creates a markdown document with frontmatter.
    pub fn with_frontmatter(frontmatter: Frontmatter, content: impl Into<String>) -> Self {
        Self {
            frontmatter,
            content: content.into(),
        }
    }

    /// Loads a markdown document from a URL (async).
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # use darkmatter::markdown::Markdown;
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

    /// Consumes the markdown document and returns `(frontmatter, content)`.
    pub fn into_parts(self) -> (Frontmatter, String) {
        (self.frontmatter, self.content)
    }

    /// Extracts typed links from document content.
    ///
    /// Link display text is preserved as visible markdown text (including inline code
    /// markers) and link metadata from markdown titles is parsed via [`Link`].
    pub fn links(&self) -> Vec<Link> {
        let parser = Parser::new_ext(&self.content, markdown_parse_options());

        let mut links = Vec::new();
        let mut in_link = false;
        let mut current_href = String::new();
        let mut current_title = String::new();
        let mut current_display = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Link {
                    dest_url, title, ..
                }) => {
                    in_link = true;
                    current_href = dest_url.to_string();
                    current_title = title.to_string();
                    current_display.clear();
                }
                Event::End(TagEnd::Link) if in_link => {
                    let display = std::mem::take(&mut current_display);
                    let href = std::mem::take(&mut current_href);
                    let title = std::mem::take(&mut current_title);

                    if let Ok(link) = Link::with_title_parsed(display, href, &title) {
                        links.push(link);
                    }

                    in_link = false;
                }
                Event::Text(text) if in_link => {
                    current_display.push_str(&text);
                }
                Event::Code(code) if in_link => {
                    current_display.push('`');
                    current_display.push_str(&code);
                    current_display.push('`');
                }
                Event::SoftBreak if in_link => {
                    current_display.push(' ');
                }
                Event::HardBreak if in_link => {
                    current_display.push('\n');
                }
                _ => {}
            }
        }

        links
    }

    /// Extracts typed image references from document content.
    ///
    /// The extraction path round-trips through markdown image parsing so `ImageRef`
    /// behaviors (such as width hints in alt text and metadata title payloads) are
    /// applied consistently with standalone `ImageRef::try_from`.
    pub fn images(&self) -> Vec<ImageRef> {
        let parser = Parser::new_ext(&self.content, markdown_parse_options());

        let mut images = Vec::new();
        let mut in_image = false;
        let mut current_src = String::new();
        let mut current_title = String::new();
        let mut current_alt = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Image {
                    dest_url, title, ..
                }) => {
                    in_image = true;
                    current_src = dest_url.to_string();
                    current_title = title.to_string();
                    current_alt.clear();
                }
                Event::End(TagEnd::Image) if in_image => {
                    let alt = std::mem::take(&mut current_alt);
                    let src = std::mem::take(&mut current_src);
                    let title = std::mem::take(&mut current_title);

                    if let Some(image_ref) = image_ref_from_parts(&alt, &src, &title) {
                        images.push(image_ref);
                    }

                    in_image = false;
                }
                Event::Text(text) if in_image => {
                    current_alt.push_str(&text);
                }
                Event::Code(code) if in_image => {
                    current_alt.push('`');
                    current_alt.push_str(&code);
                    current_alt.push('`');
                }
                Event::SoftBreak if in_image => {
                    current_alt.push(' ');
                }
                Event::HardBreak if in_image => {
                    current_alt.push('\n');
                }
                _ => {}
            }
        }

        images
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
    /// use darkmatter::markdown::Markdown;
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

    /// Cleans up markdown content and enforces a consistent list indentation width.
    ///
    /// Each nested list level is normalized to `indent_size` spaces.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    ///
    /// let content = "- Parent\n  - Child";
    /// let mut md: Markdown = content.into();
    /// md.cleanup_with_indent(4);
    /// assert!(md.content().contains("\n    - Child"));
    /// ```
    pub fn cleanup_with_indent(&mut self, indent_size: usize) -> &mut Self {
        self.content = cleanup::cleanup_content_with_indent(&self.content, indent_size);
        self
    }

    /// Removes a heading section from the document by pattern.
    ///
    /// A "section" is the heading itself plus all content until the next heading
    /// at the same or shallower level (or end of document).
    ///
    /// ## Pattern format
    ///
    /// - `"## Title"` — exact match on heading level and title
    /// - `"## Title*"` — prefix match (title starts with text before `*`)
    /// - `"!prelude"` — content before the first heading of any level
    ///
    /// ## Returns
    ///
    /// `true` if a section was removed, `false` if no match was found.
    pub fn remove_section(&mut self, pattern: &str) -> bool {
        let pattern = pattern.trim();

        // Handle !prelude: remove content before the first heading
        if pattern == "!prelude" {
            let headings = normalize::extract_headings(&self.content);
            if headings.is_empty() {
                // No headings — entire document is prelude
                self.content.clear();
                return true;
            }
            let first_start = headings[0].byte_start;
            if first_start == 0 {
                return false; // No prelude content
            }
            self.content = self.content[first_start..].to_string();
            // Trim leading blank lines from the result
            self.content = self.content.trim_start_matches('\n').to_string();
            return true;
        }

        // Parse pattern: "## Title" or "## Title*"
        let (level, title_pattern, is_prefix) = match parse_heading_pattern(pattern) {
            Some(parsed) => parsed,
            None => return false,
        };

        let headings = normalize::extract_headings(&self.content);

        // Find matching heading
        let match_idx = headings
            .iter()
            .position(|h| h.level == level && heading_matches(&h.title, &title_pattern, is_prefix));

        let Some(idx) = match_idx else {
            return false;
        };

        let section_start = headings[idx].byte_start;
        let match_level = headings[idx].level;

        // Find end: next heading at same or shallower level
        let section_end = headings[idx + 1..]
            .iter()
            .find(|h| h.level <= match_level)
            .map(|h| h.byte_start)
            .unwrap_or(self.content.len());

        self.content = format!(
            "{}{}",
            &self.content[..section_start],
            &self.content[section_end..]
        );

        // Clean up double blank lines
        while self.content.contains("\n\n\n") {
            self.content = self.content.replace("\n\n\n", "\n\n");
        }

        true
    }

    /// Removes multiple heading sections by pattern.
    ///
    /// Patterns are applied in reverse document order to preserve byte offsets.
    /// Returns the number of sections successfully removed.
    pub fn remove_sections(&mut self, patterns: &[String]) -> usize {
        let mut count = 0;
        for pattern in patterns {
            if self.remove_section(pattern) {
                count += 1;
            }
        }
        count
    }

    /// Converts the markdown document to a string representation.
    ///
    /// If the document has frontmatter, it will be serialized as YAML between
    /// `---` delimiters. The content follows after the frontmatter block.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
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
    /// use darkmatter::markdown::Markdown;
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
    /// use darkmatter::markdown::Markdown;
    /// use darkmatter::markdown::output::HtmlOptions;
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

    /// Renders the markdown document as ANSI-styled terminal output.
    ///
    /// Returns a string containing ANSI escape codes for syntax highlighting,
    /// styled headings, and formatted block elements including inline images
    /// (via Kitty/iTerm2 protocols through biscuit-terminal).
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    /// use darkmatter::markdown::output::TerminalOptions;
    ///
    /// let md = Markdown::new("# Hello\n\nWorld".to_string());
    /// let rendered = md.as_terminal(TerminalOptions::default()).unwrap();
    /// assert!(rendered.contains("Hello"));
    /// assert!(rendered.contains("World"));
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns an error if terminal rendering fails (e.g. theme loading issues).
    pub fn as_terminal(&self, options: output::TerminalOptions) -> MarkdownResult<String> {
        output::for_terminal(self, options)
    }

    /// Extracts a Table of Contents from the markdown document.
    ///
    /// Returns a `MarkdownToc` struct containing:
    /// - Hierarchical heading structure with hashes
    /// - Code block information
    /// - Internal link tracking
    /// - Document metadata (title, preamble)
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    ///
    /// let content = "# Introduction\n\nWelcome.\n\n## Getting Started\n\nFirst steps.";
    /// let md: Markdown = content.into();
    /// let toc = md.toc();
    ///
    /// assert_eq!(toc.heading_count(), 2);
    /// assert_eq!(toc.root_level(), Some(1));
    /// assert_eq!(toc.title, Some("Introduction".to_string()));
    /// ```
    pub fn toc(&self) -> MarkdownToc {
        MarkdownToc::from(self)
    }

    /// Compares this document with another and returns a detailed delta analysis.
    ///
    /// Returns a `MarkdownDelta` struct containing:
    /// - High-level change classification
    /// - Statistics about additions, removals, modifications
    /// - Frontmatter changes
    /// - Section movements
    /// - Broken link detection
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    ///
    /// let original: Markdown = "# Hello\n\nWorld".into();
    /// let updated: Markdown = "# Hello\n\nUniverse".into();
    ///
    /// let delta = original.delta(&updated);
    ///
    /// if delta.is_unchanged() {
    ///     println!("No changes detected");
    /// } else {
    ///     println!("{}", delta.summary());
    /// }
    /// ```
    pub fn delta(&self, other: &Markdown) -> MarkdownDelta {
        delta::compute_delta(self, other)
    }

    /// Validates the document's heading structure.
    ///
    /// A document with no headings is trivially well-formed (there is no
    /// heading structure to violate). When headings are present, the document
    /// is well-formed when all of the following hold:
    ///
    /// - **No hierarchy violations** — no heading appears shallower than the
    ///   root level (the level of the first heading). For example, if the
    ///   document starts with H3, a later H2 is a violation.
    /// - **No skipped levels** — headings descend at most one level at a time.
    ///   For example, H2 followed directly by H4 (skipping H3) is an issue.
    /// - **At most one H1** — multiple H1 headings are flagged.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    /// use darkmatter::markdown::HeadingLevel;
    ///
    /// let doc: Markdown = "## Intro\n### Details\n## Conclusion".into();
    /// let validation = doc.validate_structure();
    ///
    /// assert!(validation.is_well_formed());
    /// assert_eq!(validation.root_level, Some(HeadingLevel::H2));
    /// assert_eq!(validation.heading_count, 3);
    /// ```
    pub fn validate_structure(&self) -> StructureValidation {
        normalize::validate_structure(&self.content)
    }

    /// Normalizes the document's heading levels.
    ///
    /// ## Parameters
    ///
    /// - `target`: The desired root level. If `None`, uses the current root level
    ///   (effectively just fixing hierarchy violations without changing depth).
    ///
    /// ## Behavior
    ///
    /// 1. **Level Adjustment**: All headings are shifted so the root level matches
    ///    the target. For example, normalizing an H3-rooted document to H1 promotes
    ///    all headings by 2 levels.
    ///
    /// 2. **Hierarchy Violation Correction**: If any heading appears at a level
    ///    shallower than the document's root, it is demoted to match the root level,
    ///    and its children are adjusted proportionally.
    ///
    /// ## Returns
    ///
    /// A tuple of the normalized `Markdown` document and a `NormalizationReport`
    /// describing all changes made.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The document has no headings (nothing to normalize)
    /// - Re-leveling would push headings beyond H6
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::{Markdown, HeadingLevel};
    ///
    /// // Promote an H3-rooted document to H1
    /// let doc: Markdown = "### Intro\n#### Details".into();
    /// let (normalized, report) = doc.normalize(Some(HeadingLevel::H1)).unwrap();
    ///
    /// assert!(normalized.content().starts_with("# Intro"));
    /// assert_eq!(report.level_adjustment, -2); // Promoted by 2 levels
    /// ```
    pub fn normalize(
        &self,
        target: Option<HeadingLevel>,
    ) -> Result<(Markdown, NormalizationReport), NormalizationError> {
        let (new_content, report) = normalize::normalize(&self.content, target)?;
        let new_md = Markdown::with_frontmatter(self.frontmatter.clone(), new_content);
        Ok((new_md, report))
    }

    /// Normalizes the document in place, returning only the report.
    ///
    /// This is a convenience method that modifies `self` instead of returning
    /// a new `Markdown` instance.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::{Markdown, HeadingLevel};
    ///
    /// let mut doc: Markdown = "### Intro\n#### Details".into();
    /// let report = doc.normalize_mut(Some(HeadingLevel::H1)).unwrap();
    ///
    /// assert!(doc.content().starts_with("# Intro"));
    /// assert!(report.has_changes());
    /// ```
    pub fn normalize_mut(
        &mut self,
        target: Option<HeadingLevel>,
    ) -> Result<NormalizationReport, NormalizationError> {
        let (new_content, report) = normalize::normalize(&self.content, target)?;
        self.content = new_content;
        Ok(report)
    }

    /// Re-levels the document to a specific target level.
    ///
    /// This is a simpler operation than `normalize()` - it only shifts all
    /// heading levels uniformly without correcting hierarchy violations.
    ///
    /// ## Parameters
    ///
    /// - `target`: The desired root level for the document.
    ///
    /// ## Returns
    ///
    /// A tuple of the re-leveled `Markdown` document and the level adjustment
    /// that was applied (positive = demoted, negative = promoted).
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The document has no headings
    /// - Re-leveling would push headings beyond H6
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::{Markdown, HeadingLevel};
    ///
    /// // Demote an H1-rooted document to H2 (for embedding as a subsection)
    /// let doc: Markdown = "# Main\n## Sub\n### Detail".into();
    /// let (releveled, adjustment) = doc.relevel(HeadingLevel::H2).unwrap();
    ///
    /// assert!(releveled.content().starts_with("## Main"));
    /// assert_eq!(adjustment, 1); // Demoted by 1 level
    /// ```
    pub fn relevel(&self, target: HeadingLevel) -> Result<(Markdown, i8), NormalizationError> {
        let (new_content, adjustment) = normalize::relevel(&self.content, target)?;
        let new_md = Markdown::with_frontmatter(self.frontmatter.clone(), new_content);
        Ok((new_md, adjustment))
    }
}

impl From<String> for Markdown {
    fn from(content: String) -> Self {
        match frontmatter::parse_frontmatter(&content) {
            Ok((frontmatter, remaining_content)) => {
                Self::with_frontmatter(frontmatter, remaining_content)
            }
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

fn markdown_parse_options() -> Options {
    Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH
}

fn image_ref_from_parts(alt: &str, src: &str, title: &str) -> Option<ImageRef> {
    let markdown = build_markdown_image_literal(alt, src, title);

    if let Ok(image_ref) = ImageRef::try_from(markdown.as_str()) {
        return Some(image_ref);
    }

    let mut image_ref = ImageRef::new(src, alt).ok()?;
    if !title.trim().is_empty() {
        image_ref = image_ref.with_title(title);
    }

    Some(image_ref)
}

fn build_markdown_image_literal(alt: &str, src: &str, title: &str) -> String {
    let alt = escape_markdown_image_alt(alt);
    let src = escape_markdown_image_url(src);

    if title.trim().is_empty() {
        return format!("![{alt}]({src})");
    }

    let title = escape_markdown_title(title);
    format!("![{alt}]({src} \"{title}\")")
}

fn escape_markdown_image_alt(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn escape_markdown_image_url(value: &str) -> String {
    value.replace('(', "%28").replace(')', "%29")
}

fn escape_markdown_title(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Parses a heading pattern like `"## Title"` or `"## Title*"`.
///
/// Returns `(level, title, is_prefix)`.
fn parse_heading_pattern(pattern: &str) -> Option<(normalize::HeadingLevel, String, bool)> {
    let trimmed = pattern.trim();
    if !trimmed.starts_with('#') {
        return None;
    }

    let hash_count = trimmed.chars().take_while(|&c| c == '#').count();
    let level = normalize::HeadingLevel::new(hash_count as u8)?;

    let title_part = trimmed[hash_count..].trim();
    if title_part.is_empty() {
        return None;
    }

    let is_prefix = title_part.ends_with('*');
    let title = if is_prefix {
        title_part[..title_part.len() - 1].to_string()
    } else {
        title_part.to_string()
    };

    Some((level, title, is_prefix))
}

/// Checks if a heading title matches a pattern.
fn heading_matches(title: &str, pattern: &str, is_prefix: bool) -> bool {
    if is_prefix {
        title.starts_with(pattern)
    } else {
        title == pattern
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

        md.frontmatter_mut()
            .insert("title", json!("Added"))
            .unwrap();
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

        md.cleanup().fm_insert("author", "Alice").unwrap();

        let author: Option<String> = md.fm_get("author").unwrap();
        assert_eq!(author, Some("Alice".to_string()));
    }

    #[test]
    fn test_cleanup_with_indent_method() {
        let content = "- Parent\n  - Child\n    - Grandchild";
        let mut md: Markdown = content.into();

        md.cleanup_with_indent(4);

        assert!(md.content().contains("\n    - Child"));
        assert!(md.content().contains("\n        - Grandchild"));
    }

    #[test]
    fn test_markdown_into_parts() {
        let mut md = Markdown::new("# Hi");
        md.fm_insert("title", "Doc").unwrap();

        let (frontmatter, content) = md.into_parts();
        let title: Option<String> = frontmatter.get("title").unwrap();

        assert_eq!(title, Some("Doc".to_string()));
        assert_eq!(content, "# Hi");
    }

    #[test]
    fn test_markdown_links_extract_structured_metadata() {
        let md: Markdown =
            r#"[Click](https://example.com "class='btn' prompt='Read docs'")"#.into();
        let links = md.links();

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].display(), "Click");
        assert_eq!(links[0].href(), "https://example.com");
        assert_eq!(links[0].class(), Some("btn"));
        assert_eq!(links[0].prompt(), Some("Read docs"));
    }

    #[test]
    fn test_markdown_images_extract_width_hint() {
        let md: Markdown = "![Diagram|50%](./diagram.png)".into();
        let images = md.images();

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].alt(), "Diagram");
        assert_eq!(images[0].src(), Some("./diagram.png"));

        let html = images[0].to_html();
        assert!(
            html.contains(r#"style="width: 50%;""#),
            "expected width hint in style, got: {html}"
        );
    }

    #[test]
    fn test_markdown_images_preserve_inline_code_in_alt() {
        let md: Markdown = "![Use `cargo test` here](./diagram.png)".into();
        let images = md.images();

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].alt(), "Use `cargo test` here");
    }

    // ============================================
    // remove_section tests
    // ============================================

    #[test]
    fn remove_section_by_exact_heading() {
        let content = "## Foo\n\nFoo body.\n\n## Bar\n\nBar body.";
        let mut md: Markdown = content.into();

        assert!(md.remove_section("## Foo"));
        assert!(!md.content().contains("Foo"));
        assert!(md.content().contains("## Bar"));
        assert!(md.content().contains("Bar body."));
    }

    #[test]
    fn remove_section_by_prefix_wildcard() {
        let content = "## Foobar\n\nContent.\n\n## Other\n\nKept.";
        let mut md: Markdown = content.into();

        assert!(md.remove_section("## Foo*"));
        assert!(!md.content().contains("Foobar"));
        assert!(md.content().contains("## Other"));
    }

    #[test]
    fn remove_section_preserves_sibling() {
        let content = "## A\n\nA content.\n\n## B\n\nB content.\n\n## C\n\nC content.";
        let mut md: Markdown = content.into();

        assert!(md.remove_section("## B"));
        assert!(md.content().contains("## A"));
        assert!(md.content().contains("A content."));
        assert!(!md.content().contains("## B"));
        assert!(!md.content().contains("B content."));
        assert!(md.content().contains("## C"));
        assert!(md.content().contains("C content."));
    }

    #[test]
    fn remove_section_removes_nested_children() {
        let content = "## A\n\n### A1\n\nNested.\n\n## B\n\nKept.";
        let mut md: Markdown = content.into();

        assert!(md.remove_section("## A"));
        assert!(!md.content().contains("## A"));
        assert!(!md.content().contains("### A1"));
        assert!(!md.content().contains("Nested."));
        assert!(md.content().contains("## B"));
    }

    #[test]
    fn remove_prelude() {
        let content = "Some prelude text.\n\n## First Heading\n\nBody.";
        let mut md: Markdown = content.into();

        assert!(md.remove_section("!prelude"));
        assert!(!md.content().contains("prelude text"));
        assert!(md.content().contains("## First Heading"));
        assert!(md.content().contains("Body."));
    }

    #[test]
    fn remove_section_no_match_returns_false() {
        let content = "## Existing\n\nContent.";
        let mut md: Markdown = content.into();

        assert!(!md.remove_section("## Missing"));
        assert_eq!(md.content(), content);
    }

    #[test]
    fn remove_sections_multiple() {
        let content = "## A\n\nA body.\n\n## B\n\nB body.\n\n## C\n\nC body.";
        let mut md: Markdown = content.into();

        let count = md.remove_sections(&["## A".to_string(), "## C".to_string()]);
        assert_eq!(count, 2);
        assert!(!md.content().contains("## A"));
        assert!(md.content().contains("## B"));
        assert!(!md.content().contains("## C"));
    }
}
