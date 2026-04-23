# HorizontalRule Component

The `HorizontalRule` component provides customizable horizontal separator lines for terminal and browser rendering.

## Struct Definition

```rust
pub struct HorizontalRule {
    pub style: RuleStyle,
    pub placement: RulePlacement,
    pub weight: RuleWeight,
    pub width: Option<String>,
    pub color: Option<String>,
}
```

## Enums

### RuleStyle

Defines the visual style of the horizontal rule:

```rust
pub enum RuleStyle {
    Dashes,
    Dots,
    Waves,
    LineStar,
    LineCircle,
    InsetLine,
    CurtainRod,
}
```

### RulePlacement

Defines the horizontal placement of the rule:

```rust
pub enum RulePlacement {
    Full,
    Centered,
    Left,
    Right,
}
```

### RuleWeight

Defines the thickness/weight of the rule:

```rust
pub enum RuleWeight {
    Thin,
    Medium,
    Thick,
}
```

## Trait Implementations

### Renderable (Terminal Rendering)

The `HorizontalRule` implements the `Renderable` trait for terminal output with three-tier progressive enhancement:

1. **Tier 1**: SVG→PNG via resvg with `TerminalImage` (when terminal supports inline images)
2. **Tier 2**: Unicode fallback characters (when terminal supports Unicode)
3. **Tier 3**: ASCII fallback characters (basic compatibility)

### BrowserRenderable (Browser Rendering)

The `HorizontalRule` implements the `BrowserRenderable` trait for web output:

- Generates SVG with `stroke="currentColor"` for proper CSS inheritance
- Uses CSS variables for dynamic scaling and theming
- Supports all style, placement, weight, width, and color attributes

## Usage Example

```rust
use biscuit_terminal::components::{HorizontalRule, RuleStyle, RulePlacement, RuleWeight};

let rule = HorizontalRule {
    style: RuleStyle::Waves,
    placement: RulePlacement::Centered,
    weight: RuleWeight::Medium,
    width: Some("75%".to_string()),
    color: None,
};

// Terminal rendering
rule.render(&mut terminal)?;

// Browser rendering  
let svg = rule.render_to_browser();
```

## Style Matrix

| Style | SVG | Unicode | ASCII |
|-------|-----|---------|-------|
| Dashes | SVG path | `─` | `-` |
| Dots | SVG circles | `•` | `*` |
| Waves | SVG wave path | `~` | `~` |
| LineStar | SVG with stars | `*` | `*` |
| LineCircle | SVG with circles | `○` | `o` |
| InsetLine | SVG inset effect | `═` | `=` |
| CurtainRod | SVG curtain rod | `≡` | `=` |