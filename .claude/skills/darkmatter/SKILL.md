---
name: darkmatter
description: Expert knowledge for the darkmatter Rust library - markdown parsing, rendering (terminal/HTML), syntax highlighting, frontmatter, Mermaid diagrams, document comparison, and TOC extraction. Delegates terminal rendering to biscuit-terminal. Use when parsing markdown, generating terminal/HTML output, working with frontmatter, comparing documents, or normalizing heading structures.
last_updated: 2026-01-30T00:00:00Z
---

# darkmatter

Markdown parsing, rendering, and transformation library. Part of the dockhand monorepo.

**Key architectural principle**: darkmatter focuses on **markdown parsing and transformation**. All terminal-specific rendering (images, mermaid diagrams, terminal detection) is delegated to `biscuit-terminal`.

## Responsibility Split

| Responsibility | Package |
|----------------|---------|
| Markdown parsing (CommonMark + GFM) | darkmatter-lib |
| Syntax highlighting | darkmatter-lib (syntect) |
| Frontmatter extraction | darkmatter-lib |
| Document comparison/normalization | darkmatter-lib |
| HTML output | darkmatter-lib |
| Terminal detection | **biscuit-terminal** |
| Image rendering (Kitty/iTerm2) | **biscuit-terminal** |
| Mermaid diagram rendering | **biscuit-terminal** |
| Color depth, italic support | **biscuit-terminal** |

## Quick Start

```rust
use darkmatter_lib::markdown::{Markdown, output::{TerminalOptions, write_terminal}};

let md: Markdown = "# Hello\n\nWorld".into();
let mut stdout = std::io::stdout();
write_terminal(&mut stdout, &md, TerminalOptions::default())?;
```

## Topics

### Core

- **Markdown Type** - Parse, manipulate, and render markdown documents
- **Frontmatter** - YAML parsing with typed access and merge strategies
- **Document Comparison** - Structural diff with change classification

### Output Formats

- **Terminal Output** - ANSI escape codes with syntax highlighting (uses biscuit-terminal)
- **HTML Output** - Web-ready HTML with CSS classes
- **MDAST JSON** - Abstract syntax tree for programmatic manipulation

### Document Processing

- **Heading Normalization** - Fix hierarchy violations, relevel documents
- **Table of Contents** - Hierarchical extraction with content hashing
- **Cleanup** - Normalize spacing, align tables

## Terminal Rendering

darkmatter's terminal output uses biscuit-terminal for:

### Terminal Detection

```rust
// Detection delegated to biscuit-terminal
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::discovery::detection::color_depth;

let term = Terminal::new();
let depth = color_depth();  // TrueColor, Colors256, etc.
```

### Image Rendering

```rust
// Images rendered via biscuit-terminal
use biscuit_terminal::components::terminal_image::TerminalImage;
use biscuit_terminal::components::image_options::TerminalImageOptions;

let options = TerminalImageOptions::builder()
    .base_path(base_path)
    .max_file_size(10 * 1024 * 1024)
    .build();

let img = TerminalImage::new(path)?;
img.render_with_options(&options)?;
```

### Mermaid Diagrams

For terminal output, use biscuit-terminal's `MermaidRenderer`:

```rust
use biscuit_terminal::components::mermaid::MermaidRenderer;

let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
match renderer.render_for_terminal() {
    Ok(()) => {},
    Err(_) => println!("{}", renderer.fallback_code_block()),
}
```

For HTML output, use darkmatter's theming:

```rust
use darkmatter_lib::mermaid::{Mermaid, MermaidTheme};

let diagram = Mermaid::new("flowchart LR\n    A --> B")
    .with_title("My Flowchart");
let html = diagram.render_for_html();
```

## TerminalOptions

```rust
pub struct TerminalOptions {
    pub code_theme: ThemePair,        // Theme for code blocks
    pub prose_theme: ThemePair,       // Theme for prose
    pub color_mode: ColorMode,        // Light or Dark
    pub include_line_numbers: bool,   // Show line numbers
    pub color_depth: Option<ColorDepth>,  // Auto-detect if None
    pub render_images: bool,          // Enable image rendering
    pub base_path: Option<PathBuf>,   // For relative image paths
    pub italic_mode: ItalicMode,      // Auto, Always, Never
    pub max_width: Option<u16>,       // Text wrapping width
    pub mermaid_mode: MermaidMode,    // Off, Image, Text
}
```

## Syntax Highlighting

### Theme Pairs

Themes come in light/dark pairs:

| Theme | Light | Dark |
|-------|-------|------|
| `Github` | GitHub Light | GitHub Dark |
| `OneHalf` | One Half Light | One Half Dark |
| `Gruvbox` | Gruvbox Light | Gruvbox Dark |
| `Solarized` | Solarized Light | Solarized Dark |
| `Nord` | Nord | Nord |
| `Dracula` | Dracula | Dracula |
| `Monokai` | Monokai | Monokai |

### Color Mode Detection

```rust
use biscuit_terminal::terminal::Terminal;

let mode = Terminal::color_mode();  // Light, Dark, or Unknown
```

## Frontmatter Operations

```rust
let mut md: Markdown = content.into();

// Typed access
let title: Option<String> = md.fm_get("title")?;

// Insert values
md.fm_insert("version", "1.0")?;

// Merge with strategy
md.fm_merge_with(json!({"tags": ["rust"]}), MergeStrategy::ErrorOnConflict)?;

// Set defaults (document wins)
md.fm_set_defaults(json!({"draft": false}))?;
```

## Document Comparison

```rust
let original: Markdown = old_content.into();
let updated: Markdown = new_content.into();

let delta = original.delta(&updated);

if !delta.is_unchanged() {
    println!("Classification: {:?}", delta.classification);
    println!("{}", delta.summary());
}
```

## Module Structure

```
darkmatter_lib/
├── markdown/
│   ├── mod.rs              # Markdown type
│   ├── frontmatter/        # YAML frontmatter handling
│   ├── output/
│   │   ├── terminal.rs     # ANSI output (uses biscuit-terminal)
│   │   └── html.rs         # HTML output
│   ├── highlighting/       # Syntax highlighting (syntect)
│   └── dsl/                # Code block metadata parsing
├── mermaid/
│   ├── mod.rs              # Mermaid type
│   ├── render_html.rs      # HTML rendering
│   └── render_terminal.rs  # Delegates to biscuit-terminal
├── terminal/
│   ├── ansi.rs             # ANSI escape code builders
│   └── supports.rs         # Thin wrappers over biscuit-terminal
└── render/
    └── link.rs             # Hyperlink rendering
```

## Dependencies

- **pulldown-cmark**: CommonMark parsing with GFM extensions
- **syntect**: Syntax highlighting engine
- **two-face**: Theme loading with bat-curated themes
- **biscuit-terminal**: Terminal detection, image rendering, mermaid diagrams
- **comfy-table**: Table rendering with box-drawing characters
- **serde**: Frontmatter serialization

## CLI

The `darkmatter-cli` package provides the `md` binary:

```bash
# Render to terminal
md doc.md

# Clean document
md doc.md --clean

# Render to HTML
md doc.md --html

# Render as JSON AST
md doc.md --ast
```

## Resources

- [darkmatter/lib](../../../darkmatter/lib/) - Library source
- [darkmatter/cli](../../../darkmatter/cli/) - CLI source
- [biscuit-terminal skill](../biscuit-terminal/SKILL.md) - Terminal rendering dependency
