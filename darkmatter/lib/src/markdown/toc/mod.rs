//! Markdown Table of Contents extraction and analysis.
//!
//! This module provides functionality to extract a structured Table of Contents
//! from markdown documents, including heading hierarchy, content hashing,
//! code block tracking, and internal link detection.
//!
//! ## Examples
//!
//! ```rust
//! use darkmatter::markdown::Markdown;
//! use darkmatter::markdown::normalize::HeadingLevel;
//!
//! let content = "# Introduction\n\nWelcome.\n\n## Getting Started\n\nFirst steps.";
//! let md: Markdown = content.into();
//! let toc = md.toc();
//!
//! assert_eq!(toc.heading_count(), 2);
//! assert_eq!(toc.root_level(), Some(HeadingLevel::H1));
//! assert_eq!(toc.title, Some("Introduction".to_string()));
//! ```

mod tree;
mod types;

pub use tree::TocTree;
pub use types::{CodeBlockInfo, InternalLinkInfo, MarkdownToc, MarkdownTocNode, PreludeNode};

use crate::markdown::Markdown;
use crate::markdown::normalize::HeadingLevel as OurHeadingLevel;
use crate::markdown::span::{SourceSpan, line_at_offset, newline_offset_table};
use biscuit_file::serde_yaml_ng;
use biscuit_hash::{HashVariant, xx_hash, xx_hash_variant};
use pulldown_cmark::{Event, HeadingLevel as PulldownHeadingLevel, Parser, Tag, TagEnd};

/// Generates a URL-safe slug from heading text.
///
/// Converts to lowercase, replaces spaces with hyphens, removes non-alphanumeric
/// characters (except hyphens), and collapses multiple hyphens.
fn generate_slug(text: &str) -> String {
    let mut slug = String::with_capacity(text.len());

    for c in text.chars() {
        if c.is_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if (c.is_whitespace() || c == '-' || c == '_') && !slug.ends_with('-') {
            slug.push('-');
        }
    }

    // Trim leading/trailing hyphens
    slug.trim_matches('-').to_string()
}

/// Converts pulldown_cmark HeadingLevel to u8.
fn heading_level_to_u8(level: PulldownHeadingLevel) -> u8 {
    match level {
        PulldownHeadingLevel::H1 => 1,
        PulldownHeadingLevel::H2 => 2,
        PulldownHeadingLevel::H3 => 3,
        PulldownHeadingLevel::H4 => 4,
        PulldownHeadingLevel::H5 => 5,
        PulldownHeadingLevel::H6 => 6,
    }
}

/// Generates the URL-safe slug Darkmatter uses for a heading anchor.
///
/// This is the single slug authority: the same function TOC extraction uses
/// internally, so callers (e.g. language tooling) match Darkmatter's anchors
/// exactly instead of reimplementing the rules. Returns the unsuffixed base
/// slug; duplicate-heading disambiguation is applied by [`extract_headings`].
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::generate_heading_slug;
///
/// assert_eq!(generate_heading_slug("Getting Started"), "getting-started");
/// assert_eq!(generate_heading_slug("What's New?"), "whats-new");
/// ```
pub fn generate_heading_slug(text: &str) -> String {
    generate_slug(text)
}

/// A heading located in Markdown source, with byte spans and a
/// document-unique slug.
///
/// Produced by [`extract_headings`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadingRecord {
    /// Heading level (H1-H6).
    pub level: OurHeadingLevel,
    /// Rendered inline text of the heading (markup markers stripped).
    pub title: String,
    /// Document-unique anchor slug: the [`generate_heading_slug`] base, with
    /// `-1`, `-2`, … appended to repeated slugs in document order (GitHub
    /// anchor semantics).
    pub slug: String,
    /// Byte span of the heading's inline content. Empty (anchored at
    /// `heading_span.start`) for headings with no inline text.
    pub title_span: SourceSpan,
    /// Byte span of the full heading element as reported by the parser.
    pub heading_span: SourceSpan,
    /// 1-indexed source line the heading starts on.
    pub line: usize,
}

