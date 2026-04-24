> **Status (Review 2, 2026-04-23):** The current implementation does not match the intended authoring model. This spec makes `alignment` the positioning terminology, restores standard Markdown horizontal-rule syntax as the primary path, and makes page-level `hr` frontmatter the default styling mechanism. Terminal image rendering is no longer considered optional for capable terminals: when the terminal reports Kitty-compatible image support, Darkmatter should use the SVG/PNG image path before falling back to Unicode or ASCII.

In Markdown the "horizontal rule" (e.g., a horizontal line used to more visibly separate sections in a document) is represented by any of the following when it is on its own line:

- `---` (three dashes)
- `___` (three underscores)
- `***` (three asterisks)

The three syntaxes are equivalent and just a matter of preference for the author. Markdown authors should be able to write a plain, valid CommonMark horizontal rule and have Darkmatter apply page-level styling without requiring a non-standard suffix.

## Styling Model

Darkmatter supports two layers of styling:

1. **Page defaults from frontmatter** using the top-level `hr` property.
2. **Per-rule overrides** using the existing attribute-block syntax after an HR marker.

Page-level frontmatter is the preferred way to style every horizontal rule in a page while preserving standard Markdown source:

```yaml
---
hr:
  width: 50%
  alignment: centered
  weight: medium
  style: waves
---
```

With that frontmatter, each of the following remains valid Markdown and renders with the configured defaults:

```markdown
---

___

***
```

The attribute-block form remains supported for cases where an individual rule needs to override the page default:

```markdown
--- { style: dots, width: "75%" }
___ { alignment: left, weight: thick }
*** { style: line-star, color: "#ff0000" }
```

Per-rule attributes override only the keys they specify. Missing keys inherit from `hr` frontmatter. Missing frontmatter keys fall back to component defaults.

### Precedence

For each horizontal-rule option, Darkmatter resolves the effective value in this order:

1. Per-rule attribute block.
2. Page frontmatter `hr` configuration.
3. `HorizontalRule` component default.

This lets an author set a consistent page style without making the Markdown source non-standard, while still allowing occasional exceptions.

## Supported Options

Darkmatter offers rendering options based on the following dimensions:

- `alignment` (`full`, `centered`, `left`, `right`)
- `width` (percentage or CSS-like length string)
- `weight` (`thin`, `medium`, `thick`)
- `style` (`dashes`, `dots`, `waves`, `line-star`, `line-circle`, `inset-line`, `curtain-rod`)
- `color` (CSS color string for browser output and ANSI color mapping where supported)

### Alignment

`alignment` describes where the horizontal rule appears horizontally.

1. **Full**

   The most common rendering spans the full available width, respecting margins and padding.

2. **Centered**

   A fixed-width rule is centered within the available width.

3. **Left** and **Right**

   A fixed-width rule is aligned to the left or right edge of the available width.

### Width

`width` controls the length of the horizontal rule.

- In terminal output, percentages resolve against the current terminal column width.
- In browser output, percentages and CSS lengths are emitted into the SVG/CSS representation.
- Invalid or unsupported widths should fall back to the component default and emit a diagnostic warning.

### Weight

`weight` controls stroke thickness for browser/image rendering and selects the closest available glyph pattern for Unicode rendering.

- `thin`
- `medium`
- `thick`

ASCII fallback may not be able to represent weight faithfully; it should prioritize a stable readable separator.

### Visual Styles

Some visual styles can be represented acceptably with text glyphs. Others require SVG/image rendering to preserve the intended design.

Supported styles:

- `dashes`
- `dots`
- `waves`
- `line-star`
- `line-circle`
- `inset-line`
- `curtain-rod`

## Terminal Rendering

Terminal output uses progressive enhancement:

1. **Tier 1: Image rendering**

   When the terminal capability detector reports Kitty-compatible image support, render the selected HR style as SVG, rasterize to PNG with `resvg`, and display it through `TerminalImage`. WezTerm is expected to use this path when configured/detected as supporting the relevant image protocol.

2. **Tier 2: Unicode rendering**

   When image rendering is unavailable but the locale supports UTF-8, render with Unicode glyphs.

3. **Tier 3: ASCII rendering**

   When neither image nor Unicode rendering is appropriate, render a plain ASCII fallback.

The image path is important for styles like `waves`, `line-star`, `line-circle`, `inset-line`, and `curtain-rod`, where text approximations are visibly inferior.

## Browser Rendering

Browser output should render as SVG and preserve the same effective configuration used by terminal rendering.

- SVG uses `currentColor` or an explicit configured color.
- Width, weight, color, and style-specific parameters should be represented with CSS variables where useful.
- Per-rule attributes should become per-instance overrides.
- Page-level `hr` frontmatter should define defaults for the page.

## SVG Implementation

- SVGs use `currentColor` for stroke/fill color by default.
- Explicit `color` overrides should map to target-appropriate color handling.
- CSS variables should provide useful scaling without warping the intended style.

### Example

```svg
<svg xmlns="http://www.w3.org/2000/svg" height="40" style="width: var(--hr-width, 100%);">
  <line x1="0" y1="50%" x2="100%" y2="50%" stroke="currentColor" style="stroke-width: var(--hr-line-weight, 6px);" />
  <circle cx="50%" cy="50%" fill="currentColor" style="r: var(--hr-circle-radius, 16px);" />
</svg>
```

This example is the SVG shape for the `line-circle` visual style.

## Proper Abstraction

While Darkmatter is a significant consumer of horizontal-rule functionality, the rendering component should remain reusable by other terminal applications. The renderable component lives in `biscuit-terminal` as `HorizontalRule`.

- Darkmatter parses Markdown/frontmatter and resolves the effective HR configuration.
- `biscuit-terminal` owns terminal capability handling and HR rendering.
- Browser rendering support can remain available through `BrowserRenderable` so the same component can generate SVG for HTML output.

### BrowserRenderable Trait

The `biscuit-terminal` library already defines the `Renderable` trait for terminal rendering. The HR component also needs browser rendering:

```rust
pub trait BrowserRenderable {
    /// Renders the component for a browser based target (for example HTML/SVG).
    fn render_to_browser(&self) -> String;

    /// Renders the component for a browser based target with instance-specific
    /// CSS variable overrides.
    fn render_to_browser_with_inline_variables(&self, variables: CssVariables) -> String;
}
```

## Parsing Requirements

Darkmatter must preserve standard Markdown behavior:

- Bare `---`, `___`, and `***` are parsed as normal horizontal-rule blocks.
- Page-level `hr` frontmatter styles those blocks without changing the Markdown source.
- Attribute-block HR syntax is non-standard but supported as a Darkmatter extension.
- `alignment` is the canonical key.
- Unknown HR keys should be treated like any other unknown key.

## Validation Requirements

Darkmatter should validate both frontmatter and per-rule attributes.

- Unknown `style`, `alignment`, or `weight` values fall back to defaults and emit `tracing::warn!`.
- Unknown HR keys are ignored and emit `tracing::warn!`.
- Invalid width/color values should warn and fall back where validation is practical.

## Documentation

Once this corrected behavior has been implemented, update the author-facing and agent-facing docs:

- `darkmatter/docs/topics/horizontal-rules.md`
- `biscuit-terminal/docs/components/horizontal-rule.md`
- `biscuit-terminal/docs/components/browser-renderable-trait.md`
- `.claude/skills/darkmatter/SKILL.md`
- `.claude/skills/biscuit-terminal/SKILL.md`
