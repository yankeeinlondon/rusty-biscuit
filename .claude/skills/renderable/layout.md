---
description: Target-agnostic layout configuration in the renderable library — Layout, TargetValue, Length, Edges, Width, and Alignment.
hash: 5aebd30fe1ddb962-385386ef1295d3c9
---

# Layout Module

`renderable::layout` is the single, target-agnostic layout primitive. It
describes a **block-level** component's CSS box — margins, padding, content-box
width, max-width, alignment within the parent, and content wrapping. Appearance
(background, border) is a `Style` concern and is *not* represented here; the
painted inner gutter is `padding` (geometry, here) + `Style.background` (paint).

The layout rides on render-tree nodes via `NodeAttrs::set_layout` and is
consumed by the Browser renderer (lowered to inline CSS) and the Terminal
renderer (margins resolved to cells, block alignment, and `max_width` cap).
The Markdown renderer deliberately ignores it. `max_width` is honored on both
Terminal and Browser.

Terminal ANSI-width application lives in `biscuit-terminal` as the
`LayoutTerminalExt` extension trait.

> The legacy `LayoutHints`, the old `MaxWidth` / `RowFill` enums, and the old
> `Margin` enum (`Margin::Chars` / `Margin::Percent` / `Margin::Offset`) have
> been removed. There is now one `Layout` struct. The four-sided box is named
> `Edges` (no `Margin` type alias remains).

## Layout

```rust
use renderable::layout::{Layout, Edges, Width, Alignment, Length, TargetValue, WordWrap};

let layout = Layout {
    margin: Edges::x(Length::ch(4)),     // transparent outer space
    padding: Edges::x(Length::ch(2)),    // reserved inner space; painted by Style.background
    width: Width::FitContent,            // size the content box to the widest line
    alignment: Alignment::Center,
    max_width: Some(TargetValue::universal(Length::ch(80))),
    word_wrap: WordWrap::WrapProse(Some(8), None),
};
```

`Layout` follows the CSS box model: `margin` is transparent outer space,
`padding` is reserved inner space that `Style.background` paints, and `width`
is the content-box sizing mode (orthogonal to the `max_width` cap). Total
horizontal occupancy is `margin + border + padding + width` (content-box
sizing, CSS default).

`Layout::default()` is zero margins, zero padding, `Width::Auto`,
`Alignment::Left`, no `max_width`, and `WordWrap::None` — note `word_wrap` is
the hand-written default, *not* `WordWrap::default()` (which is a wrapping
policy). `Layout::default()` paired with `Style::default()` is bit-identical to
a node with no layout/style config — a node with *no* `Layout` and *no* `Style`
attr renders identically and skips the styling pass entirely.

`Layout::validate()` (and `Edges::validate()` / `Width::validate()` /
`TargetValue::validate()`) returns the first `LayoutError` from `margin`,
`padding`, `width`, or `max_width`. Validation is **opt-in**: a caller
constructing a `Layout` should call it, but the render pipeline does not — once
a `Layout` is on the tree the renderers lower it as-is. Only *placement*
(block-only) is enforced, by tree validation.

## Length

The universal layout unit.

```rust
use renderable::layout::Length;

Length::Zero            // unit-independent zero
Length::ch(4)           // 4 whole cells (columns horizontally, rows vertically)
Length::percent(10.0)?  // 10% of available width (0.0..=100.0; Result)
Length::css(sizing)     // target-native CSS length — per-target branch only
```

`Zero`, `Ch`, and `Percent` are **universal units** (valid on every target).
`Css` is target-native and valid only inside a per-target `TargetValue`.

## TargetValue&lt;T&gt;

A layout value that is either universal or specified per render target.

```rust
use renderable::layout::{Length, TargetValue};

TargetValue::universal(Length::ch(2))  // same value for every target
// TargetValue::PerTarget(map)         // per-target; non-empty; native units OK
```

A target absent from a `PerTarget` map does not receive that property.

## Edges

A four-sided box (`top` / `right` / `bottom` / `left`), used for **both**
`margin` and `padding`; each side is a `TargetValue<Length>`. Renamed from the
former `Margin` struct so it reads honestly for both fields.

```rust
use renderable::layout::{Edges, Length};

Edges::default()            // all four sides Zero
Edges::all(Length::ch(2))   // every side
Edges::x(Length::ch(4))     // left + right
Edges::y(Length::ch(1))     // top + bottom
```

The browser renderer lowers vertical sides (`top` / `bottom`) to `lh`.

## Width

The content-box sizing mode (CSS `width`), orthogonal to and composing with the
`max_width` cap.

```rust
use renderable::layout::{Width, Length, TargetValue};

Width::Auto                                              // fill the parent's available width (default)
Width::FitContent                                        // size to the content's widest line (CSS fit-content)
Width::Fixed(TargetValue::universal(Length::ch(60)))     // an explicit width
Width::fit_content()                                     // FitContent constructor
```

`Width` serializes `snake_case`: `"auto"`, `"fit_content"`, and `"fixed"`.
`width` and `max_width` are independent, so `FitContent` + a `max_width` cap, or
`Auto` + a cap, are both expressible. The sizing rule, in target units after
`TargetValue` resolves:

```text
base = match width { Auto => available_width,
                     FitContent => content_widest_line,
                     Fixed(n) => n };
used_width = clamp(base, 0, min(available_width, max_width.unwrap_or(available_width)));
```

> The painted-band capabilities of the deleted `Fill` map onto this vocabulary:
> a painted inner gutter is `padding` + `Style.background`; a band hugging the
> text is `Width::FitContent` + `alignment` + `Style.background`. See
> [Style Module](./style.md) for the `Background::subtle()` / `pronounced()`
> tints.

## Alignment

Horizontal alignment within the parent's available width.

```rust
use renderable::layout::Alignment;

Alignment::Left    // default
Alignment::Center
Alignment::Right
```

The tree-renderer terminal path block-aligns the component as a unit.

## LayoutError

```rust
use renderable::layout::LayoutError;

LayoutError::InvalidPercent(f32)    // percentage outside 0.0..=100.0, or non-finite
LayoutError::NonUniversalUnit(_)    // Length::Css used in a Universal branch
LayoutError::EmptyPerTarget         // empty PerTarget map
```

## Carrying Layout on the Render Tree

`Layout` is stored on a node as the typed `NodeAttrs::layout` sparse field
(`Option<Box<Layout>>`) via `NodeAttrs::set_layout`, and recovered with
`NodeAttrs::layout` (clone) or `NodeAttrs::layout_ref` (borrowed, hot path) —
no serde round-trip through the `data` bag.

```rust
use renderable::layout::Layout;
use renderable::tree::NodeAttrs;

let mut attrs = NodeAttrs::default();
attrs.set_layout(&Layout::default());
assert_eq!(attrs.layout(), Some(Layout::default()));
```

Tree validation enforces **block-only** layout: a layout attribute on an
inline node is a validation error ("layout attributes are permitted only on
block-level nodes"). Components emit their `Layout` onto their root node
during tree projection; `TreeRenderable::tree_layout` is the optional hook
for supplying it.