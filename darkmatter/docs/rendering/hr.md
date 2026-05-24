# Horizontal Rules

Darkmatter styles CommonMark horizontal rules (`---`, `___`, `***`) with configurable visual styles, alignment, weight, width, and color. Rendering is delegated to biscuit-terminal's `HorizontalRule` component.

## Styling Model

Two layers of configuration control how every horizontal rule looks:

1. **Page defaults** via frontmatter `hr:` — the preferred mechanism for consistent styling across a document.
2. **Per-rule overrides** via an attribute block after the marker — for occasional exceptions.

Per-rule attributes override only the keys they specify. Missing keys inherit from frontmatter defaults. Missing frontmatter keys fall back to the `HorizontalRule` component default.

### Precedence

For each option, Darkmatter resolves the effective value in this order:

1. Per-rule attribute block
2. Page frontmatter `hr:` configuration
3. `HorizontalRule` component default

## Markdown Syntax

### Bare Rules (Standard Markdown)

A plain `---`, `___`, or `***` on its own line is valid CommonMark. When `hr:` frontmatter is present, the rule picks up those defaults; otherwise it renders with the component default (dashes, full-width, medium weight).

```markdown
---

___

***
```

### Attribute-Block Rules (Darkmatter Extension)

Attach a YAML flow mapping after the marker to override specific attributes for that rule:

```markdown
--- { style: waves, width: "50%" }
___ { style: dots, alignment: centered, weight: thick }
*** { style: line-star, color: "#ff0000" }
```

The attribute block is a YAML flow mapping parsed by `serde_yaml_ng`, so quoted values with embedded commas or colons (e.g., `color: "rgb(255, 0, 0)"`) are handled correctly. A legacy ad-hoc splitter serves as a graceful fallback when the YAML parser rejects malformed input.

### Rules Inside Blockquotes

Bare and attribute-block rules inside blockquotes are supported:

```markdown
> ---

> --- { style: waves }
```

The resulting `HorizontalRule` event stays wrapped by the surrounding blockquote tags — it is not promoted to document level. Page-level `hr` frontmatter defaults apply inside blockquotes just as they do at the top level.

## Supported Attributes

All attributes are optional.

| Attribute   | Values                                        | Default    |
|-------------|-----------------------------------------------|------------|
| `style`     | `dashes`, `dots`, `waves`, `line-star`, `line-circle`, `inset-line`, `curtain-rod` | `dashes`   |
| `alignment` | `full`, `centered`, `left`, `right`           | `full`     |
| `weight`    | `thin`, `medium`, `thick`                     | `medium`   |
| `width`     | CSS-like string (`"75%"`, `"200px"`)          | _none_     |
| `color`     | CSS color name or `#rrggbb`                   | _none_     |

### `style`

Controls the visual pattern of the rule. Some styles (e.g., `waves`, `line-star`, `curtain-rod`) benefit significantly from image rendering; Unicode approximations are lower fidelity.

### `alignment`

- **`full`** — spans the full available width, respecting margins and padding.
- **`centered`** — a fixed-width rule centered within the available width.
- **`left`** / **`right`** — a fixed-width rule aligned to the respective edge.

### `width`

A CSS-like length string. In terminal output, percentages resolve against the current terminal column width. In browser output, the value is emitted into the SVG/CSS representation.

### `weight`

Controls stroke thickness for image/browser rendering and selects the closest available glyph pattern for Unicode rendering. ASCII fallback may not represent weight faithfully.

### `color`

A CSS color name or hex value. When no color is specified, image rendering detects the terminal's color mode and uses `white` for dark terminals and `black` for light terminals, avoiding invisible output.

## Page-Level Defaults in Frontmatter

Set a top-level `hr:` mapping in YAML frontmatter to configure all horizontal rules in the document:

```yaml
---
hr:
  style: waves
  width: "50%"
  alignment: centered
  weight: medium
---
```

Non-string scalars (numbers, booleans) are coerced to strings so `width: 50` works the same as `width: "50"`. Unknown keys emit a `tracing::warn!` and are dropped.

## Parsing

The `RuleProcessor` iterator adapter intercepts paragraph events that contain a single text node matching the horizontal-rule pattern (`--- { ... }`). Standard `Event::Rule` events from pulldown-cmark (bare `---` without an attribute block) are handled separately in the renderers.

### Parsing Rules

- At least three of the same character (`-`, `_`, or `*`) are required; mixed markers (e.g., `-*-`) are not recognized.
- Attribute blocks must be wrapped in `{ }`.
- Content inside fenced code blocks is never transformed.
- Content inside list items is never transformed (it remains part of the list paragraph).

## Rendering to the Terminal

Terminal rendering uses progressive enhancement across three tiers:

| Tier | Condition                              | Behavior                                                        |
|------|----------------------------------------|-----------------------------------------------------------------|
| 1    | Kitty/iTerm2 image support detected    | SVG rasterized to PNG via `resvg`, displayed as a `TerminalImage` |
| 2    | No image support, UTF-8 locale         | Unicode glyphs (e.g., `≋` for waves)                           |
| 3    | No image support, no UTF-8             | ASCII fallback                                                 |

The `write_horizontal_rule` helper in `darkmatter/lib/src/markdown/output/terminal.rs` respects the rule's layout margins. A default-layout rule produces a single trailing blank line to match the surrounding markdown rhythm.

## Rendering to the Browser

Browser output is an inline SVG with CSS custom properties for styling:

- `--hr-weight` — stroke thickness
- `--hr-color` — stroke/fill color
- `--hr-width` — rule width

The `HtmlOptions.hr_css_variables` map can substitute concrete values for these variables per-instance via `BrowserRenderable::render_to_browser_with_inline_variables`. When the map is empty (the default), the SVG retains its `var(--hr-*, …)` expressions so page-level CSS or downstream code can control the appearance.

## Validation

- **Unknown enum values** for `style`, `alignment`, or `weight` fall back to the component default and emit `tracing::warn!`. The renderer always produces output.
- **Unknown attribute keys** are dropped during parsing with `tracing::warn!`.
- **Non-scalar values** (arrays, objects, null) for recognized keys are skipped with a warning; remaining sibling keys still apply.
- Warnings are visible via `RUST_LOG=darkmatter=warn`.

## Source Files

| File | Purpose |
|------|---------|
| `darkmatter/lib/src/markdown/inline/types.rs` | `HorizontalRuleAttrs` struct and `InlineEvent::HorizontalRule` variant |
| `darkmatter/lib/src/markdown/block/rule_processor.rs` | `RuleProcessor` — parses attribute blocks from markdown |
| `darkmatter/lib/src/markdown/block/hr_builder.rs` | `build_rule_with_defaults`, `hr_defaults_from_frontmatter` — shared builder |
| `darkmatter/lib/src/markdown/output/terminal.rs` | Terminal rendering via `write_horizontal_rule` |
| `darkmatter/lib/src/markdown/output/html.rs` | Browser rendering via `render_rule_browser` |
| `biscuit-terminal` | `HorizontalRule` component (`Renderable` + `BrowserRenderable`) |
