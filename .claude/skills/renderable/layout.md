---
description: Target-agnostic layout configuration in the renderable library — Layout, Alignment, Margin, RowFill, and MaxWidth.
---

# Layout Module

Target-agnostic layout configuration data. Terminal ANSI-width application lives in `biscuit-terminal` as the `LayoutTerminalExt` extension trait.

## Layout

Controls margins, alignment, word-wrapping, and background color.

```rust
use renderable::layout::{Layout, Margin, Alignment, WordWrap};

// Centered content with margins
let layout = Layout {
    left_margin: Margin::Chars(4),
    right_margin: Margin::Chars(4),
    alignment: Alignment::Center,
    ..Default::default()
};

// Word-wrapped content at 50% width
let wrapped = Layout {
    left_margin: Margin::Percent(25.0),
    right_margin: Margin::Percent(25.0),
    word_wrap: WordWrap::WrapProse(Some(8), Some(4)),
    ..Default::default()
};
```

## Alignment

Horizontal alignment within available width.

```rust
use renderable::layout::Alignment;

Alignment::Left    // default
Alignment::Center
Alignment::Right
```

Per-line vs block alignment depends on the terminal layout method used (see biscuit-terminal).

## Margin

Whitespace around content. Can be fixed, percentage-based, or composed.

```rust
use renderable::layout::Margin;

Margin::None                    // No margin
Margin::Chars(4)               // Fixed 4 characters
Margin::Percent(10.0)          // 10% of terminal width
Margin::Percent(10.0).add_chars(4)  // 10% + 4 characters (deferred resolution)
```

### Composition

```rust
// Common cases are optimized:
Margin::None.add_chars(4)           // → Margin::Chars(4)
Margin::Chars(2).add_chars(3)       // → Margin::Chars(5)
Margin::Percent(10.0).add_chars(4)  // → Margin::Offset(Percent(10.0), 4)
```

### Resolution

Margins resolve to character counts at render time:

```rust
use renderable::layout::{Layout, Margin};

assert_eq!(Layout::resolve_margin(&Margin::Chars(4), 80), 4);
assert_eq!(Layout::resolve_margin(&Margin::Percent(10.0), 100), 10);
assert_eq!(Layout::resolve_margin(&Margin::None, 80), 0);

let nested = Margin::Percent(10.0).add_chars(4).add_chars(2);
assert_eq!(Layout::resolve_margin(&nested, 100), 16); // 10 + 4 + 2
```

## RowFill

Row padding strategy for text blocks.

```rust
use renderable::layout::RowFill;

RowFill::Auto   // Pad only when background color is not default (default)
RowFill::Fill   // Always pad to max width
RowFill::Exact  // Never add padding
```

## MaxWidth

Width constraints.

```rust
use renderable::layout::MaxWidth;

MaxWidth::None
MaxWidth::Chars(80)
MaxWidth::Percent(80.0)
```

## Available Width

```rust
use renderable::layout::{Layout, Margin};

let layout = Layout {
    left_margin: Margin::Chars(10),
    right_margin: Margin::Chars(10),
    ..Layout::default()
};

assert_eq!(layout.available_width(80), 60);
```
