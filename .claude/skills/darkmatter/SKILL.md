---
name: darkmatter
description: Expert knowledge for the darkmatter Rust library - markdown parsing, rendering (terminal/HTML), syntax highlighting, frontmatter, and document comparison. Delegates terminal rendering to biscuit-terminal. Use when parsing markdown, generating terminal/HTML output, working with frontmatter, or comparing documents.
---

# darkmatter

Markdown parsing, rendering, and composition library. Part of the dockhand monorepo.

**Key principle**: darkmatter handles **markdown parsing and composition**. Terminal rendering (images, mermaid, detection) is delegated to `biscuit-terminal`.

## Responsibility Split

| Responsibility | Package |
|----------------|---------|
| Markdown parsing (CommonMark + GFM) | darkmatter |
| Compose pipeline (Stage 1 + Stage 2 transclusion) | darkmatter |
| Syntax highlighting | darkmatter (syntect) |
| Frontmatter extraction | darkmatter |
| HTML output | darkmatter |
| Visual diff utilities | darkmatter |
| Document comparison/normalization | darkmatter |
| Terminal detection, image/mermaid rendering, tables | **biscuit-terminal** |

## Quick Start

```rust
use darkmatter::markdown::{Markdown, output::{TerminalOptions, write_terminal}};

let md: Markdown = "# Hello\n\nWorld".into();
let mut stdout = std::io::stdout();
write_terminal(&mut stdout, &md, TerminalOptions::default())?;
```

## Compose Pipeline

Two-stage pipeline for document preparation:

**Stage 1** (in-memory transforms):
1. **Text Replacement** - `replace:` frontmatter replaces literal strings
2. **Interpolation** - `{{ variable }}` expressions expand to values
3. **Cleanup** - Normalizes formatting (spacing, tables)
4. **Normalization** - Adjusts heading levels

**Stage 2** (file-based transclusion):
- `::file ./doc.md` - Markdown transclusion with recursive processing
- `::code ./main.rs` - Code/text transclusion with fenced block generation
- `prologue` / `epilogue` - Frontmatter-driven transclusion
- `when="..."` conditions, cycle detection, depth limits

```rust
use darkmatter::markdown::{Markdown, compose::ComposeOptions};

// Stage 1 only
let mut md: Markdown = content.into();
let report = md.compose_mut()?;

// Stage 1 + Stage 2 (requires source file path)
let md = Markdown::try_from(std::path::Path::new("docs/root.md"))?;
let options = ComposeOptions::new()
    .with_source_file("docs/root.md");
let (composed, report) = md.compose_with(options)?;
```

### Interpolation Expressions

| Expression | Example | Description |
|------------|---------|-------------|
| Variable | `{{ name }}` | Frontmatter value |
| Nested | `{{ user.email }}` | Nested object access |
| Context | `{{ ctx.today }}` | Runtime context (today, year, etc.) |
| Environment | `{{ env.HOME }}` | Environment variable |
| Fallback | `{{ color \| "unknown" }}` | Default if missing |
| Ternary | `{{ x ? "yes" : "no" }}` | Conditional |
| Comparison | `{{ count > 0 ? "has" : "empty" }}` | Numeric comparison |
| Functions | `{{ length(items) }}` | Helper functions |

### Helper Functions

- `length(x)` - String/array/object length
- `number(x, default)` - Parse as number
- `round(x, default)` - Round to integer

## Detailed Topics

- [Compose Pipeline](./compose.md) - Text replacement, interpolation, cleanup
- [Terminal Output](./terminal.md) - ANSI rendering, themes, options
- [Frontmatter](./frontmatter.md) - YAML parsing, merge strategies
- [Document Comparison](./comparison.md) - Structural diff, change classification
- [Module Structure](./structure.md) - Package organization and dependencies

## CLI

```bash
md doc.md                        # Render to terminal (auto mode)
md doc.md --output html          # HTML output
md doc.md --output json          # AST JSON output
md doc.md --output markdown      # Plain markdown text
md doc.md --clean                # Normalize formatting (stdout)
md doc.md --clean-save           # Normalize and save back to file
md doc.md --output html --show   # Open in default app
md toc doc.md                    # Table of contents
md delta old.md new.md           # Document comparison
md -v delta old.md new.md        # Verbose comparison (-v is top-level)
```

## See Also

- [biscuit-terminal skill](../biscuit-terminal/SKILL.md) - Terminal rendering dependency
