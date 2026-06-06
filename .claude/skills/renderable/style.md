---
description: Target-agnostic appearance configuration in the renderable library — Style, PerMode, TextEmphasis, Border, and Background.
---

# Style Module

`renderable::style` is the single, target-agnostic **appearance** primitive —
the sibling of `Layout`. `Layout` decides *where the box sits*; `Style`
decides *what the box looks like*. A component declares a `Style`; the tree
renderers apply it. A component never hand-writes ANSI or CSS.

The style rides on render-tree nodes via `NodeAttrs::set_style` and is
consumed by the Terminal renderer first (lowered to ANSI SGR, box-drawing
characters, and background bands, with capability-aware degradation). The
Browser renderer lowers text appearance and box colors to HTML/CSS. The
Markdown renderer deliberately ignores it, so Markdown output is unaffected
by appearance.

`Style` may attach to **block nodes and inline `Span` nodes** — it is not
block-only (unlike `Layout`).

## Style

```rust
use renderable::style::{Style, Border, BorderSides, PerMode};
use renderable::layout::TargetValue;
use renderable::color::{Color, Tailwind};

let style = Style {
    color: Some(TargetValue::universal(PerMode::adaptive(
        Color::Tailwind(Tailwind::Blue700),
        Color::Tailwind(Tailwind::Blue300),
    ))),
    background: Some(TargetValue::universal(
        PerMode::universal(Color::Tailwind(Tailwind::Slate100)),
    )),
    ..Style::default()
};
```

`Style::default()` is all-none / all-false; `Style::is_empty()` reports it. A
`Style` carrying only `Background::subtle()` / `pronounced()` is **not** empty.

Fields: `color`, `background`, `emphasis`, `border`. (`background` paints the
component's content box *and* its `Layout.padding` box, matching CSS — there is
no separate `fill` field.)

### Inheritance

Only the **text-appearance** fields inherit through render-tree traversal:

- `color` — a `None` child color falls back to the parent's color.
- `emphasis` — the union of the parent's and child's emphasis.

`background` and `border` are box-painting properties and **never inherit** —
they stay explicit on the node that paints them.
`Style::inherited_from(&parent)` computes the effective child style.

## PerMode&lt;T&gt;

A style value that adapts to the terminal / page background color mode.

```rust
use renderable::style::PerMode;

PerMode::universal(value)        // one value for every background mode
PerMode::adaptive(light, dark)   // distinct light / dark values
```

`resolve(mode)` returns `light` for `ColorMode::Light` and `dark` for both
`ColorMode::Dark` and `ColorMode::Unknown`. Composes with `TargetValue` as
the common `TargetValue<PerMode<Color>>` shape: `TargetValue` selects per
render target, `PerMode` then adapts to light/dark within a target.

## TextEmphasis

The shared, target-neutral text weight and decoration leaf — reused by
`biscuit-terminal`'s `Prose` and by `Style::emphasis`.

```rust
use renderable::style::{TextEmphasis, UnderlineStyle};

let emphasis = TextEmphasis {
    bold: true,
    italic: true,
    underline: Some(UnderlineStyle::Curly),
    ..TextEmphasis::default()
};
```

- `sgr_ops()` — non-underline SGR open codes, in nesting order. Underline is
  excluded because its escape depends on a capability-aware degradation
  decision the terminal emitter makes (pair the `underline` field with
  `UnderlineStyle::sgr_open`).
- `html_wrappers()` — `(open, close)` HTML pairs; semantic styles use
  semantic HTML (`<strong>`, `<em>`, `<s>`).
- `inherited_from(&parent)` — boolean union; the underline variant falls back
  to the parent's when this side has none.

`TextEmphasis` fields: `bold`, `dim`, `italic`, `strikethrough`, `blink`,
`inverse` (reverse video; SGR set `7` / reset `27`, browser `filter:invert(1)`),
and `underline`. `inverse` is `#[serde(default)]`, so render trees serialized
before it existed deserialize with `inverse == false`.

`UnderlineStyle`: `Straight`, `Double`, `Curly`, `Dotted`, `Dashed`.
`EmphasisLayer`: `Weight`, `Italic`, `Underline`, `Strikethrough`, `Blink`,
`Inverse` — each knows its `sgr_reset()` code.

## Border

A component's border appearance — does **not** inherit.

