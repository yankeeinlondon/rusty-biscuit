# Darkmatter Library

Markdown parsing, rendering, and Mermaid diagram support for terminal and HTML output.

## Features

- **Multi-format output**: Terminal (ANSI), HTML, MDAST JSON, plain string
- **Syntax highlighting**: 200+ languages via syntect with curated theme pairs
- **Frontmatter support**: YAML parsing with typed access and merge strategies
- **Mermaid diagrams**: Render to terminal images or HTML with theme support
- **Document comparison**: Structural diff with change classification
- **Visual diff utilities**: Reusable string/file diff rendering (no Markdown dependency)
- **Table of Contents**: Hierarchical extraction with content hashing
- **Heading normalization**: Fix hierarchy violations, relevel documents
- **Image rendering**: Inline images via biscuit-terminal (Kitty/iTerm2 protocols)

## Architecture

Darkmatter focuses on **markdown parsing and transformation**. Visual diff rendering is exposed as a standalone module (`diff::visual`) for reuse outside Markdown. Terminal-specific capabilities are delegated to `biscuit-terminal`:

| Responsibility | Package |
|----------------|---------|
| Markdown parsing (CommonMark + GFM) | darkmatter-lib |
| Syntax highlighting | darkmatter-lib (syntect) |
| Frontmatter extraction | darkmatter-lib |
| Document comparison/normalization | darkmatter-lib |
| Terminal detection | biscuit-terminal |
| Image rendering (Kitty/iTerm2) | biscuit-terminal |
| Mermaid diagram rendering | biscuit-terminal |
| Color depth, italic support | biscuit-terminal |

## Quick Start

```rust
use darkmatter::markdown::{Markdown, output::{TerminalOptions, write_terminal}};

let md: Markdown = "# Hello\n\nWorld".into();
let mut stdout = std::io::stdout();
write_terminal(&mut stdout, &md, TerminalOptions::default())?;
```

## Modules

| Module | Description |
|--------|-------------|
| `markdown` | Core `Markdown` type with frontmatter, rendering, and manipulation |
| `diff` | Visual diff utilities for strings and files |
| `mermaid` | Mermaid diagram theming (terminal rendering via biscuit-terminal) |
| `render` | Hyperlink rendering utilities (OSC 8 terminal hyperlinks) |
| `terminal` | ANSI code generation and color depth constants |
| `testing` | Test utilities for terminal output verification |

## API Reference

### The Markdown Type

```rust
use darkmatter::markdown::Markdown;

// Create from string
let md: Markdown = "# Title\n\nContent".into();

// Load from file
let md = Markdown::try_from(Path::new("README.md"))?;

// Load from URL (async)
let md = Markdown::from_url(&url).await?;
```

### Frontmatter Operations

```rust
let content = r#"---
title: Hello
author: Alice
---
# Document"#;

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

### Output Formats

#### Terminal Output

```rust
use darkmatter::markdown::output::{TerminalOptions, write_terminal, for_terminal};

let options = TerminalOptions {
    include_line_numbers: true,
    mermaid_mode: MermaidMode::Image,
    ..Default::default()
};

// Write to stdout
write_terminal(&mut std::io::stdout(), &md, options)?;

// Get as string
let output = for_terminal(&md, options)?;
```

#### HTML Output

```rust
use darkmatter::markdown::output::{HtmlOptions, as_html};

let options = HtmlOptions::default();
let html = md.as_html(options)?;
```

#### MDAST JSON

```rust
let ast = md.as_ast()?;
let json = serde_json::to_string_pretty(&ast)?;
```

#### Plain String

```rust
let output = md.as_string();  // Includes frontmatter if present
```

### Document Cleanup

```rust
let mut md: Markdown = content.into();
md.cleanup();  // Normalize spacing, align tables
```

### Table of Contents

```rust
let toc = md.toc();
println!("Heading count: {}", toc.heading_count());
println!("Root level: {:?}", toc.root_level());
println!("Title: {:?}", toc.title);
```

### Document Comparison

```rust
let original: Markdown = old_content.into();
let updated: Markdown = new_content.into();

