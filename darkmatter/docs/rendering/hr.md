# Horizontal Rules

Darkmatter styles CommonMark horizontal rules (`---`, `___`, `***`) with configurable visual styles, alignment, weight, width, and color. Rendering is delegated to biscuit-terminal's `HorizontalRule` component.

## Styling Model

Two layers of configuration control how every horizontal rule looks:

1. **Page defaults** via frontmatter `style.hr.*` — the preferred mechanism for consistent styling across a document.
2. **Per-rule overrides** via an attribute block after the marker — for occasional exceptions.

Per-rule attributes override only the keys they specify. Missing keys inherit from frontmatter defaults. Missing frontmatter keys fall back to the `HorizontalRule` component default.

### Precedence

For each option, Darkmatter resolves the effective value in this order:

1. Per-rule attribute block
2. Page frontmatter `style.hr.*` configuration
3. `HorizontalRule` component default

## Markdown Syntax

### Bare Rules (Standard Markdown)

A plain `---`, `___`, or `***` on its own line is valid CommonMark. When `style.hr` frontmatter is present, the rule picks up those defaults; otherwise it renders with the component default (dashes, full-width, medium weight).

```markdown
---

___

***
```

### Attribute-Block Rules (Darkmatter Extension)

Attach a YAML flow mapping after the marker to override specific attributes for that rule. The canonical key for the visual style is `kind`:

```markdown
--- { kind: waves, width: "50%" }
___ { kind: dots, alignment: center, weight: thick }
*** { kind: line-star, color: "#ff0000" }
```

The legacy inline key `style` is accepted as a deprecated alias for one release cycle:

```markdown
--- { style: waves }
```

If both `kind` and `style` are present, `kind` wins and a deprecation warning is still emitted because the document contains deprecated syntax.

The attribute block is a YAML flow mapping parsed by `serde_yaml_ng`, so quoted values with embedded commas or colons (e.g., `color: "rgb(255, 0, 0)"`) are handled correctly. A legacy ad-hoc splitter serves as a graceful fallback when the YAML parser rejects malformed input.

### Rules Inside Blockquotes

Bare and attribute-block rules inside blockquotes are supported:

```markdown
> ---

> --- { kind: waves }
```

The resulting `ThematicBreak` node stays nested under the surrounding blockquote — it is not promoted to document level. Page-level `style.hr` frontmatter defaults apply inside blockquotes just as they do at the top level.

## Supported Attributes

All attributes are optional.

| Attribute   | Values                                        | Default    |
|-------------|-----------------------------------------------|------------|
| `kind`      | `dashes`, `dots`, `waves`, `line-star`, `line-circle`, `inset-line`, `curtain-rod` | `dashes`   |
| `alignment` | `full`, `center`, `left`, `right`             | `full`     |
| `weight`    | `thin`, `medium`, `thick`                     | `medium`   |
| `width`     | CSS-like string (`"75%"`, `"200px"`)          | _none_     |
| `color`     | CSS color name or `#rrggbb`                   | _none_     |

### `kind`

Controls the visual pattern of the rule. Some styles (e.g., `waves`, `line-star`, `curtain-rod`) benefit significantly from image rendering; Unicode approximations are lower fidelity.

### `alignment`

- **`full`** — spans the full available width, respecting margins and padding.
- **`center`** — a fixed-width rule centered within the available width.
- **`left`** / **`right`** — a fixed-width rule aligned to the respective edge.

The legacy spelling `centered` is accepted as a deprecated alias for `center`.

### `width`

A CSS-like length string. In terminal output, percentages resolve against the current terminal column width. In browser output, the value is emitted into the SVG/CSS representation.

### `weight`

Controls stroke thickness for image/browser rendering and selects the closest available glyph pattern for Unicode rendering. ASCII fallback may not represent weight faithfully.

### `color`

A CSS color name or hex value. When no color is specified, image rendering detects the terminal's color mode and uses `white` for dark terminals and `black` for light terminals, avoiding invisible output.

## Page-Level Defaults in Frontmatter

Set `style.hr` in YAML frontmatter to configure all horizontal rules in the document:

```yaml
---
style:
  hr:
    kind: waves
    width: "50%"
    alignment: center
    weight: medium
---
```

Top-level `hr:` frontmatter is no longer an HR styling surface. Put page-wide horizontal-rule defaults under `style.hr`; a root `hr:` block is not merged into the active style tree.

## `--strict-style`

`md --strict-style` treats deprecated HR syntax as an error:

- Documents with inline `--- { style: waves }` fail validation.
- Documents with `style.hr.alignment: centered` fail validation.

Use strict mode in CI to catch deprecated syntax early.

## Parsing

The span-aware render-tree fold (`markdown/render_tree/block_extension.rs`) rewrites paragraphs that contain a single text node matching the horizontal-rule pattern (`--- { ... }`) into a `ThematicBreak` node carrying `darkmatter.hr.*` hints, parsing the attribute block through `block::hr_parser`. Standard `Event::Rule` events from pulldown-cmark (bare `---` without an attribute block) fold to a hint-less `ThematicBreak`.

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

The render-tree terminal renderer folds the `ThematicBreak` node (and its `darkmatter.hr.*` hints) into output via biscuit-terminal's `HorizontalRule` component, respecting the rule's layout margins. A default-layout rule produces a single trailing blank line to match the surrounding markdown rhythm.

## Rendering to the Browser

Browser output is an inline SVG with CSS custom properties for styling:

- `--hr-weight` — stroke thickness
- `--hr-color` — stroke/fill color
- `--hr-width` — rule width

The `HtmlOptions.hr_css_variables` map is lowered to a page-level `:root` declaration (sorted, with unsafe `<`/newline-bearing entries dropped) so the declared override resolves against the SVG's `var(--hr-*, …)` expressions in the browser. When the map is empty (the default), no `:root` override is emitted and the SVG retains its `var(--hr-*, …)` expressions so page-level CSS or downstream code can control the appearance.

## Validation

- **Unknown enum values** for `kind`, `alignment`, or `weight` fall back to the component default and emit `tracing::warn!`. The renderer always produces output.
- **Unknown attribute keys** are dropped during parsing with `tracing::warn!`.
- **Non-scalar values** (arrays, objects, null) for recognized keys are skipped with a warning; remaining sibling keys still apply.
- Warnings are visible via `RUST_LOG=darkmatter=warn`.

## Source Files

| File | Purpose |
|------|---------|
| `darkmatter/lib/src/markdown/inline/types.rs` | `HorizontalRuleAttrs` — the parsed HR attribute struct (lowered onto the `darkmatter.hr.*` tree hints) |
| `darkmatter/lib/src/markdown/block/hr_parser.rs` | HR attribute-block parser (`try_parse_hr_attrs`, `parse_hr_attribute_block`, `scan_inline_hr_warnings`) — single source of truth for `--- { … }` directives |
| `darkmatter/lib/src/markdown/block/hr_builder.rs` | Maps the `style.hr` schema enums to the canonical strings the render-tree `darkmatter.hr.*` hints carry |
| `darkmatter/lib/src/markdown/render_tree/block_extension.rs` | Span-aware fold of `---` / attribute rules into `ThematicBreak` nodes carrying `darkmatter.hr.*` hints |
| `darkmatter/lib/src/markdown/render_tree/entrypoints.rs` | Applies page (`style.hr.*`) defaults to bare rules (`apply_hr_defaults`) before tree rendering |
| `biscuit-terminal` | `HorizontalRule` component; the render-tree terminal / browser renderers fold the `ThematicBreak` hints into output |
