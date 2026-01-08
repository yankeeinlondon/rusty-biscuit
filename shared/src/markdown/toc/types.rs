//! Type definitions for Markdown Table of Contents.

use serde::Serialize;
use std::collections::HashMap;

/// A node in the Table of Contents for a Markdown page.
///
/// Each node represents a heading and its associated content, forming
/// a hierarchical tree structure that mirrors the document's heading
/// organization.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MarkdownTocNode {
    /// The level of the heading node (1-6).
    pub level: u8,

    /// The text content of this heading, minus the leading `#` markers.
    /// Preserves inline formatting (bold, code, etc.) as raw markdown.
    pub title: String,

    /// An xxHash of the title text.
    pub title_hash: u64,

    /// An xxHash of the title text after trimming whitespace.
    pub title_hash_trimmed: u64,

    /// The generated anchor slug for this heading (e.g., "my-heading" for "## My Heading").
    /// Used for internal link detection and TOC link generation.
    pub slug: String,

    // ─────────────────────────────────────────────────────────────
    // Location Information
    // ─────────────────────────────────────────────────────────────
    /// Byte offset range in the source document [start, end).
    /// Covers from the heading line through all content until the next
    /// heading at the same or higher level (or end of document).
    pub source_span: (usize, usize),

    /// Line number range [start_line, end_line) (1-indexed).
    pub line_range: (usize, usize),

    // ─────────────────────────────────────────────────────────────
    // Own Content (this section only, excluding children)
    // ─────────────────────────────────────────────────────────────
    /// The textual content of THIS section only.
    /// Spans from after the heading line to either:
    /// - The first child heading, OR
    /// - The next sibling/parent heading, OR
    /// - End of document
    /// Does NOT include child section content.
    pub own_content: Option<String>,

    /// An xxHash of `own_content`.
    pub own_content_hash: u64,

    /// An xxHash of `own_content` after trimming whitespace.
    pub own_content_hash_trimmed: u64,

    /// An xxHash of `own_content` after removing all blank lines.
    /// More robust than trimmed for detecting whitespace-only changes
    /// when blank lines are added/removed within content.
    pub own_content_hash_normalized: u64,

    // ─────────────────────────────────────────────────────────────
    // Subtree Hashes (for quick "has anything changed?" checks)
    // ─────────────────────────────────────────────────────────────
    /// An xxHash combining this node's content AND all descendant content.
    /// Useful for quick subtree change detection without traversing children.
    pub subtree_hash: u64,

    /// An xxHash of the subtree content after trimming all whitespace.
    pub subtree_hash_trimmed: u64,

    // ─────────────────────────────────────────────────────────────
    // Hierarchy
    // ─────────────────────────────────────────────────────────────
    /// Child nodes (headings at a deeper level under this heading).
    pub children: Vec<MarkdownTocNode>,
}

impl MarkdownTocNode {
    /// Creates a new TOC node with the given heading information.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        level: u8,
        title: String,
        slug: String,
        source_span: (usize, usize),
        line_range: (usize, usize),
    ) -> Self {
        use crate::hashing::{xx_hash, xx_hash_trimmed};

        let title_hash = xx_hash(&title);
        let title_hash_trimmed = xx_hash_trimmed(&title);

        Self {
            level,
            title,
            title_hash,
            title_hash_trimmed,
            slug,
            source_span,
            line_range,
            own_content: None,
            own_content_hash: 0,
            own_content_hash_trimmed: 0,
            own_content_hash_normalized: 0,
            subtree_hash: 0,
            subtree_hash_trimmed: 0,
            children: Vec::new(),
        }
    }

    /// Sets the own content for this node and computes its hash.
    pub fn set_own_content(&mut self, content: Option<String>) {
        use crate::hashing::{xx_hash, xx_hash_semantic, xx_hash_trimmed};

        if let Some(ref c) = content {
            self.own_content_hash = xx_hash(c);
            self.own_content_hash_trimmed = xx_hash_trimmed(c);
            // Use semantic hash for whitespace-insensitive comparison
            // (removes blank lines AND trims each line)
            self.own_content_hash_normalized = xx_hash_semantic(c);
        } else {
            self.own_content_hash = 0;
            self.own_content_hash_trimmed = 0;
            self.own_content_hash_normalized = 0;
        }
        self.own_content = content;
    }

    /// Computes the subtree hash by combining own content with all children.
    pub fn compute_subtree_hash(&mut self) {
        use crate::hashing::{xx_hash, xx_hash_trimmed};

        // Build combined content for subtree
        let mut combined = self.own_content.clone().unwrap_or_default();

        for child in &mut self.children {
            child.compute_subtree_hash();
            if let Some(ref content) = child.own_content {
                combined.push_str(content);
            }
        }

        self.subtree_hash = xx_hash(&combined);
        self.subtree_hash_trimmed = xx_hash_trimmed(&combined);
    }

    /// Returns the path to this node (list of ancestor titles including this one).
    pub fn path(&self) -> Vec<String> {
        vec![self.title.clone()]
    }

    /// Returns the total number of nodes in this subtree (including self).
    pub fn node_count(&self) -> usize {
        1 + self.children.iter().map(|c| c.node_count()).sum::<usize>()
    }

    /// Finds a node by its slug in this subtree.
    pub fn find_by_slug(&self, slug: &str) -> Option<&MarkdownTocNode> {
        if self.slug == slug {
            return Some(self);
        }
        for child in &self.children {
            if let Some(node) = child.find_by_slug(slug) {
                return Some(node);
            }
        }
        None
    }
}

