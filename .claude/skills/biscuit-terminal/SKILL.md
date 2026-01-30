---
name: biscuit-terminal
description: Expert knowledge for the biscuit-terminal Rust library - the authority for terminal capability detection (12+ emulators) and rich terminal rendering. Provides inline image rendering (Kitty/iTerm2 via viuer), Mermaid diagram rendering, OS/font detection, escape code analysis, and styled output. Use when building CLI apps with terminal-aware features, rendering images or diagrams inline, detecting color/underline support, or querying terminal environment. Darkmatter depends on this for all terminal rendering.
last_updated: 2026-01-30T00:00:00Z
---

# biscuit-terminal

The **authority for terminal detection and rendering** in the dockhand monorepo. Provides terminal capability detection, rich terminal rendering, and inline media display.

Other packages (like darkmatter) depend on biscuit-terminal for:
- Terminal detection (color depth, image support, italics, underlines)
- Image rendering (Kitty/iTerm2 protocols via viuer)
- Mermaid diagram rendering

## Core Principles

- **Detection before rendering**: Always check terminal capabilities before using features
- **Graceful fallback**: Use `fallback_render()` or alt text for unsupported terminals
- **Static vs dynamic**: Use `Terminal` struct fields for static properties, methods for dynamic queries
- **Protocol awareness**: Image rendering automatically selects Kitty or iTerm2 based on terminal
- **Security by default**: Path traversal protection, file size limits (10MB), remote URL blocking
- **TTY-aware**: Most features check `is_tty` and degrade gracefully when piped
- **Feature-gated viuer**: The `viuer` feature (default-enabled) provides robust image rendering

## Quick Start

```rust
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::discovery::detection::ImageSupport;

let term = Terminal::new();

// Static properties
println!("Terminal: {:?}", term.app);
println!("Image support: {:?}", term.image_support);

// Dynamic queries
let width = Terminal::width();
let mode = Terminal::color_mode();

// Conditional feature usage
if term.supports_italic {
    println!("\x1b[3mItalic text\x1b[0m");
}
```

## Topics

### Core

- [Terminal Struct](./terminal-struct.md) - Main struct with all capabilities, static vs dynamic properties
- [Image Rendering](./image-rendering.md) - Kitty/iTerm2 protocols, width specs, security features, fallbacks

### Rendering

- **Mermaid Diagrams** - `MermaidRenderer` for terminal diagram display via mmdc CLI

### Discovery

- [Detection Functions](./discovery.md) - Terminal app, color, underline, multiplex, connection detection
- [OS & Environment](./os-environment.md) - OS type, Linux distro, CI detection, fonts, locale

### Utilities

- [Escape Codes](./escape-codes.md) - Stripping, analysis, visual width calculation
- [Styling](./styling.md) - Terminal-aware styling and Prose component

### CLI

- [bt Command](./cli.md) - Terminal inspector CLI tool

## Terminal Support Matrix

| Terminal | Image | OSC8 | Italics | Underlines |
|----------|-------|------|---------|------------|
| WezTerm | Kitty | Yes | Yes | Full |
| Kitty | Kitty | Yes | Yes | Full |
| iTerm2 | Kitty* | Yes | Yes | Full |
| Ghostty | Kitty | Yes | Yes | Full |
| Alacritty | - | Yes | Yes | Full |
| Konsole | Kitty | Yes | Yes | Partial |
| VS Code | - | Yes | Yes | Full |

*iTerm2 uses its native protocol even if Kitty is advertised.

## Common Patterns

### Secure Image Rendering with Options

```rust
use biscuit_terminal::components::image_options::TerminalImageOptions;
use biscuit_terminal::components::terminal_image::{TerminalImage, ImageWidth};

let options = TerminalImageOptions::builder()
    .base_path(PathBuf::from("/safe/directory"))
    .max_file_size(5 * 1024 * 1024) // 5MB limit
    .allow_remote(false)
    .width(ImageWidth::Percent(0.75))
    .use_viuer(true)
    .build();

let img = TerminalImage::new(path)?;
img.render_with_options(&options)?;  // Applies all security guards
```

### Conditional Image Rendering

```rust
match term.image_support {
    ImageSupport::Kitty | ImageSupport::ITerm => {
        let img = TerminalImage::new(path)?;
        print!("{}", img.render_to_terminal(&term)?);
    }
    ImageSupport::None => println!("[Image: {}]", path.display()),
}
```

### Mermaid Diagram Rendering

```rust
use biscuit_terminal::components::mermaid::MermaidRenderer;

let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
match renderer.render_for_terminal() {
    Ok(()) => {},
    Err(_) => println!("{}", renderer.fallback_code_block()),
}
```

### Light/Dark Mode Adaptation

```rust
let fg = match Terminal::color_mode() {
    ColorMode::Light => "black",
    ColorMode::Dark | ColorMode::Unknown => "white",
};
```

### CI-Aware Output

```rust
if term.is_ci {
    // Simplify output, disable interactive features
}
```

## Module Structure

```
biscuit_terminal/
├── terminal.rs           # Terminal struct with all capabilities
├── discovery/            # Detection modules
│   ├── detection.rs      # App, color, image, underline, ImageSupportResult
│   ├── os_detection.rs   # OS, distro, CI
│   ├── fonts.rs          # Font name, size, Nerd Font
│   ├── osc_queries.rs    # OSC10/11/12 color queries
│   ├── clipboard.rs      # OSC52 clipboard
│   └── eval.rs           # Escape code analysis
├── components/           # Rendering components
│   ├── terminal_image.rs # Image rendering (Kitty/iTerm2 via viuer)
│   ├── image_options.rs  # TerminalImageOptions with security guards
│   ├── mermaid.rs        # Mermaid diagram rendering via mmdc CLI
│   └── prose.rs          # Styled text
└── utils/                # Helpers
    ├── escape_codes.rs   # Strip/analyze
    └── styling.rs        # Terminal-aware styles
```

## Integration with darkmatter

Darkmatter uses biscuit-terminal for all terminal rendering:

```rust
// darkmatter delegates terminal detection to biscuit-terminal
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::discovery::detection::{color_depth, italics_support, underline_support};

// darkmatter uses biscuit-terminal for image rendering
use biscuit_terminal::components::terminal_image::TerminalImage;
use biscuit_terminal::components::image_options::TerminalImageOptions;

// darkmatter uses biscuit-terminal for mermaid diagrams
use biscuit_terminal::components::mermaid::MermaidRenderer;
```

## Resources

- [biscuit-terminal/lib](../../../biscuit-terminal/lib/) - Library source
- [biscuit-terminal/cli](../../../biscuit-terminal/cli/) - CLI source
- [Kitty Graphics Protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/)
- [iTerm2 Inline Images](https://iterm2.com/documentation-images.html)
