---
name: biscuit-terminal
description: Expert knowledge for the biscuit-terminal Rust library providing terminal capability detection (12+ emulators), inline image rendering (Kitty/iTerm2 protocols), OS/font detection, escape code analysis, and styled output utilities. Use when building CLI apps with terminal-aware features, rendering images inline, detecting color/underline support, or querying terminal environment.
last_updated: 2025-01-29T00:00:00Z
---

# biscuit-terminal

A Rust library for terminal capability detection and rich terminal rendering. Part of the dockhand monorepo.

## Core Principles

- **Detection before rendering**: Always check terminal capabilities before using features
- **Graceful fallback**: Use `fallback_render()` or alt text for unsupported terminals
- **Static vs dynamic**: Use `Terminal` struct fields for static properties, methods for dynamic queries
- **Protocol awareness**: Image rendering automatically selects Kitty or iTerm2 based on terminal
- **TTY-aware**: Most features check `is_tty` and degrade gracefully when piped

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
- [Image Rendering](./image-rendering.md) - Kitty/iTerm2 protocols, width specs, fallbacks

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
├── terminal.rs           # Terminal struct
├── discovery/            # Detection modules
│   ├── detection.rs      # App, color, image, underline
│   ├── os_detection.rs   # OS, distro, CI
│   ├── fonts.rs          # Font name, size, Nerd Font
│   ├── osc_queries.rs    # OSC10/11/12 color queries
│   ├── clipboard.rs      # OSC52 clipboard
│   └── eval.rs           # Escape code analysis
├── components/           # Rendering
│   ├── terminal_image.rs # Image rendering
│   └── prose.rs          # Styled text
└── utils/                # Helpers
    ├── escape_codes.rs   # Strip/analyze
    └── styling.rs        # Terminal-aware styles
```

## Resources

- [biscuit-terminal/lib](../../../biscuit-terminal/lib/) - Library source
- [biscuit-terminal/cli](../../../biscuit-terminal/cli/) - CLI source
- [Kitty Graphics Protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/)
- [iTerm2 Inline Images](https://iterm2.com/documentation-images.html)