/// Information about a fenced code block in the document.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CodeBlockInfo {
    /// The language identifier (e.g., "rust", "javascript"), if specified.
    pub language: Option<String>,

    /// The full info string from the fence (e.g., "mermaid title=\"foo\"").
    /// Includes language and any metadata attributes.
    pub info_string: String,

    /// The raw content of the code block (excluding fence markers).
    pub content: String,

    /// An xxHash of the code block content.
    pub content_hash: u64,

    /// An xxHash of the code block content after trimming whitespace.
    pub content_hash_trimmed: u64,

    /// Line number range [start_line, end_line) (1-indexed).
    pub line_range: (usize, usize),

    /// Path to the containing section (e.g., ["Introduction", "Setup"]).
    pub parent_section_path: Vec<String>,
}

impl CodeBlockInfo {
    /// Creates a new code block info.
    pub fn new(
        language: Option<String>,
        info_string: String,
        content: String,
        line_range: (usize, usize),
        parent_section_path: Vec<String>,
    ) -> Self {
        use crate::hashing::{xx_hash, xx_hash_trimmed};

        let content_hash = xx_hash(&content);
        let content_hash_trimmed = xx_hash_trimmed(&content);

        Self {
            language,
            info_string,
            content,
            content_hash,
            content_hash_trimmed,
            line_range,
            parent_section_path,
        }
    }
}

/// Information about an internal link (anchor reference) in the document.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InternalLinkInfo {
    /// The target anchor (e.g., "my-heading" from `[text](#my-heading)`).
    pub target_slug: String,

    /// The link text.
    pub link_text: String,

    /// Line number where this link appears (1-indexed).
    pub line_number: usize,

    /// Byte offset of the link in the source document.
    pub byte_offset: usize,

    /// Path to the containing section.
    pub parent_section_path: Vec<String>,
}

impl InternalLinkInfo {
    /// Creates a new internal link info.
    pub fn new(
        target_slug: String,
        link_text: String,
        line_number: usize,
        byte_offset: usize,
        parent_section_path: Vec<String>,
    ) -> Self {
        Self {
            target_slug,
            link_text,
            line_number,
            byte_offset,
            parent_section_path,
        }
    }
}

/// Table of Contents for a Markdown document.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MarkdownToc {
    // ─────────────────────────────────────────────────────────────
    // Document Identity
    // ─────────────────────────────────────────────────────────────
    /// The document title, determined by:
    /// 1. The `title` frontmatter property (if set), OR
    /// 2. The text of the single H1 heading (if exactly one H1 exists), OR
    /// 3. None
    pub title: Option<String>,

    /// An xxHash of the entire page content (excluding frontmatter).
    pub page_hash: u64,

    /// An xxHash of the page content after trimming (excluding frontmatter).
    pub page_hash_trimmed: u64,

    // ─────────────────────────────────────────────────────────────
    // Frontmatter
    // ─────────────────────────────────────────────────────────────
    /// An xxHash of the raw frontmatter YAML string.
    /// Hashes the raw text to detect formatting/ordering changes.
    pub frontmatter_hash: u64,

    /// An xxHash of the frontmatter after serializing to canonical form.
    /// Ignores key ordering and whitespace differences.
    pub frontmatter_hash_normalized: u64,

    // ─────────────────────────────────────────────────────────────
    // Preamble (content before first heading)
    // ─────────────────────────────────────────────────────────────
    /// The text before the first heading tag is encountered.
    pub preamble: String,

    /// An xxHash of the preamble text.
    pub preamble_hash: u64,

    /// An xxHash of the preamble after trimming whitespace.
    pub preamble_hash_trimmed: u64,

    // ─────────────────────────────────────────────────────────────
    // Structure
    // ─────────────────────────────────────────────────────────────
    /// The hierarchical structure of headings in the document.
    ///
    /// ## Fragment Rules
    ///
    /// The `structure` vector contains one or more "fragments". A new fragment
    /// starts when:
    ///
    /// 1. At the first heading in the document
    /// 2. When encountering a heading at a level <= the current fragment's root level
    /// 3. When encountering an H1 (always starts a new fragment)
    pub structure: Vec<MarkdownTocNode>,

    // ─────────────────────────────────────────────────────────────
    // Additional Tracking
    // ─────────────────────────────────────────────────────────────
    /// All code blocks in the document, in order of appearance.
    pub code_blocks: Vec<CodeBlockInfo>,

    /// All internal links (anchor references) in the document.
    pub internal_links: Vec<InternalLinkInfo>,

    /// All heading slugs in the document (for quick lookup).
    /// Maps slug -> Vec<(section_path, line_number)> to handle duplicates.
    pub slug_index: HashMap<String, Vec<(Vec<String>, usize)>>,
}

