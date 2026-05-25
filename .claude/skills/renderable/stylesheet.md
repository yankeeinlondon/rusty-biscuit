---
description: Type-safe CSS stylesheet builder in the renderable library — CssStyle, CssRule, Stylesheet, typed properties and values.
---

# Stylesheet Module

Type-safe CSS declaration builder independent of any render target.

## Design

The type system is layered:

1. **Properties** — `CssProp` (runtime) plus typed subsets: `CssSizingProp`, `CssSizingMultiProp`, `CssColorProp`, `CssIntegerProp`, and `CssCustomProp`
2. **Values** — `CssValue` with five categories matching `CssValueKind`: `Sizing`, `SizingMulti`, `Color`, `Integer`, `Raw`
3. **Compile-time checking** — `CssTypedProperty` trait pairs each property subset with its accepted value type

## CssStyle

A declaration block (`property: value` pairs).

```rust
use renderable::stylesheet::{
    CssStyle, CssSizingProp, CssSizing, CssColorProp, CssColor, CssIntegerProp
};

let style = CssStyle::new()
    .add(CssSizingProp::Width, CssSizing::px(320.0))
    .add(CssColorProp::Color, CssColor::rgb(0x33, 0x66, 0x99))
    .add(CssIntegerProp::ZIndex, 10);

assert_eq!(
    style.to_css(),
    "width: 320px;\ncolor: rgb(51, 102, 153);\nz-index: 10;"
);
```

### Output Formats

```rust
// CSS text
let css = style.to_css();

// Pretty-printed JSON
let json = style.to_json();

// JSON5 with unquoted keys and trailing commas
let json5 = style.to_json5();
```

### Dynamic Parsing

```rust
use renderable::stylesheet::CssStyle;

let style = CssStyle::try_from("color: #336699; margin: 8px 16px;")?;
assert_eq!(style.len(), 2);
```

## CssRule

A `(selector, CssStyle)` pair.

```rust
use renderable::stylesheet::{CssRule, CssStyle, CssSizingProp, CssSizing};

let rule = CssRule::new(".my-class", CssStyle::new()
    .add(CssSizingProp::Width, CssSizing::Percent(100.0)));
```

## Stylesheet

An ordered collection of `CssRule` entries.

```rust
use renderable::stylesheet::Stylesheet;

let mut sheet = Stylesheet::new();
sheet.push(rule1);
sheet.push(rule2);

// Iterate entries
for rule in sheet.entries() {
    println!("{} {{ {} }}", rule.selector, rule.style.to_css());
}
```

## Typed Properties and Values

### Sizing

```rust
use renderable::stylesheet::{CssSizingProp, CssSizing, CssUnit};

CssSizing::px(100.0)        // 100px
CssSizing::em(1.5)          // 1.5em
CssSizing::percent(50.0)    // 50%
CssSizing::raw("calc(100% - 20px)") // calc(...)
```

### Color

```rust
use renderable::stylesheet::{CssColorProp, CssColor};

CssColor::rgb(255, 0, 0)
CssColor::hex(0xFF0000)
CssColor::named("red")
CssColor::hsl(120.0, 100.0, 50.0)
```

### Integer

```rust
use renderable::stylesheet::{CssIntegerProp, CssColorProp, CssColor};

CssStyle::new()
    .add(CssIntegerProp::ZIndex, 10)
    .add(CssIntegerProp::Opacity, 1); // Note: opacity is 0-1 float in CSS
```

### Multi-value Sizing

For properties like `margin` and `padding` that accept 1–4 values:

```rust
use renderable::stylesheet::{CssSizingMultiProp, CssSizingMulti, CssSizing};

CssSizingMulti::one(CssSizing::px(8.0))                    // 8px
CssSizingMulti::two(CssSizing::px(8.0), CssSizing::px(16.0)) // 8px 16px
CssSizingMulti::four(/* top right bottom left */)
```

## Custom Properties

```rust
use renderable::stylesheet::{CssCustomProp, CssValue};

let style = CssStyle::new()
    .add(CssCustomProp::new("--primary"), CssColor::hex(0x336699));
```

## Error Handling

```rust
use renderable::stylesheet::StylesheetError;

// try_add and try_from return Result<..., StylesheetError>
let result = CssStyle::try_from("invalid-syntax");
// Err(StylesheetError::ParseError("..."))
```
