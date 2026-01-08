# Markdown struct

```rust
struct Markdown {
    frontmatter: HashMap,
    content: String
}
```

- this struct must implement the `From` trait for
    - actual markdown content (`String` or `&str`)
- this struct must implement the `TryFrom` trait for
    - a file path reference or file URI (e.g., `file::`) to a markdown file (`String` or `&str`)
    - a URL to a resource which is _expected_ to return a Markdown file (`String`, `&str`, `Url`, `&Url`)

## Markdown Cleanup

This struct should implement a `cleanup` method which will cleanup the markdown content.

- this cleanup will be achieved by using `pulldown-cmark-to-cmark` crate alongside the `pulldown-cmark` crate.
    - `pulldown-cmark` acts as the Producer / Parser
    - `pulldown-cmark-to-cmark` acts as the Consumer / Renderer
- this process must ensure the output Markdown is valid CommonMark markdown (allowing for GFM)
- in addition we do want to ensure that a header, code block, or list is always isolated by blank lines, you need to manually inspect the Event stream and inject a "spacing" logic (see example 1).
- Although not a requirement for CommonMark Markdown, we want to ensure that Markdown tables have blank spaces padded into the cell to align the columns of the tables; making them much more readable by humans.


[Markdown Cleanup Examples](./md/markdown-cleanup.md)

## Frontmatter Mutations

- `fm_merge_with<T>(obj: T)`
    - merges the document's markdown with an external dictionary of key/values
    - the external set has _precedence_ and will override what the document started with
- `fm_defaults<T>(obj: T)`
    - merges the document's markdown with an external dictionary of key/values
    - the document's key/values are given _precedence_ in the merge process; allowing the external dictionary to only _add_ new key/values but not change the document's original key/value pairings

## Table of Contents (TOC)

The `Markdown` struct provides a `toc()` method that extracts a structured Table of Contents from the document.

### MarkdownToc

```rust
let md: Markdown = content.into();
let toc = md.toc();

// Access document structure
assert_eq!(toc.heading_count(), 5);
assert_eq!(toc.root_level(), Some(HeadingLevel::H1));
```

The `MarkdownToc` struct provides:

- **Document Identity**: Title, page hash, frontmatter hash
- **Preamble**: Content before the first heading with hashes
- **Structure**: Hierarchical `Vec<MarkdownTocNode>` representing heading tree
- **Code Blocks**: `Vec<CodeBlockInfo>` for all fenced code blocks
- **Internal Links**: `Vec<InternalLinkInfo>` for anchor references
- **Slug Index**: `HashMap<String, Vec<(SectionPath, usize)>>` for quick lookup

### MarkdownTocNode

Each node in the TOC represents a heading with:

- `level: u8` - Heading level (1-6)
- `title: String` - Heading text
- `title_hash: u64` / `title_hash_trimmed: u64` - xxHash of title
- `slug: String` - URL-safe anchor slug
- `source_span: (usize, usize)` - Byte offset range
- `line_range: (usize, usize)` - Line number range (1-indexed)
- `own_content: Option<String>` - Section content (excluding children)
- `own_content_hash: u64` / `own_content_hash_trimmed: u64`
- `subtree_hash: u64` / `subtree_hash_trimmed: u64` - Hash including all descendants
- `children: Vec<MarkdownTocNode>` - Child headings

### Usage Example

```rust
use shared::markdown::Markdown;

let content = r#"
# Introduction

Welcome to the guide.

## Getting Started

First steps here.

### Prerequisites

What you need.
"#;

let md: Markdown = content.into();
let toc = md.toc();

// Quick checks
assert_eq!(toc.heading_count(), 3);
assert_eq!(toc.root_level(), Some(HeadingLevel::H1));
assert_eq!(toc.title, Some("Introduction".to_string()));

// Find by slug
if let Some(node) = toc.find_by_slug("getting-started") {
    println!("Found: {}", node.title);
}

// Check for broken links
if toc.has_broken_links() {
    for link in toc.broken_links() {
        println!("Broken link: #{}", link.target_slug);
    }
}
```

## Document Delta

The `Markdown` struct provides a `delta()` method for comparing two documents.

### MarkdownDelta

```rust
let original: Markdown = "# Hello\n\nWorld".into();
let updated: Markdown = "# Hello\n\nUniverse".into();

let delta = original.delta(&updated);

if delta.is_unchanged() {
    println!("No changes");
} else {
    println!("{}", delta.summary());
}
```

The `MarkdownDelta` struct provides:

- **Classification**: `DocumentChange` enum (NoChange, WhitespaceOnly, ContentMinor, Rewritten, etc.)
- **Statistics**: `DeltaStatistics` with metrics like `sections_added`, `content_change_ratio`
- **Frontmatter Changes**: `Vec<FrontmatterChange>` for property-level changes
- **Content Changes**: `added`, `removed`, `modified` vectors of `ContentChange`
- **Moved Sections**: `Vec<MovedSection>` for sections that relocated
- **Code Block Changes**: `Vec<CodeBlockChange>`
- **Broken Links**: `Vec<BrokenLink>` with optional suggested replacements

### ChangeAction Enum

```rust
pub enum ChangeAction {
    // Structural
    Added, Removed, Renamed, Promoted, Demoted, Reordered,
    MovedSameLevel, MovedDifferentLevel,

    // Content
    ContentModified, WhitespaceOnly,

    // Frontmatter
    PropertyAdded, PropertyRemoved, PropertyUpdated, PropertyReordered,
}
```

### DocumentChange Classification

```rust
pub enum DocumentChange {
    NoChange,           // All hashes match
    WhitespaceOnly,     // Trimmed hashes match
    FrontmatterOnly,    // Body unchanged
    StructuralOnly,     // Content unchanged, sections moved
    ContentMinor,       // < 10% changed
    ContentModerate,    // 10-40% changed
    ContentMajor,       // 40-80% changed
    Rewritten,          // > 80% changed
}
```

### Usage Example

```rust
use shared::markdown::Markdown;

let v1: Markdown = std::fs::read_to_string("doc_v1.md")?.into();
let v2: Markdown = std::fs::read_to_string("doc_v2.md")?.into();

let delta = v1.delta(&v2);

// Summary
println!("{}", delta.summary());
// Output: "ContentModerate: 2 added, 1 removed, 3 modified, 0 moved (25.0% changed)"

// Iterate changes
for change in &delta.added {
    println!("+ Added: {:?}", change.new_path);
}

for change in &delta.removed {
    println!("- Removed: {:?}", change.original_path);
}

// Check broken links
if delta.has_broken_links() {
    for link in &delta.broken_links {
        println!("Warning: Link [{}](#{}) would break", link.link_text, link.target_slug);
        if let Some(suggestion) = &link.suggested_replacement {
            println!("  Suggestion: #{}", suggestion);
        }
    }
}
```

## Structure Validation and Normalization

The `Markdown` struct provides methods for validating and normalizing heading structure.

### HeadingLevel

Type-safe representation of heading levels (H1-H6):

```rust
use shared::markdown::HeadingLevel;

let h2 = HeadingLevel::H2;
assert_eq!(h2.as_u8(), 2);
assert_eq!(h2.hash_count(), 2);  // Number of # characters

let h3 = h2.deeper().unwrap();   // H2 -> H3
let h1 = h2.shallower().unwrap(); // H2 -> H1

assert_eq!(h1.delta_to(h3), 2);  // H1 to H3 = +2 levels
```

### validate_structure()

Validates heading hierarchy and returns issues:

```rust
let doc: Markdown = "## Intro\n### Details\n## Conclusion".into();
let validation = doc.validate_structure();

assert!(validation.is_well_formed());
assert_eq!(validation.root_level, Some(HeadingLevel::H2));
assert_eq!(validation.heading_count, 3);
```

#### StructureIssueKind

```rust
pub enum StructureIssueKind {
    HierarchyViolation,  // Heading shallower than root (e.g., H2 in H3-rooted doc)
    SkippedLevel,        // Jump like H2 -> H4
    MultipleH1,          // More than one H1
    NoHeadings,          // Document has no headings
    LevelOverflow,       // Re-leveling would exceed H6
}
```

### normalize()

Adjusts all heading levels to a target root:

```rust
// Promote H3-rooted document to H1
let doc: Markdown = "### Intro\n#### Details".into();
let (normalized, report) = doc.normalize(Some(HeadingLevel::H1))?;

assert!(normalized.content().starts_with("# Intro"));
assert_eq!(report.level_adjustment, -2);  // Promoted by 2 levels
```

### normalize_mut()

In-place normalization:

```rust
let mut doc: Markdown = "### Intro\n#### Details".into();
let report = doc.normalize_mut(Some(HeadingLevel::H1))?;

assert!(doc.content().starts_with("# Intro"));
assert!(report.has_changes());
```

### relevel()

Simple uniform level shift:

```rust
// Demote H1-rooted document to H2 (for embedding as subsection)
let doc: Markdown = "# Main\n## Sub\n### Detail".into();
let (releveled, adjustment) = doc.relevel(HeadingLevel::H2)?;

assert!(releveled.content().starts_with("## Main"));
assert_eq!(adjustment, 1);  // Demoted by 1 level
```

