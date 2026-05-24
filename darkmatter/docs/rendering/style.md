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
| #4       | `style.ul.*` / `style.ol.*` / `style.li.*` (`width`, `max-width`, `alignment`, `ul.left-margin`) | **Live** |
| #5       | `color` / `bg-color` application  | Pending         |
| #6       | `style.hr.*` migration            | Pending         |
| #7       | Final bespoke knobs               | Pending         |

The library exposes `ACTIVE_STYLE_WIRING_SUB_SPEC` (currently `4`) so `KnownButInactive` warnings only fire for keys whose wiring sub-spec has not yet landed. The color and background-color knobs on every bucket — `style.table.color`, `style.table.bg-color`, `style.images.color`, `style.images.bg-color`, `style.block-quote.color`, `style.block-quote.bg-color`, `style.ul.color`, `style.ul.bg-color`, `style.ol.color`, `style.ol.bg-color`, `style.li.color`, `style.li.bg-color` — remain inactive and continue to emit `KnownButInactive { sub_spec: 5 }`.

## Schema & Parser Architecture (Sub-Spec #1)

The `style:` frontmatter is parsed by the `darkmatter::style` module into a typed, sparse tree of Rust structs anchored to `renderable` primitives. No darkmatter-local layout, style, or color runtime types are introduced — every length is `renderable::layout::Length`, every alignment is `renderable::layout::Alignment`, and every color is `renderable::color::Color` (wrapped only by `StyleColor` for opacity).

### Module Location

```text
darkmatter/lib/src/style/
├── mod.rs              # Re-exports
├── schema.rs           # StyleFrontmatter + per-bucket structs
├── length.rs           # Custom deserializers producing renderable::layout::Length
├── color.rs            # StyleColor wrapper + deserializers producing renderable::color::Color
├── parse.rs            # from_frontmatter / from_json_value / into_strict
├── warning.rs          # StyleWarning + StyleWarningKind
└── tests/              # Parser tests + roundtrip tests
```

### Entry Points

Three public functions form the parse API:

- **`from_frontmatter(fm: &Frontmatter)`** — reads `frontmatter.as_map().get("style")` and delegates to `from_json_value`. Returns `(StyleFrontmatter::default(), vec![])` when no `style:` key is present.
- **`from_json_value(value: &serde_json::Value)`** — the canonical entry point. `Frontmatter` stores its map as `IndexMap<String, serde_json::Value>`, so callers that already hold the raw value go here directly. Unknown keys are collected into warnings; structural or type errors short-circuit via `StyleParseError`.
- **`into_strict(parsed)`** — promotes schema-validation warnings (`UnknownKey`, `Deprecated`) into errors. `KnownButInactive` warnings are deliberately ignored so a schema-strict caller does not fail on a forward-compatible document.

### Warning Channel

Every parse produces zero or more `StyleWarning` values alongside the parsed `StyleFrontmatter`:

```rust
struct StyleWarning {
    path: String,           // Fully-qualified YAML path, e.g. "style.page.lft-margin"
    kind: StyleWarningKind, // Discriminated diagnostic category
    source_span: Option<StyleSpan>, // Always None in v1; reserved for future spans
}
```

The `StyleWarningKind` enum distinguishes three categories:

| Variant | Meaning | Strict mode behavior |
|---|---|---|
| `UnknownKey` | Path does not appear anywhere in the schema (likely a typo). | Upgraded to error |
| `Deprecated { replacement }` | Path matched a snake-case alias for a canonicalized kebab-case key. | Upgraded to error |
| `KnownButInactive { sub_spec }` | Key is schema-valid but no rendering wiring exists for it yet. Carries the sub-spec number that will wire it. | **Never** causes strict failure |

### Error Type

Structural and type errors short-circuit the parse via `StyleParseError`:

| Variant | Trigger |
|---|---|
| `Structure { path, expected, actual }` | Wrong YAML structure (e.g. a string where a map was expected). |
| `InvalidLength { path, raw, reason }` | Malformed length value (negative, unsupported unit, empty). |
| `InvalidPercent { path, value }` | Percent outside `0.0..=100.0` range. |
| `InvalidColor { path, raw, reason }` | Unrecognized color syntax. |
| `Strict { warnings }` | `into_strict` found one or more `UnknownKey`/`Deprecated` warnings. |
| `Serde(..)` | Underlying serde deserialization failure. |

