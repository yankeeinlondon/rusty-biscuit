# TextBlock

A uniformly styled block of text. Applies consistent styling (colors, font weight, italic, underline, strikethrough, blink) across the entire content. Renders to Terminal, Browser, and Markdown via the canonical render tree. Unlike `Prose` which supports inline markup, TextBlock applies a single style to the whole block.

## IR Status

`TextBlock` implements `TreeRenderable`, `TerminalRenderable`, `BrowserRenderable`, and `MarkdownRenderable`. The default `render()` path routes through the render tree (`Paragraph(Text)` with `Style` and optional `Layout`). The legacy bespoke renderer is retained as `#[doc(hidden)] pub fn render_bespoke()` for parity testing.

After the IR flip, every stored style field is active when rendering through the tree:

- Foreground color, background color, underline, strikethrough, and blink were previously stored but inert in the bespoke renderer. The tree path activates them for Terminal and Browser output. This is an intentional public behavior fix.
- Markdown (both portable and MarkdownPlus) ignores `Style` by contract and emits plain text.
- The optional underline color carried by `UnderliningRequest` has no `Style` slot today and is intentionally dropped at the projection boundary.

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

`bt text-block <TEXT>` instantiates `TextBlock` and exercises its render-tree path.

```bash
# Basic terminal output
bt text-block "Release candidate passed" --fg green --bold --underline

# Cross-target rendering
bt text-block "Hello" --bold --fg red --html
bt text-block "Hello" --bold --md
bt text-block "Hello" --bold --md-plus

# Representative example
bt text-block --example
```

| Flag | Description |
|------|-------------|
| `--bold` / `--dim` / `--italic` | Font weight and italic |
| `--strikethrough` (alias `--strike`) | Strikethrough |
| `--underline` / `--double-underline` / `--curly-underline` / `--dotted-underline` / `--dashed-underline` | Underline variant (mutually exclusive) |
| `--blink` | Blinking text |
| `--fg <color>` / `--bg <color>` | Foreground / background color (named or `#rrggbb`) |
| `--html` / `--md` / `--md-plus` | Cross-target rendering (mutually exclusive; terminal is default) |
| `--example` / `-e` | Render a representative example and print the command |
| Layout flags | `--margin-left`, `--margin-right`, `--margin-top`, `--margin-bottom`, `--alignment` |

`bt text-block` is intentionally separate from `bt block`: the latter is a generic render-tree style exerciser and exposes border/fill concepts that `TextBlock` does not. `bt text-block` exercises the actual component.

For inline styling with markup, use `Prose` instead.
