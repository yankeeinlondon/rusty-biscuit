# Markdown Table of Contents

Extract hierarchical table of contents from markdown documents.

## Features

- **Heading Hierarchy**: Nested structure matching document outline
- **Content Hashing**: xxHash fingerprints at title, prelude, subtree, and page levels
- **Code Block Tracking**: Language, content, and location of code blocks
- **Internal Link Detection**: Track internal anchor links and detect broken references
- **Preamble Extraction**: Content before the first heading
- **Prelude Extraction**: Content between a heading and its first child heading

## Usage

```rust
use darkmatter::markdown::Markdown;

let content = "# Introduction\n\nWelcome.\n\n## Getting Started\n\nFirst steps.";
let md: Markdown = content.into();
let toc = md.toc();

assert_eq!(toc.heading_count(), 2);
assert_eq!(toc.root_level(), Some(1));
assert_eq!(toc.title, Some("Introduction".to_string()));
```

## Key Types

| Type | Description |
|------|-------------|
| `MarkdownToc` | Complete TOC with structure, code blocks, links, and hashes |
| `MarkdownTocNode` | Single heading with location, hashes, prelude, and children |
| `CodeBlockInfo` | Code block metadata (language, content, hashes, location) |
| `InternalLinkInfo` | Internal anchor link target and location |

Note: `PreludeNode` is used internally within `MarkdownTocNode` but is not re-exported from the `markdown` module.

## Node Structure

```rust
pub struct MarkdownTocNode {
    pub level: u8,                       // Heading level (1-6)
    pub title: String,                   // Heading text
    pub title_hash: u64,                 // xxHash of title
    pub title_hash_trimmed: u64,         // xxHash of title after trimming
    pub slug: String,                    // URL-safe anchor
    pub source_span: (usize, usize),     // Byte offset range [start, end)
    pub line_range: (usize, usize),      // Line number range [start, end), 1-indexed
    pub prelude: Option<PreludeNode>,    // Content before first child heading
    pub subtree_hash: u64,               // xxHash of this node + all descendants
    pub subtree_hash_trimmed: u64,       // xxHash of subtree after trimming
    pub children: Vec<MarkdownTocNode>,  // Child headings
}
```

## MarkdownToc Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `heading_count()` | `usize` | Total headings in the document |
| `root_level()` | `Option<u8>` | Level of the first heading |
| `max_level()` | `Option<u8>` | Deepest heading level |
| `find_by_slug(slug)` | `Option<&MarkdownTocNode>` | Find a heading by its slug |
| `all_headings()` | `Vec<&MarkdownTocNode>` | Flat depth-first list of all headings |
| `has_broken_links()` | `bool` | Whether any internal links have no target |
| `broken_links()` | `Vec<&InternalLinkInfo>` | Internal links with no matching slug |

## Traversal

```rust
fn visit_node(node: &MarkdownTocNode, depth: usize) {
    let indent = "  ".repeat(depth);
    println!("{}{} ({})", indent, node.title, node.slug);
    for child in &node.children {
        visit_node(child, depth + 1);
    }
}

for root in &toc.structure {
    visit_node(root, 0);
}
```
