---
description: Cross-target color types in the renderable library — Color, CssColor, WebColor, BasicColor, Tailwind, RGB, and HDR support.
---

# Color Module

Cross-target color system shared across render targets.

## Color

The main color enum used throughout the library.

```rust
use renderable::color::Color;

let color = Color::rgb(255, 0, 0);
let color = Color::hex(0xFF0000);
let color = Color::named("red");
```

## CssColor

CSS-compatible color values for stylesheet generation.

```rust
use renderable::stylesheet::CssColor;

CssColor::rgb(255, 0, 0)           // rgb(255, 0, 0)
CssColor::rgba(255, 0, 0, 0.5)     // rgba(255, 0, 0, 0.5)
CssColor::hex(0xFF0000)            // #FF0000
CssColor::hsl(120.0, 100.0, 50.0)  // hsl(120, 100%, 50%)
CssColor::named("cornflowerblue")   // cornflowerblue
```

## WebColor

Web-standard named colors.

```rust
use renderable::color::WebColor;

let color = WebColor::CornflowerBlue;
let color = WebColor::RebeccaPurple;
```

## BasicColor

Basic terminal/ANSI colors.

```rust
use renderable::color::BasicColor;

let color = BasicColor::Red;
let color = BasicColor::BrightCyan;
```

## Tailwind

Tailwind CSS color palette with shade variants.

```rust
use renderable::color::Tailwind;

let color = Tailwind::Blue(500);   // text-blue-500
let color = Tailwind::Emerald(600);
let color = Tailwind::Slate(200);
```

## RgbColor and Octet

RGB representation with 8-bit per channel.

```rust
use renderable::color::{RgbColor, Octet};

let rgb = RgbColor::new(255, 128, 0);
let red = Octet(255);
```

## HdrColor

HDR color support for high-dynamic-range rendering.

```rust
use renderable::color::HdrColor;

let hdr = HdrColor::new(1.5, 0.8, 0.2); // Values can exceed 1.0
```

## Conversions

Colors can be converted between representations:

```rust
use renderable::color::{Color, CssColor, RgbColor};

let color = Color::rgb(255, 0, 0);
let css: CssColor = color.into();
let rgb: RgbColor = css.into();
```