/// Extracts every heading in `content` with spans and document-unique slugs.
///
/// Uses the same parser pass and slug authority as [`MarkdownToc`]; unlike
/// the TOC (which keeps duplicate slugs identical), repeated slugs are
/// disambiguated with `-1`/`-2` suffixes in document order.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::extract_headings;
///
/// let headings = extract_headings("# Setup\n\n## Setup\n");
/// assert_eq!(headings[0].slug, "setup");
/// assert_eq!(headings[1].slug, "setup-1");
/// ```
pub fn extract_headings(content: &str) -> Vec<HeadingRecord> {
    let (headings, _, _) = extract_elements(content);
    let mut slug_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    headings
        .into_iter()
        .map(|heading| {
            let count = slug_counts.entry(heading.slug.clone()).or_insert(0);
            let slug = if *count == 0 {
                heading.slug
            } else {
                format!("{}-{}", heading.slug, count)
            };
            *count += 1;

            HeadingRecord {
                level: OurHeadingLevel::new(heading.level).unwrap_or(OurHeadingLevel::H1),
                title: heading.title,
                slug,
                title_span: heading.title_span,
                heading_span: heading.span,
                line: heading.start_line,
            }
        })
        .collect()
}

/// Information about a heading extracted during parsing.
struct HeadingInfo {
    level: u8,
    title: String,
    slug: String,
    /// Full heading element span as reported by the parser.
    span: SourceSpan,
    /// Span of the heading's inline text; empty (at `span.start`) for
    /// headings with no inline content.
    title_span: SourceSpan,
    start_line: usize,
}

/// In-flight heading state while walking parser events.
struct HeadingCapture {
    level: PulldownHeadingLevel,
    title: String,
    span: SourceSpan,
    title_span: Option<SourceSpan>,
}

/// Information about a code block extracted during parsing.
struct CodeBlockExtract {
    language: Option<String>,
    /// Full info string from fence (e.g., "mermaid title=\"foo\"")
    info_string: String,
    content: String,
    start_line: usize,
    end_line: usize,
}

/// Information about an internal link extracted during parsing.
struct InternalLinkExtract {
    target_slug: String,
    link_text: String,
    line_number: usize,
    byte_offset: usize,
}


/// Extracts headings, code blocks, and internal links from markdown content.
fn extract_elements(
    content: &str,
) -> (
    Vec<HeadingInfo>,
    Vec<CodeBlockExtract>,
    Vec<InternalLinkExtract>,
) {
    let parser = Parser::new(content);

    // Precomputed once so per-event line lookups binary-search instead of
    // rescanning `content[..offset]`, turning the former O(n²) per-event prefix
    // scan into O(n log n) while keeping byte-identical line numbers.
    let newline_offsets = newline_offset_table(content);
    let line_of = |offset: usize| line_at_offset(&newline_offsets, content, offset);

    let mut headings = Vec::new();
    let mut code_blocks = Vec::new();
    let mut internal_links = Vec::new();

    let mut current_heading: Option<HeadingCapture> = None;
    // (language, info_string, content, start_line)
    let mut current_code_block: Option<(Option<String>, String, String, usize)> = None;
    let mut current_link: Option<(String, String, usize)> = None;
    let mut in_link = false;
    let mut link_text = String::new();

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current_heading = Some(HeadingCapture {
                    level,
                    title: String::new(),
                    span: range.clone(),
                    title_span: None,
                });
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(capture) = current_heading.take() {
                    let slug = generate_slug(&capture.title);
                    let title_span = capture
                        .title_span
                        .unwrap_or(capture.span.start..capture.span.start);
                    headings.push(HeadingInfo {
                        level: heading_level_to_u8(capture.level),
                        title: capture.title,
                        slug,
                        start_line: line_of(capture.span.start),
                        span: capture.span,
                        title_span,
                    });
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some(capture) = current_heading.as_mut() {
                    capture.title.push_str(&text);
                    capture.title_span = Some(match capture.title_span.take() {
                        Some(existing) => existing.start..range.end,
                        None => range.clone(),
                    });
                }
                if let Some((_, _, ref mut code_content, _)) = current_code_block {
                    code_content.push_str(&text);
                }
                if in_link {
                    link_text.push_str(&text);
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let (lang, info_string) = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(info) => {
                        let info_str = info.to_string();
                        let lang_str = info.split_whitespace().next().unwrap_or("");
                        let lang = if lang_str.is_empty() {
                            None
                        } else {
                            Some(lang_str.to_string())
                        };
                        (lang, info_str)
                    }
                    pulldown_cmark::CodeBlockKind::Indented => (None, String::new()),
                };
                current_code_block =
                    Some((lang, info_string, String::new(), line_of(range.start)));
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some((language, info_string, code_content, start_line)) =
                    current_code_block.take()
                {
                    let end_line = line_of(range.start);
                    code_blocks.push(CodeBlockExtract {
                        language,
                        info_string,
                        content: code_content,
                        start_line,
                        end_line,
                    });
                }
            }
            // Check if it's an internal link (starts with #)
            Event::Start(Tag::Link { dest_url, .. }) if dest_url.starts_with('#') => {
                let target = dest_url.trim_start_matches('#').to_string();
                current_link = Some((target, String::new(), range.start));
                in_link = true;
                link_text.clear();
            }
            Event::End(TagEnd::Link) => {
                if let Some((target_slug, _, byte_offset)) = current_link.take() {
                    internal_links.push(InternalLinkExtract {
                        target_slug,
                        link_text: std::mem::take(&mut link_text),
                        line_number: line_of(range.start),
                        byte_offset,
                    });
                }
                in_link = false;
            }
            _ => {}
        }
    }

    (headings, code_blocks, internal_links)
}

