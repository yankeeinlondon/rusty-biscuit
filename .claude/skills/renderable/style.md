---
description: Target-agnostic appearance configuration in the renderable library — Style, PerMode, TextEmphasis, Border, and Fill.
---

# Style Module

`renderable::style` is the single, target-agnostic **appearance** primitive —
the sibling of `Layout`. `Layout` decides *where the box sits*; `Style`
decides *what the box looks like*. A component declares a `Style`; the tree
renderers apply it. A component never hand-writes ANSI or CSS.

The style rides on render-tree nodes via `NodeAttrs::set_style` and is
consumed by the Terminal renderer first (lowered to ANSI SGR, box-drawing
characters, and background bands, with capability-aware degradation). The
Markdown renderer deliberately ignores it, so Markdown output is unaffected
by appearance. Browser lowering to CSS is sketched but deferred.

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

`Style::default()` is all-none / all-false; `Style::is_empty()` reports it.

Fields: `color`, `background`, `emphasis`, `border`, `fill`.

### Inheritance

Only the **text-appearance** fields inherit through render-tree traversal:

- `color` — a `None` child color falls back to the parent's color.
- `emphasis` — the union of the parent's and child's emphasis.

`background`, `border`, and `fill` are box-painting properties and **never
inherit** — they stay explicit on the node that paints them.
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

`UnderlineStyle`: `Straight`, `Double`, `Curly`, `Dotted`, `Dashed`.
`EmphasisLayer`: `Weight`, `Italic`, `Underline`, `Strikethrough`, `Blink` —
each knows its `sgr_reset()` code.

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

## Fill

How a component paints its band of available width — does **not** inherit.
`Fill` models painted-band *behavior* and is intentionally separate from
`Style::background` (plain adaptive color).

- `FillIntensity`: `Transparent`, `Subtle` (default), `Pronounced`.
- `FillBand`: `Full` (default), `Padded`, `Indented`.

## Carrying Style on the Render Tree

`Style` is stored on a node via `NodeAttrs::set_style` (the `renderable.style`
hint namespace) and recovered with `NodeAttrs::style`.

```rust
use renderable::style::Style;
use renderable::tree::NodeAttrs;

let mut attrs = NodeAttrs::default();
attrs.set_style(&Style::default());
assert_eq!(attrs.style(), Some(Style::default()));
```

`Style`, `PerMode`, `Border`, `Fill`, and the emphasis leaves all derive
`serde` with `snake_case` enum casing, so a style serializes with the tree.

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
