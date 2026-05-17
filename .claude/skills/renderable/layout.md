---
description: Target-agnostic layout configuration in the renderable library — Layout, TargetValue, Length, Margin, and Alignment.
---

# Layout Module

`renderable::layout` is the single, target-agnostic layout primitive. It
describes a **block-level** component's relationship to its parent — margins,
alignment within the parent, max-width, and content wrapping. Appearance
(background, fill) is a `Style` concern and is *not* represented here.

The layout rides on render-tree nodes via `NodeAttrs::set_layout` and is
consumed by the Browser renderer (lowered to inline CSS) and the Terminal
renderer (cell margins / alignment). The Markdown renderer deliberately
ignores it.

Terminal ANSI-width application lives in `biscuit-terminal` as the
`LayoutTerminalExt` extension trait.

> The legacy `LayoutHints`, the old `MaxWidth` / `RowFill` enums, and the old
> `Margin` enum (`Margin::Chars` / `Margin::Percent` / `Margin::Offset`) have
> been removed. There is now one `Layout` struct.

## Layout

```rust
use renderable::layout::{Layout, Margin, Alignment, Length, TargetValue, WordWrap};

let layout = Layout {
    margin: Margin {
        left: TargetValue::universal(Length::ch(4)),
        right: TargetValue::universal(Length::ch(4)),
        ..Margin::default()
    },
    alignment: Alignment::Center,
    max_width: Some(TargetValue::universal(Length::ch(80))),
    word_wrap: WordWrap::WrapProse(Some(8), None),
};
```

`Layout::default()` is zero margins, `Alignment::Left`, no `max_width`, and
`WordWrap::None`. `Layout::validate()` returns the first `LayoutError` from
its `margin` or `max_width`.

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

## Margin

A four-sided box; each side is a `TargetValue<Length>`.

```rust
use renderable::layout::{Margin, Length};

Margin::default()       // all four sides Zero
Margin::all(Length::ch(2))   // every side
Margin::x(Length::ch(4))     // left + right
Margin::y(Length::ch(1))     // top + bottom
```

The browser renderer lowers vertical sides (`top` / `bottom`) to `lh`.

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

`Layout` is stored on a node via `NodeAttrs::set_layout` and recovered with
`NodeAttrs::layout`.

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