### NormalizationError

```rust
pub enum NormalizationError {
    NoHeadings,
    LevelOverflow { target, affected_count, deepest_title, would_become },
    ValidationFailed(String),
}
```

## Output Formats

The `Markdown` struct has several methods added for exporting to different formats:

- `as_string()` - will merge the frontmatter and content together as text content
- `as_ast()` - exports an AST representation of the Markdown file
- `as_html()` - exports the markdown in HTML format, allows theming
- `for_terminal()` - exports the markdown as text but with Terminal escape codes which support theming


### To AST Format

We will use the `markdown-rs` crate to export an **MDAST** AST representation of the markdown content.

- [MDAST Specification](https://github.com/syntax-tree/mdast)

### AST Code Example

```rust
use markdown::{to_mdast, ParseOptions};
use serde_json;

fn main() -> Result<(), String> {
    // 1. Define your Markdown input
    let markdown_input = "# Hello World\n\nThis is a **serialized** AST.";

    // 2. Parse the Markdown into an MDAST (Markdown Abstract Syntax Tree)
    // We use ParseOptions::default(), but you can enable GFM (GitHub Flavored Markdown) here.
    let ast = to_mdast(markdown_input, &ParseOptions::default())?;

    // 3. Serialize the AST to a JSON string
    // markdown-rs nodes implement serde::Serialize by default
    let json_ast = serde_json::to_string_pretty(&ast)
        .map_err(|e| e.to_string())?;

    // 4. Output the result
    println!("{}", json_ast);

    Ok(())
}
```

**NOTE:** make sure you're using at least the 1.x version of the crate (use an alpha or beta if not yet at stable 1.0). Earlier versions do not support MDAST.


### Exporting to HTML and Terminal

To export our content to either HTML (for the browser) or escaped-coded text (for the terminal) we will need the following crates:

- `pulldown-cmark` - parsing and mutation via pull events
- `pulldown-cmark-to-cmark` - for cleaning up Markdown content
- `syntect` - for code (including Markdown) highlighting


Because this is a bit more involved then the other parts of this `struct` we have included these additional knowledge documents:

The `syntect` crate is critical to our ability to output to the terminal and browser/html:

- [Using `syntect` to output to Terminal](./md/syntect-terminal-output.md)
- [Using `syntect` to output to HTML](./md/syntect-html-output.md)

When exporting Markdown content as either HTML or to the terminal we leverage the idea of using **Grammars** and **Themes**. In both cases this will involve the use of [syntect](https://crates.io/crates/syntect) but we've added the [two-face](https://crates.io/crates/two-face) crate to _extend_ the themes and grammars.

- [Themes](./md/themes.md)
- [Grammars](./md/grammars.md)

When we are parsing fenced code blocks inside of Markdown content we will be using a _different_ theme for these code blocks then we use for the surrounding Markdown.

- [Theme Strategy](./md/theme-strategy.md)

The code blocks we are highlighting allow us to capture the full first line of the code block which includes an area we will call the **Code Block Meta**. This Code Block Meta allows us to provide a small DSL for more control over formatting of these code blocks.

- [Handling Code Block Meta](./md/handling-code-block-meta.md)
- [Code Block DSL](./md/code-block-dsl.md)

The discussion up to now for code highlighting has been focused on highlighting the code blocks in the Markdown, but we need to make sure that the surrounding Markdown is _also_ highlighted.

- [Highlighting the Prose](./md/highlighting-the-prose.md)

## Hashing Utilities

The shared library provides hashing utilities used by the TOC and Delta features.

### xxHash (Fast, Non-Cryptographic)

```rust
use shared::hashing::{xx_hash, xx_hash_trimmed, xx_hash_normalized};

let hash = xx_hash("content");

// Trimmed hash ignores leading/trailing whitespace
assert_eq!(xx_hash_trimmed("  hello  "), xx_hash_trimmed("hello"));

// Normalized hash ignores blank lines
let with_blanks = "line1\n\nline2";
let without_blanks = "line1\nline2";
assert_eq!(xx_hash_normalized(with_blanks), xx_hash_normalized(without_blanks));
```

### BLAKE3 (Cryptographically Secure)

```rust
use shared::hashing::{blake3_hash, blake3_hash_bytes};

let hash = blake3_hash("content");        // Returns hex string (64 chars)
let bytes = blake3_hash_bytes(b"content"); // Returns [u8; 32]
```

### When to Use Which

- **xxHash**: Fast, non-cryptographic. Use for change detection, cache keys, deduplication.
- **BLAKE3**: Cryptographically secure. Use for integrity verification, secure fingerprinting.
