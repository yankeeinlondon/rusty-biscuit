# Document Comparison

## Contents

- Delta (Structural Diff)
- DocumentChange Classification (9 variants)
- ChangeAction (14 variants)
- Key Types
- Visual Diff (Standalone)
- Heading Normalization
- Table of Contents

Use heading search to jump to the listed subsystem.


Structural diff with change classification for markdown documents, plus visual diff for arbitrary strings/files.

## Delta (Structural Diff)

```rust
let original: Markdown = old_content.into();
let updated: Markdown = new_content.into();

let delta = original.delta(&updated);

if !delta.is_unchanged() {
    println!("Classification: {:?}", delta.classification);
    println!("{}", delta.summary());
}
```

## DocumentChange Classification (9 variants)

| Variant | Description |
|---------|-------------|
| `NoChange` | No differences |
| `WhitespaceOnly` | Only whitespace changed |
| `FrontmatterOnly` | Only frontmatter metadata changed |
| `FrontmatterAndWhitespace` | Both frontmatter and whitespace |
| `StructuralOnly` | Heading structure changed |
| `ContentMinor` | Small content edits |
| `ContentModerate` | Moderate content changes |
| `ContentMajor` | Significant content changes |
| `Rewritten` | Completely different document |

## ChangeAction (14 variants)

| Category | Variants |
|----------|----------|
| Structural | `Added`, `Removed`, `Renamed`, `Promoted`, `Demoted`, `Reordered`, `MovedSameLevel`, `MovedDifferentLevel` |
| Content | `ContentModified`, `WhitespaceOnly` |
| Frontmatter | `PropertyAdded`, `PropertyRemoved`, `PropertyUpdated`, `PropertyReordered` |

## Key Types

| Type | Description |
|------|-------------|
| `MarkdownDelta` | Complete diff result with classification, statistics, changes |
| `DocumentChange` | Change classification enum (9 variants) |
| `DeltaStatistics` | Numeric change counts |
| `ContentChange` | Individual content change with `ChangeAction` |
| `MovedSection` | Section relocation info |
| `FrontmatterChange` | Frontmatter key change |
| `BrokenLink` | Potentially broken internal link |
| `CodeBlockChange` | Code block modification |

## Visual Diff (Standalone)

The `diff::visual` module provides markdown-agnostic visual diff for any strings or files:

```rust
use darkmatter::diff::visual::{render_visual_diff_str, VisualDiffOptions};

let output = render_visual_diff_str(original, updated, &VisualDiffOptions::default());
println!("{}", output);
```

```rust
use darkmatter::diff::visual::render_visual_diff_files;

let output = render_visual_diff_files(
    Path::new("old.txt"),
    Path::new("new.txt"),
    &VisualDiffOptions::default(),
)?;
```

## Heading Normalization

```rust
use darkmatter::markdown::HeadingLevel;

// Validate structure
let validation = md.validate_structure();
if !validation.is_well_formed() {
    println!("Issues: {:?}", validation.issues);
}

// Normalize to target root level
let (normalized, report) = md.normalize(Some(HeadingLevel::H1))?;

// Relevel for embedding as subsection
let (releveled, adjustment) = md.relevel(HeadingLevel::H2)?;
```

## Table of Contents

```rust
let toc = md.toc();

println!("Heading count: {}", toc.heading_count());
println!("Root level: {:?}", toc.root_level());
println!("Title: {:?}", toc.title);
```

### MarkdownToc Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `heading_count()` | `usize` | Total headings |
| `root_level()` | `Option<u8>` | Level of first heading |
| `max_level()` | `Option<u8>` | Deepest heading level |
| `find_by_slug(slug)` | `Option<&MarkdownTocNode>` | Find heading by slug |
| `all_headings()` | `Vec<&MarkdownTocNode>` | Flat depth-first list |
| `has_broken_links()` | `bool` | Any broken internal links |
| `broken_links()` | `Vec<&InternalLinkInfo>` | Links with no target |
