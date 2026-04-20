---
name: biscuit-terminal
description: Expert knowledge for the biscuit-terminal Rust library - the authority for terminal capability detection (13+ emulators) and rich terminal rendering. Provides inline image rendering (Kitty/iTerm2 protocols), terminal-facing Mermaid and graph adapters backed by biscuit-visualized, OS/font detection, escape code analysis, color system (BasicColor, WebColor, Tailwind), and composable rendering components. Use when building CLI apps with terminal-aware features, rendering images or diagrams inline, detecting color/underline support, or querying terminal environment. Darkmatter depends on this for terminal Mermaid rendering.
---

# biscuit-terminal

Terminal detection and rich rendering library for Rust. The authority for terminal-aware features in the dockhand monorepo.

## Core Principles

1. **Detection before rendering**: Always check terminal capabilities first
2. **Graceful fallback**: Use `fallback_render()` and explicit app-level text fallback where needed
3. **Static vs dynamic**: `Terminal` struct fields for static properties, methods for dynamic
4. **Input validation + policy**: `TerminalImage::new()` validates local paths; apply app-level policies with `TerminalImageOptions`

## Quick Start

```rust
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::discovery::detection::ImageSupport;

let term = Terminal::new();

// Static properties (detected once)
println!("Terminal: {:?}, Image: {:?}", term.app, term.image_support);

// Dynamic queries (width/height are instance methods, color_mode is static)
let (width, mode) = (term.width(), Terminal::color_mode());

// Conditional rendering
if term.supports_italic { println!("\x1b[3mItalic\x1b[0m"); }
```

## Topics

| Topic | Description |
|-------|-------------|
| [Terminal Struct](./terminal-struct.md) | Main struct, static vs dynamic properties, enums |
| [Components](./components.md) | All renderable components: BlockQuote, Compose, FileSystem, GraphExpression, InlineContent, MermaidDiagram, OrderedList, UnorderedList, PadLeft, PadRight, Progress, Prose, Section, Status, StatusBlock, Table, TerminalImage, TextBlock, Todo, TwoColumn |
| [Image Rendering](./image-rendering.md) | Kitty/iTerm2 protocols, width parsing, cursor behavior, policy controls |
| [Mermaid Diagrams](./mermaid-diagrams.md) | Terminal-facing `MermaidDiagram` adapter backed by biscuit-visualized |
| [Color System](./color-system.md) | BasicColor, RgbColor, WebColor, Tailwind, HdrColor with TermColor trait |
| [Detection Functions](./discovery.md) | App, color, underline, multiplex detection |
| [OS & Environment](./os-environment.md) | OS, distro, CI, fonts, locale |
| [Escape Codes](./escape-codes.md) | Strip, analyze, visual width calculation |
| [Styling](./styling.md) | Terminal-aware styling, Prose component, TextBlock |
| [bt Command](./cli.md) | CLI tool: 17 commands for inspection, diagrams, text, and filesystem |

## Common Patterns

### Conditional Image Rendering

```rust
match term.image_support {
    ImageSupport::Kitty | ImageSupport::ITerm => {
        TerminalImage::new(path)?.render_to_terminal(&term)?;
    }
    ImageSupport::None => println!("[Image: {}]", path.display()),
}
```

### Mermaid Diagram

```rust
use biscuit_terminal::components::mermaid::MermaidDiagram;
use biscuit_terminal::terminal::Terminal;

let diagram = MermaidDiagram::new("flowchart LR\n    A --> B");
let term = Terminal::new();

if let Err(err) = diagram.try_render(&term) {
    eprintln!("Diagram render failed: {err}");
    // Optional app-level fallback if you want textual output:
    println!("{}", diagram.fallback_code_block());
}
```

### Graph Diagram

```rust
use biscuit_terminal::components::graph_expression::{
    GraphExpression, GraphInputSyntax, GraphOrientation,
};

let graph = GraphExpression::for_terminal("a -> b -> c", GraphInputSyntax::Auto)?
    .with_orientation(GraphOrientation::LeftToRight)
    .with_title("Example graph");
```

### Light/Dark Adaptation

```rust
let fg = match Terminal::color_mode() {
    ColorMode::Light => "black",
    ColorMode::Dark | ColorMode::Unknown => "white",
};
```

### Status Blocks

```rust
use biscuit_terminal::prelude::{Prose, StatusBlock, StatusState};

let block = StatusBlock::new(StatusState::Error)
    .header("<b>Shell Expansion Failed</b>")
    .body(Prose::new("Missing closing brace in `${...}` directive."))
    .hint("Check the template syntax and retry.");
```

Use `StatusBlock` when you need the common Claudine-style `Status` header plus a colored
`BlockQuote` body and optional hint as one renderable. It defaults to a `┃ ` border,
`left_margin = 0`, `right_margin = 5`, and `WordWrap::WrapProse(Some(8), None)` so the
body border lines up with the preceding `Status` icon/header line.

`StatusState::Error` is now the canonical error severity. `StatusState::Failure` remains as a
deprecated compatibility variant, and persisted JSON `"Failure"` still deserializes as
`StatusState::Error`. Prefer `StatusState::default_color()` when you want the canonical border
or accent color for a severity instead of re-encoding the Tailwind mapping yourself. See
[`biscuit-terminal/README.md`](../../../biscuit-terminal/README.md) for the full severity table
and override knobs.

## Terminal Support Matrix

