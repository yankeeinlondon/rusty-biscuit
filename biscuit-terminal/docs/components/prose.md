# Prose

Styled text component with token and block tag support for rich terminal output. Prose is the primary text styling component in biscuit-terminal, supporting both atomic tokens (`{{bold}}`) and self-closing block tags (`<bold>text</bold>`) that get rendered as ANSI escape codes.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;

// Atomic tokens (require manual {{reset}})
let prose = Prose::new("{{bold}}Important:{{reset}} normal text");

// Block tags (auto-reset after closing tag)
let prose = Prose::new("<bold>This is bold</bold> and <red>this is red</red>");

// Hyperlinks
let prose = Prose::new(r#"<a href="https://example.com">Click here</a>"#);

// RGB colors
let prose = Prose::new("<rgb #ff0000>Red text</rgb>");

// Nested tags
let prose = Prose::new("<bold><blue>Bold blue text</blue></bold>");

// Escaping literal characters
let prose = Prose::new(r"\<not a tag\>");

// With layout configuration
let prose = Prose::new("Centered content")
    .with_layout(Layout {
        alignment: Alignment::Center,
        ..Layout::default()
    });

// Render
let term = Terminal::default();
println!("{}", prose.display(&term));
```

### Supported Tags/Tokens

**Text Styling**: `bold`, `dim`, `italic`, `underline`, `strikethrough`, `blink`

**Colors** (foreground): `red`, `green`, `blue`, `yellow`, `cyan`, `magenta`, `white`, `black`, plus bright variants (`bright-red`, etc.), Tailwind colors (`gray-800`, `blue-400`), and web colors (`coral`, `salmon`)

**Background Colors**: Prefix with `bg-` (e.g., `<bg-blue>`, `<bg-coral>`)

**Special**: `<a href="url">text</a>` for hyperlinks, `<rgb #hex>text</rgb>` for arbitrary colors, `{{reset}}` to clear all styles

### Prose in Other Components

`Todo` and `Status` both offer a `from_prose` constructor that renders the description through Prose at render time, so markup is resolved with full terminal context:

```rust
use biscuit_terminal::prelude::*;

let todo = Todo::from_prose("review <red>critical</red> PR");
let status = Status::from_prose("this is a <b>test</b>")
    .state(StatusState::Success);
```

### Key API

| Method | Description |
|--------|-------------|
| `Prose::new(text)` | Create with styled content |
| `.content()` | Get the raw content string |
| `.with_word_wrap(WordWrap)` | Set word wrap strategy |
| `.with_left_margin(Margin)` | Set left margin |
| `.with_right_margin(Margin)` | Set right margin |
| `.with_layout(Layout)` | Set full layout configuration |

## CLI

Exposed via `bt prose`:

```bash
bt prose "<bold>Hello</bold> <red>world</red>"
```
