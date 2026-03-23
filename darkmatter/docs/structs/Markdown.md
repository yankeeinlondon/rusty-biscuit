---
blast_radius:
  - darkmatter/lib/src/markdown/mod.rs
  - darkmatter/lib/src/markdown/frontmatter.rs
  - darkmatter/lib/src/markdown/types.rs
  - darkmatter/lib/src/markdown/cleanup.rs
  - darkmatter/lib/src/markdown/normalize/mod.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/hash.rs
  - darkmatter/lib/src/markdown/output/mod.rs
  - darkmatter/lib/src/markdown/output/string.rs
  - darkmatter/lib/src/markdown/output/ast.rs
  - darkmatter/lib/src/markdown/output/html.rs
  - darkmatter/lib/src/markdown/output/terminal.rs
  - darkmatter/lib/src/markdown/toc/mod.rs
  - darkmatter/lib/src/markdown/toc/types.rs
  - darkmatter/lib/src/markdown/delta/mod.rs
  - darkmatter/lib/src/markdown/delta/types.rs
  - darkmatter/lib/src/render/link.rs
  - darkmatter/lib/src/render/image_ref.rs
  - darkmatter/docs/structs/README.md
---

# `Markdown` Struct

The `Markdown` struct is the central type in Darkmatter's library. It represents a complete markdown document -- content plus optional YAML frontmatter -- and provides a rich API for construction, introspection, transformation, and multi-format rendering.

## Overview

`Markdown` encapsulates two concerns into a single type:

1. **Frontmatter** -- structured YAML metadata at the top of the document
2. **Content** -- the markdown body itself

From this foundation, it offers methods spanning five functional areas: construction, frontmatter management, content extraction, transformation, and output rendering.

```rust
use darkmatter::Markdown;

let md: Markdown = r#"---
title: Getting Started
tags: [rust, markdown]
---
# Welcome

This is a **darkmatter** document.
"#.into();

let title: Option<String> = md.fm_get("title").unwrap();
assert_eq!(title, Some("Getting Started".to_string()));
```

## Core State

| Field | Type | Description |
|-------|------|-------------|
| `frontmatter` | `Frontmatter` | Optional YAML metadata stored in an order-preserving `IndexMap<String, serde_json::Value>` |
| `content` | `String` | The markdown body, excluding frontmatter delimiters |

Both fields are private; access is provided through dedicated methods.

## Creating a `Markdown` struct

### `new`

Creates a document with content only (empty frontmatter).

```rust
use darkmatter::Markdown;

let md = Markdown::new("# Hello\n\nSome content.");
```

### `with_frontmatter`

Creates a document with explicit frontmatter and content.

```rust
use darkmatter::{Markdown, Frontmatter};

let fm = Frontmatter::new();
let md = Markdown::with_frontmatter(fm, "# Hello");
```

### `from_url` (async)

Fetches markdown from a URL via HTTP GET and parses frontmatter automatically.

```rust
use darkmatter::Markdown;
use url::Url;

let url = Url::parse("https://example.com/doc.md").unwrap();
let md = Markdown::from_url(&url).await?;
```

### `From<&str>` and `From<String>`

The most common construction path. Auto-detects and parses YAML frontmatter if present.

```rust
use darkmatter::Markdown;

let md: Markdown = "---\ntitle: Example\n---\n# Content".into();
```

### `TryFrom<&Path>`

Loads a markdown file from disk. Returns `MarkdownError` on I/O failure.

```rust
use darkmatter::Markdown;
use std::path::Path;

let md = Markdown::try_from(Path::new("./README.md"))?;
```

## Frontmatter

Frontmatter methods provide typed access to YAML metadata. All return `MarkdownResult<T>`.

### Reading Values

**`fm_get<T>(key) -> MarkdownResult<Option<T>>`** retrieves a typed value by key. Returns `None` if the key doesn't exist; returns an error if deserialization fails.

```rust
let title: Option<String> = md.fm_get("title")?;
let count: Option<u32> = md.fm_get("word_count")?;
```

**`frontmatter()`** and **`frontmatter_mut()`** provide direct access to the underlying `Frontmatter` struct for bulk operations.

### Writing Values

**`fm_insert(key, value)`** sets a single frontmatter key, overwriting any existing value.

```rust
md.fm_insert("draft", true)?;
md.fm_insert("author", "Ken")?;
```

