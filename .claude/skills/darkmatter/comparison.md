# Document Comparison

Structural diff with change classification for markdown documents.

## Basic Usage

```rust
let original: Markdown = old_content.into();
let updated: Markdown = new_content.into();

let delta = original.delta(&updated);

if !delta.is_unchanged() {
    println!("Classification: {:?}", delta.classification);
    println!("{}", delta.summary());
}
```

## Change Classification

| Classification | Description |
|----------------|-------------|
| `Unchanged` | No differences |
| `ContentOnly` | Body changed, structure intact |
| `StructuralMinor` | Minor heading changes |
| `StructuralMajor` | Significant heading reorganization |
| `FrontmatterOnly` | Only metadata changed |
| `Mixed` | Multiple change types |

## Delta Fields

```rust
pub struct Delta {
    pub classification: Classification,
    pub frontmatter_changed: bool,
    pub heading_changes: Vec<HeadingChange>,
    pub content_diff: Option<ContentDiff>,
}

impl Delta {
    pub fn is_unchanged(&self) -> bool;
    pub fn summary(&self) -> String;
}
```

## Heading Normalization

```rust
// Fix hierarchy violations
let normalized = md.normalize_headings()?;

// Relevel entire document (e.g., start at h2)
let releveled = md.relevel(2)?;
```

## Table of Contents

```rust
// Extract hierarchical TOC
let toc = md.table_of_contents();

for entry in toc.entries {
    println!("{}{}", "  ".repeat(entry.level - 1), entry.text);
}
```
