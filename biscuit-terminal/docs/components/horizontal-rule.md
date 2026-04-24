# HorizontalRule Component

The `HorizontalRule` component provides customizable horizontal separator lines for terminal and browser rendering.

## Struct Definition

```rust
use biscuit_terminal::prelude::*;

let rule = HorizontalRule::new()
    .style(RuleStyle::Waves)
    .alignment(RuleAlignment::Centered)
    .weight(RuleWeight::Medium)
    .width("75%");

// Terminal rendering
let terminal = Terminal::default();
let output = rule.render(&terminal);

// Browser rendering
let svg = rule.render_to_browser();
```

## Enums

### RuleStyle

Defines the visual style of the horizontal rule:

```rust
pub enum RuleStyle {
    Dashes,
    Dots,
    Waves,
    LineStar,
    LineCircle,
    InsetLine,
    CurtainRod,
}
```

### RuleAlignment

Defines the horizontal alignment of the rule:

```rust
pub enum RuleAlignment {
    Full,
    Centered,
    Left,
    Right,
}
```

### RuleWeight

Defines the thickness/weight of the rule:

```rust
pub enum RuleWeight {
    Thin,
    Medium,
    Thick,
}
```

## Trait Implementations

### Renderable (Terminal Rendering)

The `HorizontalRule` implements the `Renderable` trait for terminal output with three-tier progressive enhancement:

1. **Tier 1**: SVG → PNG via `resvg` and `TerminalImage` when the terminal advertises Kitty-compatible image support.
2. **Tier 2**: Unicode fallback characters when image rendering is unavailable and the terminal's locale signals UTF-8.
3. **Tier 3**: ASCII fallback characters (basic compatibility).

### BrowserRenderable (Browser Rendering)

The `HorizontalRule` implements the `BrowserRenderable` trait for web output:

- Generates SVG with `stroke="var(--hr-color, currentColor)"` for proper CSS inheritance
- Declares `--hr-weight`, `--hr-color`, and `--hr-width` CSS custom properties on the root `<svg>`
- Supports all style, alignment, weight, width, and color attributes

## Usage Example

```rust
use biscuit_terminal::prelude::*;

let rule = HorizontalRule::new()
    .style(RuleStyle::Waves)
    .alignment(RuleAlignment::Centered)
    .weight(RuleWeight::Medium)
    .width("75%");

// Terminal rendering
let terminal = Terminal::default();
let _ = rule.render(&terminal);

// Browser rendering
let svg = rule.render_to_browser();
```

## Style Matrix

| Style       | SVG            | Unicode           | ASCII       |
|-------------|----------------|-------------------|-------------|
| Dashes      | dashed line    | `╌` / `╍` (thick) | `-`         |
| Dots        | dotted line    | `·` / `•` (thick) | `.`         |
| Waves       | wavy path      | `≋`               | `~`         |
| LineStar    | line + star    | `─★─` / `━★━`     | `---*---`   |
| LineCircle  | line + circle  | `─●─` / `━●━`     | `---o---`   |
| InsetLine   | centered line  | `  ─  ` / `  ━  ` | `  -  `     |
| CurtainRod  | line + ends    | `┤─┤─...─├`       | `[---]`     |

## Weight

`RuleWeight` maps to the terminal and browser as follows:

| Weight   | Terminal (Unicode)                  | Terminal (ASCII) | Browser (SVG stroke-width) |
|----------|-------------------------------------|------------------|----------------------------|
| `Thin`   | Single-line chars (`╌`, `·`, `─`)   | `-`, `.`, …      | `2`                        |
| `Medium` | Single-line chars (`╌`, `·`, `─`)   | `-`, `.`, …      | `4`                        |
| `Thick`  | Heavy variants (`╍`, `•`, `━`)      | `-`, `.`, …      | `8`                        |

### Unicode thick substitutions

- `╌` → `╍` (dashes)
- `·` → `•` (dots)
- `─` → `━` (line-*, inset-line, curtain-rod body)

### Limitations

- **Waves** has no heavy Unicode variant — `≋` is used for every weight. `weight=thick` is still honored in the browser (8px stroke) but produces the same terminal characters as `medium`.
- **ASCII** has no heavy variants — weight is a no-op in Tier 3.

## CSS Variables

`render_to_browser` declares three custom properties on the root `<svg>`:

| Variable      | Default source                                            |
|---------------|-----------------------------------------------------------|
| `--hr-weight` | numeric stroke width derived from `RuleWeight`            |
| `--hr-color`  | `self.color` if set, otherwise `currentColor`             |
| `--hr-width`  | `self.width` if set, otherwise `100%`                     |

Shape primitives reference these with `var(--hr-weight, <fallback>)` etc., so the SVG stays valid even if the inline style is stripped.

`render_to_browser_with_inline_variables` takes a `HashMap<String, String>` and substitutes each `var(--<key>)` token before returning the string — letting callers override stroke width, color, or width per-instance:

```rust
use std::collections::HashMap;
use biscuit_terminal::prelude::*;

let rule = HorizontalRule::new().style(RuleStyle::Dashes);
let mut overrides = HashMap::new();
overrides.insert("hr-weight".to_string(), "12".to_string());
let svg = rule.render_to_browser_with_inline_variables(&overrides);
// `var(--hr-weight, …)` occurrences are replaced with `12`.
```

## Color

`color` applies to both targets:

- **Browser**: sets the `--hr-color` CSS variable (and `stroke="var(--hr-color, currentColor)"`).
- **Terminal**: wraps the rendered content in an ANSI color escape **when** `term.color_depth != ColorDepth::None`. Supported forms:
  - CSS basic-16 names: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`, plus `bright_*` variants and `gray`/`grey` (alias for bright black).
  - `#rrggbb` hex — upgraded to 24-bit RGB on `ColorDepth::TrueColor` terminals; mapped to the nearest basic color otherwise.

Unrecognized color strings are ignored for terminal rendering (the raw string is preserved for the browser target) and emit a `tracing::warn!`.

## Tier 1 Image Rendering

The component ships Tier 1 rendering via `HorizontalRule::render_image_tier`, invoked from the `Renderable` impl before the Unicode / ASCII fallbacks.

- **Capability gate:** Tier 1 activates only when **both** `term.is_tty == true` **and** `term.image_support == ImageSupport::Kitty`. Non-TTY sinks and terminals without Kitty-compatible image support skip straight to Tier 2 / Tier 3.
- **Rasterization:** the same SVG produced for browser rendering is rasterized to a PNG via [`resvg`](https://crates.io/crates/resvg), then emitted through `TerminalImage::render_kitty_cells` as a Kitty graphics escape sequence.
- **Cell sizing:** rule dimensions are computed against `term.cell_size()` when available; if the terminal does not report a cell size, an 8x16 pixel default is assumed.
- **Fallback contract:** any rasterization failure logs a `tracing::warn!` and causes `render_image_tier` to return `None`, at which point `Renderable::render` proceeds to Tier 2 (Unicode) or Tier 3 (ASCII) using the same style / weight / alignment / color settings.
