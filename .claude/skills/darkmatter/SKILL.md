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
| Markdown parsing (CommonMark + GFM) | darkmatter |
| Transform pipeline (replacement, interpolation) | darkmatter |
| Syntax highlighting | darkmatter (syntect) |
| Frontmatter extraction | darkmatter |
| HTML output | darkmatter |
| Document comparison/normalization | darkmatter |
| Terminal detection, image/mermaid rendering | **biscuit-terminal** |

## Quick Start

```rust
use darkmatter::markdown::{Markdown, output::{TerminalOptions, write_terminal}};

let md: Markdown = "# Hello\n\nWorld".into();
let mut stdout = std::io::stdout();
write_terminal(&mut stdout, &md, TerminalOptions::default())?;
```

## Transform Pipeline

The `transform()` API processes markdown through 4 stages:

1. **Text Replacement** - `replace:` frontmatter replaces literal strings
2. **Interpolation** - `{{ variable }}` expressions expand to values
3. **Cleanup** - Normalizes formatting (spacing, tables)
4. **Normalization** - Adjusts heading levels

```rust
use darkmatter::markdown::{Markdown, transform::TransformOptions};

let content = r#"---
replace:
  PLACEHOLDER: actual
name: Alice
---
# Welcome {{ name }}
PLACEHOLDER content here.
"#;

let mut md: Markdown = content.into();
let report = md.transform_mut()?;

// Content is now: "# Welcome Alice\nactual content here."
println!("Replacements: {}", report.replacements_applied);
println!("Interpolations: {}", report.interpolations_applied);
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

- [Transform Pipeline](./transform.md) - Text replacement, interpolation, cleanup
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