/// Builds the hierarchical TOC structure from flat heading list.
fn build_hierarchy(headings: &[HeadingInfo], content: &str) -> (Vec<MarkdownTocNode>, String) {
    if headings.is_empty() {
        return (Vec::new(), content.to_string());
    }

    // Calculate preamble (content before first heading)
    let preamble = content[..headings[0].span.start].to_string();

    // Build nodes with byte ranges
    let mut nodes_with_ranges: Vec<(MarkdownTocNode, usize, usize)> = Vec::new();

    for (i, heading) in headings.iter().enumerate() {
        let start_byte = heading.span.start;
        let end_byte = if i + 1 < headings.len() {
            headings[i + 1].span.start
        } else {
            content.len()
        };

        let start_line = heading.start_line;
        let end_line = if i + 1 < headings.len() {
            headings[i + 1].start_line
        } else {
            content.lines().count() + 1
        };

        let mut node = MarkdownTocNode::new(
            OurHeadingLevel::new(heading.level).unwrap_or(OurHeadingLevel::H1),
            heading.title.clone(),
            heading.slug.clone(),
            (start_byte, end_byte),
            (start_line, end_line),
        );

        // Extract prelude content (from after heading line to next heading)
        let section_content = &content[start_byte..end_byte];
        if let Some(newline_pos) = section_content.find('\n') {
            let prelude_start_byte = start_byte + newline_pos + 1;
            let prelude_content = &section_content[newline_pos + 1..];

            // Calculate prelude line range
            let prelude_start_line = start_line + 1; // Line after heading
            let prelude_end_line = end_line;

            node.set_prelude(
                Some(prelude_content.to_string()),
                (prelude_start_byte, end_byte),
                (prelude_start_line, prelude_end_line),
            );
        }

        nodes_with_ranges.push((node, start_byte, end_byte));
    }

    // Build hierarchy using a stack-based approach
    let mut result: Vec<MarkdownTocNode> = Vec::new();
    let mut stack: Vec<MarkdownTocNode> = Vec::new();

    for (node, _, _) in nodes_with_ranges {
        // Pop nodes from stack that are at same or higher level
        while let Some(top) = stack.last() {
            if top.level >= node.level {
                let popped = stack.pop().unwrap();
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(popped);
                } else {
                    result.push(popped);
                }
            } else {
                break;
            }
        }
        stack.push(node);
    }

    // Pop remaining nodes
    while let Some(popped) = stack.pop() {
        if let Some(parent) = stack.last_mut() {
            parent.children.push(popped);
        } else {
            result.push(popped);
        }
    }

    // Update parent line_range to encompass children
    // This is needed so get_section_path can correctly find which section contains
    // a given line (e.g., for code blocks)
    fn update_line_ranges(node: &mut MarkdownTocNode) {
        for child in &mut node.children {
            update_line_ranges(child);
        }
        if let Some(last_child) = node.children.last() {
            // Extend parent's line_range to include all children
            node.line_range.1 = node.line_range.1.max(last_child.line_range.1);
        }
    }
    for node in &mut result {
        update_line_ranges(node);
    }

    // Compute subtree hashes
    for node in &mut result {
        node.compute_subtree_hash();
    }

    (result, preamble)
}