impl Default for MarkdownToc {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownToc {
    /// Creates a new empty TOC.
    pub fn new() -> Self {
        Self {
            title: None,
            page_hash: 0,
            page_hash_trimmed: 0,
            frontmatter_hash: 0,
            frontmatter_hash_normalized: 0,
            preamble: String::new(),
            preamble_hash: 0,
            preamble_hash_trimmed: 0,
            structure: Vec::new(),
            code_blocks: Vec::new(),
            internal_links: Vec::new(),
            slug_index: HashMap::new(),
        }
    }

    /// Returns the total number of headings in the document.
    pub fn heading_count(&self) -> usize {
        self.structure.iter().map(|n| n.node_count()).sum()
    }

    /// Returns the root level of the document (level of first heading).
    pub fn root_level(&self) -> Option<u8> {
        self.structure.first().map(|n| n.level)
    }

    /// Returns the maximum (deepest) heading level in the document.
    pub fn max_level(&self) -> Option<u8> {
        fn max_level_recursive(node: &MarkdownTocNode) -> u8 {
            let child_max = node
                .children
                .iter()
                .map(max_level_recursive)
                .max()
                .unwrap_or(0);
            node.level.max(child_max)
        }

        self.structure.iter().map(max_level_recursive).max()
    }

    /// Finds a heading by its slug.
    pub fn find_by_slug(&self, slug: &str) -> Option<&MarkdownTocNode> {
        for node in &self.structure {
            if let Some(found) = node.find_by_slug(slug) {
                return Some(found);
            }
        }
        None
    }

    /// Returns all headings as a flat list (depth-first).
    pub fn all_headings(&self) -> Vec<&MarkdownTocNode> {
        fn collect_recursive<'a>(node: &'a MarkdownTocNode, result: &mut Vec<&'a MarkdownTocNode>) {
            result.push(node);
            for child in &node.children {
                collect_recursive(child, result);
            }
        }

        let mut result = Vec::new();
        for node in &self.structure {
            collect_recursive(node, &mut result);
        }
        result
    }

    /// Checks if all internal links have valid targets.
    pub fn has_broken_links(&self) -> bool {
        self.internal_links
            .iter()
            .any(|link| !self.slug_index.contains_key(&link.target_slug))
    }

    /// Returns all internal links that don't have valid targets.
    pub fn broken_links(&self) -> Vec<&InternalLinkInfo> {
        self.internal_links
            .iter()
            .filter(|link| !self.slug_index.contains_key(&link.target_slug))
            .collect()
    }

