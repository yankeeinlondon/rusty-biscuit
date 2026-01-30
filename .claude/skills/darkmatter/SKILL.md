---
name: darkmatter
description: Expert knowledge for the darkmatter Rust library - markdown parsing, rendering (terminal/HTML), syntax highlighting, frontmatter, and document comparison. Delegates terminal rendering to biscuit-terminal. Use when parsing markdown, generating terminal/HTML output, working with frontmatter, or comparing documents.
---

# darkmatter

Markdown parsing, rendering, and transformation library. Part of the dockhand monorepo.

**Key principle**: darkmatter handles **markdown parsing and transformation**. Terminal rendering (images, mermaid, detection) is delegated to `biscuit-terminal`.

## Responsibility Split

| Responsibility | Package |
|----------------|---------|
| Markdown parsing (CommonMark + GFM) | darkmatter-lib |
| Syntax highlighting | darkmatter-lib (syntect) |
| Frontmatter extraction | darkmatter-lib |
| HTML output | darkmatter-lib |
| Document comparison/normalization | darkmatter-lib |
| Terminal detection, image/mermaid rendering | **biscuit-terminal** |

## Quick Start

```rust
use darkmatter_lib::markdown::{Markdown, output::{TerminalOptions, write_terminal}};

let md: Markdown = "# Hello\n\nWorld".into();
let mut stdout = std::io::stdout();
write_terminal(&mut stdout, &md, TerminalOptions::default())?;
```

## Detailed Topics

- [Terminal Output](./terminal.md) - ANSI rendering, themes, options
- [Frontmatter](./frontmatter.md) - YAML parsing, merge strategies
- [Document Comparison](./comparison.md) - Structural diff, change classification
- [Module Structure](./structure.md) - Package organization and dependencies

## CLI

```bash
md doc.md              # Render to terminal
md doc.md --clean      # Clean document
md doc.md --html       # Render to HTML
md doc.md --ast        # Render as JSON AST
```

## See Also

- [biscuit-terminal skill](../biscuit-terminal/SKILL.md) - Terminal rendering dependency
