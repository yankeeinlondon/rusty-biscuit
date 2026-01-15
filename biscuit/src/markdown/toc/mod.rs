//! Markdown Table of Contents extraction and analysis.
//!
//! This module provides functionality to extract a structured Table of Contents
//! from markdown documents, including heading hierarchy, content hashing,
//! code block tracking, and internal link detection.
//!
//! ## Examples
//!
//! ```rust
//! use shared::markdown::Markdown;
//!
//! let content = "# Introduction\n\nWelcome.\n\n## Getting Started\n\nFirst steps.";
//! let md: Markdown = content.into();
//! let toc = md.toc();
//!
//! assert_eq!(toc.heading_count(), 2);
//! assert_eq!(toc.root_level(), Some(1));
//! assert_eq!(toc.title, Some("Introduction".to_string()));
//! ```

mod types;

pub use types::{CodeBlockInfo, InternalLinkInfo, MarkdownToc, MarkdownTocNode, PreludeNode};

use crate::hashing::{HashVariant, xx_hash, xx_hash_variant};
use crate::markdown::Markdown;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

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
fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Information about a heading extracted during parsing.
struct HeadingInfo {
    level: u8,
    title: String,
    slug: String,
    start_byte: usize,
    start_line: usize,
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

    let mut headings = Vec::new();
    let mut code_blocks = Vec::new();
    let mut internal_links = Vec::new();

    let mut current_heading: Option<(HeadingLevel, String, usize)> = None;
    // (language, info_string, content, start_line)
    let mut current_code_block: Option<(Option<String>, String, String, usize)> = None;
    let mut current_link: Option<(String, String, usize)> = None;
    let mut in_link = false;
    let mut link_text = String::new();

    for (event, range) in parser.into_offset_iter() {
        let line_number = content[..range.start].lines().count() + 1;

        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current_heading = Some((level, String::new(), range.start));
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, title, start_byte)) = current_heading.take() {
                    let slug = generate_slug(&title);
                    headings.push(HeadingInfo {
                        level: heading_level_to_u8(level),
                        title,
                        slug,
                        start_byte,
                        start_line: content[..start_byte].lines().count() + 1,
                    });
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, ref mut title, _)) = current_heading {
                    title.push_str(&text);
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
                current_code_block = Some((lang, info_string, String::new(), line_number));
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some((language, info_string, code_content, start_line)) =
                    current_code_block.take()
                {
                    let end_line = line_number;
                    code_blocks.push(CodeBlockExtract {
                        language,
                        info_string,
                        content: code_content,
                        start_line,
                        end_line,
                    });
                }
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                // Check if it's an internal link (starts with #)
                if dest_url.starts_with('#') {
                    let target = dest_url.trim_start_matches('#').to_string();
                    current_link = Some((target, String::new(), range.start));
                    in_link = true;
                    link_text.clear();
                }
            }
            Event::End(TagEnd::Link) => {
                if let Some((target_slug, _, byte_offset)) = current_link.take() {
                    internal_links.push(InternalLinkExtract {
                        target_slug,
                        link_text: std::mem::take(&mut link_text),
                        line_number,
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
    let preamble = content[..headings[0].start_byte].to_string();

    // Build nodes with byte ranges
    let mut nodes_with_ranges: Vec<(MarkdownTocNode, usize, usize)> = Vec::new();

    for (i, heading) in headings.iter().enumerate() {
        let start_byte = heading.start_byte;
        let end_byte = if i + 1 < headings.len() {
            headings[i + 1].start_byte
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
            heading.level,
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
            let raw_fm = serde_yaml::to_string(&frontmatter.as_map()).unwrap_or_default();
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
                let h1s: Vec<_> = toc.structure.iter().filter(|n| n.level == 1).collect();
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
        assert_eq!(toc.root_level(), Some(1));
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

        assert_eq!(toc.max_level(), Some(5));
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
}