    /// Adds a slug to the index.
    pub fn add_to_slug_index(&mut self, slug: String, section_path: Vec<String>, line_number: usize) {
        self.slug_index
            .entry(slug)
            .or_default()
            .push((section_path, line_number));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toc_node_new() {
        let node = MarkdownTocNode::new(2, "Test Heading".to_string(), "test-heading".to_string(), (0, 100), (1, 10));

        assert_eq!(node.level, 2);
        assert_eq!(node.title, "Test Heading");
        assert_eq!(node.slug, "test-heading");
        assert!(node.title_hash > 0);
        assert!(node.children.is_empty());
    }

    #[test]
    fn test_toc_node_set_own_content() {
        let mut node = MarkdownTocNode::new(2, "Test".to_string(), "test".to_string(), (0, 100), (1, 10));

        node.set_own_content(Some("Hello world".to_string()));
        assert_eq!(node.own_content, Some("Hello world".to_string()));
        assert!(node.own_content_hash > 0);
    }

    #[test]
    fn test_toc_node_count() {
        let mut root = MarkdownTocNode::new(1, "Root".to_string(), "root".to_string(), (0, 100), (1, 10));
        let child1 = MarkdownTocNode::new(2, "Child1".to_string(), "child1".to_string(), (10, 50), (2, 5));
        let child2 = MarkdownTocNode::new(2, "Child2".to_string(), "child2".to_string(), (50, 100), (5, 10));
        root.children.push(child1);
        root.children.push(child2);

        assert_eq!(root.node_count(), 3);
    }

    #[test]
    fn test_toc_node_find_by_slug() {
        let mut root = MarkdownTocNode::new(1, "Root".to_string(), "root".to_string(), (0, 100), (1, 10));
        let child = MarkdownTocNode::new(2, "Child".to_string(), "child".to_string(), (10, 50), (2, 5));
        root.children.push(child);

        assert!(root.find_by_slug("root").is_some());
        assert!(root.find_by_slug("child").is_some());
        assert!(root.find_by_slug("nonexistent").is_none());
    }

    #[test]
    fn test_code_block_info() {
        let info = CodeBlockInfo::new(
            Some("rust".to_string()),
            "rust".to_string(),
            "fn main() {}".to_string(),
            (1, 3),
            vec!["Introduction".to_string()],
        );

        assert_eq!(info.language, Some("rust".to_string()));
        assert_eq!(info.info_string, "rust");
        assert!(info.content_hash > 0);
    }

    #[test]
    fn test_internal_link_info() {
        let info = InternalLinkInfo::new(
            "my-heading".to_string(),
            "link text".to_string(),
            5,
            100,
            vec!["Section".to_string()],
        );

        assert_eq!(info.target_slug, "my-heading");
        assert_eq!(info.line_number, 5);
    }

    #[test]
    fn test_markdown_toc_new() {
        let toc = MarkdownToc::new();

        assert!(toc.title.is_none());
        assert!(toc.structure.is_empty());
        assert_eq!(toc.heading_count(), 0);
    }

    #[test]
    fn test_markdown_toc_heading_count() {
        let mut toc = MarkdownToc::new();
        let mut root = MarkdownTocNode::new(1, "Root".to_string(), "root".to_string(), (0, 100), (1, 10));
        root.children.push(MarkdownTocNode::new(2, "Child".to_string(), "child".to_string(), (10, 50), (2, 5)));
        toc.structure.push(root);

        assert_eq!(toc.heading_count(), 2);
    }

    #[test]
    fn test_markdown_toc_root_level() {
        let mut toc = MarkdownToc::new();
        toc.structure.push(MarkdownTocNode::new(2, "H2".to_string(), "h2".to_string(), (0, 100), (1, 10)));

        assert_eq!(toc.root_level(), Some(2));
    }

    #[test]
    fn test_markdown_toc_max_level() {
        let mut toc = MarkdownToc::new();
        let mut h1 = MarkdownTocNode::new(1, "H1".to_string(), "h1".to_string(), (0, 100), (1, 10));
        let mut h2 = MarkdownTocNode::new(2, "H2".to_string(), "h2".to_string(), (10, 50), (2, 5));
        h2.children.push(MarkdownTocNode::new(4, "H4".to_string(), "h4".to_string(), (20, 40), (3, 4)));
        h1.children.push(h2);
        toc.structure.push(h1);

        assert_eq!(toc.max_level(), Some(4));
    }

    #[test]
    fn test_markdown_toc_slug_index() {
        let mut toc = MarkdownToc::new();
        toc.add_to_slug_index("test-slug".to_string(), vec!["Section".to_string()], 5);

        assert!(toc.slug_index.contains_key("test-slug"));
        assert_eq!(toc.slug_index["test-slug"].len(), 1);
    }

    #[test]
    fn test_markdown_toc_broken_links() {
        let mut toc = MarkdownToc::new();
        toc.add_to_slug_index("valid-slug".to_string(), vec![], 1);
        toc.internal_links.push(InternalLinkInfo::new(
            "valid-slug".to_string(),
            "Valid".to_string(),
            1,
            0,
            vec![],
        ));
        toc.internal_links.push(InternalLinkInfo::new(
            "invalid-slug".to_string(),
            "Invalid".to_string(),
            2,
            50,
            vec![],
        ));

        assert!(toc.has_broken_links());
        assert_eq!(toc.broken_links().len(), 1);
        assert_eq!(toc.broken_links()[0].target_slug, "invalid-slug");
    }
}
