# Interpolate Module

String replacement utilities ranging from simple find/replace to context-aware document transformations.

## Overview

The interpolate module provides a hierarchy of replacement functions:

1. **Context-Unaware**: Simple string and regex replacement
2. **Context-Aware**: Scoped replacement within Markdown or HTML structure

## Quick Start

```rust
use shared::interpolate::{interpolate, interpolate_regex, md_interpolate, html_interpolate};
use shared::isolate::{MarkdownScope, HtmlScope, HtmlTag};

// Simple replacement
let result = interpolate("Hello world", "world", "Rust");
assert_eq!(result, "Hello Rust");

// Regex with capture groups
let result = interpolate_regex("Hello 123", r"(\d+)", "[$1]")?;
assert_eq!(result, "Hello [123]");

// Only replace in headings
let md = "# Hello World\n\nHello paragraph.";
let result = md_interpolate(md, MarkdownScope::Heading, "Hello", "Hi")?;
assert_eq!(result, "# Hi World\n\nHello paragraph.");

// Only replace in div elements
let html = "<div>Hello</div><span>Hello</span>";
let result = html_interpolate(html, HtmlScope::InnerHtml(HtmlTag::Div), "Hello", "Hi")?;
// Only the div's "Hello" is replaced
```

## Functions

### Context-Unaware

#### `interpolate`

Simple string find and replace.

```rust
pub fn interpolate<'a>(content: &'a str, find: &str, replace: &str) -> Cow<'a, str>
```

- Returns `Cow::Borrowed` if no match found (zero allocation)
- Returns `Cow::Owned` if replacement occurred
- Replaces all occurrences

#### `interpolate_regex`

Regex-based replacement with capture group support.

```rust
pub fn interpolate_regex<'a>(
    content: &'a str,
    pattern: &str,
    replace: &str
) -> Result<Cow<'a, str>, InterpolateError>
```

Capture group syntax:
- `$0` - Entire match
- `$1`, `$2`, ... - Numbered groups
- `$name` - Named groups (when pattern uses `(?P<name>...)`)

```rust
// Swap words
let result = interpolate_regex("Hello World", r"(\w+) (\w+)", "$2 $1")?;
assert_eq!(result, "World Hello");

// Named capture
let result = interpolate_regex("2024-01-15", r"(?P<y>\d{4})-(?P<m>\d{2})-(?P<d>\d{2})", "$m/$d/$y")?;
assert_eq!(result, "01/15/2024");
```

### Context-Aware: Markdown

#### `md_interpolate`

Replace content only within specific Markdown scopes.

```rust
pub fn md_interpolate<'a>(
    content: &'a str,
    scope: MarkdownScope,
    find: &str,
    replace: &str,
) -> Result<Cow<'a, str>, InterpolateError>
```

#### `md_interpolate_regex`

Regex replacement scoped to Markdown elements.

```rust
pub fn md_interpolate_regex<'a>(
    content: &'a str,
    scope: MarkdownScope,
    pattern: &str,
    replace: &str,
) -> Result<Cow<'a, str>, InterpolateError>
```

**Example Use Cases:**

```rust
// Update version only in code blocks
let content = "Version: 1.0\n\n```\nversion = \"1.0\"\n```";
let result = md_interpolate(content, MarkdownScope::CodeBlock, "1.0", "2.0")?;
// Only the code block's "1.0" is changed

// Fix typo only in prose (not in code)
let result = md_interpolate(content, MarkdownScope::Prose, "teh", "the")?;

// Update all links
let result = md_interpolate_regex(
    content,
    MarkdownScope::Links,
    r"http://",
    "https://"
)?;
```

### Context-Aware: HTML

#### `html_interpolate`

Replace content only within specific HTML scopes.

```rust
pub fn html_interpolate<'a>(
    content: &'a str,
    scope: HtmlScope,
    find: &str,
    replace: &str,
) -> Result<Cow<'a, str>, InterpolateError>
```

#### `html_interpolate_regex`

Regex replacement scoped to HTML elements.

```rust
pub fn html_interpolate_regex<'a>(
    content: &'a str,
    scope: HtmlScope,
    pattern: &str,
    replace: &str,
) -> Result<Cow<'a, str>, InterpolateError>
```

**Example Use Cases:**

```rust
// Update text in headings only
let html = "<h1>Hello World</h1><p>Hello paragraph</p>";
let result = html_interpolate(html, HtmlScope::InnerHtml(HtmlTag::H1), "Hello", "Hi")?;

// Update all elements matching a selector
let result = html_interpolate(html, HtmlScope::Selector(".highlight".into()), "old", "new")?;

// Strip content from scripts
let result = html_interpolate(html, HtmlScope::InnerHtml(HtmlTag::Script), r".*", "")?;
```

## Available Scopes

### MarkdownScope

| Scope | Targets |
|-------|---------|
| `Frontmatter` | YAML metadata between `---` |
| `Prose` | Regular paragraph text |
| `CodeBlock` | Fenced and indented code |
| `BlockQuote` | Quoted content |
| `Heading` | All heading levels |
| `Stylized` | Bold, italic, strikethrough |
| `Italicized` | Only italic text |
| `NonItalicized` | Everything except italic |
| `Links` | Link text and URLs |
| `Images` | Image alt and src |
| `Lists` | List item content |
| `Tables` | Table cells and headers |
| `FootnoteDefinitions` | Footnote content |

### HtmlScope

| Scope | Targets |
|-------|---------|
| `TagAttributes(tag)` | Opening tag + attributes |
| `InnerHtml(tag)` | Content between tags |
| `OuterHtml(tag)` | Complete element |
| `Selector(css)` | CSS selector matches |
| `Prose` | Text content only |

## Error Handling

```rust
use shared::isolate::InterpolateError;

match md_interpolate_regex(content, scope, pattern, replace) {
    Ok(result) => { /* use result */ }
    Err(InterpolateError::InvalidPattern(e)) => {
        // Invalid regex pattern
        eprintln!("Bad regex: {}", e);
    }
    Err(InterpolateError::IsolateError(e)) => {
        // Underlying isolation failed
        eprintln!("Isolation failed: {}", e);
    }
}
```

## Zero-Copy Behavior

All interpolation functions return `Cow<'a, str>`:

- **No match found**: Returns `Cow::Borrowed(content)` - zero allocation
- **Match found**: Returns `Cow::Owned(modified)` - new string allocated

```rust
let result = interpolate("Hello", "xyz", "abc");
assert!(matches!(result, Cow::Borrowed(_))); // No allocation

let result = interpolate("Hello", "ell", "i");
assert!(matches!(result, Cow::Owned(_))); // Allocated "Hio"
```

## Implementation Notes

### Markdown Replacement Strategy

Uses byte-range tracking for precise scoped replacement:

1. Parse markdown to identify scoped regions with byte positions
2. Find all occurrences of search pattern within scoped regions
3. Sort matches by position (descending)
4. Apply replacements from end to start (preserves byte positions)

This approach maintains document structure and handles nested scopes correctly.

### HTML Replacement Limitation

Due to `scraper`'s DOM model not preserving byte offsets, HTML interpolation uses string-based replacement. This means:

- If identical content appears in multiple matching elements, all will be replaced
- The original HTML formatting may be slightly normalized

This is a documented trade-off for robust HTML5-compliant parsing.

## Related

- [`isolate`](../isolate/README.md) - Content extraction (used internally)
- [`markdown`](../markdown/README.md) - Higher-level Markdown utilities
