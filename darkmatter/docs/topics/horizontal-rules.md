# Horizontal Rules

Horizontal rules in darkmatter provide a rich set of visual separators that can be customized with various styles, placements, weights, and colors.

## Markdown Syntax

Horizontal rules are created using the standard markdown horizontal rule syntax (`---`, `___`, or `***`) followed by an attribute block in curly braces:

```markdown
--- { style: waves, width: "50%" }
___ { style: dots, placement: centered, weight: thick }
*** { style: line-star, color: "#ff0000" }
```

## Available Options

### Style

The `style` attribute controls the visual appearance of the horizontal rule. Available styles:

- `dashes` - Simple dashed line (default)
- `dots` - Series of dots
- `waves` - Wavy line
- `line-star` - Line with star symbols
- `line-circle` - Line with circle symbols  
- `inset-line` - Inset line effect
- `curtain-rod` - Curtain rod style

### Placement

The `placement` attribute controls where the rule appears horizontally:

- `full` - Spans the full width (default)
- `centered` - Centered with equal margins
- `left` - Left-aligned with right margin
- `right` - Right-aligned with left margin

### Weight

The `weight` attribute controls the thickness of the rule:

- `thin` - Thin line
- `medium` - Medium thickness (default)
- `thick` - Thick line

### Width

The `width` attribute allows you to specify a custom width as a percentage or CSS length:

```markdown
--- { width: "75%" }
--- { width: "200px" }
```

### Color

The `color` attribute allows you to specify a custom color using CSS color values:

```markdown
--- { color: "#ff0000" }
--- { color: "rgb(255, 0, 0)" }
--- { color: "red" }
```

## Examples

### Basic Usage

```markdown
---
```

Renders as a simple dashed line spanning the full width.

### Styled Rules

```markdown
--- { style: waves }
___ { style: dots, weight: thick }
*** { style: line-star, placement: centered }
```

### Custom Width and Color

```markdown
--- { style: inset-line, width: "60%", color: "#007acc" }
```

## Terminal vs Browser Rendering

Horizontal rules are rendered differently depending on the output target:

- **Terminal**: Two-tier progressive enhancement today — Unicode characters when the locale signals UTF-8, or ASCII fallback characters otherwise. (A planned third tier that rasterizes SVGs to inline PNGs via `resvg` + `TerminalImage` is **deferred** in the initial release.)
- **Browser**: Renders as SVG with `stroke="var(--hr-color, currentColor)"` and declares `--hr-weight`, `--hr-color`, `--hr-width` CSS custom properties on the root `<svg>` for per-instance overrides.

The same markdown syntax works in both contexts, with appropriate fallbacks for terminal environments that don't support advanced graphics.

## Attribute Honoring by Target

| Attribute   | Terminal                                                | Browser                                  |
|-------------|---------------------------------------------------------|------------------------------------------|
| `style`     | Picks the Unicode / ASCII character pattern             | Picks the SVG shape primitive            |
| `placement` | Centers / aligns the rendered string                    | `margin` attribute on the `<svg>` root   |
| `weight`    | Heavy vs light Unicode glyphs (no-op in ASCII/waves)    | `stroke-width` via `--hr-weight`         |
| `width`     | Clamped to the terminal's column width                  | `width` attribute + `--hr-width` CSS var |
| `color`     | ANSI escape wrap (when `color_depth != None`)           | `stroke` via `--hr-color`                |

## Attribute Validation

darkmatter validates the three enumerated attributes (`style`, `placement`, `weight`) against their allowed sets at parse time:

- **Unknown values** fall back to the component default silently in the rendered output but emit a `tracing::warn!` diagnostic. Run with `RUST_LOG=darkmatter=warn` (or `info`) to see them.
- **Unknown attribute keys** (e.g., `--- { margin: 4 }`) are ignored and similarly warned.

This "warn + continue" contract keeps documents renderable when someone mistypes (`dashse` instead of `dashes`) while still surfacing the typo to anyone actively watching the log stream.