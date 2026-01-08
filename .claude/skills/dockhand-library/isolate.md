# Content Isolation

The `isolate` module extracts specific content from structured documents (markdown and HTML) based on CSS-like selectors and scoping rules.

## Core Concepts

The module provides isolation capabilities for:
- **Markdown documents**: Extract sections by heading patterns
- **HTML documents**: Extract elements by CSS selectors

## Markdown Isolation

### Basic Usage

```rust
use shared::isolate::{md_isolate, MdScope};

let markdown = r#"
# Introduction

Welcome to the guide.

## Getting Started

First steps here.

### Installation

Run `cargo install`.

## Advanced Topics

Deep dive content.

# Conclusion

Final thoughts.
"#;

// Extract a specific section
let scope = MdScope::new("## Getting Started");
let result = md_isolate(markdown, &scope)?;
// Returns: "## Getting Started\n\nFirst steps here.\n\n### Installation\n\nRun `cargo install`."
```

### Scope Types

```rust
use shared::isolate::MdScope;

// Exact heading match
let scope = MdScope::new("## Configuration");

// Pattern matching (with wildcards)
let scope = MdScope::new("## *Settings*");

// Heading level selection
let scope = MdScope::level(2); // All H2 sections

// Range selection
let scope = MdScope::range("## Start", "## End");

// Multiple scopes
let scopes = vec![
    MdScope::new("## Installation"),
    MdScope::new("## Configuration"),
];
```

### Advanced Patterns

```rust
// Extract with children
let scope = MdScope::new("## API Reference")
    .with_children(true); // Includes all subsections

// Extract without children
let scope = MdScope::new("## Overview")
    .with_children(false); // Only the overview content

// Level-based extraction
let scope = MdScope::level(1); // All top-level sections
let scope = MdScope::level_range(2, 3); // H2 and H3 sections
```

## HTML Isolation

### Basic Usage

```rust
use shared::isolate::{html_isolate, HtmlScope};

let html = r#"
<html>
<body>
    <div class="content">
        <h1>Title</h1>
        <p class="intro">Introduction text</p>
        <div id="main">
            <p>Main content</p>
        </div>
    </div>
</body>
</html>
"#;

// Extract by CSS selector
let scope = HtmlScope::new("#main");
let result = html_isolate(html, &scope)?;
// Returns: "<div id=\"main\"><p>Main content</p></div>"
```

### CSS Selectors

```rust
use shared::isolate::HtmlScope;

// ID selector
let scope = HtmlScope::new("#content");

// Class selector
let scope = HtmlScope::new(".article");

// Element selector
let scope = HtmlScope::new("section");

// Compound selectors
let scope = HtmlScope::new("div.content");
let scope = HtmlScope::new("article > p");
let scope = HtmlScope::new("h2 + p"); // Next sibling
```

### Multiple Matches

```rust
// Extract all matching elements
let scope = HtmlScope::new("p.important");
let results = html_isolate_all(html, &scope)?;
// Returns vector of all matching <p class="important"> elements

// Extract first match only
let scope = HtmlScope::new(".content").first_only();
let result = html_isolate(html, &scope)?;
```

## Isolation Types

### Include vs Exclude Modes

```rust
use shared::isolate::{IsolateMode, MdScope};

// Include mode (default) - extract matching content
let scope = MdScope::new("## Configuration")
    .with_mode(IsolateMode::Include);

// Exclude mode - remove matching content
let scope = MdScope::new("## Internal Notes")
    .with_mode(IsolateMode::Exclude);

// Combination - extract some, exclude others
let include_scope = MdScope::new("## Public API");
let exclude_scope = MdScope::new("### Deprecated")
    .with_mode(IsolateMode::Exclude);
```

### Content Transformation

```rust
// Extract and transform
let scope = HtmlScope::new("pre.code-block")
    .transform(|content| {
        // Remove line numbers, clean up formatting
        content.lines()
            .map(|line| line.trim_start_matches(|c: char| c.is_numeric()))
            .collect::<Vec<_>>()
            .join("\n")
    });
```

## Error Handling

```rust
use shared::isolate::{IsolateError, md_isolate};

match md_isolate(content, &scope) {
    Ok(isolated) => println!("Extracted: {}", isolated),

    Err(IsolateError::ScopeNotFound(scope)) => {
        eprintln!("Could not find: {}", scope);
    }

    Err(IsolateError::InvalidSelector(sel)) => {
        eprintln!("Invalid CSS selector: {}", sel);
    }

    Err(IsolateError::ParseError(msg)) => {
        eprintln!("Failed to parse document: {}", msg);
    }
}
```

## Common Use Cases

### Documentation Extraction

```rust
// Extract installation instructions from README
let readme = std::fs::read_to_string("README.md")?;
let install_scope = MdScope::new("## Installation");
let install_docs = md_isolate(&readme, &install_scope)?;

// Extract API reference
let api_scope = MdScope::new("# API Reference").with_children(true);
let api_docs = md_isolate(&readme, &api_scope)?;
```

### Content Filtering

```rust
// Remove private sections before publishing
let public_content = md_isolate_excluding(
    content,
    vec![
        MdScope::new("## Internal Notes"),
        MdScope::new("### TODO"),
        MdScope::new("## Debug Information"),
    ]
)?;
```

### HTML Scraping

```rust
// Extract article content
let article_scope = HtmlScope::new("article.post");
let article = html_isolate(&webpage, &article_scope)?;

// Extract all code examples
let code_scope = HtmlScope::new("pre > code");
let examples = html_isolate_all(&webpage, &code_scope)?;
```

## Integration with Other Modules

### With Markdown Module

```rust
use shared::markdown::Markdown;
use shared::isolate::{md_isolate, MdScope};

// Load and process markdown
let mut md = Markdown::try_from(Path::new("doc.md"))?;
let content = md.content();

// Extract specific section
let scope = MdScope::new("## Configuration");
let config_section = md_isolate(content, &scope)?;

// Create new document from extraction
let config_doc = Markdown::new(config_section);
```

### With Interpolation

```rust
// Extract template section and interpolate
let template_scope = MdScope::new("## Template");
let template = md_isolate(content, &template_scope)?;

let interpolated = interpolate(&template, &context);
```

## Performance Tips

1. **Pre-compile selectors**: For repeated use, compile CSS selectors once
2. **Use specific selectors**: More specific = faster matching
3. **Limit scope depth**: Use `with_children(false)` when possible
4. **Cache results**: For static content, cache extractions

## Examples

### Multi-Section Extraction

```rust
// Extract multiple sections into a new document
fn extract_public_docs(content: &str) -> Result<String> {
    let sections = vec![
        "# Overview",
        "## Installation",
        "## Quick Start",
        "## API Reference",
    ];

    let mut output = String::new();
    for section in sections {
        if let Ok(content) = md_isolate(content, &MdScope::new(section)) {
            output.push_str(&content);
            output.push_str("\n\n");
        }
    }

    Ok(output)
}
```

### Conditional Extraction

```rust
// Extract based on frontmatter
let md = Markdown::from(content);
let is_public: bool = md.fm_get("public")?.unwrap_or(false);

let scope = if is_public {
    MdScope::new("# Full Documentation")
} else {
    MdScope::new("## Public API Only")
};

let result = md_isolate(md.content(), &scope)?;
```