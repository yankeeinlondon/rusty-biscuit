# biscuit-terminal

Terminal capability detection, rendering utilities, and image/diagram display for Rust applications.

## Features

- **Terminal App Detection**: Recognize 12+ terminal emulators with capability profiles
- **Image Rendering**: Inline images via Kitty/iTerm2 protocols with security guards
- **Mermaid Diagrams**: Render diagrams to terminal using mmdc CLI with viuer
- **OS Detection**: Identify operating system and Linux distribution
- **Font Detection**: Extract font name and size from terminal config files
- **Color Support**: Query color depth, mode (light/dark), and background color
- **Escape Code Analysis**: Calculate visual line widths, detect escape codes
- **Clipboard**: OSC52 clipboard support for compatible terminals
- **Styled Output**: Composable rendering components (Prose, Table, List)

## Quick Start

```rust
use biscuit_terminal::terminal::Terminal;

fn main() {
    let term = Terminal::new();

    println!("Running in {:?}", term.app);
    println!("Terminal size: {}x{}", Terminal::width(), Terminal::height());

    if term.supports_italic {
        println!("\x1b[3mItalic text!\x1b[0m");
    }
}
```

## Modules

- `terminal` - Main `Terminal` struct with all capabilities
- `discovery::detection` - Low-level detection functions
- `discovery::os_detection` - OS and Linux distribution detection
- `discovery::fonts` - Font name/size detection via config parsing
- `discovery::config_paths` - Terminal config file paths
- `discovery::osc_queries` - Terminal color queries
- `discovery::clipboard` - OSC52 clipboard support
- `discovery::mode_2027` - Unicode grapheme cluster support
- `discovery::eval` - Escape code analysis utilities
- `components::terminal_image` - Terminal image rendering (Kitty/iTerm2 with fallbacks)
- `components::mermaid` - Mermaid diagram rendering via mmdc CLI

## Terminal Images (TerminalImage)

`TerminalImage` renders inline images using the Kitty graphics protocol with automatic iTerm2 handling and a graceful text fallback.

### Width syntax
- `path.png` → default 50% of available width
- `path.png|50%` → percentage of available columns
- `path.png|80` → fixed columns
- `path.png|fill` → fill available width

### Protocol selection
- Kitty-capable terminals (Kitty, WezTerm, Ghostty, etc.): use Kitty protocol
- iTerm2: uses iTerm’s native inline images even if iTerm advertises Kitty, to avoid Kitty-path failures
- Others / unsupported: falls back to alt text

### Aspect ratio handling
- Uses measured cell size when available (`discovery::fonts::cell_size`) to compute pixel targets; falls back to 8×16 px cells. This keeps images from looking “squished” in terminals with non-2:1 cells (e.g., WezTerm).
- Respects user width specs and preserves aspect ratio; explicit widths are allowed to upscale, while the implicit 50% keeps a no-upscale guard.

### Kitty specifics
- Sends cell-based sizing (`c=`/`r=`) rather than pixel sizing for consistent layout.
- Advances the cursor below the image after rendering so prompts don’t overlap.

### iTerm2 specifics
- Forces iTerm path when `TERM_PROGRAM=iTerm.app`, even if Kitty is advertised.
- Uses `inline=1;preserveAspectRatio=1;width=<user spec>;size=auto`.
- Appends a cursor advance based on measured cell height to avoid prompt collisions; avoids extra escape clutter that previously caused ENOENT errors in iTerm.

### Security Features

`TerminalImage` includes built-in security guards:

- **Path traversal protection**: Rejects paths containing `..` or absolute paths outside the base path
- **File size limits**: Configurable maximum file size (prevents memory exhaustion)
- **Remote URL blocking**: Only local files are allowed; remote URLs return an error

```rust
// Path traversal is blocked
let result = TerminalImage::new(Path::new("../../../etc/passwd"));
assert!(matches!(result, Err(TerminalImageError::PathTraversalBlocked { .. })));

// Large files are rejected
let result = TerminalImage::new_with_max_size(Path::new("huge.png"), 1_000_000);
```

### Gotchas and notes
- If `cell_size` cannot be detected, default 8×16 is used; images may appear slightly off if the terminal font has a very different aspect. Provide a width in columns (e.g., `|80`) to get predictable sizing.
- Large images: we don't upscale the default 50% case; explicit widths can upscale.
- Unsupported terminals: you'll see the generated alt text instead of an image.

## Mermaid Diagrams (MermaidRenderer)

`MermaidRenderer` renders Mermaid diagrams to the terminal using the `mmdc` CLI tool.

### Basic Usage

```rust
use biscuit_terminal::components::mermaid::MermaidRenderer;

// Simple usage with default settings
let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");

match renderer.render_for_terminal() {
    Ok(()) => println!("Diagram rendered!"),
    Err(_) => println!("{}", renderer.fallback_code_block()),
}
```

### Terminal-Aware Rendering

For best results, use `for_terminal()` which automatically detects color mode:

```rust
use biscuit_terminal::components::mermaid::MermaidRenderer;

// Automatically uses appropriate theme and transparent background
let renderer = MermaidRenderer::for_terminal("flowchart LR\n    A --> B");
renderer.render_for_terminal()?;
```