### Merging

**`fm_merge_with(data, strategy)`** merges external data into the frontmatter using a conflict resolution strategy:

| Strategy | Behavior on Duplicate Keys |
|----------|---------------------------|
| `MergeStrategy::ErrorOnConflict` | Returns an error |
| `MergeStrategy::PreferExternal` | External values win |
| `MergeStrategy::PreferDocument` | Document values win |

```rust
use darkmatter::MergeStrategy;

md.fm_merge_with(
    serde_json::json!({"author": "Ken", "draft": false}),
    MergeStrategy::PreferDocument,
)?;
```

### Defaults

**`fm_set_defaults(defaults)`** fills in missing keys without overwriting existing ones.

```rust
md.fm_set_defaults(serde_json::json!({
    "draft": true,
    "language": "en"
}))?;
```

## Content Access

| Method | Returns | Description |
|--------|---------|-------------|
| `content()` | `&str` | Immutable reference to the markdown body |
| `content_mut()` | `&mut String` | Mutable reference for direct manipulation |
| `into_parts()` | `(Frontmatter, String)` | Consumes the struct, returning ownership of both parts |

## Document References

A document can reference other external assets through the following means:

- **Hyperlinks**

    This includes both hyperlinks using Markdown syntax _and_ hyperlinks using inline HTML using the `<a>` tag

- **Image References**

    This includes Markdown syntax for image references as well as inline HTML using the `<img>` tag

- **Transclusions**

    Because Darkmatter's DSL includes _transclusions_ (aka, file references where the content isn't a _link_ but rather it will be directly included in the document during a "compose" operation )


- **Inline Tags:**

    - **Inline CSS and Imports**

        Markdown in Darkmatter would **not** typically have CSS imports -- nor would it render in Markdown differently based on this -- but if some inline HTML with a CSS import were present then we'd want to preserve it and it could have a visual impact when we render to HTML.

        ```rust
        impl for Markdown {
            pub fn has_inline_css(): bool;
            pub fn has_css_imports(): bool;

            /** returns a list of URLs */
            pub fn get_css_imports(): Vec<String>;
            /** returns blocks of inline CSS */
            pub fn get_inline_css(): Vec<CssBlock>;
            pub fn resolve_css_imports(): 
        }
        ```

    - **Fonts** and **Scripts**

        Similarly to how we treat CSS Imports, both font imports and script imports would NOT be expected in normal Markdown content but because inline 

        ```rust
        impl for Markdown {
            pub fn has_inline_scripts(): bool;
            pub fn has_script_imports(): bool;
            pub fn has_font_imports(): bool;

            pub fn get_script_imports(): Vec<String>;
            pub fn get_inline_script_blocks(): Vec<String>;

        }
        ```

    - **Meta** tags

        Meta tags are uncommon in Markdown too but like the two prior sections we want to be able to detect and preserve these tags. However, in this case we want to add a couple of additional nuances:

        - it would be good to be able to:
            - convert meta tags to frontmatter key/values (part of builder interface)
            - have a `get_meta_tags()` implementation which 


These methods parse the markdown body and return structured data.

### `links() -> Vec<Link>`

Extracts Markdown-native hyperlinks from the document as typed [`Link`](./Link.md) structs. Preserves inline formatting in display text (bold, code, line breaks) and parses metadata from the title attribute.

```rust
let links = md.links();
for link in &links {
    println!("{} -> {}", link.display(), link.href());
}
```

### `has_inline_html() -> bool`

Returns `true` when the markdown body appears to contain raw HTML. This is a cheap fast-path that can be used before calling the HTML-specific extraction helpers.

```rust
if md.has_inline_html() {
    println!("document contains inline HTML");
}
```

### `inline_html_links() -> Vec<Link>`

Extracts HTML `<a>` tags from the markdown body as typed [`Link`](./Link.md) structs. This complements `links()`, which remains Markdown-syntax only.

```rust
let html_links = md.inline_html_links();
for link in &html_links {
    println!("{} -> {}", link.display(), link.href());
}
```

### `image_references() -> Vec<ImageRef>`

Extracts Markdown-native image references as typed [`ImageRef`](./ImageRef.md) structs. Handles width specifications in alt text (e.g., `![photo|50%](img.png)`).

```rust
let images = md.image_references();
for img in &images {
    println!("{} -> {}", img.alt(), img.src());
}
```

