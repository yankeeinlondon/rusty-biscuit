# Markdown Processing

The `markdown` module provides comprehensive document manipulation with frontmatter support, AST operations, and delta analysis.

## Core Types

### Markdown Document

```rust
use shared::markdown::Markdown;

// Parse from string with frontmatter detection
let content = r#"---
title: My Document
author: Alice
---
# Introduction

Content here..."#;

let md: Markdown = content.into();

// Access frontmatter
let title: Option<String> = md.fm_get("title")?;

// Access content (without frontmatter)
let body = md.content();
```

### Loading Documents

```rust
// From file
let md = Markdown::try_from(Path::new("README.md"))?;

// From URL (async)
let url = Url::parse("https://example.com/doc.md")?;
let md = Markdown::from_url(&url).await?;

// From string
let md: Markdown = "# Hello".into();
```

## Frontmatter Operations

### Typed Access

```rust
// Get typed values
let tags: Vec<String> = md.fm_get("tags")?.unwrap_or_default();
let published: bool = md.fm_get("published")?.unwrap_or(false);

// Insert values
md.fm_insert("updated", chrono::Utc::now())?;
md.fm_insert("version", 2)?;
```

### Merge Strategies

```rust
use shared::markdown::MergeStrategy;
use serde_json::json;

// Error on conflict
let data = json!({"author": "Bob", "tags": ["new"]});
md.fm_merge_with(data, MergeStrategy::ErrorOnConflict)?;

// Prefer new values
md.fm_merge_with(data, MergeStrategy::PreferNew)?;

// Keep existing values
md.fm_merge_with(data, MergeStrategy::PreferExisting)?;

// Set defaults (only fills missing keys)
let defaults = json!({"draft": true, "author": "Anonymous"});
md.fm_set_defaults(defaults)?;
```

## Heading Structure

### Validation

```rust
use shared::markdown::HeadingLevel;

let validation = md.validate_structure();

if validation.is_well_formed() {
    println!("Document structure is valid");
    println!("Root level: {:?}", validation.root_level);
    println!("Total headings: {}", validation.heading_count);
} else {
    for issue in &validation.issues {
        println!("Issue: {:?}", issue);
    }
}
```

### Normalization

```rust
// Normalize to H1 root
let (normalized, report) = md.normalize(Some(HeadingLevel::H1))?;

// Keep current root, fix violations only
let (normalized, report) = md.normalize(None)?;

// In-place normalization
let report = md.normalize_mut(Some(HeadingLevel::H1))?;

println!("Changes made: {}", report.has_changes());
println!("Level adjustment: {}", report.level_adjustment);
```

### Re-leveling

```rust
// Demote all headings by 1 level (for embedding)
let (releveled, adjustment) = md.relevel(HeadingLevel::H2)?;
assert_eq!(adjustment, 1); // Positive = demoted
```

## Table of Contents

```rust
let toc = md.toc();

println!("Title: {:?}", toc.title);
println!("Heading count: {}", toc.heading_count());
println!("Code blocks: {}", toc.code_blocks().len());
println!("Internal links: {}", toc.internal_links().len());

// Iterate heading tree
for node in toc.nodes() {
    println!("{} {}", "#".repeat(node.level as usize), node.text);
}
```

## Delta Analysis

Compare two documents for detailed change tracking:

```rust
let original: Markdown = old_content.into();
let updated: Markdown = new_content.into();

let delta = original.delta(&updated);

// High-level classification
if delta.is_unchanged() {
    println!("No changes");
} else if delta.is_minor_edit() {
    println!("Minor edits only");
} else if delta.is_major_rewrite() {
    println!("Major rewrite detected");
}

// Detailed statistics
println!("{}", delta.summary());
println!("Lines added: {}", delta.statistics.lines_added);
println!("Sections moved: {}", delta.moved_sections.len());

// Frontmatter changes
for change in &delta.frontmatter_changes {
    println!("Frontmatter: {:?}", change);
}
```

## Output Formats

### String Representation

```rust
// Serialize back to markdown with frontmatter
let output = md.as_string();
```

### HTML Rendering

```rust
use shared::markdown::output::HtmlOptions;
use shared::markdown::highlighting::{ThemePair, ColorMode};

let options = HtmlOptions::default()
    .with_theme_pair(ThemePair::Gruvbox)
    .with_color_mode(ColorMode::Dark)
    .with_mermaid_support(true);

let html = md.as_html(options)?;
```

### AST Representation

```rust
// Get mdast (Markdown Abstract Syntax Tree)
let ast = md.as_ast()?;

// Serialize to JSON
let json = serde_json::to_string_pretty(&ast)?;
```

## Content Cleanup

```rust
// Normalize formatting (spacing, table alignment)
md.cleanup();

// Chain operations
md.cleanup()
  .fm_insert("updated", chrono::Utc::now())?;
```

## Visual Diffs

The delta module provides visual diff outputs:

```rust
use shared::markdown::delta::visual::{unified_diff, side_by_side_diff};

// Unified diff format (like git)
let unified = unified_diff(&original, &updated)?;

// Side-by-side comparison
let side_by_side = side_by_side_diff(&original, &updated)?;
```

## Common Patterns

### Document Template

```rust
// Create document with metadata
let mut md = Markdown::new("# Title\n\nContent...".to_string());
md.fm_insert("title", "My Document")?;
md.fm_insert("date", chrono::Utc::now())?;
md.fm_insert("tags", vec!["rust", "markdown"])?;
md.cleanup();

let output = md.as_string();
```

### Batch Processing

```rust
use tokio::fs;

// Process all markdown files
let entries = fs::read_dir("docs").await?;
while let Some(entry) = entries.next_entry().await? {
    if entry.path().extension() == Some("md".as_ref()) {
        let mut md = Markdown::try_from(entry.path().as_path())?;
        md.normalize_mut(Some(HeadingLevel::H1))?;
        md.cleanup();
        fs::write(entry.path(), md.as_string()).await?;
    }
}
```

## Error Handling

```rust
use shared::markdown::{MarkdownError, NormalizationError};

match md.normalize(Some(HeadingLevel::H1)) {
    Ok((normalized, report)) => { /* ... */ }
    Err(NormalizationError::NoHeadings) => {
        println!("Document has no headings to normalize");
    }
    Err(NormalizationError::LevelOutOfBounds { .. }) => {
        println!("Would push headings beyond H6");
    }
}
```