# Styling Utilities

Terminal-aware styling functions and the Prose component for rich text rendering.

## Styling Functions

All functions check terminal capabilities and fall back gracefully:

```rust
use biscuit_terminal::utils::styling::*;
use biscuit_terminal::terminal::Terminal;

let term = Terminal::new();

// Italic (checks supports_italic and is_tty)
let text = italic("emphasized", &term);

// Underline variants (check underline_support)
let text = underline("important", &term);
let text = double_underline("very important", &term);
let text = curly_underline("error", &term);      // LSP-style
let text = dotted_underline("warning", &term);
let text = dashed_underline("hint", &term);
```

### Fallback Behavior

```rust
// If curly not supported, falls back to straight
let text = curly_underline("error", &term);
// Returns "\x1b[4:3m...\x1b[0m" or "\x1b[4m...\x1b[0m"

// If no underline support, returns plain text
```

### Underline Escape Codes

| Style | Code |
|-------|------|
| Straight | `\x1b[4m` or `\x1b[4:1m` |
| Double | `\x1b[4:2m` |
| Curly | `\x1b[4:3m` |
| Dotted | `\x1b[4:4m` |
| Dashed | `\x1b[4:5m` |
| Colored | `\x1b[58:2::R:G:Bm` |

## Prose Component

The `Prose` struct allows styled text with inline tokens:

```rust
use biscuit_terminal::components::prose::Prose;

let prose = Prose::default();
// (Implementation is still in progress - render() is todo!())
```

### Atomic Tokens

Single-point style changes that require manual reset:

```
{{bold}}      {{dim}}       {{italic}}
{{underline}} {{strikethrough}}
{{red}}       {{green}}     {{blue}}
{{yellow}}    {{cyan}}      {{magenta}}
{{bright-red}} {{bright-green}} ...
{{bg-red}}    {{bg-blue}}   ...
{{reset}}     {{reset_fg}}  {{reset_bg}}
```

Example:
```
"This is {{bold}}important{{reset}} text"
→ "This is \x1b[1mimportant\x1b[0m text"
```

### Block Tokens

HTML-like tags that auto-reset:

| Token | Effect |
|-------|--------|
| `<i>text</i>` | Italic |
| `<b>text</b>` | Bold |
| `<u>text</u>` | Underline |
| `<uu>text</uu>` | Double underline |
| `<~>text</~>` | Strikethrough |
| `<a href="...">text</a>` | OSC8 link |
| `<rgb 255,128,0>text</rgb>` | RGB foreground |
| `<red>text</red>` | Named color |
| `<clipboard>fallback</clipboard>` | Clipboard content |

Example:
```
"Click <a href=\"https://example.com\">here</a> for more"
→ "Click \x1b]8;;https://example.com\x07here\x1b]8;;\x07 for more"
```

### Prose Options

```rust
pub struct Prose {
    content: String,
    word_wrap: bool,           // Default: true
    margin_left: Option<u32>,  // Left padding
    margin_right: Option<u32>, // Right padding
}
```

## Manual Styling

For direct escape code output:

### Colors

```rust
// Basic colors (30-37 fg, 40-47 bg)
println!("\x1b[31mRed\x1b[0m");
println!("\x1b[44mBlue background\x1b[0m");

// 256-color palette
println!("\x1b[38;5;208mOrange\x1b[0m");

// RGB (true color)
println!("\x1b[38;2;255;100;50mCustom\x1b[0m");
```

### Text Attributes

```rust
println!("\x1b[1mBold\x1b[0m");
println!("\x1b[3mItalic\x1b[0m");
println!("\x1b[4mUnderline\x1b[0m");
println!("\x1b[1;3;4mBold italic underline\x1b[0m");
```

### Combined Example

```rust
fn styled_error(msg: &str, term: &Terminal) -> String {
    if !term.is_tty {
        return format!("ERROR: {}", msg);
    }

    let underline = if term.underline_support.curly {
        "\x1b[4:3m"  // Curly
    } else {
        "\x1b[4m"    // Straight
    };

    format!("\x1b[1;31m{}ERROR:\x1b[0m {}", underline, msg)
}
```

## Hyperlinks

### OSC8 Links

```rust
fn hyperlink(text: &str, url: &str, term: &Terminal) -> String {
    if term.osc_link_support {
        format!("\x1b]8;;{}\x07{}\x1b]8;;\x07", url, text)
    } else {
        format!("{} ({})", text, url)
    }
}
```

### With ID Parameter

```rust
// For grouping related links
format!("\x1b]8;id=mylink;{}\x07{}\x1b]8;;\x07", url, text)
```

## Best Practices

### Always Check Capabilities

```rust
fn styled_output(term: &Terminal) {
    // Check TTY first
    if !term.is_tty {
        println!("Plain output");
        return;
    }

    // Check specific features
    if term.supports_italic {
        println!("\x1b[3mItalic\x1b[0m");
    }

    // Check color depth
    match term.color_depth {
        ColorDepth::TrueColor => {
            println!("\x1b[38;2;255;100;0mRGB\x1b[0m");
        }
        ColorDepth::Enhanced => {
            println!("\x1b[38;5;208m256-color\x1b[0m");
        }
        _ => {
            println!("\x1b[33mBasic yellow\x1b[0m");
        }
    }
}
```

### Reset After Styling

```rust
// Always reset to prevent style bleeding
println!("\x1b[1mBold\x1b[0m normal");

// Or use specific resets
println!("\x1b[1mBold\x1b[22m normal weight");
```

### Respect NO_COLOR

```rust
fn colored_output(msg: &str) {
    if std::env::var("NO_COLOR").is_ok() {
        println!("{}", msg);
    } else {
        println!("\x1b[32m{}\x1b[0m", msg);
    }
}
```

## Related

- [Escape Codes](./escape-codes.md) - Stripping and analyzing escape codes
- [Terminal Struct](./terminal-struct.md) - Capability detection