### `inline_html_image_references() -> Vec<ImageRef>`

Extracts HTML `<img>` tags from the markdown body as typed [`ImageRef`](./ImageRef.md) structs. This complements `image_references()`, which remains Markdown-syntax only.

```rust
let html_images = md.inline_html_image_references();
for img in &html_images {
    println!("{} -> {:?}", img.alt(), img.src());
}
```

## Cleanup and Formatting

Cleanup methods normalize whitespace and formatting. All return `&mut Self` for method chaining.

| Method | Behavior |
|--------|----------|
| `cleanup()` | Injects blank lines between block elements; aligns table columns |
| `cleanup_with_indent(size)` | Cleanup + enforces consistent list indentation width |
| `cleanup_compact()` | Cleanup with compact lists (no blank lines between items) |
| `cleanup_loose()` | Cleanup with loose lists (blank lines between all items) |
| `cleanup_with_indent_compact(size)` | Custom indentation + compact lists |
| `cleanup_with_indent_loose(size)` | Custom indentation + loose lists |

```rust
let mut md = Markdown::new("- one\n- two\n## Next\ntext");
md.cleanup();
```

## Section Removal

### `remove_section(pattern) -> bool`

Removes a heading section and all its content up to the next sibling or parent heading. Returns whether the section was found.

Pattern formats:

| Pattern | Behavior |
|---------|----------|
| `"## Title"` | Exact match on heading level and title |
| `"## Title*"` | Prefix match (title starts with the text before `*`) |
| `"!prelude"` | Removes all content before the first heading |

```rust
let mut md: Markdown = "# Doc\n## Keep\nyes\n## Remove\nno\n## Also Keep\nyes".into();
assert!(md.remove_section("## Remove"));
assert!(!md.content().contains("no"));
```

### `remove_sections(patterns) -> usize`

Removes multiple sections in a single pass. Returns the count of sections removed.

## Heading Validation and Normalization

### `validate_structure() -> StructureValidation`

Checks heading hierarchy for common issues:

- Headings shallower than the root level
- Skipped heading levels (e.g., H2 followed by H4)
- More than one H1

Returns a `StructureValidation` with an `is_valid` flag, detected root/min/max levels, heading count, and a list of issues.

### `normalize(target) -> Result<(Markdown, NormalizationReport), NormalizationError>`

Normalizes heading levels, producing a new `Markdown` and a report of changes.

- **`Some(HeadingLevel)`** -- adjusts all headings so the root matches the target level
- **`None`** -- keeps the current root level but fixes hierarchy violations

Fails if re-leveling would push any heading beyond H6.

### `normalize_mut(target) -> Result<NormalizationReport, NormalizationError>`

In-place variant of `normalize`. Modifies `self` and returns only the report.

### `relevel(target) -> Result<(Markdown, i8), NormalizationError>`

Simpler alternative to `normalize`: uniformly shifts all headings to the target root level without fixing structural violations. Returns the new document and the shift amount (positive = demoted, negative = promoted).

## Table of Contents

### `toc() -> MarkdownToc`

Generates a complete table of contents from the document's heading structure.

`MarkdownToc` provides:

- `title` -- the document title (first H1, if any)
- `preamble` -- content before the first heading
- `structure` -- hierarchical tree of `MarkdownTocNode` entries
- `heading_count()`, `root_level()`, `max_level()`
- `find_by_slug(slug)` -- locate a heading by its URL slug
- `all_headings()` -- flat iterator over all headings
- `code_blocks` -- collection of code block metadata
- `internal_links` -- collection of internal link references
- `has_broken_links()`, `broken_links()` -- link integrity checking
- Per-node and per-subtree content hashes for change detection

## Document Comparison

### `delta(other) -> MarkdownDelta`

Compares two markdown documents and returns a detailed change analysis.

`MarkdownDelta` provides:

| Field | Description |
|-------|-------------|
| `classification` | High-level change type (see below) |
| `statistics` | Quantifiable metrics (lines added/removed, etc.) |
| `frontmatter_changes` | Frontmatter key-level diffs |
| `added` / `removed` / `modified` / `moved` | Section-level changes |
| `code_block_changes` | Changes to fenced code blocks |
| `broken_links` | Links broken by the changes |

**`DocumentChange` classification:**

