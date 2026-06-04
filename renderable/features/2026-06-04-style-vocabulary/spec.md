---
status: ready for planning
date: 2026-06-04
owner: ken
parent: renderable/features/2026-06-04-css-box-architecture/spec.md
---

# Style Vocabulary — The CSS-Faithful Layout/Style Types

The root of the [CSS Box Architecture](../2026-06-04-css-box-architecture/spec.md):
the `renderable::layout` and `renderable::style` types every component,
renderer, and `style:` frontmatter key resolves to. Every other sub-spec
builds on the vocabulary fixed here.

## Background

### Today's split, and the smell it created

`renderable` already separates geometry from paint, but the line is in the
wrong place:

```rust
// renderable::layout
Layout { margin: Margin, alignment: Alignment, max_width: Option<…>, word_wrap }
// renderable::style
Style  { color, background, emphasis, border, fill: Option<Fill> }
Fill   { color, intensity: {Transparent|Subtle|Pronounced},
         band: {Full|Padded|Indented}, inset }
```

`Layout` has **no `padding`**. So to express a *painted inner gutter*,
`renderable` invented `Fill` — a primitive that simultaneously reserves inner
space (`inset`), paints a band (`intensity`/`color`), and sizes that band to
the content (`band: Padded`/`Indented`). That bundling of three orthogonal
concerns into one enum is the recurring friction point, and it forces the
deprecated darkmatter `PageFill` to carry a parallel representation bridged at
the edges.

### The CSS insight

In CSS there is no "fill." `background` paints the **content box *and* the
padding box** (out to the border edge) but **not** the margin. So:

- *painted inner gutter* = `padding` + `background`
- *transparent outer space* = `margin`
- *a band hugging the text* = a content-sized box (`width: fit-content`) +
  `background`

Geometry and paint stay orthogonal; the box model is the only vocabulary
needed. Restoring `padding` (and a `fit-content` width mode) to `Layout` lets
`Fill` be **deleted with no loss of capability** — verified against the actual
terminal paint code (`biscuit-terminal/lib/src/render_tree/style.rs`:
`fill_sgr`, `fill_band`), which shows `Fill` is exactly those three bundled
concerns.

## Goals

- Move `renderable::layout::Layout` to the full CSS box model: add `padding`
  and a `width` mode (incl. `fit-content`); keep `max_width` as an orthogonal
  cap.
- Make `Style.background` paint the padding box (CSS semantics).
- Delete `Fill`, `FillBand`, `FillIntensity`; preserve their *conveniences*
  (the `Subtle`/`Pronounced` adaptive tint) as `background` constructors.
- Rename the four-sided `Margin` box to `Edges`; reuse it for both `margin`
  and `padding`.
- Define a **defaulting contract** under which an un-styled node is bit-
  identical to today, and a `style:`-styled node lowers to the same geometry
  and paint — expressed in the new primitives.

## Non-Goals (deferred to sibling sub-specs)

- Teaching the terminal/browser renderers to paint the padding box, honor
  `fit-content`, or lower `padding`/`width` (→ *renderer-folds*).
- `NodeAttrs` storage layout and inheritance semantics (→ *tree-attrs*).
- darkmatter's cutover: `style:` → attrs, deleting `Page*` / `LayoutContext` /
  `build_component_css` (→ *darkmatter-cutover*).
- `min_width` (no `Page*` analog exists; speculative). The orthogonal design
  leaves room to add it later as a third independent field with no rework.

## The vocabulary

### `renderable::layout` — geometry only

```rust
/// A block's CSS box: where it sits and how big it is. Paint is a `Style`
/// concern and is not represented here.
pub struct Layout {
    pub margin:    Edges,                        // transparent outer space
    pub padding:   Edges,                        // reserved inner space; PAINTED by Style.background
    pub width:     Width,                        // how the box sizes itself
    pub max_width: Option<TargetValue<Length>>,  // orthogonal upper cap; None = uncapped
    pub alignment: Alignment,                    // placement within the parent's free width
    pub word_wrap: WordWrap,
}

/// How a block sizes its content box horizontally.
pub enum Width {
    /// Fill the parent's available width (CSS `width: auto` on a block).
    Auto,
    /// Size to the content's widest line (CSS `width: fit-content`).
    FitContent,
    /// An explicit width (cells / percent / per-target CSS length).
    Fixed(TargetValue<Length>),
}
```