### Schema Shape

The schema is a sparse tree of `Option` fields keyed by component. All buckets are `Option` so a sparse user input (e.g. only `page.left-margin`) does not materialize default values across the tree.

#### Root: `StyleFrontmatter`

```rust
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
struct StyleFrontmatter {
    page:         Option<PageStyle>,
    table:        Option<TableStyle>,
    hyperlinks:   Option<HyperlinkStyle>,
    images:       Option<ImageStyle>,
    hr:           Option<HrStyle>,
    ul:           Option<UlStyle>,
    ol:           Option<OlStyle>,
    li:           Option<LiStyle>,
    block_quote:  Option<BlockQuoteStyle>,  // alias: "block_quote"
}
```

`#[serde(rename_all = "kebab-case")]` rewrites the field `block_quote` to the wire key `block-quote`; the `alias` accepts the legacy `block_quote` spelling and triggers a `Deprecated` warning.

#### Common Mutations: `CommonStyle`

Most component buckets embed a flattened `CommonStyle` providing the five shared mutations:

| Field | Type | Wire key | Alias |
|---|---|---|---|
| `width` | `Option<renderable::layout::Length>` | `width` | — |
| `max_width` | `Option<renderable::layout::Length>` | `max-width` | `max_width` |
| `alignment` | `Option<renderable::layout::Alignment>` | `alignment` | — |
| `color` | `Option<StyleColor>` | `color` | — |
| `bg_color` | `Option<StyleColor>` | `bg-color` | `bg_color` |

#### Per-Component Buckets

| Bucket | Struct | Common? | Extra fields |
|---|---|---|---|
| `page` | `PageStyle` | inline (not flattened) | 4 margins, 4 paddings, `background`, `stylesheet`, `meta`, `code` |
| `table` | `TableStyle` | `#[serde(flatten)]` | — |
| `block-quote` | `BlockQuoteStyle` | `#[serde(flatten)]` | — |
| `ul` | `UlStyle` | `#[serde(flatten)]` | `left_margin: Option<Length>` (indent for list content) |
| `ol` | `OlStyle` | `#[serde(flatten)]` | — |
| `li` | `LiStyle` | `#[serde(flatten)]` | — |
| `hyperlinks` | `HyperlinkStyle` | `#[serde(flatten)]` | `local_style: Option<Box<CommonStyle>>` (file-local links only) |
| `images` | `ImageStyle` | `#[serde(flatten)]` | `local_style: Option<Box<CommonStyle>>` (file-local links only) |
| `hr` | `HrStyle` | `#[serde(flatten)]` | `kind: Option<String>` (replaces legacy per-block `style: waves`) |

`PageStyle` is special: it does not flatten `CommonStyle` because its margins and paddings use a mix of horizontal `Length` and vertical `u16` fields, while `CommonStyle` is purely horizontal. It inlines `alignment`, `color`, `bg_color`, and `max_width` directly.

### Value Deserializers

#### Length

Horizontal fields (`left-margin`, `right-margin`, `left-padding`, `right-padding`, `width`, `max-width`) accept `renderable::layout::Length` via a custom string deserializer:

| Input form | Result |
|---|---|
| `"2ch"` / `"2 ch"` | `Length::Ch(2)` |
| `"40"` (bare number) | `Length::Ch(40)` |
| `"50%"` / `"50.5%"` | `Length::Percent(50.0)` / `Length::Percent(50.5)` |

Rejected forms: negative values (`"-2"`), unsupported units (`"2px"`, `"2em"`, `"2rem"`), empty string, malformed (`"50%%"`), percent out of range (`"101%"`).

