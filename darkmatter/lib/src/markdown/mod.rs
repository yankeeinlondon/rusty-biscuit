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

pub mod block;
pub mod cleanup;
pub mod code_block;
pub mod compose;
pub mod delta;
pub mod dsl;
pub mod errors;
mod frontmatter;
pub mod fs;
pub mod hash;
pub mod highlighting;
pub mod inline;
mod inline_html;
pub mod language_grammar;
pub mod normalize;
pub mod output;
pub mod reference;
pub mod render_tree;
pub mod schemas;
pub mod span;
pub mod toc;
mod types;
pub mod yaml_block;

pub use delta::{
    BrokenLink, ChangeAction, CodeBlockChange, ContentChange, DeltaStatistics, DocumentChange,
    FrontmatterChange, MarkdownDelta, MovedSection, SectionId, SectionPath,
};
pub use code_block::{CodeBlock, CodeBlockError};
pub use frontmatter::{
    Frontmatter, FrontmatterExtraction, MergeStrategy, extract_frontmatter_block,
};
pub use hash::{
    ComputedHash, DetailedValue, FmHashPair, MdHashKind, MdHashOptions, ParseMdHashKindError,
    SectionTuple,
};
pub use language_grammar::{LanguageGrammar, LanguageGrammarError};
pub use normalize::{
    HeadingAdjustment, HeadingLevel, NormalizationError, NormalizationReport, StructureIssue,
    StructureIssueKind, StructureValidation, ViolationCorrection,
};
pub use reference::file_tree::{FileTree, FileTreeError};
pub use reference::{
    DependencyMismatchKind, ReferenceError, ReferenceGraph, ReferenceGraphMismatchError,
    ReferenceGraphMismatchKind, ReferenceGraphOptions, ReferenceKind, ReferenceRecord,
    ReferenceSet, TransclusionRef, extract_document_references,
};
#[allow(deprecated)]
pub use render_tree::TerminalCodeRenderer;
pub use span::{SourceSpan, Spanned, line_col_of_offset, line_of_offset};
pub use toc::{
    CodeBlockInfo, HeadingRecord, InternalLinkInfo, MarkdownToc, MarkdownTocNode,
    extract_headings, generate_heading_slug,
};
pub use types::{FrontmatterMap, MarkdownError, MarkdownResult, SourceRef};
#[allow(deprecated)]
pub use yaml_block::{YamlBlock, YamlBlockError};

use std::path::Path;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use url::Url;

use crate::render::{ImageRef, Link};
use compose::ComposeSource;

/// A markdown document with frontmatter support.
#[derive(Debug, Clone, PartialEq)]
pub struct Markdown {
    frontmatter: Frontmatter,
    content: String,
    source: Option<ComposeSource>,
}

