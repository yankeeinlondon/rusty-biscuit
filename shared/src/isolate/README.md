# Isolate Module

Content isolation for extracting specific regions from Markdown and HTML documents.

## Overview

The isolate module provides functions to extract content from structured documents based on semantic "scopes". This enables targeted content extraction without manual parsing.

## Functions

### Markdown Isolation

```rust
use shared::isolate::{md_isolate, MarkdownScope, IsolateAction, IsolateResult};

let content = "# Hello World\n\nSome **bold** text.";

// Extract all headings as a vector
let result = md_isolate(content, MarkdownScope::Heading, IsolateAction::LeaveAsVector)?;

// Extract and concatenate with newlines
let result = md_isolate(content, MarkdownScope::Prose, IsolateAction::Concatenate(Some("\n".into())))?;
```

### HTML Isolation

```rust
use shared::isolate::{html_isolate, HtmlScope, HtmlTag, IsolateAction};

let html = "<div>Hello</div><span>World</span>";

// Extract inner HTML from all divs
let result = html_isolate(html, HtmlScope::InnerHtml(HtmlTag::Div), IsolateAction::LeaveAsVector)?;

// Extract text content only (no tags)
let result = html_isolate(html, HtmlScope::Prose, IsolateAction::LeaveAsVector)?;
```

## Types

### IsolateAction

Controls how isolated content is returned:

| Variant | Description |
|---------|-------------|
| `LeaveAsVector` | Returns each match as a separate element |
| `Concatenate(None)` | Joins all matches with no delimiter |
| `Concatenate(Some(delim))` | Joins all matches with the specified delimiter |

### IsolateResult

The return type for isolation operations:

| Variant | Description |
|---------|-------------|
| `Vector(Vec<Cow<'a, str>>)` | Collection of isolated content (may be borrowed) |
| `Concatenated(String)` | Single joined string (always owned) |

### MarkdownScope

Targets specific Markdown elements:

| Variant | Description | Example Match |
|---------|-------------|---------------|
| `Frontmatter` | YAML between `---` markers | `title: Hello` |
| `Prose` | Text outside special elements | Regular paragraph text |
| `CodeBlock` | Fenced or indented code | ` ```rust ... ``` ` |
| `BlockQuote` | Block quote content | `> quoted text` |
| `Heading` | Heading text (any level) | `# Title` |
| `Stylized` | Bold, italic, strikethrough | `**bold**`, `*italic*` |
| `Italicized` | Only italic content | `*italic*`, `_italic_` |
| `NonItalicized` | Everything except italic | All non-italic content |
| `Links` | Link text and URLs | `[text](url)` |
| `Images` | Image alt text and URLs | `![alt](src)` |
| `Lists` | List item content | `- item` or `1. item` |
| `Tables` | Table headers and cells | GFM tables |
| `FootnoteDefinitions` | Footnote content | `[^1]: definition` |

### HtmlScope

Targets specific HTML elements or content:

| Variant | Description |
|---------|-------------|
| `TagAttributes(HtmlTag)` | Opening tag with attributes |
| `InnerHtml(HtmlTag)` | Content between opening/closing tags |
| `OuterHtml(HtmlTag)` | Complete element including tags |
| `Selector(String)` | Elements matching CSS selector |
| `Prose` | Text content only (tags stripped) |

### HtmlTag

Supported HTML tag types:

- **Structural**: `All`, `Body`, `Head`, `Div`, `Span`, `Section`, `Aside`, `Header`
- **Headings**: `H1`, `H2`, `H3`, `H4`, `H5`, `H6`
- **Special**: `Meta`, `Script`, `Pre`, `PreBlock`, `PreInline`

## Error Handling

```rust
use shared::isolate::IsolateError;

match md_isolate(content, scope, action) {
    Ok(result) => { /* use result */ }
    Err(IsolateError::InvalidSelector(sel)) => { /* invalid CSS selector */ }
    Err(IsolateError::MarkdownParse(msg)) => { /* markdown parsing failed */ }
    Err(IsolateError::HtmlParse(msg)) => { /* HTML parsing failed */ }
    Err(IsolateError::InvalidByteRange { start, end }) => { /* UTF-8 boundary error */ }
}
```

## Zero-Copy Behavior

### Markdown

The `md_isolate` function uses `pulldown_cmark`'s byte offset iterator to return `Cow::Borrowed` references to the original content when possible. This avoids allocations for simple extractions.

```rust
let result = md_isolate(content, MarkdownScope::Heading, IsolateAction::LeaveAsVector)?;
if let IsolateResult::Vector(items) = result {
    // items[0] is likely Cow::Borrowed(&content[start..end])
}
```

### HTML

The `html_isolate` function always returns `Cow::Owned` strings because the `scraper` crate's DOM model doesn't preserve byte offsets from the original source. This is a documented trade-off for robust HTML5-compliant parsing.

## Implementation Details

### Markdown Parser

Uses `pulldown_cmark` with these options enabled:

- `ENABLE_GFM` - GitHub Flavored Markdown extensions
- `ENABLE_TABLES` - Table support
- `ENABLE_FOOTNOTES` - Footnote support

### HTML Parser

Uses `scraper` (built on `html5ever`) which provides:

- Full HTML5 compliance
- CSS selector support
- Robust handling of malformed HTML

## Related

- [`interpolate`](../interpolate/README.md) - Uses these scopes for context-aware replacement
- [`markdown`](../markdown/README.md) - Higher-level Markdown utilities
