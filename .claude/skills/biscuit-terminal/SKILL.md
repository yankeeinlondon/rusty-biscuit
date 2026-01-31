---
name: biscuit-terminal
description: Expert knowledge for the biscuit-terminal Rust library - the authority for terminal capability detection (12+ emulators) and rich terminal rendering. Provides inline image rendering (Kitty/iTerm2 via viuer), Mermaid diagram rendering (10 diagram types), OS/font detection, escape code analysis, and styled output. Use when building CLI apps with terminal-aware features, rendering images or diagrams inline, detecting color/underline support, or querying terminal environment. Darkmatter depends on this for all terminal rendering.
---

# biscuit-terminal

Terminal detection and rich rendering library for Rust. The authority for terminal-aware features in the dockhand monorepo.

## Core Principles

1. **Detection before rendering**: Always check terminal capabilities first
2. **Graceful fallback**: Use `fallback_render()` or alt text for unsupported terminals
3. **Static vs dynamic**: `Terminal` struct fields for static properties, methods for dynamic
4. **Security by default**: Path traversal protection, file size limits, remote URL blocking

## Quick Start

```rust
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::discovery::detection::ImageSupport;

let term = Terminal::new();

// Static properties (detected once)
println!("Terminal: {:?}, Image: {:?}", term.app, term.image_support);

// Dynamic queries
let (width, mode) = (Terminal::width(), Terminal::color_mode());

// Conditional rendering
if term.supports_italic { println!("\x1b[3mItalic\x1b[0m"); }
```

## Topics

| Topic | Description |
|-------|-------------|
| [Terminal Struct](./terminal-struct.md) | Main struct, static vs dynamic properties, enums |
| [Image Rendering](./image-rendering.md) | Kitty/iTerm2 protocols, width specs, security |
| [Mermaid Diagrams](./mermaid-diagrams.md) | 10 diagram types via mmdc CLI |
| [Detection Functions](./discovery.md) | App, color, underline, multiplex detection |
| [OS & Environment](./os-environment.md) | OS, distro, CI, fonts, locale |
| [Escape Codes](./escape-codes.md) | Strip, analyze, visual width calculation |
| [Styling](./styling.md) | Terminal-aware styling, Prose component |
| [bt Command](./cli.md) | CLI tool for inspection and diagrams |

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
use biscuit_terminal::components::mermaid::MermaidRenderer;

let renderer = MermaidRenderer::for_terminal("flowchart LR\n    A --> B");
renderer.render_for_terminal().unwrap_or_else(|_| {
    println!("{}", renderer.fallback_code_block());
});
```

### Light/Dark Adaptation

```rust
let fg = match Terminal::color_mode() {
    ColorMode::Light => "black",
    ColorMode::Dark | ColorMode::Unknown => "white",
};
```

## Terminal Support Matrix

| Terminal | Image | OSC8 | Italics |
|----------|-------|------|---------|
| WezTerm | Kitty | Yes | Yes |
| Kitty | Kitty | Yes | Yes |
| iTerm2 | Kitty* | Yes | Yes |
| Ghostty | Kitty | Yes | Yes |
| Alacritty | - | Yes | Yes |
| Konsole | Kitty | Yes | Yes |
| VS Code | - | Yes | Yes |

*iTerm2 uses native protocol even if Kitty advertised.

## bt CLI Diagram Commands

```bash
bt flowchart "A --> B --> C"
bt quadrant "Task: [0.5, 0.5]"
bt pie-chart "Dogs: 50" "Cats: 30"
bt git-graph "commit" "branch feature" "merge feature"
bt bar-chart 10 20 15 25
bt line-chart 1 8 7 5
bt timeline "2020: Started" "2022: Launch"
bt state-diagram "[*] --> Idle" "Idle --> Running"
bt erd "Customer ||--o{ Order : places"
```

All support: `--example`, `--width`, `--inverse`, `--title`, `--json`

## Module Structure

```
biscuit_terminal/
├── terminal.rs           # Terminal struct
├── discovery/            # Detection
│   ├── detection.rs      # App, color, image
│   ├── os_detection.rs   # OS, distro, CI
│   ├── fonts.rs          # Font name, size
│   ├── osc_queries.rs    # Color queries
│   └── eval.rs           # Escape analysis
├── components/           # Rendering
│   ├── terminal_image.rs # Image (viuer)
│   ├── image_options.rs  # Security guards
│   ├── mermaid.rs        # Diagram rendering
│   └── prose.rs          # Styled text
└── utils/
    ├── escape_codes.rs   # Strip/analyze
    └── styling.rs        # Terminal styles
```

## Resources

- [biscuit-terminal/lib](../../../biscuit-terminal/lib/) - Library source
- [biscuit-terminal/cli](../../../biscuit-terminal/cli/) - CLI source