### Theme and Rendering Options

```rust
use biscuit_terminal::components::mermaid::{MermaidRenderer, MermaidTheme};

let renderer = MermaidRenderer::new("flowchart LR\n    A --> B")
    .with_theme(MermaidTheme::Dark)        // dark, default, forest, neutral
    .with_scale(3)                          // Higher resolution (default: 2)
    .with_transparent_background(true);     // Blend with terminal background
```

### Themes

- **`MermaidTheme::Dark`**: Light text on dark background (default for dark terminals)
- **`MermaidTheme::Default`**: Dark text on light background (default for light terminals)
- **`MermaidTheme::Forest`**: Green tones
- **`MermaidTheme::Neutral`**: Grayscale, works well with transparent backgrounds

Use `MermaidTheme::for_color_mode()` to automatically select based on terminal:

```rust
use biscuit_terminal::components::mermaid::MermaidTheme;
use biscuit_terminal::terminal::Terminal;

let theme = MermaidTheme::for_color_mode(Terminal::color_mode());
let inverse_theme = theme.inverse();  // For solid background rendering
```

### CLI Detection

The module uses a fallback chain:
1. **Direct `mmdc`**: If in PATH, use directly
2. **npx fallback**: If `npx` is available, use `npx mmdc` with a warning
3. **Error**: If neither is available, return an error

### Icon Pack Support

Mermaid diagrams support icons via `--iconPacks`:
- `@iconify-json/fa7-brands` - Font Awesome 7 brand icons
- `@iconify-json/lucide` - Lucide icons
- `@iconify-json/carbon` - Carbon Design icons
- `@iconify-json/system-uicons` - System UI icons

Usage: `A[icon:fa7-brands:github]`

### Security Features

- **Size limit**: Diagrams over 10KB are rejected (prevents CLI abuse)
- **Terminal check**: Only renders when image protocols are supported

## Terminal Detection

The library detects these terminal emulators:

| Terminal | Image Support | OSC8 Links | Italics |
|----------|--------------|------------|---------|
| WezTerm | Kitty | Yes | Yes |
| Kitty | Kitty | Yes | Yes |
| iTerm2 | Kitty | Yes | Yes |
| Ghostty | Kitty | Yes | Yes |
| Alacritty | None | Yes | Yes |
| Apple Terminal | None | No | Yes |
| GNOME Terminal | None | Yes | Yes |
| Konsole | Kitty | Yes | Yes |
| Foot | None | Yes | Yes |
| Contour | None | Yes | Yes |
| VS Code | None | Yes | Yes |
| Warp | Kitty | Yes | Yes |

## OS Detection

```rust
use biscuit_terminal::discovery::os_detection::{detect_os_type, detect_linux_distro, OsType};

let os = detect_os_type();
match os {
    OsType::Linux => {
        if let Some(distro) = detect_linux_distro() {
            println!("Running on {} ({})", distro.name, distro.family);
        }
    }
    OsType::MacOS => println!("Running on macOS"),
    OsType::Windows => println!("Running on Windows"),
    _ => println!("Running on {:?}", os),
}
```

## Font Detection

Font detection works by parsing terminal configuration files:

| Terminal | Config Format | Font Setting | Size Setting |
|----------|--------------|--------------|--------------|
| WezTerm | Lua | `config.font = wezterm.font("Name")` | `config.font_size = N` |
| Ghostty | Key=Value | `font-family = Name` | `font-size = N` |
| Kitty | Conf | `font_family Name` | `font_size N` |
| Alacritty | TOML | `[font.normal] family = "Name"` | `[font] size = N` |

```rust
use biscuit_terminal::discovery::fonts::{font_name, font_size, ligature_support_likely};

if let Some(name) = font_name() {
    println!("Font: {}", name);
}
if let Some(size) = font_size() {
    println!("Size: {}pt", size);
}
if ligature_support_likely() {
    println!("Ligatures likely supported");
}
```

The `Terminal` struct also exposes font fields:

```rust
use biscuit_terminal::terminal::Terminal;

let term = Terminal::new();
if let Some(font) = &term.font {
    println!("Using font: {}", font);
}
if let Some(size) = term.font_size {
    println!("Font size: {}pt", size);
}
```

## Escape Code Analysis

```rust
use biscuit_terminal::discovery::eval::{line_widths, has_escape_codes};

// Calculate visual width (escape codes don't count)
assert_eq!(line_widths("\x1b[31mred\x1b[0m"), vec![3]);

// Detect escape codes
assert!(has_escape_codes("\x1b[1mBold\x1b[0m"));
assert!(!has_escape_codes("plain text"));
```

## Clipboard (OSC52)

```rust
use biscuit_terminal::discovery::clipboard::{osc52_support, set_clipboard};

if osc52_support() {
    set_clipboard("Hello from terminal!").ok();
}
```

## Examples

Run the examples to see the library in action:

```bash
# Show terminal information
cargo run -p biscuit-terminal --example terminal_info

# Analyze escape codes
cargo run -p biscuit-terminal --example escape_analysis
```

## CLI

The package includes a `bt` CLI (in the `cli` crate):

```bash
# Show terminal metadata (default)
bt

# Output as JSON
bt --json
```

## License

AGPL-3.0
