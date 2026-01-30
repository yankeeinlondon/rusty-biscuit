# Markdown Table of Contents

Extract hierarchical table of contents from markdown documents.

## Features

- **Heading Hierarchy**: Nested structure matching document outline
- **Content Hashing**: XXH64 hash of each section's content
- **Code Block Tracking**: Language, content, and location of code blocks
- **Internal Link Detection**: Track internal anchor links
- **Preamble Extraction**: Content before first heading

## Usage

```rust
use darkmatter_lib::markdown::Markdown;

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
| `MarkdownToc` | Complete TOC structure |
| `MarkdownTocNode` | Single heading with children |
| `CodeBlockInfo` | Code block metadata |
| `InternalLinkInfo` | Internal link target info |
| `PreludeNode` | Content before first heading |

## Node Structure

```rust
pub struct MarkdownTocNode {
    pub level: u8,           // Heading level (1-6)
    pub title: String,       // Heading text
    pub slug: String,        // URL-safe anchor
    pub hash: u64,           // Content hash
    pub start_byte: usize,   // Position in source
    pub start_line: usize,   // Line number
    pub children: Vec<MarkdownTocNode>,
}
```

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
