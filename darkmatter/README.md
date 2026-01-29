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

| Package | Description |
|---------|-------------|
| [`cli/`](cli/) | The `md` command-line tool for rendering markdown |
| [`lib/`](lib/) | Core library for markdown parsing, rendering, and manipulation |

## Features

- **Terminal rendering**: ANSI escape codes with automatic color depth detection
- **HTML output**: Standalone HTML with embedded styles
- **Syntax highlighting**: Language-aware code block highlighting via syntect
- **Image rendering**: Inline images in supported terminals (iTerm2, Kitty, etc.)
- **Mermaid diagrams**: Render mermaid diagrams to terminal or HTML
- **Theme support**: Multiple prose and code themes with light/dark detection
- **Markdown cleanup**: Normalize markdown formatting
- **Document comparison**: Structural diff between markdown documents
- **Table of contents**: Extract document structure as tree or JSON

## Common Commands

```bash
# Build both packages
just -f darkmatter/justfile build

# Run tests
just -f darkmatter/justfile test

# Lint with clippy
just -f darkmatter/justfile lint

# Open library documentation
just -f darkmatter/justfile docs
```

## License

AGPL-3.0-only
