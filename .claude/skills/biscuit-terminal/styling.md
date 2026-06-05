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

The `Prose` struct allows styled text with bracketed tags and a Markdown subset.
It parses directly into the shared `renderable::tree::RenderNode` tree and
renders Terminal, Browser, Markdown, and MarkdownPlus through the shared tree
renderers — it carries no component-local IR.

```rust
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;

// Create prose with inline styling
let prose = Prose::new("Hello <b>world</b>!");
let output = prose.render_optimistic(None);
// → "Hello \x1b[1mworld\x1b[0m!\x1b[0m"

// With builder pattern
let prose = Prose::new("<b>Important</b> message")
    .with_word_wrap(WordWrap::WrapProse(None, None))
    .with_left_margin(Margin::Chars(4));
```

### Block Tags

HTML-like tags that auto-reset. Former atomic tokens (`{{bold}}`,
`{{reset}}`, etc.) were removed in the 2026-05-17 Prose cross-target work and
now render as literal text.

| Token | Alias | Effect |
|-------|-------|--------|
| `<bold>` | `<b>` | Bold |
| `<dim>` | | Dim |
| `<italic>` | `<i>` | Italic |
| `<underline>` | `<u>` | Underline |
| `<double-underline>` | `<uu>` | Double underline |
| `<curly-underline>` | | Curly underline |
| `<dotted-underline>` | | Dotted underline |
| `<dashed-underline>` | | Dashed underline |
| `<blink>` | | Blinking |
| `<inverse>` | `<reverse>` | Inverse video |
| `<hidden>` | | Hidden |
| `<strikethrough>` | `<~>` | Strikethrough |
| `<a href="...">` | | OSC8 link |
| `<rgb 255,128,0>` | | RGB foreground |
| `<red>`, `<bright-red>` | | Basic/bright colors |
| `<coral>`, `<alice-blue>` | | Web/CSS colors |
| `<purple-500>`, `<slate-700>` | | Tailwind colors |
| `<clipboard>` | | Clipboard content |

#### Color Blocks

Foreground colors:
```
<red>basic color</red>
<bright-red>bright variant</bright-red>
<alice-blue>web/CSS color</alice-blue>
<purple-500>Tailwind color</purple-500>
<rgb 255,0,0>RGB color</rgb>
```

Background colors (prefix with `bg-`):
```
<bg-red>red background</bg-red>
<bg-blue>blue background</bg-blue>
<bg-coral>web/CSS background</bg-coral>
<bg-red-800>Tailwind background</bg-red-800>
<bg-rgb 255,128,0>RGB background</bg-rgb>
```

#### Hyperlinks

OSC8 links with relative path support:
```
<a href="https://example.com">URL</a>
<a href="/absolute/path">absolute file</a>
<a href="./relative/path">relative to CWD</a>
<a href="src/main.rs">relative to git/package root</a>
```

Relative paths (without `./`) resolve from:
1. Package root (for monorepos: nearest Cargo.toml)
2. Git repository root
3. CWD (fallback)

Example:
```
"Click <a href=\"https://example.com\">here</a> for more"
→ "Click \x1b]8;;https://example.com\x07here\x1b]8;;\x07 for more"
```

### Markdown Subset

A strict CommonMark subset is recognised in addition to block tags. Markdown forms are pre-processed into the equivalent internal tag form before rendering:

| Markdown        | Equivalent block tag           |
|-----------------|--------------------------------|
| `[desc](ref)`   | `<a href="ref">desc</a>`       |
| `**text**`      | `<b>text</b>`                  |
| `_text_`        | `<i>text</i>`                  |

Strict subset — `__bold__` and `*italics*` are **not** recognised; both pass through as literal text. The pre-processor runs in a fixed order: links → bold → italics → block-tag parser. Link URLs are placeholdered before the bold/italics phases so a URL like `https://example.com/path_with_underscores` is never re-interpreted.

#### Flanking rule (intra-word inhibition)

`_` and `**` are treated as **literal text** when they sit between two word characters (Unicode alphanumerics on both sides). This protects identifier-shaped strings from being mangled when interpolated into Prose format strings:

| Input | Output | Why |
|-------|--------|-----|
| `OPENCODE_CONFIG_CONTENT` | `OPENCODE_CONFIG_CONTENT` | Every `_` is intra-word |
| `_foo_bar_` | `<i>foo_bar</i>` | Outer `_` flanked by start/end; inner `_` skipped |
| `foo**bar**baz` | `foo**bar**baz` | Both `**` runs intra-word |
| `**foo**bar**baz**` | `<b>foo**bar**baz</b>` | Outer `**` flanked; inner pairs intra-word |
| `(_text_)` | `(<i>text</i>)` | Punctuation neighbours form boundaries |
| `<dim>=OPENCODE_CONFIG_CONTENT</dim>` | unchanged | Tag wrapper preserved; intra-word `_` not triggered inside body |

The rule is symmetric across openers and closers and applies to both `_` and `**`. It is a deliberately simpler rule than full CommonMark left/right-flanking — terminal markup is overwhelmingly ASCII identifiers and predictability beats spec parity.

**Practical consequence for callers:** dynamic content interpolated into a Prose format string (env-var keys, file paths, identifiers) usually does **not** require escaping — the flanking rule already protects identifier-shaped values.

#### Escape mechanism

A backslash escapes the immediately following character. Escapable: `* _ [ ] ( ) < > { \`. Use this when you need a literal Markdown sigil at a position where the flanking rule would *not* already inhibit it (e.g. `\_emphasis_` for a leading literal `_`).

```
\_text\_  →  _text_   (literal underscores, no italics)
\*\*x\*\* →  **x**    (literal asterisks)
\\        →  \        (literal backslash)
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

- [Color System](./color-system.md) - Full color type hierarchy (BasicColor, RgbColor, WebColor, Tailwind)
- [Escape Codes](./escape-codes.md) - Stripping and analyzing escape codes
- [Terminal Struct](./terminal-struct.md) - Capability detection