impl From<&Markdown> for MarkdownToc {
    fn from(md: &Markdown) -> Self {
        let content = md.content();
        let frontmatter = md.frontmatter();

        let mut toc = MarkdownToc::new();

        // Compute page hashes
        toc.page_hash = xx_hash(content);
        toc.page_hash_trimmed = xx_hash_variant(content, vec![HashVariant::BlockTrimming]);

        // Compute frontmatter hashes
        if !frontmatter.is_empty() {
            // Raw hash (preserves formatting)
            let raw_fm = serde_yaml_ng::to_string(&frontmatter.as_map()).unwrap_or_default();
            toc.frontmatter_hash = xx_hash(&raw_fm);

            // Normalized hash (canonical JSON for comparison)
            let normalized_fm = serde_json::to_string(&frontmatter.as_map()).unwrap_or_default();
            toc.frontmatter_hash_normalized = xx_hash(&normalized_fm);
        }

        // Extract elements
        let (headings, code_blocks, internal_links) = extract_elements(content);

        // Build hierarchy
        let (structure, preamble) = build_hierarchy(&headings, content);
        toc.structure = structure;

        // Set preamble
        toc.preamble = preamble.clone();
        toc.preamble_hash = xx_hash(&preamble);
        toc.preamble_hash_trimmed = xx_hash_variant(&preamble, vec![HashVariant::BlockTrimming]);

        // Determine title
        toc.title = frontmatter
            .get::<String>("title")
            .ok()
            .flatten()
            .or_else(|| {
                // Check for single H1
                let h1s: Vec<_> = toc
                    .structure
                    .iter()
                    .filter(|n| n.level == OurHeadingLevel::H1)
                    .collect();
                if h1s.len() == 1 {
                    Some(h1s[0].title.clone())
                } else {
                    None
                }
            });

        // Build section path helper
        fn get_section_path(structure: &[MarkdownTocNode], target_line: usize) -> Vec<String> {
            fn find_path(
                node: &MarkdownTocNode,
                target_line: usize,
                path: &mut Vec<String>,
            ) -> bool {
                if target_line >= node.line_range.0 && target_line < node.line_range.1 {
                    path.push(node.title.clone());
                    for child in &node.children {
                        if find_path(child, target_line, path) {
                            return true;
                        }
                    }
                    return true;
                }
                false
            }

            let mut path = Vec::new();
            for node in structure {
                if find_path(node, target_line, &mut path) {
                    break;
                }
            }
            path
        }

        // Add code blocks
        for cb in code_blocks {
            let section_path = get_section_path(&toc.structure, cb.start_line);
            toc.code_blocks.push(CodeBlockInfo::new(
                cb.language,
                cb.info_string,
                cb.content,
                (cb.start_line, cb.end_line),
                section_path,
            ));
        }

        // Add internal links
        for link in internal_links {
            let section_path = get_section_path(&toc.structure, link.line_number);
            toc.internal_links.push(InternalLinkInfo::new(
                link.target_slug,
                link.link_text,
                link.line_number,
                link.byte_offset,
                section_path,
            ));
        }

        // Build slug index
        fn add_slugs_to_index(
            node: &MarkdownTocNode,
            index: &mut std::collections::HashMap<String, Vec<(Vec<String>, usize)>>,
            path: Vec<String>,
        ) {
            let mut current_path = path;
            current_path.push(node.title.clone());

            index
                .entry(node.slug.clone())
                .or_default()
                .push((current_path.clone(), node.line_range.0));

            for child in &node.children {
                add_slugs_to_index(child, index, current_path.clone());
            }
        }

        for node in &toc.structure {
            add_slugs_to_index(node, &mut toc.slug_index, Vec::new());
        }

        toc
    }
}

