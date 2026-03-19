# biscuit-terminal

Terminal capability detection, rendering utilities, and image/diagram display for Rust applications.

## Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `clap` | No | Derive `clap::ValueEnum` on enums for CLI integration with shell completions |

> **Note:** Image rendering via [viuer](https://crates.io/crates/viuer) is always available (unconditional dependency).

### When to use the `clap` feature

Enable the `clap` feature if you're building a CLI application that uses types like `QuadrantTheme` as command-line arguments:

```toml
[dependencies]
biscuit-terminal = { version = "0.1", features = ["clap"] }
```

This derives `clap::ValueEnum` on supported enums, enabling:

- Shell tab-completion for enum values (e.g., `--theme <TAB>` shows `default`, `magic-quadrangle`)
- Automatic `--help` text listing valid values
- Direct use in clap argument definitions with `#[arg(value_enum)]`

**Don't enable this feature** if you're using the library programmatically without clap - it adds an unnecessary dependency.

## Capabilities

- **Terminal App Detection**: Recognize 13+ terminal emulators with capability profiles
- **Image Rendering**: Inline images via Kitty/iTerm2 protocols with security guards
- **Mermaid Diagrams**: Adapter for `biscuit-visualized` Mermaid rendering (pure Rust, no external dependencies)
- **Graph Visualization**: Adapter for `biscuit-visualized` graph rendering with multiple syntaxes
- **OS Detection**: Identify operating system and Linux distribution
- **Repo Detection**: Detect git repo root and monorepo status via `sniff`
- **Font Detection**: Extract font name and size from terminal config files
- **Color Support**: Query color depth, mode (light/dark), and background color; render with BasicColor, RGB, 148 CSS WebColors, or full Tailwind palettes
- **Escape Code Analysis**: Calculate visual line widths, detect escape codes
- **Clipboard**: OSC52 clipboard support for compatible terminals
- **Styled Output**: Composable rendering components (Prose, Table, List, Section, FileSystem, TwoColumn, and more)

## Quick Start

```rust
use biscuit_terminal::terminal::Terminal;

fn main() {
    let term = Terminal::new();

    println!("Running in {:?}", term.app);
    println!("Terminal size: {}x{}", term.width(), term.height());

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
- `discovery::cursor_position` - Cursor position queries
- `discovery::locale` - Locale detection
- `discovery::eval` - Escape code analysis utilities
- `components::terminal_image` - Terminal image rendering (Kitty/iTerm2 with fallbacks)
- `components::mermaid` - Mermaid diagram adapter (delegates to biscuit-visualized)
- `components::graph_expression` - Graph visualization adapter (delegates to biscuit-visualized)
- `components::prose` - Styled prose rendering
- `components::table` - Table rendering
- `components::list` - List rendering
- `components::block_quote` - Block quote rendering
- `components::compose` - Component composition utilities
- `components::filesystem` - File/directory tree rendering
- `components::image_options` - Image rendering configuration
- `components::inline_content` - Inline concatenation of items without newlines
- `components::progress` - Progress indicator rendering
- `components::renderable` - Renderable trait for components
- `components::section` - Section rendering
- `components::text_block` - Text block rendering
- `components::todo` - Todo item rendering
- `components::two_column` - Two-column layout rendering
- `utils::color` - Color types (BasicColor, RgbColor, HdrColor, WebColor, Tailwind)
- `utils::styling` - Stylist trait, FontWeight, Style
- `utils::layout` - Layout, Margin, WordWrap, Alignment
- `utils::escape_codes` - ANSI escape code generation
- `utils::block_constraint` - Visual width and line splitting
- `utils::word_wrap` - Word wrapping strategies
- `utils::text` - Content length calculation (escape-aware)
- `utils::multiplex` - Multiplexing detection

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

- Default behavior uses `c=` (columns only) so Kitty preserves aspect ratio.
- WezTerm requires `c=` + `r=` (both columns and rows) for correct aspect ratio; Kitty/Ghostty use `c=` only.
- `render_to_terminal()` handles cursor positioning per terminal: Kitty/Ghostty auto-advance and overshoot by 1 row (corrected with CUU), Wezterm doesn't auto-advance (explicit CUD + CR applied).
- Usually does not append a trailing newline; it may append one line feed when bottom-of-screen scroll compensation is needed.

### iTerm2 specifics

- Forces iTerm path when `TERM_PROGRAM=iTerm.app`, even if Kitty is advertised.
- Uses `inline=1;preserveAspectRatio=1;width=<user spec>;size=auto`.
- `render_to_terminal()` sends the original image without pre-resizing, letting iTerm2 handle scaling natively for better accuracy. Corrects auto-advance overshoot with CUU(1).

### Inline layout notes (TwoColumn)

`TwoColumn` can render `TerminalImage` inline next to text. Inline images are not normal text: they occupy rows without printable cells, so the layout uses an overlay strategy:

- The image column is emitted as a single escape sequence, then the text column is drawn with a cursor offset.
- Terminals disagree on cursor save/restore semantics. The layout applies terminal-specific cursor resets to keep the right column aligned to the top of the image.
- WezTerm, Ghostty, Kitty, and iTerm2 use tailored cursor moves; other terminals (including Warp) use the standard save/restore fallback path.

### Path and Input Validation

`TerminalImage::new()` validates that the input path exists and can be canonicalized to a local file path.
Width strings are validated by `parse_width_spec()` (`50%`, `80`, `80ch`, `fill`).

For policy-level controls in your app (base-path checks, max file size, remote URL allowance), use `TerminalImageOptions`:

```rust
use biscuit_terminal::components::image_options::TerminalImageOptions;

let options = TerminalImageOptions::builder()
    .max_file_size(5 * 1024 * 1024)
    .allow_remote(false)
    .build();

assert!(options.is_size_allowed(1_024));
```

### Image rendering architecture

All image rendering is string-based: `render()`, `render_optimistic()`, and `render_to_terminal()` return escape sequences as strings. `render_to_terminal()` applies terminal-aware cursor management by saving/restoring cursor position, explicitly advancing rows by computed image height, and normalizing to column 0 with a trailing `\r`. It usually avoids trailing newlines, but may append one line feed when bottom-of-screen scroll compensation is needed. The Kitty graphics protocol is bidirectional — terminals respond with `\x1b_Gi=<id>;OK\x1b\\` after receiving image data. To prevent this response from appearing as garbage text, all Kitty sequences include `q=2` (quiet mode) which suppresses terminal responses entirely.

### Gotchas and notes

- If `cell_size` cannot be detected, default 8×16 is used; images may appear slightly off if the terminal font has a very different aspect. Provide a width in columns (e.g., `|80`) to get predictable sizing.
- Large images: we don't upscale the default 50% case; explicit widths can upscale.
- Unsupported terminals: you'll see the generated alt text instead of an image.

## Mermaid Diagrams (MermaidRenderer)

`MermaidRenderer` provides a terminal-aware adapter for rendering Mermaid diagrams via `biscuit-visualized`. The rendering uses pure Rust (`mermaid-rs-renderer`) with no external dependencies like Node.js or mmdc. Rendered diagrams are cached using content-addressed xxHash keys.

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

### Quadrant Chart Configuration

Use `MermaidConfig` to customize quadrant chart styling:

```rust
use biscuit_terminal::components::mermaid::{MermaidRenderer, MermaidConfig};

let config = MermaidConfig::new()
    .with_point_label_font_size(18)    // Font size for point labels (default: 12)
    .with_point_radius(10)             // Default point radius (default: 5)
    .with_quadrant_fill(1, "#1e2a1e")  // Top-right (quadrant-1) fill color
    .with_quadrant_fill(3, "#2a1e1e"); // Bottom-left (quadrant-3) fill color

let renderer = MermaidRenderer::new("quadrantChart\n    Item: [0.5, 0.5]")
    .with_config(config);
```

**Quadrant numbering** (matches Mermaid convention):

```
        +-------------+-------------+
        |  quadrant-2 |  quadrant-1 |
        |  (top-left) | (top-right) |
        +-------------+-------------+
        |  quadrant-3 |  quadrant-4 |
        |(bottom-left)|(bottom-right)|
        +-------------+-------------+
```

### Quadrant Themes

Use preset themes for common quadrant chart styles:

```rust
use biscuit_terminal::components::mermaid::{MermaidConfig, QuadrantTheme};
use biscuit_terminal::terminal::Terminal;

// Magic Quadrangle style: subtle green top-right, red bottom-left
// Colors automatically adapt to terminal light/dark mode
let color_mode = Terminal::color_mode();
let config = QuadrantTheme::MagicQuadrangle.apply(MermaidConfig::new(), color_mode);

// Parse theme from string
let theme = QuadrantTheme::parse("magic-quadrangle").unwrap();
```

**Available themes:**

| Theme | Description |
|-------|-------------|
| `QuadrantTheme::Default` | Standard Mermaid colors (no customization) |
| `QuadrantTheme::MagicQuadrangle` | Gartner-style: subtle green top-right (leaders), subtle red bottom-left (niche). Top-left and bottom-right use a neutral color. Colors adapt to terminal color mode. |

**Magic Quadrangle colors by terminal mode:**

| Mode | Top-right (q1) | Top-left (q2) | Bottom-left (q3) | Bottom-right (q4) |
|------|----------------|---------------|------------------|-------------------|
| Dark | `#1e2a1e` (green tint) | `#1a1a1a` (neutral) | `#2a1e1e` (red tint) | `#1a1a1a` (neutral) |
| Light | `#f6faf6` (green tint) | `#f8f8f8` (neutral) | `#faf6f6` (red tint) | `#f8f8f8` (neutral) |

The neutral quadrants (q2, q4) use the same color - a dark grey in dark mode, light grey in light mode - creating visual balance while highlighting the key diagonal (top-right leaders vs bottom-left niche).

### Inline Point Styling

Individual points can override defaults using comma-separated properties:

```
Item A: [0.3, 0.6] color: #ff3300, radius: 12
Item B: [0.7, 0.4] color: #00ff00
```

**Important:** Multiple properties must be comma-separated. Space-only separation causes parsing errors (e.g., `color: #ff3300 radius: 10` fails).

Available properties: `color`, `radius`, `stroke-color`, `stroke-width`

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

### Security Features

- **Size limit**: Diagrams over 10KB are rejected
- **Terminal check**: Only renders when image protocols are supported

### Display Notes

**Aspect Ratio Preservation**: `render_to_terminal()` preserves aspect ratio using terminal-aware rendering:

- **Kitty/Ghostty**: Specifies only `c=` (columns), letting the terminal calculate rows from the image's native aspect ratio.
- **Wezterm**: Specifies both `c=` and `r=` (columns and rows) since Wezterm requires explicit row count for correct proportions. Height is calculated from `cell_size()` (falls back to 8×16 px).
- **iTerm2**: Sends the original image with `preserveAspectRatio=1`, letting iTerm2 handle scaling natively.

This ensures correct proportions in all supported terminals.

```rust
use biscuit_terminal::components::terminal_image::{TerminalImage, ImageWidth};
use biscuit_terminal::terminal::Terminal;

// Default width is 50% of terminal
let term_image = TerminalImage::new(&png_path)?;

// Or specify a width - aspect ratio is always preserved
let term_image = TerminalImage::new(&png_path)?
    .with_width(ImageWidth::Percent(0.5));  // 50% of terminal width

let term_image = TerminalImage::new(&png_path)?
    .with_width(ImageWidth::Characters(80));  // 80 columns wide

let term_image = TerminalImage::new(&png_path)?
    .with_width(ImageWidth::Fill);  // Full terminal width

// Render to terminal
let terminal = Terminal::new();
term_image.render_to_terminal(&terminal)?;
```

### Width Specification Parsing

Use `parse_width_spec` to parse user-provided width strings:

```rust
use biscuit_terminal::components::terminal_image::{parse_width_spec, ImageWidth};

// Supported formats:
parse_width_spec("50%");   // ImageWidth::Percent(0.5)
parse_width_spec("80ch");  // ImageWidth::Characters(80)
parse_width_spec("80");    // ImageWidth::Characters(80)
parse_width_spec("fill");  // ImageWidth::Fill
```

The `ch` suffix provides explicit character-based sizing, useful for CLI tools accepting width from users.

## Terminal Detection

The library detects these terminal emulators:

| Terminal | Image Support | OSC8 Links | Italics |
|----------|--------------|------------|---------|
| WezTerm | Kitty | Yes | Yes |
| Kitty | Kitty | Yes | Yes |
| iTerm2 | ITerm | Yes | Yes |
| Ghostty | Kitty | Yes | Yes |
| Alacritty | None | Yes | Yes |
| Apple Terminal | None | No | Yes |
| GNOME Terminal | None | Yes | Yes |
| Konsole | Kitty | Yes | Yes |
| Foot | None | Yes | Yes |
| Contour | None | Yes | Yes |
| VS Code | Kitty* | Yes | Yes |
| Warp | Kitty | Yes | Yes |
| Wast | Kitty | No | No |

\* Requires `terminal.integrated.enableImages` and GPU acceleration enabled in VS Code settings.

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

// Kitty graphics protocol escape sequences are treated as zero-width
assert_eq!(line_widths("\x1b_Gf=100,a=T,t=d,c=10,m=0;AAAA\x1b\\text"), vec![4]);

// Detect escape codes
assert!(has_escape_codes("\x1b[1mBold\x1b[0m"));
assert!(!has_escape_codes("plain text"));
```

## Color System

The library provides a layered color system through the `utils::color` module, supporting everything from basic 16-color ANSI to full Tailwind CSS palettes. All color types implement the `TermColor` trait for foreground (`fg`) and background (`bg`) rendering.

### Color Types

| Type | Description | Escape Encoding |
|------|-------------|-----------------|
| `BasicColor` | 16 standard ANSI colors (8 normal + 8 bright) | `\x1b[31m` … `\x1b[97m` |
| `RgbColor` | Arbitrary 24-bit RGB with a `BasicColor` fallback | `\x1b[38;2;r;g;bm` |
| `HdrColor` | RGB + OKLCH perceptual values (lightness, chroma, hue) | `\x1b[38;2;r;g;bm` |
| `WebColor` | 148 CSS named colors (e.g., `Coral`, `MidnightBlue`) | 24-bit RGB via lookup table |
| `Tailwind` | Full Tailwind CSS v4 palette (22 families × 11 shades + specials) | 24-bit RGB via generated `HdrColor` |

The unified `Color` enum wraps all of the above plus `DefaultForeground`, `DefaultBackground`, and `Reset`.

### BasicColor

The 16 standard ANSI colors supported by virtually all terminals:

```rust
use biscuit_terminal::utils::color::{BasicColor, TermColor};

// Foreground
let red_text = BasicColor::Red.fg("error");
// Background
let highlighted = BasicColor::Yellow.bg("warning");
// Bright variants for higher contrast
let bright = BasicColor::BrightGreen.fg("success");
```

### RgbColor

True 24-bit color with an automatic fallback for terminals that lack truecolor support:

```rust
use biscuit_terminal::utils::color::{BasicColor, RgbColor, TermColor};

let brand_color = RgbColor::new(99, 102, 241, BasicColor::Blue);
let styled = brand_color.fg("Indigo text");
```

When rendered through the `RenderableWrapper` wrappers (used by components), color depth is detected automatically:

- **TrueColor** terminals → 24-bit `\x1b[38;2;r;g;bm`
- **Enhanced** (256-color) terminals → nearest 6×6×6 cube index `\x1b[38;5;nm`
- **Basic** terminals → the `BasicColor` fallback

### WebColor

All 148 CSS Color Module Level 4 named colors, each backed by an `RgbColor` lookup:

```rust
use biscuit_terminal::utils::color::{WebColor, TermColor};

let coral = WebColor::Coral.fg("warm text");
let navy = WebColor::Navy.bg("dark background");
```

### Tailwind

The complete Tailwind CSS v4 palette — 22 color families (Red, Orange, Amber, Yellow, Lime, Green, Emerald, Teal, Cyan, Sky, Blue, Indigo, Violet, Purple, Fuchsia, Pink, Rose, Slate, Gray, Zinc, Neutral, Stone), each with shades from 50 (lightest) to 950 (darkest), plus `Black`, `White`, `Inherit`, `Current`, and `Transparent`:

```rust
use biscuit_terminal::utils::color::{Tailwind, Color};

let primary = Tailwind::Blue500;
let bg = Tailwind::Slate50;

// Convert to RGB
let color = Color::Tailwind(Tailwind::Emerald600);
if let Some((r, g, b)) = color.to_rgb() {
    println!("RGB: ({r}, {g}, {b})");
}

// Access hex and CSS values (generated from Tailwind v4 data)
assert_eq!(Tailwind::Red500.hex(), Some("#ef4444"));
assert_eq!(Tailwind::Transparent.css_var(), "transparent");
```

Each `Tailwind` variant stores an `HdrColor` with both RGB and OKLCH (perceptual lightness, chroma, hue) values, making it suitable for accessible contrast calculations.

### Shade Guide

| Range | Usage |
|-------|-------|
| 50–200 | Light backgrounds, subtle highlights |
| 300–500 | Primary interactive elements |
| 600–700 | Active states, emphasis |
| 800–950 | Dark backgrounds, heavy text |

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

# Render an inline image
bt image photo.png

# Render a flowchart
bt flowchart "A --> B --> C"

# Render a quadrant chart
bt quadrant --title "Priority Matrix" \
            --x-axis "Low Effort --> High Effort" \
            --y-axis "Low Impact --> High Impact" \
            "Task A: [0.2, 0.8]" "Task B: [0.7, 0.3]"

# Render a quadrant chart with magic-quadrangle theme
bt quadrant --theme magic-quadrangle \
            --title "Market Position" \
            "Leaders: [0.8, 0.85]" "Niche: [0.25, 0.2]"

# Render a git graph
bt git-graph "commit" "branch feature" "commit" "checkout main" "merge feature"
```

## License

AGPL-3.0