`Edges` is today's four-sided `TargetValue<Length>` box, **renamed from
`Margin`** so it reads honestly for both fields. Constructors mirror today's:
`Edges::{all,x,y}` plus `Edges::default()` (all `Length::Zero`).

**Kept as-is** (the resolution machinery is sound): `Length`
(`Zero|Ch|Percent|Css`), `TargetValue` (universal vs. per-target), `PerMode`
(adaptive light/dark), `Alignment`, `WordWrap`.

### `renderable::style` — paint only

```rust
pub struct Style {
    pub color:      Option<TargetValue<PerMode<Color>>>,
    pub background: Option<TargetValue<PerMode<Color>>>,  // paints content + padding box
    pub emphasis:   TextEmphasis,
    pub border:     Option<Border>,
    // fill: DELETED
}
```

**Deleted:** `Fill`, `FillBand`, `FillIntensity`.

**Preserved conveniences** — the only thing `Fill` offered beyond the box model
was the *implicit adaptive tint* of `Subtle`/`Pronounced`. That survives as a
small `Background` helper in `renderable::style` whose constructors return the
`TargetValue<PerMode<Color>>` value `Style.background` already holds, expanding
to an adaptive `PerMode` color matching today's tints (e.g. dark-`subtle` ≙
`rgb(30,30,34)`, dark-`pronounced` ≙ `rgb(50,50,56)`):

```rust
// renderable::style — returns TargetValue<PerMode<Color>>
Background::subtle()       // adaptive PerMode tint, faint
Background::pronounced()   // adaptive PerMode tint, strong
// usage: Style { background: Some(Background::subtle()), ..Default::default() }
```

> `PageBackground::Pronounced`'s **color-mode flip** (inverting the code theme
> for contrast) was never part of `Fill`; it lives in darkmatter's page frame
> and stays there (handled in *darkmatter-cutover*).

## Sizing resolution

One rule, both targets:

```text
base = match width {
    Auto       => available_width,
    FitContent => content_widest_line,
    Fixed(n)   => n,
};
used_width = clamp(base, 0, min(available_width, max_width.unwrap_or(available_width)));
```

`width` and `max_width` are **orthogonal and compose** (CSS `width` +
`max-width`). This is the expressiveness gain over the flat `PageFill` enum,
which could state only one sizing fact at a time:

| Intent | Expression |
|---|---|
| Fill, never wider than 80ch | `width: Auto`, `max_width: 80ch` |
| Centered code block hugging its code, capped at 100ch | `width: FitContent`, `max_width: 100ch`, `align: Center` |
| Fixed 60ch, shrink on narrow terminals | `width: Fixed(60ch)`, `max_width: 100%` |

## `PageFill` / `Fill` mapping (no remainder)

| Deprecated | New expression |
|---|---|
| `PageFill::Full` | `width: Auto` |
| `PageFill::Pad(n)` (painted gutter) | `padding: Edges::x(n)` + `Style.background` |
| `PageFill::Indent(n)` (painted, one-sided) | `padding` on the aligned side + `Style.background` |
| `PageFill::Max(n)` | `max_width: Some(n)` |
| `PageFill::Explicit(n)` | `width: Fixed(n)` |
| `Fill { band: Padded/Indented }` (band hugging text, gutter transparent) | `width: FitContent` + `alignment` + `Style.background` |
| `Fill { intensity: Subtle/Pronounced }` | `Background::subtle()` / `pronounced()` |
| `Fill { inset }` | `padding` (painted) or `margin` (transparent) |
| `WidthUnit::Fixed(n)` / `Percent(p)` | `Length::ch(n)` / `Length::percent(p)` (already total) |

## Defaulting contract

Defaulting is configured at two levels; together they reproduce exactly what a
component with no layout/style config renders today (full width, left, no
offset, no paint, no wrap).

**1. Type-level `Default` — each field's default is the current default it
stands in for:**