impl From<Markdown> for MarkdownToc {
    fn from(md: Markdown) -> Self {
        MarkdownToc::from(&md)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_slug_simple() {
        assert_eq!(generate_slug("Hello World"), "hello-world");
    }

    #[test]
    fn test_generate_slug_special_chars() {
        assert_eq!(generate_slug("What's New?"), "whats-new");
    }

    #[test]
    fn test_generate_slug_multiple_spaces() {
        assert_eq!(generate_slug("Hello   World"), "hello-world");
    }

    #[test]
    fn test_generate_slug_with_numbers() {
        assert_eq!(generate_slug("Version 2.0"), "version-20");
    }

    #[test]
    fn test_toc_from_markdown_simple() {
        let content = "# Hello\n\nWorld\n\n## Section\n\nContent";
        let md: Markdown = content.into();
        let toc = MarkdownToc::from(&md);

        assert_eq!(toc.heading_count(), 2);
        assert_eq!(toc.root_level(), Some(OurHeadingLevel::H1));
        assert_eq!(toc.title, Some("Hello".to_string()));
    }

    #[test]
    fn test_toc_from_markdown_nested() {
        let content = r#"# Root

## Section 1

### Subsection 1.1

## Section 2

Content here.
"#;
        let md: Markdown = content.into();
        let toc = MarkdownToc::from(&md);

        assert_eq!(toc.heading_count(), 4);
        assert_eq!(toc.structure.len(), 1); // One root
        assert_eq!(toc.structure[0].children.len(), 2); // Two H2s
        assert_eq!(toc.structure[0].children[0].children.len(), 1); // One H3 under first H2
    }

    #[test]
    fn test_toc_preamble() {
        let content = "Some intro text\n\n# Heading\n\nContent";
        let md: Markdown = content.into();
        let toc = MarkdownToc::from(&md);

        assert!(toc.preamble.contains("Some intro text"));
    }

    #[test]
    fn test_toc_code_blocks() {
        let content = r#"# Code Examples

```rust
fn main() {}
```

## More

```javascript
console.log("hi");
```
"#;
        let md: Markdown = content.into();
        let toc = MarkdownToc::from(&md);

        assert_eq!(toc.code_blocks.len(), 2);
        assert_eq!(toc.code_blocks[0].language, Some("rust".to_string()));
        assert_eq!(toc.code_blocks[1].language, Some("javascript".to_string()));
    }

    #[test]
    fn test_toc_internal_links() {
        let content = r#"# Introduction

See [getting started](#getting-started).

## Getting Started

Content here.
"#;
        let md: Markdown = content.into();
        let toc = MarkdownToc::from(&md);

        assert_eq!(toc.internal_links.len(), 1);
        assert_eq!(toc.internal_links[0].target_slug, "getting-started");
        assert!(!toc.has_broken_links());
    }

    #[test]
    fn test_toc_broken_links() {
        let content = r#"# Introduction

See [nonexistent](#nonexistent).
"#;
        let md: Markdown = content.into();
        let toc = MarkdownToc::from(&md);

        assert!(toc.has_broken_links());
        assert_eq!(toc.broken_links().len(), 1);
    }

    #[test]
    fn test_toc_slug_index() {
        let content = "# Hello\n\n## World\n\n### Nested";
        let md: Markdown = content.into();
        let toc = MarkdownToc::from(&md);

        assert!(toc.slug_index.contains_key("hello"));
        assert!(toc.slug_index.contains_key("world"));
        assert!(toc.slug_index.contains_key("nested"));
    }

    #[test]
    fn test_toc_find_by_slug() {
        let content = "# Root\n\n## Child";
        let md: Markdown = content.into();
        let toc = MarkdownToc::from(&md);

        let found = toc.find_by_slug("child");
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "Child");
    }

    #[test]
    fn test_toc_all_headings() {
        let content = "# A\n\n## B\n\n### C";
        let md: Markdown = content.into();
        let toc = MarkdownToc::from(&md);

        let all = toc.all_headings();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].title, "A");
        assert_eq!(all[1].title, "B");
        assert_eq!(all[2].title, "C");
    }

