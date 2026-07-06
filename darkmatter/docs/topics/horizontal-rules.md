# Horizontal Rules

Horizontal rules in darkmatter use standard Markdown markers (`---`, `___`, or `***`) and can be styled from `style.hr` page frontmatter or per-rule attributes.

## Markdown Syntax

Bare rules are valid CommonMark and inherit any page-level `style.hr` defaults:

```markdown
---
style:
  hr:
    kind: waves
    alignment: center
    weight: thick
    width: "50%"
    color: red
---

---
___
***
```

Per-rule overrides use a YAML flow-mapping attribute block:

```markdown
--- { kind: waves, width: "50%" }
___ { kind: dots, alignment: center, weight: thick }
*** { kind: line-star, color: "#ff0000" }
```

Per-rule attributes override only the keys they specify. The resolution order is:

1. Per-rule attribute block
2. Page frontmatter `style.hr`
3. `HorizontalRule` component defaults

## Available Options

- `kind`: `dashes` (default), `dots`, `waves`, `line-star`, `line-circle`, `inset-line`, `curtain-rod`
- `alignment`: `full` (default), `center`, `left`, `right`
- `weight`: `thin`, `medium` (default), `thick`
- `width`: CSS-like string such as `"75%"`, `"200px"`, or `"20"`
- `color`: CSS color name or `#rrggbb`

## Examples

```markdown
---
style:
  hr:
    kind: waves
    alignment: center
    width: "60%"
---

---

--- { color: "#007acc" }

*** { kind: line-star, alignment: center }
```

The first bare rule uses all frontmatter defaults. The second rule inherits style, alignment, and width, then overrides only color.

## Terminal vs Browser Rendering

- **Terminal**: image-first progressive enhancement. Terminals with Kitty-compatible image support receive a rasterized SVG via `resvg` and `TerminalImage`; failures fall back to Unicode, then ASCII when the locale does not signal UTF-8. When no explicit `color` is set, the image tier chooses a visible default based on the detected terminal color mode — `white` for dark terminals and `black` for light terminals — so the rule is never invisible against the terminal background.
- **Browser**: SVG with `stroke="var(--hr-color, currentColor)"` and root-level `--hr-weight`, `--hr-color`, and `--hr-width` custom properties.

## Attribute Honoring by Target

| Attribute   | Terminal                                                | Browser                                  |
|-------------|---------------------------------------------------------|------------------------------------------|
| `style`     | Image shape, Unicode pattern, or ASCII pattern          | SVG shape primitive                      |
| `alignment` | Positions the rendered rule within terminal columns     | `margin` behavior on the root `<svg>`    |
| `weight`    | Image height/stroke or heavy Unicode glyphs             | `stroke-width` via `--hr-weight`         |
| `width`     | Clamped to the terminal's column width                  | `width` attribute + `--hr-width` CSS var |
| `color`     | Image stroke/fill or ANSI escape wrap. When omitted, the terminal image tier defaults to `white` on dark terminals and `black` on light terminals. | `stroke` / `fill` via `--hr-color`       |

## Validation

darkmatter validates `style`, `alignment`, and `weight` through the same builder path for frontmatter defaults and per-rule overrides:

- Unknown enum values fall back to the component default and emit `tracing::warn!`.
- Unknown attribute keys are ignored and warned.

Run with `RUST_LOG=darkmatter=warn` to see diagnostics.