| Field | Default | Reproduces today's… |
|---|---|---|
| `Layout.margin` | `Edges::all(Length::Zero)` | `PageMargin::ZERO` |
| `Layout.padding` | `Edges::all(Length::Zero)` | no per-component padding |
| `Layout.width` | `Width::Auto` | `PageFill::Full` / full effective width |
| `Layout.max_width` | `None` | no cap |
| `Layout.alignment` | `Alignment::Left` | `PageAlignment::Left` |
| `Layout.word_wrap` | `WordWrap::None` (hand-written, **not** derived) | the load-bearing non-wrapping default |
| `Style.background` | `None` | `PageBackground::Transparent` |
| `Style.{color,emphasis,border}` | `None` / empty | unstyled |

So `Layout::default() + Style::default()` ≡ today's "no config" node.

**2. Absence is the cheap default (and the perf gate).** A `RenderNode` with
**no `Layout` attr and no `Style` attr** renders with the target's intrinsic
defaults and **skips the styling pass entirely** — the direct replacement for
`LayoutContext::has_layout == false` / `needs_decoration() == false`. Attaching
`Layout::default()` renders *identically* to attaching nothing; absence is just
the cheap path. (The renderer short-circuit itself lands in *renderer-folds*;
this spec fixes the *contract* that absence ≡ default.)

## Acceptance Criteria

1. `renderable::layout::Layout` carries `margin`, `padding`, `width`,
   `max_width`, `alignment`, `word_wrap`; `Width` is `Auto | FitContent |
   Fixed(TargetValue<Length>)`.
2. The four-sided box is named `Edges` and is used by both `margin` and
   `padding`; `Margin` no longer exists as a type name.
3. `renderable::style::Style` has no `fill` field; `Fill`, `FillBand`,
   `FillIntensity` are deleted from `renderable`. `rg 'FillBand|FillIntensity'`
   over `renderable/` returns nothing outside historical feature docs.
4. `Background::subtle()` / `pronounced()` exist in `renderable::style`, return
   `TargetValue<PerMode<Color>>`, and produce adaptive colors equal to the
   former `FillIntensity` tints.
5. `Default` impls match the defaulting table above; `Layout::default() +
   Style::default()` is documented as ≡ today's un-styled node, with a unit
   test pinning each field.
6. `Layout` / `Style` / `Width` / `Edges` serde round-trip; `Layout::validate`
   covers `padding` and `width` (percent ranges, non-universal-unit rules).
7. Because the type change ripples into the terminal renderer and darkmatter,
   the workspace still **compiles and its tests still run** at the end of this
   spec's implementation — consumer updates land in the same coordinated change
   (the detailed renderer/darkmatter behavior is *renderer-folds* /
   *darkmatter-cutover*, but nothing may be left non-compiling).

## Risks

- **Coordinated-change blast radius.** Renaming `Margin → Edges`, adding
  `Layout` fields, and deleting `Fill` break the terminal renderer
  (`render_tree/style.rs`) and darkmatter at once. Mitigation: treat the type
  change + minimal consumer compile-fixes as a single coordinated landing;
  full renderer/darkmatter behavior follows in the sibling sub-specs. Planning
  sequences this.
- **`fit-content` is genuinely new.** It is the one capability `padding +
  background` cannot supply (a band sized to the text with transparent
  gutters). Its terminal realization (`content_widest_line`) must match the
  former `Fill::Padded`/`Indented` band sizing. Mitigation: characterization
  tests against the current `fill_band` output as a reference (not a contract).
- **Adaptive-tint fidelity.** `Background::subtle()`/`pronounced()` must
  reproduce the former tints across color depths. Mitigation: pin the resolved
  RGB + degraded-palette values in unit tests, mirroring `fill_sgr`.

## Related

- [`../2026-06-04-css-box-architecture/spec.md`](../2026-06-04-css-box-architecture/spec.md)
  — architecture overview and the unifying thesis.
- [`renderable/src/layout/`](../../../renderable/src/layout/) /
  [`renderable/src/style.rs`](../../../renderable/src/style.rs) — the types
  this spec revises.
- [`biscuit-terminal/lib/src/render_tree/style.rs`](../../../biscuit-terminal/lib/src/render_tree/style.rs)
  — `fill_sgr` / `fill_band`, the paint behavior `fit-content` + `background`
  must reproduce.
- [`renderable/docs/layout-and-style.md`](../../../renderable/docs/layout-and-style.md)
  — the current model's design notes (incl. the deliberate "no padding in
  `Layout`" decision this spec reverses).
