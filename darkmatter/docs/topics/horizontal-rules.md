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

- **Terminal**: Uses progressive enhancement with SVG→PNG via resvg (when supported), Unicode fallback characters, or ASCII fallback characters
- **Browser**: Renders as SVG with `stroke="currentColor"` and CSS variables for proper theming and scaling

The same markdown syntax works in both contexts, with appropriate fallbacks for terminal environments that don't support advanced graphics.