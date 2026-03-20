# TextBlock

A uniformly styled block of text for terminal output. Applies consistent styling (colors, font weight, italic, underline, strikethrough) across the entire content via ANSI escape sequences. Unlike `Prose` which supports inline markup, TextBlock applies a single style to the whole block.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;

// Basic usage
let block = TextBlock::new("Hello, World!");

// Builder pattern for styling
let mut styled = TextBlock::new("Important message");
styled
    .using_bold_text()
    .with_foreground_color(Color::BasicColor(BasicColor::Red))
    .with_background_color(Color::BasicColor(BasicColor::BrightBlack));

// Italic text
let mut italic = TextBlock::new("Emphasized text");
italic.using_italics();

// Strikethrough
let mut struck = TextBlock::new("Removed text");
struck.use_strikethrough_on_content();

// Render
let term = Terminal::default();
println!("{}", styled.display(&term));
```

### Key API

| Method | Description |
|--------|-------------|
| `TextBlock::new(text)` | Create with content |
| `.using_bold_text()` | Enable bold styling |
| `.using_italics()` | Enable italic styling |
| `.use_strikethrough_on_content()` | Enable strikethrough |
| `.with_foreground_color(Color)` | Set text color |
| `.with_background_color(Color)` | Set background color |

### Supported Styles

- **Font weight**: Normal, Bold, Dim
- **Colors**: Foreground and background (Basic, RGB, Web, Tailwind)
- **Italic**: Italic text
- **Strikethrough**: Strikethrough text
- **Blink**: Blinking text (rarely supported by terminals)
- **Underline**: Single, double, or curl underlining

## CLI

Not directly exposed as a CLI command. TextBlock is a programmatic building block for applying uniform styling to text blocks. For inline styling with markup, use `Prose` instead.
