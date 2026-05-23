# Darkmatter Style Hints

## Style Frontmatter

Darkmatter reserves the Frontmatter property `style` for defining stylistic preferences. This is used by the following page elements:

1. `page` — **wired (sub-spec #2)**
1. `table`
1. `hyperlinks`
1. `images`
1. `hr` - Horizontal Rules
1. `ul` - Unordered Lists
1. `ol` - Ordered Lists
1. `li` - List Item
1. `block-quote` (canonical) / `block_quote` (snake-case alias, emits `Deprecated`)

When performing a composition argument the caller can send in a style object to modify the default style of the full page graph (`style` property is passed down to child properties).

### Wiring Status

| Sub-Spec | Scope                             | Status          |
|----------|-----------------------------------|-----------------|
| #1       | Parser + schema + warnings        | Live            |
| #2       | `style.page.*` → `DarkmatterPage` | **Live**        |
| #3       | `style.table.*` / `style.images.*` / `style.block-quote.*` (`width`, `max-width`, `alignment`) | **Live** |
| #4       | `style.ul.*` / `style.ol.*` / `style.li.*` / `style.hyperlinks.*` | Pending |
| #5       | `color` / `bg-color` application  | Pending         |
| #6       | `style.hr.*` migration            | Pending         |
| #7       | Final bespoke knobs               | Pending         |

The library exposes `ACTIVE_STYLE_WIRING_SUB_SPEC` (currently `3`) so `KnownButInactive` warnings only fire for keys whose wiring sub-spec has not yet landed. The color and background-color knobs on every bucket — `style.table.color`, `style.table.bg-color`, `style.images.color`, `style.images.bg-color`, `style.block-quote.color`, `style.block-quote.bg-color` — remain inactive and continue to emit `KnownButInactive { sub_spec: 5 }`.

## Page-Level Style (Sub-Spec #2)

The `style.page.*` subset is fully wired through `DarkmatterPage` for both terminal (`md`) and HTML (`md --output html`) rendering. Use canonical kebab-case keys; snake-case aliases are still accepted but produce a `Deprecated` warning.

### Supported Keys

| Key                  | Value Shape                                     | Notes                                                       |
|----------------------|-------------------------------------------------|-------------------------------------------------------------|
| `left-margin`        | `Nch`, `N%`, or `0`                             | Percent base = captured terminal width                      |
| `right-margin`       | `Nch`, `N%`, or `0`                             | Percent base = captured terminal width                      |
| `top-margin`         | non-negative integer (row count)                | Bare number; `2ch` is rejected for vertical fields          |
| `bottom-margin`      | non-negative integer                            |                                                             |
| `left-padding`       | `Nch`, `N%`, or `0`                             |                                                             |
| `right-padding`      | `Nch`, `N%`, or `0`                             |                                                             |
| `top-padding`        | non-negative integer                            |                                                             |
| `bottom-padding`     | non-negative integer                            |                                                             |
| `max-width`          | `Nch`, `N%`, or `0` (`0` rejected at apply-time) | Percent resolves against post-margin / post-padding content width |
| `alignment`          | `left` \| `center` \| `right`                    | Broadcasts to every `PageComponent` (table/image/list/code/block-quote) |
| `background`         | `transparent` \| `subtle` \| `pronounced`        | Maps to `PageBackground`                                    |
| `stylesheet`, `meta`, `code` | (parsed but not yet wired)              | Reserved for sub-spec #7                                    |

`Length::Css(_)` (e.g. `10px`) is **not** valid for page-level terminal layout; it raises `StyleApplyError::InvalidCssLength`.

### Example

```yaml
---
style:
    page:
        left-margin: 2ch
        right-margin: 4ch
        top-margin: 1
        bottom-margin: 0
        max-width: 80%
        alignment: center
        background: subtle
---
```

### Precedence: CLI Flags Win Over Frontmatter

`md` always applies its CLI flags before falling back to `style:` frontmatter. The integration order in `darkmatter-cli` is:

1. `DarkmatterPage::new(&terminal)`
2. `apply_cli_layout_flags(page, &cli)` — CLI margin/padding/max-width/page-bg/alignment flags
3. `darkmatter::style::apply_page_style(page, &style, overrides)` — frontmatter, with `overrides` listing every field the CLI already claimed

Override claims expand the same way `apply_cli_layout_flags` resolves shorthand:

| CLI Flag        | Claims                                                 |
|-----------------|--------------------------------------------------------|
| `--margin N`    | `margin_top`, `margin_right`, `margin_bottom`, `margin_left` |
| `--mx N`        | `margin_left`, `margin_right`                          |
| `--my N`        | `margin_top`, `margin_bottom`                          |
| `--mt`/`--mr`/`--mb`/`--ml` | the named side only                        |
| `--padding`/`--px`/`--py`/`--pt`/`--pr`/`--pb`/`--pl` | mirrors margin shorthand |
| `--max-width N` | `max_width`                                            |
| `--page-bg`     | `background`                                           |
| `--alignment`   | `alignment` (also broadcasts to every component)       |

Once a field is claimed, the corresponding `style.page.*` value is skipped silently.

### `--strict-style`

`md --strict-style` promotes schema-validation warnings to errors:

- `UnknownKey` → error
- `Deprecated` → error
- `KnownButInactive` → **still informational** (never fails strict mode)

Use it in CI to catch typos and snake-case aliases.

### Errors

`apply_page_style` returns `StyleApplyError`:

- `InvalidCssLength { field }` — `Length::Css(_)` was supplied to a page-level length field.
- `InvalidMaxWidth` — `style.page.max-width` resolved to `0` cells (e.g. `50%` of a `0`-cell content width after margins consumed everything).

## Component-Level Style (Sub-Spec #3)

`style.table.*`, `style.images.*`, and `style.block-quote.*` lower onto the same `DarkmatterPage` builder used by page-level style, via `apply_component_style`. Three knobs are live on each of the three buckets:

- `width` — fixed width as `Nch`, `N%`, or `0`. Lowers to `PageFill::Explicit(WidthUnit)`.
- `max-width` — upper bound as `Nch`, `N%`, or `0`. Lowers to `PageFill::Max(WidthUnit)`.
- `alignment` — `left` \| `center` \| `right`. Sets the component's `PageAlignment` directly.

`Length::Css(_)` (e.g. `10px`) is **not** valid in component fill fields; `apply_component_style` returns `StyleApplyError::ComponentInvalidCssLength { bucket, field }`.

### Width vs. Max-Width Exclusivity

`width` and `max-width` are mutually exclusive **within the same bucket** because `DarkmatterPage` exposes a single `PageFill` slot per component. Setting both raises:

```text
`style.{bucket}.width` and `style.{bucket}.max-width` are mutually exclusive
```

Bucket names in diagnostics use canonical kebab-case (`style.block-quote.*`, not `style.block_quote.*`). Snake-case aliases still parse but emit a `Deprecated` warning.

### Example

```yaml
---
style:
    table:
        max-width: 50%
        alignment: right
    images:
        width: 40ch
        alignment: center
    block-quote:
        max-width: 60ch
---
```

### Precedence: CLI Flags, Page Broadcast, Component Frontmatter

The integration order in `darkmatter-cli` is:

1. `DarkmatterPage::new(&terminal)`
2. `apply_cli_layout_flags(page, &cli)` — global and component-specific CLI flags
3. `darkmatter::style::apply_page_style(page, &style, page_overrides)` — page-level frontmatter, including any `style.page.alignment` broadcast
4. `darkmatter::style::apply_component_style(page, &style, component_overrides)` — `style.{table|images|block-quote}.*` frontmatter

`ComponentStyleOverrides` records which fields the CLI already claimed:

| CLI Flag                | Claims                                                     |
|-------------------------|------------------------------------------------------------|
| `--align-tables`        | `tables_alignment`                                         |
| `--align-images`        | `images_alignment`                                         |
| `--align-block-quotes`  | `block_quotes_alignment`                                   |
| `--fill-tables`         | `tables_fill`                                              |
| `--fill-images`         | `images_fill`                                              |
| `--fill-block-quotes`   | `block_quotes_fill`                                        |
| `--alignment` (global)  | every `*_alignment` field                                  |
| `--fill` (global)       | every `*_fill` field                                       |

Precedence resolution at each component:

- **CLI flag wins over everything.** A CLI claim suppresses both the matching component frontmatter and the page-level alignment broadcast.
- **Component frontmatter wins over the page broadcast.** When no CLI flag has claimed a component, `style.{bucket}.alignment` overrides `style.page.alignment` for that component only; untouched components keep the broadcast.

### Errors

`apply_component_style` returns `StyleApplyError`:

- `ComponentWidthConflict { bucket }` — `width` and `max-width` set together inside the same bucket.
- `ComponentInvalidCssLength { bucket, field }` — `width` or `max-width` is a `Length::Css(_)` value.

### Inactive Keys

The remaining `style.{table|images|block-quote}.*` keys (`color`, `bg-color`, plus `images.local_style`) parse cleanly but emit `KnownButInactive { sub_spec: 5 }` until color application lands.

## Style Mutation

### Common Mutations

Each of the properties defined under `style` provide the following mutations:

- `width(ch | %)` - set's a fixed width for the element in question (ch or %)
- `max_width(ch | %)` - set's a maximum width; this can be used in conjunction with width but that only really makes sense when one has a fixed value and the other a percentage. Since the percentage is lazily loaded at render time 
- `alignment`
- `color`
- `bg_color`

### Bespoke Style

While _every_ style property provides the _common_ mutations, each of the types provide their own bespoke properties which can be set:

- `page`
    - `stylesheet` - allows you to point to an external CSS stylesheet (local file or HTTP pointer)
    - `meta`
    - `code`
    - `max_width`
    - `alignment`
- `hr`
    - **IMPORTANT:** currently the `hr` functionality has implemented their bespoke styles directly to the top-level `hr` property and that needs to be moved here as `style.hr`
        - `darkmatter/lib/src/markdown/block/hr_builder.rs:117`
    - `kind` (this replaces `hr.style` when moved from current implementation)
    - 
- `table`
    - `width`
    - `max-width`
    - `alignment`
- `hyperlinks`
    - `local_style` - provides an override of `style` but only for links to local files
- `images`
    - `local_style` - provides an override of `style` but only for links to local files

## Tailwind Colors

The `biscuit-terminal` already provides a very handy mapping to Tailwind color names to their RGB values. The base
colors supported are:

- red
- orange
- amber
- yellow
- lime
- green
- emerald
- teal
- cyan
- sky
- blue
- indigo
- violet
- purple
- fuchsia
- pink
- rose
- slate
- gray
- zinc
- neutral
- stone

Each of colors provides the following luminosity levels:

- 50, 100, 200, 300, 400, 500, 600, 700, 800, 900, and 950

- whenever someone sets a **style** value to a color, they may choose to use Tailwind names like `red-500` which combine the
color with the luminosity level.
- you can also add in Tailwind's convention of an opacity setting with `red-500/50` where the trailing `/50` indicates the opacity setting 
    - Note: the opacity is only used when targeting HTML, it is dropped everywhere else

## Code Block Theme & Contrast

The `page.code` theme (`--code-theme`, `code_theme`) is a mode-agnostic
`ThemePair`, resolved to a concrete light/dark theme at render time. Code blocks
deliberately resolve against the *inverted* color mode so the panel contrasts
against the page (light code on a dark page, and vice versa); prose, tables, and
the page background follow the real mode. This holds for **both terminal and
HTML** output, so the targets agree. Single-variant themes
(`dracula`/`nord`/`monokai`/`vs-dark`) ignore the mode by design.

See [Code Highlighting](./code-highlighting.md) for the full model.
