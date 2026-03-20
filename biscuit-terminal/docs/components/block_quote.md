# BlockQuote

Renders quoted text with a distinctive left border (`│ `), commonly used to highlight quoted content, testimonials, or notable passages.

Each line is prefixed with `│ ` (U+2502 + space). An optional attribution line can be added, preceded by `—`. Content is automatically word-wrapped to fit the terminal width. Custom colors can be applied to the text, background, and left border independently.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;

// Simple block quote from a string
let quote = BlockQuote::from("The only way to do great work is to love what you do.");

// Block quote with attribution
let quote = BlockQuote::new(
    "To be, or not to be, that is the question.".into(),
    Some("William Shakespeare"),
);

// Styled with custom colors
let quote = BlockQuote::from("Important quote")
    .with_text_color(Color::Tailwind(TailwindColor::White))
    .with_bg_color(Color::Tailwind(TailwindColor::Gray800))
    .with_left_block_color(Color::Tailwind(TailwindColor::Blue400));

// Rich content via Prose
let prose = Prose::new("This is <b>bold</b> and <i>italic</i> text.");
let quote = BlockQuote::new(
    RenderableContent::from(prose),
    Some("Anonymous"),
);

// Render
let term = Terminal::default();
println!("{}", quote.display(&term));
```

### Key API

| Method | Description |
|--------|-------------|
| `BlockQuote::from(text)` | Create from a string (no attribution) |
| `BlockQuote::new(content, attribution)` | Create with `RenderableContent` and optional attribution |
| `.with_text_color(Color)` | Set text foreground color |
| `.with_bg_color(Color)` | Set background color |
| `.with_left_block_color(Color)` | Set left border color |

## CLI

Exposed via `bt quote`:

```bash
bt quote "To be, or not to be" --attribution "Shakespeare"
```