    #[test]
    fn test_toc_with_frontmatter_title() {
        let content = "---\ntitle: Custom Title\n---\n# Heading\n\nContent";
        let md: Markdown = content.into();
        let toc = MarkdownToc::from(&md);

        assert_eq!(toc.title, Some("Custom Title".to_string()));
    }

    #[test]
    fn test_toc_multiple_h1s() {
        let content = "# First\n\n# Second";
        let md: Markdown = content.into();
        let toc = MarkdownToc::from(&md);

        // Multiple H1s = no automatic title
        assert!(toc.title.is_none());
        assert_eq!(toc.structure.len(), 2);
    }

    #[test]
    fn test_toc_max_level() {
        let content = "## H2\n\n### H3\n\n#### H4\n\n##### H5";
        let md: Markdown = content.into();
        let toc = MarkdownToc::from(&md);

        assert_eq!(toc.max_level(), Some(OurHeadingLevel::H5));
    }

    #[test]
    fn test_toc_empty_document() {
        let content = "Just some text without headings.";
        let md: Markdown = content.into();
        let toc = MarkdownToc::from(&md);

        assert_eq!(toc.heading_count(), 0);
        assert!(toc.structure.is_empty());
        assert!(toc.title.is_none());
        // The entire content becomes preamble
        assert!(toc.preamble.contains("Just some text"));
    }

    #[test]
    fn test_generate_heading_slug_parity_with_toc() {
        let content = "# Hello World\n\n## What's New?\n\n### Version 2.0\n\n## multi   space\n";
        let md: Markdown = content.into();
        let toc = MarkdownToc::from(&md);

        for node in toc.all_headings() {
            assert_eq!(generate_heading_slug(&node.title), node.slug);
        }
    }

    #[test]
    fn test_extract_headings_basic() {
        let content = "# Hello\n\nBody text.\n\n## World\n";
        let headings = extract_headings(content);

        assert_eq!(headings.len(), 2);

        let first = &headings[0];
        assert_eq!(first.level, OurHeadingLevel::H1);
        assert_eq!(first.title, "Hello");
        assert_eq!(first.slug, "hello");
        assert_eq!(first.line, 1);
        assert_eq!(&content[first.title_span.clone()], "Hello");
        assert!(content[first.heading_span.clone()].starts_with("# Hello"));

        let second = &headings[1];
        assert_eq!(second.level, OurHeadingLevel::H2);
        assert_eq!(second.line, 5);
        assert_eq!(&content[second.title_span.clone()], "World");
        assert!(content[second.heading_span.clone()].starts_with("## World"));
    }

    #[test]
    fn test_extract_headings_duplicate_slug_suffixing() {
        let content = "# Setup\n\n## Setup\n\n### Setup\n\n## Other\n";
        let slugs: Vec<String> = extract_headings(content)
            .into_iter()
            .map(|heading| heading.slug)
            .collect();

        assert_eq!(slugs, vec!["setup", "setup-1", "setup-2", "other"]);
    }

    #[test]
    fn test_extract_headings_duplicates_share_base_slug_with_toc() {
        let content = "# Setup\n\n## Setup\n";
        let headings = extract_headings(content);
        // The unsuffixed base is the TOC slug; only the record slug carries
        // the disambiguating suffix.
        for heading in &headings {
            assert_eq!(generate_heading_slug(&heading.title), "setup");
        }
    }

    #[test]
    fn test_extract_headings_inline_markup_title() {
        let content = "# Hello *World* `code`\n";
        let headings = extract_headings(content);

        assert_eq!(headings[0].title, "Hello World code");
        let title_text = &content[headings[0].title_span.clone()];
        assert!(title_text.starts_with("Hello"));
        assert!(title_text.contains("World"));
        assert!(title_text.contains("code"));
    }