| Terminal | Image | OSC8 | Italics |
|----------|-------|------|---------|
| WezTerm | Kitty | Yes | Yes |
| Kitty | Kitty | Yes | Yes |
| iTerm2 | ITerm | Yes | Yes |
| Ghostty | Kitty | Yes | Yes |
| Konsole | Kitty | Yes | Yes |
| Warp | Kitty | Yes | Yes |
| Wast | Kitty | No | No |
| Alacritty | - | Yes | Yes |
| Apple Terminal | - | No | Yes |
| GNOME Terminal | - | Yes | Yes |
| Foot | - | Yes | Yes |
| Contour | - | Yes | Yes |
| VS Code | Kitty* | Yes | Yes |

\* Requires `terminal.integrated.enableImages` and GPU acceleration enabled in VS Code settings.

## bt CLI Commands (17 commands)

```bash
# Terminal inspection
bt                              # Pretty-printed capabilities
bt --json                       # JSON output for scripting

# Styled text and layout
bt prose "Hello {{bold}}world{{reset}}!"
bt prose "<red>Error</red>: message"
bt quote --attribution "Shakespeare" "To be or not to be"
bt list "First item" "Second item" "Third item"
bt columns --gap 6 --left 40% "Title" "Description"

# Filesystem
bt dir src --depth 2 --filter ".rs"
bt dir --size --tokens --modified

# Diagrams (10 Mermaid types + 1 graph type)
bt flowchart "A --> B --> C"
bt quadrant "Task: [0.5, 0.5]"
bt pie-chart "Dogs: 50" "Cats: 30"
bt git-graph "commit" "branch feature" "merge feature"
bt bar-chart --horizontal --show-data-label --aspect-ratio 2.0 --inverse 10 20 15 25
bt line-chart --width 60% --horizontal --show-data-label 1 8 7 5
bt timeline "2020: Started" "2022: Launch"
bt state-diagram "[*] --> Idle" "Idle --> Running"
bt erd "Customer ||--o{ Order : places"
bt graph-expression "a -> b -> c"          # Arrow syntax (directed)
bt graph-expression "a -- b -- c"          # Dash syntax (undirected)
bt graph-expression --syntax dot "digraph { A -> B; }"  # DOT syntax
```

Diagram options: `--example`, `--width`, `--inverse`, `--title`, `--json`, `--meta`
Bar/line chart extras: `--horizontal`, `--show-data-label`, `--aspect-ratio`
Graph extras: `--syntax` (auto, expression, dot), `--orientation` (left-to-right, top-to-bottom)
Graph note: mixed `->` and `--` expression syntax is rejected; use separate graphs instead.

## Module Structure

```
biscuit_terminal/
├── terminal.rs           # Terminal struct + TerminalBuilder
├── prelude.rs            # Public re-exports
├── discovery/            # Detection
│   ├── detection.rs      # App, color, image, multiplex
│   ├── os_detection.rs   # OS, distro, CI
│   ├── fonts.rs          # Font name, size, cell_size
│   ├── osc_queries.rs    # bg/fg/cursor color queries
│   ├── clipboard.rs      # OSC52 clipboard
│   ├── locale.rs         # Locale, character encoding
│   ├── mode_2027.rs      # Grapheme cluster support
│   ├── cursor_position.rs # Cursor position queries
│   └── eval.rs           # Escape analysis
├── components/           # Rendering
│   ├── renderable.rs     # Renderable trait + RenderableContent
│   ├── compose.rs        # Compose (combine multiple renderables)
│   ├── section.rs        # Section with heading levels (h1-h6)
│   ├── block_quote.rs    # BlockQuote with attribution
│   ├── prose.rs          # Styled text with tokens
│   ├── status_block.rs   # Status + BlockQuote + hint composite
│   ├── text_block.rs     # Uniform block styling
│   ├── inline_content.rs # Inline concatenation without newlines
│   ├── list.rs           # OrderedList, UnorderedList
│   ├── table/            # Table with box-drawing borders
│   ├── two_column.rs     # TwoColumn side-by-side layout
│   ├── todo.rs           # Todo with states (Open, InProgress, etc.)
│   ├── progress.rs       # Progress indicator rendering
│   ├── filesystem.rs     # File/directory tree rendering
│   ├── terminal_image.rs # Image (Kitty/iTerm2 protocols)
│   ├── image_options.rs  # Policy options/helpers (app-enforced)
│   ├── mermaid.rs        # Terminal-facing Mermaid adapter
│   └── graph_expression.rs # Terminal-facing graph adapter
└── utils/
    ├── layout.rs         # Layout, Margin, WordWrap, Alignment
    ├── color.rs          # Color, BasicColor, RgbColor, WebColor, Tailwind, HdrColor
    ├── styling.rs        # Stylist trait, FontWeight, Style
    ├── escape_codes.rs   # ANSI escape code generation
    ├── block_constraint.rs # Visual width, line splitting
    ├── word_wrap.rs      # Word wrapping strategies
    ├── text.rs           # Content length calculation (escape-aware)
    ├── truncate.rs       # Text truncation
    └── multiplex.rs      # Multiplexing detection
```

## Key Dependencies

- `sniff` - Git/repo/monorepo detection used by `Terminal::new()`
- `biscuit-visualized` - Owns Mermaid and graph artifact generation, theming, rasterization, and caching

## Resources

- [biscuit-terminal/lib](../../../biscuit-terminal/lib/) - Library source
- [biscuit-terminal/cli](../../../biscuit-terminal/cli/) - CLI source