let delta = original.delta(&updated);

if !delta.is_unchanged() {
    println!("Classification: {:?}", delta.classification);
    println!("{}", delta.summary());
}
```

### Visual Diff (Strings or Files)

Darkmatter uses this visual diff renderer in the Markdown CLI for frontmatter and body comparisons, but the module itself is Markdown-agnostic and works directly with any strings or files.

```rust
use darkmatter::diff::visual::{render_visual_diff_str, VisualDiffOptions};

let original = "Hello\nWorld";
let updated = "Hello\nUniverse";

let output = render_visual_diff_str(original, updated, &VisualDiffOptions::default());
println!("{}", output);
```

### Heading Normalization

```rust
use darkmatter::markdown::HeadingLevel;

// Validate structure
let validation = md.validate_structure();
if !validation.is_well_formed() {
    println!("Issues: {:?}", validation.issues);
}

// Normalize to H1 root
let (normalized, report) = md.normalize(Some(HeadingLevel::H1))?;

// Relevel for embedding as subsection
let (releveled, adjustment) = md.relevel(HeadingLevel::H2)?;
```

### Mermaid Diagrams

For HTML output, use darkmatter's theming:

```rust
use darkmatter::mermaid::{Mermaid, MermaidTheme};

let diagram = Mermaid::new("flowchart LR\n    A --> B")
    .with_title("My Flowchart")
    .with_footer("Generated 2026-01-29");

// HTML output with theme
let html = diagram.render_for_html();
println!("<head>{}</head><body>{}</body>", html.head, html.body);
```

For terminal output, use biscuit-terminal's `MermaidRenderer`:

```rust
use biscuit_terminal::components::mermaid::MermaidRenderer;

let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
match renderer.render_for_terminal() {
    Ok(()) => {},
    Err(_) => println!("{}", renderer.fallback_code_block()),
}
```

## Syntax Highlighting

### Theme Pairs

Themes come in light/dark pairs with automatic mode detection:

| Theme | Light | Dark |
|-------|-------|------|
| `Github` | GitHub Light | GitHub Dark |
| `OneHalf` | One Half Light | One Half Dark |
| `Gruvbox` | Gruvbox Light | Gruvbox Dark |
| `Solarized` | Solarized Light | Solarized Dark |
| `Base16Ocean` | Base16 Ocean Light | Base16 Ocean Dark |
| `Nord` | Nord | Nord |
| `Dracula` | Dracula | Dracula |
| `Monokai` | Monokai | Monokai |
| `VisualStudioDark` | VS Dark | VS Dark |

### Color Mode Detection

```rust
use biscuit_terminal::terminal::Terminal;

let mode = Terminal::color_mode();  // Light, Dark, or Unknown
```

Terminal metadata also includes repository context. In monorepos, `package_root` reports the package containing the current working directory (or the repo root when run at the root).

## Terminal Options

```rust
pub struct TerminalOptions {
    pub code_theme: ThemePair,        // Theme for code blocks
    pub prose_theme: ThemePair,       // Theme for prose
    pub color_mode: ColorMode,        // Light or Dark
    pub include_line_numbers: bool,   // Show line numbers in code
    pub color_depth: Option<ColorDepth>,  // Auto-detect if None
    pub render_images: bool,          // Enable image rendering
    pub base_path: Option<PathBuf>,   // For relative image paths
    pub italic_mode: ItalicMode,      // Auto, Always, Never
    pub max_width: Option<u16>,       // Text wrapping width
    pub mermaid_mode: MermaidMode,    // Off, Image, Text
    pub hyperlink_mode: HyperlinkMode, // Auto, Always, Never
}
```

## CLI

For command-line usage, see the [darkmatter-cli](../cli/) package which provides the `md` binary.

## Dependencies

- **pulldown-cmark**: CommonMark parsing with GFM extensions
- **syntect**: Syntax highlighting engine
- **two-face**: Theme loading with bat-curated themes
- **biscuit-terminal**: Terminal detection, image rendering, mermaid diagrams
- **comfy-table**: Table rendering with box-drawing characters
- **serde**: Frontmatter serialization