impl Markdown {
    /// Creates a new markdown document with empty frontmatter.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            frontmatter: Frontmatter::new(),
            content: content.into(),
            source: None,
        }
    }

    /// Creates a markdown document with frontmatter.
    pub fn with_frontmatter(frontmatter: Frontmatter, content: impl Into<String>) -> Self {
        Self {
            frontmatter,
            content: content.into(),
            source: None,
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
        Self::try_from_content(content)
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

    /// Returns the compose source, if known.
    pub fn source(&self) -> &Option<ComposeSource> {
        &self.source
    }

    /// Build a [`SourceContext`] for error rendering from the document's
    /// source and content.
    pub fn source_context_for_errors(&self) -> biscuit_terminal::errors::SourceContext {
        use std::sync::Arc;
        let content = Arc::from(self.content.as_str());
        match &self.source {
            Some(ComposeSource::File(path)) => {
                let absolute = path.canonicalize().unwrap_or_else(|_| path.clone());
                biscuit_terminal::errors::SourceContext::new(absolute, path.clone(), content)
            }
            _ => biscuit_terminal::errors::SourceContext::new(
                std::path::PathBuf::from("unknown"),
                std::path::PathBuf::from("unknown"),
                content,
            ),
        }
    }

    /// Build a [`SourceContext`] that includes the full file content
    /// (frontmatter + body) with the frontmatter byte range set.
    ///
    /// This is the coordinate space shell-expansion errors report into:
    /// `excerpt_prose` uses file-relative lines and `frontmatter_prose`
    /// renders the composed frontmatter block.
    pub fn full_source_context_for_errors(&self) -> biscuit_terminal::errors::SourceContext {
        use std::sync::Arc;
        let (content, frontmatter) = self.reconstruct_source();
        let content = Arc::from(content.as_str());
        match &self.source {
            Some(ComposeSource::File(path)) => {
                let absolute = path.canonicalize().unwrap_or_else(|_| path.clone());
                biscuit_terminal::errors::SourceContext::with_frontmatter(
                    absolute,
                    path.clone(),
                    content,
                    frontmatter,
                )
            }
            _ => biscuit_terminal::errors::SourceContext::with_frontmatter(
                std::path::PathBuf::from("unknown"),
                std::path::PathBuf::from("unknown"),
                content,
                frontmatter,
            ),
        }
    }

    /// Number of source lines occupied by the frontmatter block, including
    /// both `---` delimiters.
    ///
    /// Returns `0` when the document has no parsed frontmatter (e.g.
    /// programmatically constructed documents without a raw source snapshot).
    pub fn frontmatter_line_count(&self) -> usize {
        match self.frontmatter.raw_source() {
            Some("") => 2,
            Some(raw) => 2 + raw.lines().count(),
            None => 0,
        }
    }

    /// Reconstruct the full source text (frontmatter + body) and the byte
    /// range of the frontmatter block, when it can be recovered from the
    /// parsed snapshot.
    fn reconstruct_source(&self) -> (String, Option<std::ops::Range<usize>>) {
        match self.frontmatter.raw_source() {
            Some(raw) => {
                let prefix = if raw.is_empty() {
                    "---\n---\n".to_string()
                } else {
                    format!("---\n{raw}\n---\n")
                };
                let full = format!("{prefix}{}", self.content);
                let end = prefix.len();
                (full, Some(0..end))
            }
            None => (self.content.clone(), None),
        }
    }

    /// Sets the compose source, returning the modified document.
    pub fn with_source(mut self, source: ComposeSource) -> Self {
        self.source = Some(source);
        self
    }

    /// Consumes the markdown document and returns `(frontmatter, content)`.
    pub fn into_parts(self) -> (Frontmatter, String) {
        (self.frontmatter, self.content)
    }

    /// Parses markdown content into a [`Markdown`], surfacing frontmatter
    /// errors instead of silently dropping them.
    ///
    /// Prefer this over the infallible [`From<String>`] / [`From<&str>`]
    /// conversions when you need to know whether the source contained
    /// malformed frontmatter (for example, in CLI loaders that should error
    /// out instead of returning a document with empty frontmatter).
    ///
    /// ## Errors
    ///
    /// Returns [`MarkdownError::FrontmatterParse`] if the YAML between the
    /// leading `---` markers fails to parse. Documents without frontmatter
    /// are still accepted and returned with an empty [`Frontmatter`].
    pub fn try_from_content(content: impl Into<String>) -> MarkdownResult<Self> {
        let content = content.into();
        let ctx = biscuit_terminal::errors::SourceContext::new(
            std::path::PathBuf::from("unknown"),
            std::path::PathBuf::from("unknown"),
            content.as_str(),
        );
        let (frontmatter, remaining) = frontmatter::parse_frontmatter(&content, ctx)?;
        Ok(Self::with_frontmatter(frontmatter, remaining))
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
    pub fn image_references(&self) -> Vec<ImageRef> {
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

    /// Returns `true` if the document content contains any inline HTML.
    ///
    /// This is a fast, allocation-light check that can be used as a gate
    /// before the heavier extraction methods. False positives are possible,
    /// but false negatives are avoided where practical.
    pub fn has_inline_html(&self) -> bool {
        inline_html::has_inline_html(&self.content)
    }

    /// Extracts typed links from HTML `<a>` tags in the document content.
    ///
    /// Complements `links()`, which extracts Markdown-native links only.
    /// Results are returned in source order without deduplication.
    /// Malformed or unterminated anchors are silently skipped.
    pub fn inline_html_links(&self) -> Vec<Link> {
        inline_html::extract_inline_html_links(&self.content)
    }

    /// Extracts typed image references from HTML `<img>` tags in the document content.
    ///
    /// Complements `image_references()`, which extracts Markdown-native images only.
    /// Results are returned in source order without deduplication.
    /// Malformed image tags are silently skipped.
    pub fn inline_html_image_references(&self) -> Vec<ImageRef> {
        inline_html::extract_inline_html_images(&self.content)
    }

    /// Cleans up markdown content by normalizing formatting.
    ///
    /// This method collapses incidental single newlines inside prose, injects
    /// blank lines between block elements, and aligns table columns for visual
    /// consistency.
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

    /// Collapses incidental single newlines inside prose.
    pub fn strip_incidental_newlines(&mut self) -> &mut Self {
        self.content = cleanup::strip_incidental_newlines(&self.content);
        self
    }

    /// Cleans up markdown content and wraps prose to a fixed display width.
    pub fn cleanup_with_fixed_width(&mut self, width: usize) -> &mut Self {
        self.content = cleanup::cleanup_to_fixed_width(&self.content, width);
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

    /// Cleans up markdown in compact mode (removes blank lines between list items).
    pub fn cleanup_compact(&mut self) -> &mut Self {
        self.content = cleanup::cleanup_content_compact(&self.content);
        self
    }

    /// Cleans up markdown in loose mode (blank lines between all list items).
    pub fn cleanup_loose(&mut self) -> &mut Self {
        self.content = cleanup::cleanup_content_loose(&self.content);
        self
    }

    /// Cleans up markdown with forced indentation in compact mode.
    pub fn cleanup_with_indent_compact(&mut self, indent_size: usize) -> &mut Self {
        self.content = cleanup::cleanup_content_with_indent_compact(&self.content, indent_size);
        self
    }

    /// Cleans up markdown with forced indentation in loose mode.
    pub fn cleanup_with_indent_loose(&mut self, indent_size: usize) -> &mut Self {
        self.content = cleanup::cleanup_content_with_indent_loose(&self.content, indent_size);
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

    /// Folds the markdown document into the canonical renderable [`Document`].
    ///
    /// This is the target-agnostic intermediate representation used by the
    /// terminal, HTML, and Markdown renderers. Serializing the returned
    /// document to JSON produces a lossless tree that includes darkmatter
    /// extensions such as disclosure blocks as native [`NodeKind::Disclosure`].
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    ///
    /// let md = Markdown::new("# Hello\n\nWorld".to_string());
    /// let document = md.as_document().unwrap();
    ///
    /// let json = serde_json::to_string_pretty(&document).unwrap();
    /// assert!(json.contains("heading"));
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns [`MarkdownError::RenderTree`](crate::markdown::MarkdownError::RenderTree)
    /// when the render tree fold fails (for example, a malformed disclosure
    /// block).
    pub fn as_document(&self) -> MarkdownResult<renderable::tree::Document> {
        Ok(render_tree::entrypoints::to_render_document(self)?.0)
    }

    /// Converts the markdown document to HTML with syntax highlighting.
    ///
    /// Routes through the render-tree browser pipeline
    /// ([`render_tree_html`](crate::markdown::render_tree::render_tree_html)):
    /// the markdown folds to a [`Document`](renderable::tree::Document) and the
    /// browser renderer streams the final HTML string. `style:` frontmatter
    /// hyperlink / image color injection is reproduced by the entry point's
    /// inline-style decoration, and fenced code blocks are syntax-highlighted
    /// through the wired [`TerminalCodeRenderer`] hook. This is the only HTML
    /// render path; the legacy event-stream serializer has been deleted.
    ///
    /// Bare-rule defaults come from [`HtmlOptions::hr_defaults`]. Document
    /// frontmatter is honored only when callers parse `style.hr.*` and project
    /// it into those options or a [`DarkmatterPage`](crate::layout::DarkmatterPage).
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    /// use darkmatter::markdown::output::HtmlOptions;
    ///
    /// let md = Markdown::new("# Hello\n\nWorld".to_string());
    /// let html = md.as_html(HtmlOptions::default()).unwrap();
    /// assert!(html.contains("<h1"));
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns [`MarkdownError::RenderTree`](crate::markdown::MarkdownError::RenderTree)
    /// when the render tree fails structural validation or a strict-mode
    /// rejection occurs.
    pub fn as_html(&self, options: output::HtmlOptions) -> MarkdownResult<String> {
        Ok(render_tree::render_tree_html(self, &options)?.output)
    }

    /// Renders the markdown document as ANSI-styled terminal output.
    ///
    /// Returns a string containing ANSI escape codes for syntax highlighting,
    /// styled headings, and formatted block elements including inline images
    /// (via Kitty/iTerm2 protocols through biscuit-terminal).
    ///
    /// Bare-rule defaults come from [`TerminalOptions::hr_defaults`](output::TerminalOptions::hr_defaults).
    /// Document frontmatter is honored only when callers parse `style.hr.*` and
    /// project it into those options or a [`DarkmatterPage`](crate::layout::DarkmatterPage).
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
    /// Returns [`MarkdownError::RenderTree`](crate::markdown::MarkdownError::RenderTree)
    /// when the render tree fails structural validation or a strict-mode
    /// rejection occurs.
    pub fn as_terminal(&self, options: output::TerminalOptions) -> MarkdownResult<String> {
        Ok(render_tree::render_tree_terminal(self, &options)?.output)
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
    /// use darkmatter::markdown::normalize::HeadingLevel;
    ///
    /// let content = "# Introduction\n\nWelcome.\n\n## Getting Started\n\nFirst steps.";
    /// let md: Markdown = content.into();
    /// let toc = md.toc();
    ///
    /// assert_eq!(toc.heading_count(), 2);
    /// assert_eq!(toc.root_level(), Some(HeadingLevel::H1));
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
        let ctx = biscuit_terminal::errors::SourceContext::new(
            std::path::PathBuf::from("unknown"),
            std::path::PathBuf::from("unknown"),
            content.as_str(),
        );
        match frontmatter::parse_frontmatter(&content, ctx) {
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
        // Parse with a path-aware context so a frontmatter error names the
        // offending file. `try_from_content` would build an "unknown" context,
        // and `with_source` only runs on success — so without this the path is
        // lost on exactly the error that needs it.
        let absolute = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let ctx = biscuit_terminal::errors::SourceContext::new(
            absolute,
            path.to_path_buf(),
            content.as_str(),
        );
        let (frontmatter, remaining) = frontmatter::parse_frontmatter(&content, ctx)?;
        let md = Self::with_frontmatter(frontmatter, remaining);
        Ok(md.with_source(ComposeSource::infer_from_path(path)))
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

    /// `as_html` must surface a malformed code-block directive (an invalid
    /// highlight range) as a fatal `MarkdownError::InvalidLineRange`, matching
    /// the fatal-directive browser contract the tree cutover restored via the
    /// `validate_code_directives` preflight (review-2 finding 3). A well-formed
    /// directive must still render cleanly.
    #[test]
    fn as_html_errors_on_malformed_code_directive() {
        let bad: Markdown = "```rust highlight=1-2-3\nfn main() {}\n```\n".into();
        let err = bad
            .as_html(output::HtmlOptions::default())
            .expect_err("malformed highlight range must fail as_html");
        assert!(
            matches!(err, MarkdownError::InvalidLineRange(_)),
            "expected MarkdownError::InvalidLineRange, got {err:?}"
        );

        let good: Markdown = "```rust highlight=2\nfn main() {}\n```\n".into();
        let html = good
            .as_html(output::HtmlOptions::default())
            .expect("well-formed directive must render");
        assert!(html.contains("main"));
    }

    /// A structured link directive must lower to real HTML attributes through
    /// the tree-backed `as_html` (review-2 finding 2): `class`, `target`, and
    /// `data-*` must survive, and the raw directive must not leak as a
    /// `title="…"` attribute. No frontmatter hyperlink style is configured,
    /// proving the lowering is unconditional. Since Phase 4 the `prompt`
    /// directive is consumed into the accessible popover structure rather than
    /// emitted as a `data-prompt` transport attribute.
    #[test]
    fn as_html_preserves_structured_link_metadata() {
        let md: Markdown = r#"[Read docs](https://example.com "class='btn' target='_blank' prompt='Read docs' data-id='42'")"#.into();
        let html = md
            .as_html(output::HtmlOptions::default())
            .expect("structured link must render");

        assert!(html.contains(r#"href="https://example.com""#), "html={html}");
        assert!(html.contains(r#"class="btn""#), "class lost; html={html}");
        assert!(html.contains(r#"target="_blank""#), "target lost; html={html}");
        assert!(html.contains(r#"data-id="42""#), "data-* lost; html={html}");
        // The prompt is consumed into the popover markup, not re-emitted as the
        // internal `data-prompt` transport.
        assert!(
            !html.contains("data-prompt="),
            "internal prompt transport leaked; html={html}"
        );
        assert!(
            html.contains(r#"popover="hint""#) && html.contains("Read docs"),
            "prompt lost from popover markup; html={html}"
        );
        assert!(
            !html.contains("title="),
            "raw structured directive leaked as title; html={html}"
        );
        assert!(html.contains("Read docs"), "link text lost; html={html}");
    }

    /// A structured link carrying only a `style='…'` directive must lower to a
    /// `style="…"` attribute even with no frontmatter hyperlink style — the
    /// per-link inline CSS the tree path previously dropped when no frontmatter
    /// style was set (review-2 finding 2).
    #[test]
    fn as_html_preserves_structured_link_inline_style() {
        let md: Markdown =
            r#"[Click here](https://example.com "class='btn' style='color:red'")"#.into();
        let html = md
            .as_html(output::HtmlOptions::default())
            .expect("structured link must render");

        assert!(html.contains(r#"class="btn""#), "class lost; html={html}");
        assert!(
            html.contains("style=") && html.contains("red"),
            "inline style lost; html={html}"
        );
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
    fn test_cleanup_with_fixed_width_method() {
        let content = "This paragraph is long enough to wrap at a narrow display width.";
        let mut md: Markdown = content.into();

        md.cleanup_with_fixed_width(24);

        assert_eq!(
            md.content(),
            "This paragraph is long\nenough to wrap at a\nnarrow display width."
        );
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
        let images = md.image_references();

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
        let images = md.image_references();

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].alt(), "Use `cargo test` here");
    }

    // ============================================
    // links() tests
    // ============================================

    #[test]
    fn links_returns_empty_for_no_links() {
        let md: Markdown = "# Heading\n\nJust plain text.".into();
        assert!(md.links().is_empty());
    }

    #[test]
    fn links_extracts_basic_link() {
        let md: Markdown = "Check [Example](https://example.com) for details.".into();
        let links = md.links();

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].display(), "Example");
        assert_eq!(links[0].href(), "https://example.com");
        assert!(links[0].title().is_none());
    }

    #[test]
    fn links_extracts_multiple_links() {
        let md: Markdown =
            "[One](https://one.com) and [Two](https://two.com) and [Three](https://three.com)"
                .into();
        let links = md.links();

        assert_eq!(links.len(), 3);
        assert_eq!(links[0].display(), "One");
        assert_eq!(links[1].display(), "Two");
        assert_eq!(links[2].display(), "Three");
    }

    #[test]
    fn links_extracts_link_with_title() {
        let md: Markdown = r#"[Docs](https://docs.rs "Official documentation")"#.into();
        let links = md.links();

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].title(), Some("Official documentation"));
    }

    #[test]
    fn links_preserves_inline_code_in_display() {
        let md: Markdown = "[Use `cargo build`](https://doc.rust-lang.org)".into();
        let links = md.links();

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].display(), "Use `cargo build`");
    }

    #[test]
    fn links_preserves_bold_text_in_display() {
        let md: Markdown = "[**Important** link](https://example.com)".into();
        let links = md.links();

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].display(), "Important link");
    }

    #[test]
    fn links_detects_file_vs_url_kind() {
        let md: Markdown = "[Local](./README.md) and [Remote](https://example.com)".into();
        let links = md.links();

        assert_eq!(links.len(), 2);
        assert!(links[0].is_file());
        assert!(links[1].is_url());
    }

    #[test]
    fn links_skips_image_references() {
        let md: Markdown = "![An image](./photo.png) and [A link](https://example.com)".into();
        let links = md.links();

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].display(), "A link");
    }

    #[test]
    fn links_handles_links_across_multiple_paragraphs() {
        let md: Markdown = "Paragraph one with [link1](https://one.com).\n\nParagraph two with [link2](https://two.com).".into();
        let links = md.links();

        assert_eq!(links.len(), 2);
        assert_eq!(links[0].href(), "https://one.com");
        assert_eq!(links[1].href(), "https://two.com");
    }

    // ============================================
    // image_references() tests
    // ============================================

    #[test]
    fn image_references_returns_empty_for_no_images() {
        let md: Markdown =
            "# Heading\n\nJust plain text with [a link](https://example.com).".into();
        assert!(md.image_references().is_empty());
    }

    #[test]
    fn image_references_extracts_basic_image() {
        let md: Markdown = "![A photo](./photo.png)".into();
        let images = md.image_references();

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].alt(), "A photo");
        assert_eq!(images[0].src(), Some("./photo.png"));
    }

    #[test]
    fn image_references_extracts_multiple_images() {
        let md: Markdown = "![First](./a.png)\n\n![Second](./b.png)\n\n![Third](./c.png)".into();
        let images = md.image_references();

        assert_eq!(images.len(), 3);
        assert_eq!(images[0].alt(), "First");
        assert_eq!(images[1].alt(), "Second");
        assert_eq!(images[2].alt(), "Third");
    }

    #[test]
    fn image_references_extracts_title() {
        let md: Markdown = r#"![Photo](./photo.png "A scenic view")"#.into();
        let images = md.image_references();

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].title(), Some("A scenic view"));
    }

    #[test]
    fn image_references_skips_hyperlinks() {
        let md: Markdown =
            "[Not an image](https://example.com) and ![An image](./photo.png)".into();
        let images = md.image_references();

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].alt(), "An image");
    }

    #[test]
    fn image_references_handles_images_across_paragraphs() {
        let md: Markdown = "Text with ![img1](./a.png).\n\nMore text with ![img2](./b.png).".into();
        let images = md.image_references();

        assert_eq!(images.len(), 2);
        assert_eq!(images[0].src(), Some("./a.png"));
        assert_eq!(images[1].src(), Some("./b.png"));
    }

    #[test]
    fn image_references_handles_url_with_special_chars() {
        let md: Markdown = "![Logo](https://example.com/images/logo%20v2.png)".into();
        let images = md.image_references();

        assert_eq!(images.len(), 1);
        assert_eq!(
            images[0].src(),
            Some("https://example.com/images/logo%20v2.png")
        );
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

    // =========================================================================
    // Dim (⌄text⌄) integration tests
    // =========================================================================

    /// Full pipeline: Markdown source `⌄dim⌄` → terminal output contains `\x1b[2m`.
    #[test]
    fn test_dim_full_pipeline_terminal() {
        use crate::markdown::highlighting::{ColorMode, ThemePair};
        use crate::markdown::output::terminal::{ColorDepth, DimMode, TerminalOptions};

        let md: Markdown = "This is ⌄dimmed⌄ text.".into();
        let options = TerminalOptions {
            code_theme: ThemePair::OneHalf,
            prose_theme: ThemePair::OneHalf,
            color_mode: ColorMode::Dark,
            include_line_numbers: false,
            color_depth: Some(ColorDepth::TrueColor),
            image_mode: crate::markdown::output::terminal::TerminalImageMode::Never,
            base_path: None,
            italic_mode: crate::markdown::output::terminal::ItalicMode::Always,
            dim_mode: DimMode::Always,
            max_width: Some(80),
            mermaid_mode: crate::markdown::output::terminal::MermaidMode::Off,
            hyperlink_mode: crate::markdown::output::terminal::HyperlinkMode::Always,
            hr_defaults: None,
            code_block_mode: crate::markdown::highlighting::CodeBlockMode::default(),
        };
        let output = md.as_terminal(options).unwrap();

        assert!(
            output.contains("\x1b[2m"),
            "Terminal output should contain dim ANSI code \\x1b[2m, got: {:?}",
            output
        );

        // Delimiters should be stripped by the inline processor
        assert!(
            !output.contains("⌄"),
            "Terminal output should not contain ⌄ delimiters"
        );
    }

    /// Full pipeline: Markdown source `⌄dim⌄` → HTML lowers the dim span to a
    /// styled `<span>` carrying the dimmed text (the `⌄` delimiters are
    /// consumed, not echoed). This is the tree path's deliberate fidelity
    /// improvement over the legacy literal-delimiter passthrough.
    #[test]
    fn test_dim_full_pipeline_html() {
        use crate::markdown::output::HtmlOptions;

        let md: Markdown = "This is ⌄dimmed⌄ text.".into();
        let html = md.as_html(HtmlOptions::default()).unwrap();

        assert!(
            html.contains("dimmed"),
            "HTML output should preserve the dimmed text, got: {}",
            html
        );
        assert!(
            !html.contains('⌄'),
            "HTML output should consume the ⌄ delimiters, got: {}",
            html
        );
        assert!(
            !html.contains("<dim>"),
            "HTML output should not contain a <dim> tag"
        );
    }

    /// Cross-format consistency: terminal-rendered `⌄text⌄` and HTML `⌄text⌄`
    /// both preserve the visible text content.
    #[test]
    fn test_dim_cross_format_consistency() {
        use crate::markdown::highlighting::{ColorMode, ThemePair};
        use crate::markdown::output::terminal::{ColorDepth, DimMode, TerminalOptions};
        use crate::markdown::output::HtmlOptions;
        use crate::testing::strip_ansi_codes;

        let md: Markdown = "The ⌄dimmed text⌄ here.".into();

        // Terminal output
        let terminal_options = TerminalOptions {
            code_theme: ThemePair::OneHalf,
            prose_theme: ThemePair::OneHalf,
            color_mode: ColorMode::Dark,
            include_line_numbers: false,
            color_depth: Some(ColorDepth::TrueColor),
            image_mode: crate::markdown::output::terminal::TerminalImageMode::Never,
            base_path: None,
            italic_mode: crate::markdown::output::terminal::ItalicMode::Always,
            dim_mode: DimMode::Always,
            max_width: Some(80),
            mermaid_mode: crate::markdown::output::terminal::MermaidMode::Off,
            hyperlink_mode: crate::markdown::output::terminal::HyperlinkMode::Always,
            hr_defaults: None,
            code_block_mode: crate::markdown::highlighting::CodeBlockMode::default(),
        };
        let terminal_output = md.as_terminal(terminal_options).unwrap();
        let terminal_plain = strip_ansi_codes(&terminal_output);

        // HTML output
        let html = md.as_html(HtmlOptions::default()).unwrap();

        // Both should contain the visible text "dimmed text"
        assert!(
            terminal_plain.contains("dimmed text"),
            "Terminal plain text should contain 'dimmed text', got: {:?}",
            terminal_plain
        );
        assert!(
            html.contains("dimmed text"),
            "HTML should contain 'dimmed text', got: {}",
            html
        );
    }

    // =========================================================================
    // Frontmatter fence mismatch cross-package validation (Phase 3)
    // =========================================================================

    /// Acceptance criterion #4: a correctly `---`-fenced document round-trips
    /// end-to-end through [`Markdown::try_from_content`] with frontmatter parsed
    /// and the body starting after the closing delimiter.
    #[test]
    fn try_from_content_three_dash_fence_round_trips() {
        let content = "---\ntitle: Test\n---\n# Hello\n";
        let md = Markdown::try_from_content(content).expect("valid --- fence must parse");

        let title: Option<String> = md.fm_get("title").unwrap();
        assert_eq!(title, Some("Test".to_string()));
        assert!(md.content().starts_with("# Hello"));
        // The delimiter and YAML must not leak into the body.
        assert!(!md.content().contains("---"));
        assert!(!md.content().contains("title:"));
    }

    /// Acceptance criterion #6: a `----`-fenced YAML map loaded via
    /// [`Markdown::try_from_content`] surfaces the typed mismatch error.
    #[test]
    fn try_from_content_four_dash_fence_returns_typed_error() {
        let content = "----\nname: cross-platform\ndescription: near-miss\n----\n# Body\n";
        let err = Markdown::try_from_content(content).expect_err("---- fence must fail");

        match err {
            MarkdownError::FrontmatterFenceMismatch { found, line, .. } => {
                assert_eq!(found, "----");
                assert_eq!(line, 1);
            }
            other => panic!("expected FrontmatterFenceMismatch, got {other:?}"),
        }
    }

    /// Acceptance criterion #6: a `----`-fenced YAML map loaded from disk via
    /// [`Markdown::try_from`] also surfaces the typed mismatch error, preserving
    /// the source path in the error context.
    #[test]
    fn try_from_path_four_dash_fence_returns_typed_error() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "----").unwrap();
        writeln!(file, "name: cross-platform").unwrap();
        writeln!(file, "description: near-miss").unwrap();
        writeln!(file, "----").unwrap();
        writeln!(file, "# Body").unwrap();

        let err = Markdown::try_from(file.path()).expect_err("---- fence must fail");
        match err {
            MarkdownError::FrontmatterFenceMismatch { found, line, .. } => {
                assert_eq!(found, "----");
                assert_eq!(line, 1);
            }
            other => panic!("expected FrontmatterFenceMismatch, got {other:?}"),
        }
    }

    /// The infallible [`From<String>`] conversion intentionally swallows
    /// frontmatter errors and returns a document whose entire source is treated
    /// as body text. Claudine's prompt-loading path uses [`Markdown::try_from`],
    /// not this conversion, so a malformed fence is still rejected there. This
    /// test documents the asymmetry so it is not mistaken for a bug.
    #[test]
    fn from_string_swallows_frontmatter_fence_mismatch_by_design() {
        let content = "----\nname: cross-platform\n----\n# Body\n".to_string();
        let md: Markdown = content.into();

        // The conversion succeeds with empty frontmatter; the raw text becomes body.
        assert!(md.frontmatter().is_empty());
        assert!(md.content().contains("name: cross-platform"));
        assert!(md.content().contains("----"));
    }
}