| Variant | Meaning |
|---------|---------|
| `NoChange` | Documents are identical |
| `WhitespaceOnly` | Only whitespace differs |
| `FrontmatterOnly` | Only frontmatter changed |
| `FrontmatterAndWhitespace` | Frontmatter + whitespace |
| `StructuralOnly` | Headings reorganized, content unchanged |
| `ContentMinor` | < 10% content changed |
| `ContentModerate` | 10--40% content changed |
| `ContentMajor` | 40--80% content changed |
| `Rewritten` | > 80% content changed |

Helper methods: `is_unchanged()`, `is_cosmetic_only()`, `has_broken_links()`, `summary()`.

## Output Rendering

### `as_string() -> String`

Serializes the full document back to a string, including YAML frontmatter between `---` delimiters (if present).

```rust
let output = md.as_string();
assert!(output.starts_with("---\n"));
```

### `as_ast() -> MarkdownResult<Node>`

Converts to an MDAST (Markdown Abstract Syntax Tree) node for programmatic manipulation. Uses GFM extensions. The resulting tree can be serialized to JSON.

```rust
let ast = md.as_ast()?;
let json = serde_json::to_string_pretty(&ast)?;
```

### `as_html(options) -> MarkdownResult<String>`

Renders the document to HTML with syntax highlighting.

`HtmlOptions` controls:

| Option | Type | Description |
|--------|------|-------------|
| `code_theme` | `ThemePair` | Theme for code blocks |
| `prose_theme` | `ThemePair` | Theme for prose elements |
| `color_mode` | `ColorMode` | Light/dark mode |
| `include_line_numbers` | `bool` | Show line numbers in code blocks |
| `include_styles` | `bool` | Embed CSS styles in output |
| `mermaid_mode` | `MermaidMode` | How to handle Mermaid diagram blocks |

### `as_terminal(options) -> MarkdownResult<String>`

Renders the document as ANSI-styled terminal output with syntax highlighting, inline image support (Kitty/iTerm2 protocols), and configurable formatting.

`TerminalOptions` controls:

| Option | Type | Description |
|--------|------|-------------|
| `code_theme` | `ThemePair` | Theme for code blocks |
| `prose_theme` | `ThemePair` | Theme for prose elements |
| `color_mode` | `ColorMode` | Light/dark mode |
| `include_line_numbers` | `bool` | Show line numbers in code blocks |
| `image_mode` | `TerminalImageMode` | Inline image rendering strategy |
| `base_path` | `Option<PathBuf>` | Base path for resolving relative image URLs |
| `italic_mode` | Mode | Italic rendering behavior |
| `hyperlink_mode` | Mode | OSC 8 hyperlink behavior |
| `mermaid_mode` | `MermaidMode` | Mermaid diagram rendering |
| `max_width` | `Option<usize>` | Maximum output width |
| `color_depth` | `Option<ColorDepth>` | Terminal color depth |

## Decomposition

The `into_parts()` method consumes the `Markdown` and returns ownership of both the `Frontmatter` and `String` content. This is useful when you need to transfer ownership to separate processing pipelines.

```rust
let (frontmatter, content) = md.into_parts();
```

## Errors

All fallible operations use `MarkdownResult<T>`, an alias for `Result<T, MarkdownError>`.

`MarkdownError` variants:

| Variant | Cause |
|---------|-------|
| `FrontmatterParse` | Invalid YAML in frontmatter |
| `FrontmatterMerge` | Merge conflict with `ErrorOnConflict` strategy |
| `FileLoad` | File I/O failure |
| `UrlFetch` | HTTP request failure |
| `ThemeLoad` | Invalid syntax highlighting theme |
| `AstParse` | Markdown-to-AST conversion failure |
| `InvalidLineRange` | Bad line range specification |
| `Serialization` | JSON serialization failure |
| `Transform` | General transformation error |
| `Transclusion` | Transclusion processing error |
| `TocLinking` | TOC link resolution error |
| `ShellExpansion` | Shell expansion processing error |
| `PageBlock` | Page block processing error |

## Trait Implementations

| Trait | Behavior |
|-------|----------|
| `Debug` | Derived debug output |
| `Clone` | Deep copy of frontmatter and content |
| `PartialEq` | Structural equality of both fields |
| `From<&str>` | Parse string with frontmatter detection |
| `From<String>` | Parse string with frontmatter detection |
| `TryFrom<&Path>` | Load from file path |
