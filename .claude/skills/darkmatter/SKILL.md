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
| Compose pipeline (Inline Pre + Transclusion + Inline Post) | darkmatter |
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

Three-phase pipeline for document preparation:

**Inline Pre** (serial):
1. **Frontmatter Interpolation** - `{{ variable }}` in frontmatter resolves before effective state is built
2. **Frontmatter Shell Expansion** - top-level `$(cmd)` frontmatter values execute after interpolation and write trimmed `stdout` back into frontmatter
3. **Text Replacement** - `replace:` frontmatter replaces literal strings
4. **Page Blocks** - `::block` / `::end-block` conditional regions; nest to arbitrary depth with stack-based pairing, lazy child evaluation (skipped parents never evaluate inner `when`), and code-fence protection at every depth
5. **Interpolation** - `{{ variable }}` expressions expand to values
6. **Shell Expansion** - `::shell` directives execute approved commands and inject combined `stdout` + `stderr`

**Transclusion** (prepared serially, resolved concurrently):
- `::file ./doc.md` - Markdown transclusion with recursive processing
- `::code ./main.rs` - Code/text transclusion with fenced block generation
- `::toc-linking ./doc.md` - Linked heading lists from another document's raw source headings
- `prologue` / `epilogue` - Frontmatter-driven transclusion
- `when="..."` conditions, ancestry-based cycle detection, depth limits

**Inline Post** (serial):
1. **Cleanup** - Normalizes formatting (spacing, tables)
2. **Normalization** - Adjusts heading levels

```rust
use darkmatter::markdown::{Markdown, compose::{ComposeOptions, ComposeOperation}};

// All operations (default)
let mut md: Markdown = content.into();
let report = md.compose_mut()?;

// Full pipeline with transclusion (requires source file path)
let md = Markdown::try_from(std::path::Path::new("docs/root.md"))?;
let options = ComposeOptions::new()
    .with_source_file("docs/root.md");
let (composed, report) = md.compose_with(options)?;

// Only specific operations
let options = ComposeOptions::new()
    .only(&[ComposeOperation::Interpolation, ComposeOperation::Cleanup]);
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

### Shell Expansion Notes

- Body `::shell` and top-level frontmatter `$(...)` share policy loading, whitelist/blacklist checks, approval, and timeout behavior.
- Frontmatter shell expansion stores trimmed `stdout` only and treats malformed matching expressions as hard errors.
- Independent top-level frontmatter shell commands execute concurrently after approval/policy resolution.
- `--allow-shell-timeout` / `ComposeOptions::with_allow_shell_timeout(true)` convert timeouts into empty-string replacements plus compose warnings.

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
md graph doc.md                  # Dependency graph visualization
md graph doc.md --follow         # Recurse into transclusions
md graph doc.md --validate       # Inline validation overlays
```

### FileTree Component

`FileTree` is a `Renderable` component that visualizes a Markdown file's dependency surface:
- References (hyperlinks, images, CSS/script imports) above the file line
- Transclusions below the file line with section-aware captions
- Optional recursive expansion via `.follow_transclusions()`
- Optional validation overlays via `.validate()`

Located in `darkmatter/lib/src/markdown/reference/file_tree/`.

## See Also

- [biscuit-terminal skill](../biscuit-terminal/SKILL.md) - Terminal rendering dependency
