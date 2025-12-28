# Styling

Ratatui provides a comprehensive styling system for colors, text modifiers, and visual effects.

## Style Basics

```rust
use ratatui::style::{Color, Modifier, Style};

// Basic styling
let style = Style::default()
    .fg(Color::Cyan)
    .bg(Color::DarkGray)
    .add_modifier(Modifier::BOLD);

// RGB color support
let custom_style = Style::default()
    .fg(Color::Rgb(255, 128, 0))  // Orange
    .add_modifier(Modifier::ITALIC);

// Combining styles
let combined = style.patch(custom_style);
```

## Colors

### Named Colors

```rust
Color::Black
Color::Red
Color::Green
Color::Yellow
Color::Blue
Color::Magenta
Color::Cyan
Color::Gray
Color::DarkGray
Color::LightRed
Color::LightGreen
Color::LightYellow
Color::LightBlue
Color::LightMagenta
Color::LightCyan
Color::White
Color::Reset  // Terminal default
```

### Indexed Colors (256-color palette)

```rust
Color::Indexed(208)  // Specific palette color
```

### True Color (RGB)

```rust
Color::Rgb(255, 128, 0)  // 24-bit color
```

## Modifiers

```rust
Modifier::BOLD
Modifier::DIM
Modifier::ITALIC
Modifier::UNDERLINED
Modifier::SLOW_BLINK
Modifier::RAPID_BLINK
Modifier::REVERSED
Modifier::HIDDEN
Modifier::CROSSED_OUT
```

Multiple modifiers:
```rust
.add_modifier(Modifier::BOLD | Modifier::ITALIC)
```

## Terminal Compatibility

### Fallback Colors

For maximum compatibility across terminals:

```rust
fn get_color(terminal_type: &str) -> Color {
    match terminal_type {
        "kitty" | "alacritty" => Color::Rgb(255, 128, 0), // True color
        "xterm" => Color::Indexed(208),  // 256-color mode
        "vt100" => Color::LightRed,      // Basic ANSI
        _ => Color::Reset,               // Fallback
    }
}
```

### macOS RGB Issues

Some macOS terminals have RGB color glitches:

```rust
let style = if cfg!(target_os = "macos") {
    Style::default().fg(Color::Indexed(208))
} else {
    Style::default().fg(Color::Rgb(255, 128, 0))
};
```

## Themes

Create consistent themes:

```rust
struct Theme {
    primary: Color,
    secondary: Color,
    background: Color,
    text: Color,
    error: Color,
    success: Color,
}

const DARK_THEME: Theme = Theme {
    primary: Color::Cyan,
    secondary: Color::Magenta,
    background: Color::Black,
    text: Color::White,
    error: Color::Red,
    success: Color::Green,
};

fn styled_block(theme: &Theme) -> Block {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.primary))
        .style(Style::default().bg(theme.background))
}
```

## Unicode and Font Handling

### ASCII Fallbacks

Provide ASCII alternatives for terminals without Unicode support:

```rust
pub fn border_symbols(use_unicode: bool) -> BorderSet {
    if use_unicode {
        BorderSet {
            top_left: "┌",
            top_right: "┐",
            bottom_left: "└",
            bottom_right: "┘",
            vertical_left: "│",
            vertical_right: "│",
            horizontal_top: "─",
            horizontal_bottom: "─",
        }
    } else {
        BorderSet {
            top_left: "+",
            top_right: "+",
            bottom_left: "+",
            bottom_right: "+",
            vertical_left: "|",
            vertical_right: "|",
            horizontal_top: "-",
            horizontal_bottom: "-",
        }
    }
}
```

## Stylize Trait

Use the `Stylize` trait for fluent styling:

```rust
use ratatui::style::Stylize;

let text = "Hello".blue().bold().on_black();
let span = Span::raw("World").yellow().italic();
```

## Best Practices

1. **Test across terminals** - Different terminals have varying color support
2. **Provide fallbacks** - Use indexed or named colors for compatibility
3. **Be consistent** - Create theme structs for unified styling
4. **Consider accessibility** - Ensure sufficient contrast for readability
5. **Avoid hardcoded colors** - Make colors configurable for user preferences