    #[test]
    fn test_extract_headings_empty_title() {
        let content = "#\n\ntext\n";
        let headings = extract_headings(content);

        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].title, "");
        assert!(headings[0].title_span.is_empty());
        assert_eq!(headings[0].title_span.start, headings[0].heading_span.start);
    }

    #[test]
    fn test_extract_headings_setext() {
        let content = "Title Line\n==========\n\nBody.\n";
        let headings = extract_headings(content);

        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].level, OurHeadingLevel::H1);
        assert_eq!(headings[0].title, "Title Line");
        assert_eq!(headings[0].line, 1);
        assert_eq!(&content[headings[0].title_span.clone()], "Title Line");
        assert!(content[headings[0].heading_span.clone()].contains("=========="));
    }

    #[test]
    fn test_extract_headings_matches_toc_order_and_lines() {
        let content = "# A\n\ntext\n\n## B\n\n### C\n\n## D\n";
        let md: Markdown = content.into();
        let toc = MarkdownToc::from(&md);
        let records = extract_headings(content);
        let toc_headings = toc.all_headings();

        assert_eq!(records.len(), toc_headings.len());
        for (record, node) in records.iter().zip(toc_headings.iter()) {
            assert_eq!(record.title, node.title);
            assert_eq!(record.slug, node.slug);
            assert_eq!(record.level, node.level);
            assert_eq!(record.line, node.line_range.0);
            assert_eq!(record.heading_span.start, node.source_span.0);
        }
    }

    #[test]
    fn test_line_at_offset_matches_naive_lines_count() {
        // The precomputed newline-table + binary search must reproduce the
        // original `content[..offset].lines().count() + 1` for every offset,
        // across the divergent cases: empty prefix, mid-line offsets, offsets
        // landing exactly on a `\n`, CRLF, and offset == len.
        let cases = [
            "",
            "no newlines here",
            "# One\n\nSee [two](#two).\n\n## Two\n\n```\ncode\n```\n\n### Three\n",
            "a\r\nb\r\nc",
            "trailing\n",
            "\n\n\n",
        ];
        for content in cases {
            let table = newline_offset_table(content);
            for offset in 0..=content.len() {
                if !content.is_char_boundary(offset) {
                    continue;
                }
                let expected = content[..offset].lines().count() + 1;
                assert_eq!(
                    line_at_offset(&table, content, offset),
                    expected,
                    "offset {offset} in {content:?}"
                );
            }
        }
    }

    #[test]
    fn test_toc_line_numbers_multi_heading_fixture() {
        // Integration guard: headings, code blocks, and internal links across a
        // multi-heading fixture must carry line numbers identical to the naive
        // per-event formula the rewrite replaced.
        let content = concat!(
            "# One\n",              // line 1
            "\n",                   // line 2
            "See [two](#two).\n",   // line 3 (internal link, mid-line)
            "\n",                   // line 4
            "## Two\n",             // line 5
            "\n",                   // line 6
            "```rust\n",            // line 7 (code block)
            "fn main() {}\n",       // line 8
            "```\n",                // line 9
            "\n",                   // line 10
            "### Three\n",          // line 11
        );
        let (headings, code_blocks, internal_links) = extract_elements(content);
        let naive = |offset: usize| content[..offset].lines().count() + 1;

        assert_eq!(
            headings.iter().map(|h| h.start_line).collect::<Vec<_>>(),
            vec![1, 5, 11]
        );
        for heading in &headings {
            assert_eq!(heading.start_line, naive(heading.span.start));
        }

        assert_eq!(code_blocks.len(), 1);
        assert_eq!(code_blocks[0].start_line, 7);

        assert_eq!(internal_links.len(), 1);
        assert_eq!(internal_links[0].line_number, naive(internal_links[0].byte_offset));
    }

    #[test]
    fn test_toc_ignores_tab_indented_frontmatter_content() {
        let content = "---\nprompt: |-\n\tLine one\n\tLine two\nlast_updated: 2026-02-27\n---\n# macOS Audio\n\n## Getting Started\n";
        let md: Markdown = content.into();
        let toc = md.toc();

        assert_eq!(toc.heading_count(), 2);
        assert_eq!(toc.structure.len(), 1);
        assert_eq!(toc.structure[0].title, "macOS Audio");
        assert_eq!(toc.structure[0].children[0].title, "Getting Started");

        let all_titles: Vec<&str> = toc
            .all_headings()
            .iter()
            .map(|node| node.title.as_str())
            .collect();
        assert_eq!(all_titles, vec!["macOS Audio", "Getting Started"]);
        assert!(
            !all_titles
                .iter()
                .any(|title| title.contains("last_updated") || title.contains("update_policy"))
        );
    }
}
