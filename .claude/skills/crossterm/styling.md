# Styling

The `style` module enables text and background color manipulation along with text attributes like bold, italic, and underline.

## Key Concepts

Styling affects the appearance of text at the current cursor position. Styles persist until reset or changed.

## Colors

### Named Colors

```rust
use crossterm::{
    execute,
    style::{Color, SetForegroundColor, SetBackgroundColor, ResetColor, Print},
};
use std::io::stdout;

// Use named colors
execute!(
    stdout(),
    SetForegroundColor(Color::Blue),
    SetBackgroundColor(Color::Yellow),
    Print("Blue on yellow"),
    ResetColor
)?;
```

**Available named colors:**
- `Black`, `DarkGrey`, `Grey`, `White`
- `Red`, `DarkRed`
- `Green`, `DarkGreen`
- `Yellow`, `DarkYellow`
- `Blue`, `DarkBlue`
- `Magenta`, `DarkMagenta`
- `Cyan`, `DarkCyan`

### RGB Colors

```rust
execute!(
    stdout(),
    SetForegroundColor(Color::Rgb { r: 255, g: 128, b: 0 }),
    Print("Orange text using RGB"),
    ResetColor
)?;
```

**Gotcha:** Not all terminals support true color (24-bit RGB). Older terminals will approximate colors. Test on target terminals.

### ANSI Colors

```rust
execute!(
    stdout(),
    SetForegroundColor(Color::AnsiValue(208)), // Orange-ish
    Print("ANSI 256-color"),
    ResetColor
)?;
```

**When to use:** ANSI 256-color mode has better terminal compatibility than RGB while offering more colors than the 16 named colors.

## Text Attributes

```rust
use crossterm::{
    execute,
    style::{SetAttribute, Attribute, Print, ResetColor},
};

// Apply individual attributes
execute!(
    stdout(),
    SetAttribute(Attribute::Bold),
    Print("Bold text"),
    ResetColor
)?;

execute!(
    stdout(),
    SetAttribute(Attribute::Italic),
    Print("Italic text"),
    ResetColor
)?;

execute!(
    stdout(),
    SetAttribute(Attribute::Underlined),
    Print("Underlined text"),
    ResetColor
)?;
```

**Available attributes:**
- `Bold`, `Dim`, `Italic`, `Underlined`, `DoubleUnderlined`
- `SlowBlink`, `RapidBlink`
- `Reverse` (swap foreground/background)
- `Hidden`, `CrossedOut`
- `Fraktur` (rarely supported)
- `NoBold`, `NoItalic`, `NoUnderline`, etc. (reset individual attributes)
- `Reset` (reset all attributes)

## Styled Content API

The `Stylize` trait provides a convenient method-chaining API:

```rust
use crossterm::{
    execute,
    style::{Stylize, PrintStyledContent},
};

execute!(
    stdout(),
    PrintStyledContent("Bold text".bold()),
    PrintStyledContent("Italic text".italic()),
    PrintStyledContent("Red text".red()),
    PrintStyledContent("Bold red text".red().bold())
)?;
```

**Methods available:**
- Color: `.black()`, `.red()`, `.green()`, `.yellow()`, `.blue()`, `.magenta()`, `.cyan()`, `.white()`, `.grey()`, `.dark_red()`, etc.
- Background: `.on_black()`, `.on_red()`, `.on_green()`, etc.
- Attributes: `.bold()`, `.italic()`, `.underlined()`, `.dim()`, `.reverse()`, `.crossed_out()`
- Custom: `.with(color)`, `.on(color)`

## Common Patterns

### Color Palette

```rust
use crossterm::style::{Color, Stylize};

fn success(msg: &str) -> String {
    msg.green().to_string()
}

fn error(msg: &str) -> String {
    msg.red().bold().to_string()
}

fn warning(msg: &str) -> String {
    msg.yellow().to_string()
}

fn info(msg: &str) -> String {
    msg.blue().to_string()
}

// Usage
println!("{}", success("Operation completed"));
println!("{}", error("Fatal error!"));
```

### Resetting Styles

```rust
use crossterm::{execute, style::{ResetColor, Print}};

execute!(
    stdout(),
    SetForegroundColor(Color::Blue),
    Print("Blue text"),
    ResetColor,  // Reset to terminal default
    Print("Default text")
)?;
```

**Critical:** Always reset colors after use. Forgetting to reset leaves the terminal in a styled state, affecting subsequent output.

### Multiple Attributes

```rust
use crossterm::style::Stylize;

// Chain multiple styles
let styled = "Important!".red().bold().underlined();
println!("{}", styled);
```

### Custom RGB Function

```rust
use crossterm::style::Color;

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb { r, g, b }
}

// Usage
execute!(
    stdout(),
    SetForegroundColor(rgb(255, 128, 0)),
    Print("Custom orange")
)?;
```

### Terminal Capability Detection

```rust
use crossterm::style::available_color_count;

match available_color_count() {
    0..=16 => println!("Basic 16-color terminal"),
    17..=256 => println!("256-color terminal"),
    _ => println!("True color (24-bit RGB) terminal"),
}
```

## Gotchas

### Style Persistence

**Issue:** Styles persist across multiple prints until explicitly reset.

```rust
// Bad - color persists
execute!(stdout(), SetForegroundColor(Color::Red))?;
println!("Red text");
println!("Still red!"); // Unintended

// Good - reset after use
execute!(
    stdout(),
    SetForegroundColor(Color::Red),
    Print("Red text"),
    ResetColor
)?;
println!("Default color");
```

### Terminal Capability Variations

**Issue:** Not all terminals support all attributes (especially italic and double underline).

**Solution:** Test on target terminals and provide fallbacks:

```rust
// Primary style with fallback
if supports_italic() {
    execute!(stdout(), SetAttribute(Attribute::Italic))?;
} else {
    execute!(stdout(), SetForegroundColor(Color::Blue))?;
}
```

## Related

- [Cursor Control](./cursor.md) - For positioning styled text
- [Terminal Control](./terminal.md) - For screen management
