# Progress

A horizontal progress bar for terminal display showing completion percentage. Renders with configurable width, fill/empty characters, bracket style, and an optional label.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;

// Basic progress bar (75% complete)
let bar = Progress::new(0.75);
let output = bar.render_optimistic(Some(40));
// Output: [████████████████····] 75%

// With label
let bar = Progress::new(0.42)
    .with_label("Downloading");
// Output: Downloading [████████············] 42%

// Customized appearance
let bar = Progress::new(0.60)
    .with_bar_width(30)
    .with_fill_char('=')
    .with_empty_char('-')
    .with_brackets('<', '>');
// Output: <==================------------>  60%

// Render
let term = Terminal::default();
println!("{}", bar.display(&term));
```

### Key API

| Method | Description |
|--------|-------------|
| `Progress::new(value)` | Create with value (clamped to 0.0..=1.0) |
| `.with_label(str)` | Label displayed before the bar |
| `.with_bar_width(u32)` | Width of the bar portion in characters (default: 20) |
| `.with_fill_char(char)` | Character for filled portion (default: `█`) |
| `.with_empty_char(char)` | Character for empty portion (default: `·`) |
| `.with_brackets(left, right)` | Bracket characters (default: `[` `]`) |

## CLI

Not directly exposed as a CLI command. Progress is a programmatic component used for building progress indicators in TUI applications and CLI output.
