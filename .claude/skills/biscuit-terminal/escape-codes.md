# Escape Code Utilities

Functions for stripping, analyzing, and measuring text with ANSI escape codes.

## Stripping Escape Codes

### Strip All Escape Codes

```rust
use biscuit_terminal::utils::escape_codes::strip_escape_codes;

let styled = "\x1b[1;31mBold Red\x1b[0m";
let plain = strip_escape_codes(styled);
assert_eq!(plain, "Bold Red");
```

Removes: CSI sequences, OSC sequences, single-character escapes

### Strip Color Codes Only

```rust
use biscuit_terminal::utils::escape_codes::strip_color_codes;

let text = "\x1b[31mRed\x1b[0m \x1b[?25lHidden cursor";
let no_color = strip_color_codes(text);
// Keeps cursor control: " \x1b[?25lHidden cursor"
```

Removes only SGR (Select Graphic Rendition) sequences.

### Strip OSC8 Links

```rust
use biscuit_terminal::utils::escape_codes::strip_osc8_links;

let linked = "\x1b]8;;https://example.com\x07Link\x1b]8;;\x07";
let plain = strip_osc8_links(linked);
assert_eq!(plain, "Link");
```

### Strip Cursor Movement

```rust
use biscuit_terminal::utils::escape_codes::strip_cursor_movement_codes;

let text = "\x1b[10;5HPositioned\x1b[2J";
let stripped = strip_cursor_movement_codes(text);
// Removes cursor positioning and screen clear
```

### Strip Query Codes

```rust
use biscuit_terminal::utils::escape_codes::strip_query_codes;

let text = "Text\x1b[5n\x1b[6nMore";  // DSR queries
let stripped = strip_query_codes(text);
```

Removes terminal query sequences (DA, DSR, etc.).

## Escape Code Analysis

### Check for Escape Codes

```rust
use biscuit_terminal::discovery::eval::has_escape_codes;

let styled = "\x1b[32mGreen\x1b[0m";
assert!(has_escape_codes(styled));
assert!(!has_escape_codes("plain text"));
```

### Check for OSC8 Links

```rust
use biscuit_terminal::discovery::eval::has_osc8_link;

let linked = "\x1b]8;;url\x07text\x1b]8;;\x07";
assert!(has_osc8_link(linked));
```

## Visual Width Calculation

### Line Widths

```rust
use biscuit_terminal::discovery::eval::line_widths;

// Escape codes don't count toward width
let widths = line_widths("\x1b[31mred\x1b[0m");
assert_eq!(widths, vec![3]);  // "red" is 3 chars

// Multiple lines
let widths = line_widths("Hello\n\x1b[1mWorld\x1b[0m");
assert_eq!(widths, vec![5, 5]);  // 5 chars each line
```

Returns `Vec<u16>` with visual width of each line.

## Escape Code Reference

### Common Sequences

| Type | Format | Example |
|------|--------|---------|
| CSI | `\x1b[...` | `\x1b[31m` (red) |
| OSC | `\x1b]...` | `\x1b]8;;url\x07` (link) |
| SGR | `\x1b[...m` | `\x1b[1;32m` (bold green) |

### SGR Codes

| Code | Effect |
|------|--------|
| 0 | Reset all |
| 1 | Bold |
| 2 | Dim |
| 3 | Italic |
| 4 | Underline |
| 7 | Inverse |
| 9 | Strikethrough |
| 22 | Normal intensity |
| 23 | No italic |
| 24 | No underline |
| 30-37 | Foreground colors |
| 40-47 | Background colors |
| 38;5;N | 256-color foreground |
| 48;5;N | 256-color background |
| 38;2;R;G;B | RGB foreground |
| 48;2;R;G;B | RGB background |

### Extended Underlines

| Code | Style |
|------|-------|
| 4:0 | No underline |
| 4:1 | Straight |
| 4:2 | Double |
| 4:3 | Curly |
| 4:4 | Dotted |
| 4:5 | Dashed |
| 58:2::R:G:B | Underline color |

### OSC8 Link Format

```
\x1b]8;;<url>\x07<text>\x1b]8;;\x07
     │  │    │    │     └── End link
     │  │    │    └── Visible text
     │  │    └── BEL terminator
     │  └── URL
     └── Optional params (id=...)
```

## Common Patterns

### Safe Text Measurement

```rust
use biscuit_terminal::discovery::eval::line_widths;

fn fit_in_width(text: &str, max_width: u16) -> bool {
    line_widths(text).iter().all(|&w| w <= max_width)
}
```

### Clean User Input

```rust
use biscuit_terminal::utils::escape_codes::strip_escape_codes;

fn sanitize_input(input: &str) -> String {
    strip_escape_codes(input)
}
```

### Detect Styled Content

```rust
use biscuit_terminal::discovery::eval::{has_escape_codes, has_osc8_link};

fn content_type(text: &str) -> &'static str {
    if has_osc8_link(text) {
        "linked"
    } else if has_escape_codes(text) {
        "styled"
    } else {
        "plain"
    }
}
```

## Regex Patterns Used

The library uses these patterns internally:

```rust
// CSI sequences: \x1b[ ... final_byte
r"\x1b\[[0-9;?]*[A-Za-z]"

// OSC sequences: \x1b] ... BEL or ST
r"\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)"

// Single-character escapes
r"\x1b[^[\]]"
```

## Related

- [Styling](./styling.md) - Creating styled output
- [Discovery Functions](./discovery.md) - Terminal capability checks
