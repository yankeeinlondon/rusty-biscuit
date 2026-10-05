# Section

A heading with optional content below it, rendered in Markdown style. The heading level (h1-h6) controls the prefix (`#`, `##`, etc.) and visual styling: h1-h3 are bold, h4-h5 are italic, h6 is plain text.

Content items can be strings, `Prose`, or any `Renderable` type. A string is one literal paragraph; a [`Prose`](./prose.md) contributes each of its paragraphs and fenced code blocks as its own block.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;

// Create a section with heading and content
let mut section = Section::new(HeadingLevel::h2, "Getting Started");
section
    .push("Welcome to the tutorial.")
    .push("Let's begin with installation.");

// Render
let term = Terminal::default();
let output = section.display(&term);
// Output:
// ## Getting Started
//
// Welcome to the tutorial.
//
// Let's begin with installation.

// Different heading levels
let h1 = Section::new(HeadingLevel::h1, "Title");       // # Title (bold)
let h3 = Section::new(HeadingLevel::h3, "Subsection");  // ### Subsection (bold)
let h5 = Section::new(HeadingLevel::h5, "Minor");       // ##### Minor (italic)

// Get numeric level
assert_eq!(HeadingLevel::h2.level(), 2);
```

### Key API

| Method | Description |
|--------|-------------|
| `Section::new(HeadingLevel, title)` | Create with heading level and title |
| `.push(content)` | Append content (string or RenderableContent) |

### HeadingLevel

| Level | Prefix | Style |
|-------|--------|-------|
| h1 | `#` | Bold |
| h2 | `##` | Bold |
| h3 | `###` | Bold |
| h4 | `####` | Italic |
| h5 | `#####` | Italic |
| h6 | `######` | Plain |

## CLI

Exposed via `bt section`. Each `--content` item is a `Prose`, so it takes the Prose markup and renders as its own paragraph:

```bash
bt section "Deployment Guide" --level 2 -c "Follow these steps to deploy." -c "Run \`md hash\` first." --html
```

```html
<section><h2>Deployment Guide</h2><p>Follow these steps to deploy.</p><p>Run <code>md hash</code> first.</p></section>
```

`--md` and `--md-plus` render Markdown instead, and `bt section --example` shows a sample.