```rust
use renderable::style::{Border, BorderWeight, BorderLineStyle, BorderSides};

let border = Border {
    weight: BorderWeight::Thin,
    line_style: BorderLineStyle::Solid,
    sides: BorderSides::Sides { top: false, right: false, bottom: false, left: true },
    ..Border::default()
};
```

- `BorderWeight`: `Thin` (default), `Medium`, `Thick`.
- `BorderLineStyle`: `Solid` (default), `Dashed`, `Dotted`, `Double`.
- `BorderSides`: `All` (default), `None`, or per-side `Sides { top, right,
  bottom, left }` — addresses each edge without a separate `LeftBorder` type.

## Background

`Background` is a zero-sized **constructor namespace**, not a stored type. Its
constructors return the `TargetValue<PerMode<Color>>` value `Style.background`
already holds, so serialized `Style.background` still contains only a color.
They preserve the adaptive tints the deleted `Fill` intensities supplied
implicitly.

```rust
use renderable::style::{Style, Background};

let style = Style {
    background: Some(Background::subtle()),   // faint adaptive tint (former subtle fill tint)
    ..Style::default()
};
Background::pronounced();   // strong adaptive tint (former pronounced fill tint)
```

- `Background::subtle()` — `rgb(235,235,238)` light / `rgb(30,30,34)` dark.
- `Background::pronounced()` — `rgb(215,215,220)` light / `rgb(50,50,56)` dark.

The former fill abstraction (its band, intensity, and inset knobs) is
**deleted**. Its capabilities are expressed with the CSS box model: a painted
inner gutter is `Layout.padding` + `background`; a band hugging the text is
`Layout.width: FitContent` + `alignment` + `background`. See
[Layout Module](./layout.md) for `Width` and `Edges`.

## Carrying Style on the Render Tree

`Style` is stored on a node as the typed `NodeAttrs::style` sparse field
(`Option<Box<Style>>`) via `NodeAttrs::set_style`, and recovered with
`NodeAttrs::style` (clone) or `NodeAttrs::style_ref` (borrowed, hot path) — no
serde round-trip through the `data` bag.

```rust
use renderable::style::Style;
use renderable::tree::NodeAttrs;

let mut attrs = NodeAttrs::default();
attrs.set_style(&Style::default());
assert_eq!(attrs.style(), Some(Style::default()));
```

`Style`, `PerMode`, `Border`, and the emphasis leaves all derive `serde` with
`snake_case` enum casing, so a style serializes with the tree. (serde ignores
the now-unknown `fill` key, so a render tree serialized before `fill` was
deleted still deserializes.)

## Component Style Slots

Rich `biscuit-terminal` components expose **typed component style structs**
rather than scattered bespoke fields:

- `TableStyle` — `Table` row striping (`striped_rows`, `striped_text`).
- `ProgressStyle` — `Progress` track / bracket glyphs (`fill_char`,
  `empty_char`, `left_bracket`, `right_bracket`).

`BlockQuote` and `Section` declare a plain `Style` directly. The former
bespoke builder methods (`with_text_color`, `alternate_background_color`, …)
remain available as compatibility shims that write into the declared `Style`
or the typed slot struct.

## Browser Coverage

Browser lowering covers `color`, `background`, `emphasis`, and `border`:
inline bold/italic/strikethrough use semantic wrappers, block-level emphasis
uses CSS declarations, and underline variants, dim, blink, and inverse
(`filter:invert(1)`) lower to CSS. `border` lowers the full matrix — `weight` →
`border-width` px step, `line_style` → `border-style`, `color` → `border-color`,
`radius` → `border-radius`, with `BorderSides::All` using shorthands and
`Sides { .. }` emitting per-side declarations. `Layout.padding` lowers to
`padding-*` and `Width` to a `width` declaration (`Auto` omits it, `FitContent`
→ `width:fit-content`, `Fixed` → an explicit length). Any node lowering a
non-default `width`, `padding`, or `border` also emits `box-sizing:content-box`,
so a global `border-box` reset cannot reinterpret the width contract. The
Terminal renderer paints the `padding` box with `Style.background`, resolves all
three `Width` modes, and block-places the box (`content_width + padding +
border`) within `available − margin` for every mode — `margin:auto` semantics —
so a sub-available `Fixed` / `FitContent` / `max_width`-capped box is
centered/right-offset as a unit.
