# Color System

A layered color system through `utils::color`, supporting 16-color ANSI to full Tailwind CSS palettes. All color types implement the `TermColor` trait for foreground (`fg`) and background (`bg`) rendering.

## Color Types

| Type | Description | Escape Encoding |
|------|-------------|-----------------|
| `BasicColor` | 16 standard ANSI colors (8 normal + 8 bright) | `\x1b[31m` ... `\x1b[97m` |
| `RgbColor` | Arbitrary 24-bit RGB with a `BasicColor` fallback | `\x1b[38;2;r;g;bm` |
| `HdrColor` | RGB + OKLCH perceptual values (lightness, chroma, hue) | `\x1b[38;2;r;g;bm` |
| `WebColor` | 148 CSS named colors (e.g., `Coral`, `MidnightBlue`) | 24-bit RGB via lookup table |
| `Tailwind` | Full Tailwind CSS v4 palette (22 families x 11 shades + specials) | 24-bit RGB via generated `HdrColor` |

The unified `Color` enum wraps all of the above plus `DefaultForeground`, `DefaultBackground`, and `Reset`.

## BasicColor

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

## RgbColor

True 24-bit color with an automatic fallback for terminals lacking truecolor:

```rust
use biscuit_terminal::utils::color::{BasicColor, RgbColor, TermColor};

let brand_color = RgbColor::new(99, 102, 241, BasicColor::Blue);
let styled = brand_color.fg("Indigo text");
```

When rendered through `RenderableWrapper`, color depth is detected automatically:

- **TrueColor** terminals: 24-bit `\x1b[38;2;r;g;bm`
- **Enhanced** (256-color) terminals: nearest 6x6x6 cube index `\x1b[38;5;nm`
- **Basic** terminals: the `BasicColor` fallback

## WebColor

All 148 CSS Color Module Level 4 named colors, each backed by an `RgbColor` lookup:

```rust
use biscuit_terminal::utils::color::{WebColor, TermColor};

let coral = WebColor::Coral.fg("warm text");
let navy = WebColor::Navy.bg("dark background");
```

## Tailwind

The complete Tailwind CSS v4 palette - 22 color families (Red, Orange, Amber, Yellow, Lime, Green, Emerald, Teal, Cyan, Sky, Blue, Indigo, Violet, Purple, Fuchsia, Pink, Rose, Slate, Gray, Zinc, Neutral, Stone), each with shades 50-950, plus `Black`, `White`, `Inherit`, `Current`, and `Transparent`:

```rust
use biscuit_terminal::utils::color::{Tailwind, Color};

let primary = Tailwind::Blue500;
let bg = Tailwind::Slate50;

// Convert to RGB
let color = Color::Tailwind(Tailwind::Emerald600);
if let Some((r, g, b)) = color.to_rgb() {
    println!("RGB: ({r}, {g}, {b})");
}

// Access hex and CSS values
assert_eq!(Tailwind::Red500.hex(), Some("#ef4444"));
assert_eq!(Tailwind::Transparent.css_var(), "transparent");
```

Each `Tailwind` variant stores an `HdrColor` with both RGB and OKLCH (perceptual lightness, chroma, hue) values, suitable for accessible contrast calculations.

### Shade Guide

| Range | Usage |
|-------|-------|
| 50-200 | Light backgrounds, subtle highlights |
| 300-500 | Primary interactive elements |
| 600-700 | Active states, emphasis |
| 800-950 | Dark backgrounds, heavy text |

## Color Enum

The unified `Color` enum for use across all components:

```rust
use biscuit_terminal::utils::color::Color;

// Construct from any color type
let c1 = Color::BasicColor(BasicColor::Red);
let c2 = Color::Web(WebColor::Coral);
let c3 = Color::Tailwind(Tailwind::Blue500);
let c4 = Color::Rgb(RgbColor::new(255, 128, 0, BasicColor::Yellow));

// Convert any color to RGB tuple
if let Some((r, g, b)) = c2.to_rgb() {
    println!("RGB: ({r}, {g}, {b})");
}
```

## Cargo Feature: `clap`

Enable the `clap` feature to derive `clap::ValueEnum` on color-related enums for CLI integration with shell completions:

```toml
[dependencies]
biscuit-terminal = { version = "0.1", features = ["clap"] }
```

## Related

- [Styling](./styling.md) - Using colors in Prose tokens and manual escape codes
- [Terminal Struct](./terminal-struct.md) - `ColorDepth` detection for capability-aware rendering