Vertical fields (`top-margin`, `bottom-margin`, `top-padding`, `bottom-padding`) accept `u16` row counts only. A `row_count` helper explicitly rejects strings (e.g. `top-margin: "2ch"` fails with a `Structure` error rather than serde's default "invalid type" message).

#### Alignment

Delegates to `renderable::layout::Alignment` which accepts `"left"`, `"center"`, `"right"` via its existing `#[serde(rename_all = "snake_case")]` derive. The wrapper adds `"centered"` as an alias for `Center`.

#### Color: `StyleColor`

A thin wrapper around `renderable::color::Color` that preserves optional opacity for HTML targets:

```rust
struct StyleColor {
    color: renderable::color::Color,
    opacity: Option<u8>,  // Tailwind-style /0..=100; HTML-only, dropped by terminal
}
```

Accepted forms:

| Syntax | Result |
|---|---|
| `"red-500"` | Tailwind enum variant, no opacity |
| `"red-500/50"` | Tailwind enum variant + `opacity: Some(50)` |
| `"#fff"`, `"#ffffff"` | `Color::Rgb(..)` |
| `"#ffffffff"` | `Color::Rgb(..)` + alpha hex converted to `opacity: Some(alpha * 100 / 255)` |
| `"orange"`, `"rebeccapurple"` | Web named via `WEB_COLOR_LOOKUP` |

The Tailwind family and level enumeration is taken directly from `renderable::color::Tailwind` (21 families × 11 levels). The parser maps `"red-500"` to the matching enum variant; it does not redefine the taxonomy.

### Two-Pass Parser Design

Because `#[serde(deny_unknown_fields)]` aborts on the first typo and `serde_ignored` has unreliable behavior under `#[serde(flatten)]` and `Option<T>` newtypes, the parser uses a two-pass approach:

1. **Pass 1 — Canonicalization walk.** Walk the raw `serde_json::Value` map. For each leaf path: compute its canonical kebab-case path against a static schema descriptor; emit `Deprecated` if the raw key is a snake-case alias; emit `UnknownKey` if the path is not in the schema at all. The walk is driven by a static schema descriptor (one entry per leaf: canonical name, accepted aliases, parent path).

2. **Pass 2 — Typed deserialization.** Deserialize the value into `StyleFrontmatter` via standard serde. Structural errors short-circuit via `StyleParseError`.

3. **Pass 3 — `KnownButInactive` annotation.** Walk the parsed `StyleFrontmatter` and emit a `KnownButInactive` warning for every `Some` field whose wiring sub-spec exceeds `ACTIVE_STYLE_WIRING_SUB_SPEC`.

This gives full control over alias detection and unknown-key reporting without depending on `serde_ignored`'s behavior under `flatten`.

### Key Spelling: Kebab-Case Canonical

All multi-word frontmatter keys use hyphens: `max-width`, `bg-color`, `local-style`, `block-quote`, `left-margin`, etc. Snake-case spellings (`max_width`, `bg_color`, `local_style`, `block_quote`) are accepted via serde `alias` for backward compatibility and emit a `Deprecated { replacement }` warning. The pass-1 walker compares raw map keys against the canonical set to detect alias hits (since serde does not surface which alias matched).

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
| `alignment`          | `left` \| `center` \| `right`                    | Broadcasts to every `PageComponent` via `use_alignment_for_all` (table/image/list/code/block-quote) |
| `background`         | `transparent` \| `subtle` \| `pronounced`        | Maps to `PageBackground` enum                               |
| `stylesheet`, `meta`, `code` | (parsed but not yet wired)              | Reserved for sub-spec #7                                    |

`Length::Css(_)` (e.g. `10px`) is **not** valid for page-level terminal layout; it raises `StyleApplyError::InvalidCssLength`.

### Length Lowering

`apply_page_style` converts `renderable::layout::Length` values to the concrete cell/row counts that `DarkmatterPage` expects:

| Length variant | Page-level lowering |
|---|---|
| `Length::Zero` | `0` |
| `Length::Ch(n)` | `u16::try_from(n).unwrap_or(u16::MAX)` |
| `Length::Percent(p)` | Resolved at apply time with rounded cell counts. Validation (`0.0..=100.0`) is already enforced by the parser. |
| `Length::Css(_)` | Returns `StyleApplyError::InvalidCssLength`. CSS units have no terminal equivalent. |

Percent resolution uses two different base widths depending on the field:

- **Horizontal margins and padding** resolve against the captured terminal width (`DarkmatterPage::new(&term)` captures it at construction time).
- **`max-width`** resolves against the **content width** after final page margin and padding values are known — including any CLI overrides. This matches `LayoutContext` component-fill semantics and ensures page `max-width` composes predictably with margins. A resolved `max-width` of `0` cells is rejected because `DarkmatterPage` treats `max_width = 0` as invalid.

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

### Public API

The page-level wiring is exposed through two public items in `darkmatter::style`:

```rust
/// CLI fields that already claimed page-level layout/style values.
///
/// Constructed by darkmatter-cli from `Cli` after applying the same
/// shorthand expansion rules as `apply_cli_layout_flags`. This type
/// lives in the darkmatter library so the style applicator does not
/// depend on CLI argument structs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PageStyleOverrides {
    pub margin_top: bool,
    pub margin_right: bool,
    pub margin_bottom: bool,
    pub margin_left: bool,
    pub padding_top: bool,
    pub padding_right: bool,
    pub padding_bottom: bool,
    pub padding_left: bool,
    pub max_width: bool,
    pub background: bool,
    pub alignment: bool,
}

/// Apply parsed page-level style onto a DarkmatterPage builder.
///
/// CLI overrides suppress frontmatter for overlapping fields. The returned
/// page has active `style.page.*` settings applied. Warnings remain owned
/// by the parser; suppression is handled by the active wiring phase.
pub fn apply_page_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
    overrides: PageStyleOverrides,
) -> Result<DarkmatterPage, StyleApplyError>;
```

`PageStyleOverrides` is CLI-agnostic by design: `darkmatter-cli` constructs it from the parsed `Cli` struct after shorthand expansion, then passes it to `apply_page_style`. This keeps the library crate independent of CLI argument types. Both `render_terminal_output` and `html_artifact` in `darkmatter/cli/src/output.rs` call this helper before invoking `page.render(md)` / `page.render_to_browser(md)`, ensuring terminal and HTML rendering share the same frontmatter behavior.

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

`style.table.*`, `style.images.*`, and `style.block-quote.*` lower onto the same `DarkmatterPage` builder used by page-level style, via `apply_component_style`. Each bucket maps to a dedicated `PageComponent` variant:

| Bucket | `PageComponent` variant |
|---|---|
| `style.table.*` | `PageComponent::Tables` |
| `style.images.*` | `PageComponent::Images` |
| `style.block-quote.*` | `PageComponent::BlockQuotes` |

Three knobs are live on each of the three buckets:

- `width` — fixed width as `Nch`, `N%`, or `0`. Maps to `PageFill::Explicit(unit)` via `DarkmatterPage`'s `with_fill` builder.
- `max-width` — upper bound as `Nch`, `N%`, or `0`. Maps to `PageFill::Max(unit)` via `DarkmatterPage`'s `with_fill` builder.
- `alignment` — `left` \| `center` \| `right`. Maps to `use_alignment(component, alignment)` on the `DarkmatterPage` builder.

`Length::Css(_)` (e.g. `10px`) is **not** valid in component fill fields; `apply_component_style` returns `StyleApplyError::ComponentInvalidCssLength { bucket, field }`.

### Length Lowering

`apply_component_style` converts `renderable::layout::Length` to `renderable::layout::WidthUnit` for component fill:

| Length variant | Component fill lowering |
|---|---|
| `Length::Zero` | `WidthUnit::Fixed(0)` |
| `Length::Ch(n)` | `WidthUnit::Fixed(u16)` via saturating cast |
| `Length::Percent(p)` | `WidthUnit::Percent(p)` |
| `Length::Css(_)` | Returns `StyleApplyError::ComponentInvalidCssLength { bucket, field }`. CSS units have no terminal equivalent. |

### Width vs. Max-Width Exclusivity

`width` and `max-width` are mutually exclusive **within the same bucket** because `DarkmatterPage` exposes a single `PageFill` slot per component. Setting both raises:

```text
`style.{bucket}.width` and `style.{bucket}.max-width` are mutually exclusive
```

The exclusivity check runs **unconditionally** — a CLI fill flag (e.g. `--fill`, `--fill-tables`) chooses which value wins at render time, but it never makes an ambiguous frontmatter bucket valid. A document that sets both `width` and `max-width` in the same bucket always fails with `ComponentWidthConflict { bucket }` before rendering, with or without a CLI fill override.

Bucket names in diagnostics use canonical kebab-case (`style.block-quote.*`, not `style.block_quote.*`). Snake-case aliases still parse but emit a `Deprecated` warning.

### Block-Quote Width Scope

`style.block-quote.width` and `style.block-quote.max-width` constrain the **whole rendered block quote** — including the quote prefix/border and body — not just the inner text. This differs from tables and images where the fill value constrains only the component itself. The block-quote terminal path applies the component width to the rendered wrapper, ensuring the prefix/border and body share the same width budget.

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

### Public API

The component-level wiring is exposed through two public items in `darkmatter::style`:

```rust
/// CLI fields that already claimed component-level layout values.
///
/// Built by darkmatter-cli from `Cli` after applying the same global-then-
/// component-specific precedence as `apply_cli_layout_flags`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ComponentStyleOverrides {
    pub tables_alignment: bool,
    pub tables_fill: bool,
    pub images_alignment: bool,
    pub images_fill: bool,
    pub block_quotes_alignment: bool,
    pub block_quotes_fill: bool,
}

/// Apply parsed table/image/block-quote style onto a DarkmatterPage builder.
///
/// CLI overrides suppress frontmatter for overlapping component fields. The
/// returned page has active sub-spec #3 settings applied.
pub fn apply_component_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
    overrides: ComponentStyleOverrides,
) -> Result<DarkmatterPage, StyleApplyError>;
```

`ComponentStyleOverrides` is CLI-agnostic by design: `darkmatter-cli` constructs it from the parsed `Cli` struct after shorthand expansion, then passes it to `apply_component_style`. Both `render_terminal_output` and `html_artifact` in `darkmatter/cli/src/output.rs` call this helper after `apply_page_style` and before rendering.

### Errors

`apply_component_style` returns `StyleApplyError`:

- `ComponentWidthConflict { bucket }` — `width` and `max-width` set together inside the same bucket.
- `ComponentInvalidCssLength { bucket, field }` — `width` or `max-width` is a `Length::Css(_)` value.

### Inactive Keys

The remaining `style.{table|images|block-quote}.*` keys (`color`, `bg-color`, plus `images.local_style`) parse cleanly but emit `KnownButInactive { sub_spec: 5 }` until color application lands.

## List-Level Style (Sub-Spec #4)

`style.ul.*`, `style.ol.*`, and `style.li.*` map onto three new `PageComponent` variants — `Ul`, `Ol`, `Li` — that replace the deprecated `PageComponent::Lists`. Each bucket supports `width`, `max-width`, and `alignment` via the same lowering pipeline as sub-spec #3, plus a bespoke `ul.left-margin` indent channel.

### PageComponent Split

| Bucket | `PageComponent` variant | Deprecated predecessor |
|---|---|---|
| `style.ul.*` | `PageComponent::Ul` | `PageComponent::Lists` |
| `style.ol.*` | `PageComponent::Ol` | `PageComponent::Lists` |
| `style.li.*` | `PageComponent::Li` | `PageComponent::Lists` |

`PageComponent::Lists` is retained for one release cycle as a **broadcast/fallback** variant:

```rust
#[deprecated(note = "use PageComponent::{Ul, Ol, Li}")]
Lists,
```

New renderer code must use `Ul`, `Ol`, or `Li`. When a concrete list variant has no value, the renderer falls back to the deprecated `Lists` entry. `PageComponent::ALL` includes only the concrete variants (`Images`, `BlockQuotes`, `Tables`, `CodeBlocks`, `Ul`, `Ol`, `Li`), not `Lists`.

`PageComponent::LISTS` provides the three concrete list components in broadcast order:

```rust
impl PageComponent {
    /// Concrete list components in broadcast order.
    pub const LISTS: [PageComponent; 3] = [Self::Ul, Self::Ol, Self::Li];
}
```

### Tag-to-Component Mapping

The pulldown-cmark event stream distinguishes list kinds at render time:

| pulldown-cmark tag | `PageComponent` |
|---|---|
| `Tag::List(None)` | `PageComponent::Ul` |
| `Tag::List(Some(_))` | `PageComponent::Ol` |
| `Tag::Item` | `PageComponent::Li` |

### Live Knobs

Three knobs are live on each of the three list buckets, using the same lowering as sub-spec #3:

- `width` — fixed width as `Nch`, `N%`, or `0`. Maps to `PageFill::Explicit(unit)`.
- `max-width` — upper bound as `Nch`, `N%`, or `0`. Maps to `PageFill::Max(unit)`.
- `alignment` — `left` | `center` | `right`. Maps to `use_alignment(component, alignment)`.

Plus the bespoke `ul.left-margin` indent channel (see below).

`Length::Css(_)` is rejected in list fill fields the same way as other component buckets, returning `StyleApplyError::ComponentInvalidCssLength`.

### Width vs. Max-Width Exclusivity

`width` and `max-width` remain mutually exclusive **per bucket** — the same exclusivity check from sub-spec #3 applies. Setting both `style.ul.width` and `style.ul.max-width` (or the equivalent for `ol`/`li`) returns `ComponentWidthConflict { bucket }` before rendering. This check runs unconditionally, regardless of CLI fill overrides.

### ul.left-margin: Independent Indent Channel

`style.ul.left-margin` applies as a list-specific left indent that can coexist with `style.ul.width` or `style.ul.max-width`. It is **not** stored as `PageFill::Indent` — the single `PageFill` slot per component cannot represent both indent and width simultaneously. Instead, `DarkmatterPage`/`LayoutContext` expose a dedicated list-indent facility:

- `with_list_left_margin(PageComponent::Ul, WidthUnit)` — builder method accepting only `PageComponent::Ul` in this sub-spec.
- `LayoutContext::list_left_margin(PageComponent)` — retrieval at render time.

Calling `with_list_left_margin` with `Ol`, `Li`, or a non-list component returns a clear apply error.

### Indent and Width Stacking Order

For unordered lists, the renderer applies layout in this fixed order:

1. **Resolve `ul.left-margin`.** The indent value is subtracted from the available body width.
2. **Apply `ul.width` or `ul.max-width`.** The width constraint operates on the remaining body width after indent.
3. **Apply alignment padding.** Any remaining alignment padding is added last.

This means `left-margin: 4ch` plus `max-width: 40` produces a 4-cell offset and a body wrapping at no more than 40 cells, capped by the remaining page width.

### Percent Resolution

| Field | Resolution base |
|---|---|
| `ul.left-margin: N%` | Page content width (after page margin/padding and page max-width are known) |
| `ul`/`ol`/`li` `width`, `max-width` | Same helper as sub-spec #3: fixed `ch` values saturate to `u16`; percentages resolve against content width in `LayoutContext` |

`Length::Css(_)` fails with `StyleApplyError` for all list length fields.

### li.* Scope: Item Bodies Only

`style.li.alignment`, `style.li.width`, and `style.li.max-width` affect each list item's **content body** after the marker prefix is emitted. They do not affect marker placement — the containing `Ul`/`Ol` component governs markers. If both `li` and the containing list set alignment or width, the `li` value wins for the item body only.

### Browser Selectors and CSS Order

| `PageComponent` | CSS selector |
|---|---|
| `Ul` | `ul` |
| `Ol` | `ol` |
| `Li` | `li` |
| `Lists` (deprecated) | `ul, ol` |

Generated CSS emits deprecated `Lists` selectors **before** concrete variant selectors so that granular list styles win by normal CSS cascade when both are present.

### CLI Flags

Broadcast flags apply to all three concrete list variants:

| CLI Flag | Effect |
|---|---|
| `--align-lists <alignment>` | Sets alignment for `Ul`, `Ol`, and `Li` |
| `--fill-lists <fill>` | Sets fill for `Ul`, `Ol`, and `Li` |

Granular flags override the broadcast for their specific component:

| CLI Flag | Effect |
|---|---|
| `--align-ul` / `--align-ol` / `--align-li` | Alignment for a single list variant |
| `--fill-ul` / `--fill-ol` / `--fill-li` | Fill for a single list variant |

Precedence follows the same model as sub-specs #2 and #3: global CLI flags (`--alignment`, `--fill`) claim all list components; broadcast list flags claim `Ul`, `Ol`, and `Li`; granular flags claim only their concrete component. CLI flags always win over frontmatter.

### Precedence and Integration Order

The integration order in `darkmatter-cli` extends sub-spec #3's pipeline:

1. `DarkmatterPage::new(&terminal)`
2. `apply_cli_layout_flags(page, &cli)` — global and component-specific CLI flags
3. `darkmatter::style::apply_page_style(page, &style, page_overrides)`
4. `darkmatter::style::apply_component_style(page, &style, component_overrides)` — table/image/block-quote frontmatter
5. `darkmatter::style::apply_list_style(page, &style, list_overrides)` — ul/ol/li frontmatter

### Public API

The list-level wiring is exposed through two public items in `darkmatter::style`:

```rust
/// CLI fields that already claimed list-level layout values.
///
/// Built by darkmatter-cli from `Cli` after applying broadcast-then-granular
/// precedence rules for list flags.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ListStyleOverrides {
    pub ul_alignment: bool,
    pub ul_fill: bool,
    pub ul_left_margin: bool,
    pub ol_alignment: bool,
    pub ol_fill: bool,
    pub li_alignment: bool,
    pub li_fill: bool,
}

/// Apply parsed ul/ol/li style onto a DarkmatterPage builder.
///
/// CLI overrides suppress frontmatter for overlapping list fields. The returned
/// page has active sub-spec #4 settings applied.
pub fn apply_list_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
    overrides: ListStyleOverrides,
) -> Result<DarkmatterPage, StyleApplyError>;
```

`ListStyleOverrides` is CLI-agnostic by design: `darkmatter-cli` constructs it from the parsed `Cli` struct after shorthand expansion, then passes it to `apply_list_style`. Both `render_terminal_output` and `html_artifact` in `darkmatter/cli/src/output.rs` call this helper after `apply_component_style` and before rendering.

### Example

```yaml
---
style:
    ul:
        left-margin: 4ch
        max-width: 40
    ol:
        alignment: right
    li:
        alignment: right
---
```

### Errors

`apply_list_style` returns `StyleApplyError`:

- `ComponentWidthConflict { bucket }` — `width` and `max-width` set together inside the same list bucket.
- `ComponentInvalidCssLength { bucket, field }` — `width` or `max-width` is a `Length::Css(_)` value.
- List-indent apply error — `with_list_left_margin` called with a non-`Ul` component returns a clear error.

### Inactive Keys

Color and background-color knobs on list buckets (`style.ul.color`, `style.ul.bg-color`, `style.ol.color`, `style.ol.bg-color`, `style.li.color`, `style.li.bg-color`) parse cleanly but emit `KnownButInactive { sub_spec: 5 }` until color application lands.

## Style Mutation

### Common Mutations

Each of the component buckets embeds a `CommonStyle` providing five shared mutations:

| Mutation | Wire key | Value shape | Notes |
|---|---|---|---|
| `width` | `width` | `Nch` or `N%` | Fixed width; mutually exclusive with `max-width` within the same bucket |
| `max-width` | `max-width` | `Nch` or `N%` | Upper bound; mutually exclusive with `width` within the same bucket |
| `alignment` | `alignment` | `left` \| `center` \| `right` | Overrides page-level broadcast for this component |
| `color` | `color` | Tailwind, hex, or web named | Sub-spec #5 wiring pending |
| `bg-color` | `bg-color` | Tailwind, hex, or web named | Sub-spec #5 wiring pending |

### Bespoke Style

While every component provides the common mutations, several components offer additional bespoke properties:

- **`page`** — `stylesheet` (CSS file path or URL), `meta` (opaque map), `code` (code block theme), `background` (`transparent` | `subtle` | `pronounced`)
- **`hr`** — `kind` replaces the legacy per-block `style: waves` attribute (`darkmatter/lib/src/markdown/block/hr_builder.rs:117`). Migration to `style.hr.*` is sub-spec #6.
- **`hyperlinks`** — `local-style` provides a `CommonStyle` override for file-local links only
- **`images`** — `local-style` provides a `CommonStyle` override for file-local links only
- **`ul`** — `left-margin` controls list indent (e.g. `style.ul.left-margin: 4ch`)

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
