# Darkmatter Library

Markdown parsing, rendering, and Mermaid diagram support for terminal and HTML output.

## Modules

| Module | Description |
|--------|-------------|
| `markdown` | Document manipulation with frontmatter, parsing, and output rendering |
| `mermaid` | Mermaid diagram theming and rendering (terminal images or HTML) |
| `render` | Hyperlink rendering utilities |
| `terminal` | Terminal color depth and capability detection |
| `testing` | Test utilities for terminal output verification |

## Key Types

### Markdown Module

- `Markdown` - A parsed markdown document with frontmatter
- `TerminalOptions` - Configuration for terminal rendering
- `HtmlOptions` - Configuration for HTML output
- `ThemePair` - Light/dark theme pair for syntax highlighting

### Output Functions

```rust
use darkmatter_lib::markdown::{Markdown, TerminalOptions, write_terminal};

let md: Markdown = "# Hello\n\nWorld".into();
let options = TerminalOptions::default();

let mut stdout = std::io::stdout();
write_terminal(&mut stdout, &md, options)?;
```

### HTML Output

```rust
use darkmatter_lib::markdown::{Markdown, HtmlOptions, as_html};

let md: Markdown = "# Hello\n\nWorld".into();
let options = HtmlOptions::default();

let html = as_html(&md, options);
```

### Document Cleanup

```rust
use darkmatter_lib::markdown::cleanup;

let cleaned = cleanup("# Title\nsome text")?;
```

### Table of Contents

```rust
use darkmatter_lib::markdown::toc::TableOfContents;

let md: Markdown = content.into();
let toc = TableOfContents::from(&md);
```

## Submodules

### markdown::highlighting

Syntax highlighting with syntect and two-face theme support.

### markdown::delta

Document comparison and structural diffing.

### markdown::frontmatter

YAML frontmatter parsing and manipulation.

### markdown::normalize

Markdown normalization and cleanup.

## CLI

For command-line usage, see the [darkmatter-cli](../cli/) package.
