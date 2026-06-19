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

`bt progress` renders a progress bar through the render tree
(`render_terminal_node`), so the typed `ProgressStyle` slot colors are applied
by the terminal tree renderer.

```bash
bt progress 60
bt progress 60 --label Loading
bt progress 75 --width 30 --fill-color green --bracket-color cyan
```

Options:

- `<PERCENT>`: Completion percentage, `0`–`100` (positional, required)
- `--label`: Text shown before the bar
- `--width`: Width of the bar portion in characters
- `--fill-color`: Color of the filled track (named or `#rrggbb`)
- `--empty-color`: Color of the empty track (named or `#rrggbb`)
- `--bracket-color`: Color of the bracket glyphs (named or `#rrggbb`)

Progress is also used programmatically for building progress indicators in TUI
applications and CLI output.
