# Darkmatter

A themed markdown renderer for terminal and browser output with syntax highlighting, Mermaid diagrams, and document processing.

## Quick Start

```bash
# Install the CLI
cargo install --path cli

# Or run in development mode
just -f darkmatter/justfile cli README.md
```

## Packages

| Package | Crate | Description |
|---------|-------|-------------|
| [`cli/`](cli/) | `darkmatter-cli` | The `md` command-line tool for rendering markdown |
| [`lib/`](lib/) | `darkmatter` | Core library for markdown parsing, rendering, and manipulation |

## Features

- **Terminal rendering**: ANSI escape codes with automatic color depth detection
- **HTML output**: Standalone HTML with embedded styles
- **Syntax highlighting**: Language-aware code block highlighting via syntect with two-face themes
- **Image rendering**: Inline images in supported terminals (Kitty, iTerm2, sixel)
- **Mermaid diagrams**: Terminal rendering via biscuit-terminal, HTML rendering via mermaid.js
- **Theme support**: 9 theme pairs with automatic light/dark detection
- **Hyperlink rendering**: Clickable links in supported terminals via OSC 8
- **Markdown cleanup**: Normalize formatting and heading levels
- **Transclusion pipeline**: Stage 2 support for `::file`, `::code`, `prologue`, and `epilogue`
- **Frontmatter operations**: Parse, extract, and manipulate YAML frontmatter with key-order preservation
- **Visual diff**: Colored inline diffs for strings and files
- **Table of contents**: Extract document structure as tree or JSON

## Library Modules

| Module | Purpose |
|--------|---------|
| `markdown` | Core `Markdown` type with frontmatter, rendering, and manipulation |
| `diff` | Visual diff utilities for strings and files |
| `mermaid` | Mermaid diagram theming |
| `render` | Hyperlink rendering (OSC 8 terminal links) |
| `terminal` | ANSI color depth detection utilities |
| `testing` | Test utilities for terminal output verification |

## Transclusion Support

Darkmatter's transform pipeline now includes Stage 2 transclusion:

- Block directives: `::file ./doc.md`, `::code ./main.rs`
- Frontmatter directives: `prologue`, `epilogue`
- Recursive includes with cycle detection and max-depth limits
- Conditional includes via `when=\"...\"`
- Heading re-leveling for included markdown (with graceful H6 overflow handling)

## Common Commands

```bash
# Build both packages
just -f darkmatter/justfile build

# Run tests
just -f darkmatter/justfile test

# Lint with clippy
just -f darkmatter/justfile lint
```

## License

AGPL-3.0-only
