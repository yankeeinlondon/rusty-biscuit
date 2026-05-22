# Terminal Output

darkmatter's terminal rendering with syntax highlighting, themes, and integration with biscuit-terminal.

## TerminalOptions

```rust
pub struct TerminalOptions {
    pub code_theme: ThemePair,        // Theme for code blocks
    pub prose_theme: ThemePair,       // Theme for prose
    pub color_mode: ColorMode,        // Light or Dark
    pub include_line_numbers: bool,   // Show line numbers in code
    pub color_depth: Option<ColorDepth>,  // Auto-detect if None
    pub image_mode: TerminalImageMode, // Auto, Never, Force
    pub base_path: Option<PathBuf>,   // For relative image paths
    pub italic_mode: ItalicMode,      // Auto, Always, Never
    pub max_width: Option<u16>,       // Text wrapping width
    pub mermaid_mode: MermaidMode,    // Off, Image, Text
    pub hyperlink_mode: HyperlinkMode, // Auto, Always, Never
}
```

## Output Functions

```rust
use darkmatter::markdown::output::{TerminalOptions, write_terminal, for_terminal};

// Write directly to a writer
write_terminal(&mut std::io::stdout(), &md, TerminalOptions::default())?;

// Get as string
let output = for_terminal(&md, TerminalOptions::default())?;
```

## Theme Pairs

Themes come in light/dark pairs with automatic mode detection:

| Theme | Light | Dark |
|-------|-------|------|
| `Github` | GitHub Light | GitHub Dark |
| `OneHalf` | One Half Light | One Half Dark |
| `Gruvbox` | Gruvbox Light | Gruvbox Dark |
| `Solarized` | Solarized Light | Solarized Dark |
| `Base16Ocean` | Base16 Ocean Light | Base16 Ocean Dark |
| `Nord` | Nord | Nord |
| `Dracula` | Dracula | Dracula |
| `Monokai` | Monokai | Monokai |
| `VisualStudioDark` | VS Dark | VS Dark |

**`ThemePair` is an abstract, mode-agnostic name.** `ThemePair::resolve(ColorMode)`
maps the name **plus** a mode to a concrete light/dark theme (`(Github, Dark) →
GithubDark`). The bottom four pairs are **single-variant by design** — they ignore
the mode and resolve to one theme. Do not confuse the user-facing name with a
concrete light/dark theme.

### Code blocks invert for page contrast (terminal only)

Code blocks resolve their theme *variant* against the **inverted** terminal mode
(`ColorMode::inverted`): a *light* code panel in a dark terminal, and vice versa.
This lifts the code panel off the page. Prose, headings, tables, and the page
background follow the terminal's real mode so body text stays readable.

- Paired themes contrast correctly (dark terminal → light variant).
- Single-variant themes (`dracula`/`nord`/`monokai`/`vs-dark`) are a deliberate
  no-op — they have no opposite variant, so they cannot lift contrast. Documented,
  not a bug.
- The panel's *internal* contrast (header-pill text color, highlight-line
  background math) keys off the **resolved** theme background
  (`code_block::mode_for_background`), not the requested mode — so a single-variant
  dark theme still gets light header text.
- **HTML inverts too** (Defect D): the `color_mode` is the caller-declared page
  mode, and the code theme resolves against its inverse just like the terminal,
  so a Markdown code fence and a `YamlBlock` render byte-identically.

## Color Mode Detection

```rust
use biscuit_terminal::terminal::Terminal;

let mode = Terminal::color_mode();  // Light, Dark, or Unknown
```

## Image Rendering

Uses biscuit-terminal:

```rust
use biscuit_terminal::components::terminal_image::TerminalImage;
use biscuit_terminal::components::image_options::TerminalImageOptions;

let options = TerminalImageOptions::builder()
    .base_path(base_path)
    .max_file_size(10 * 1024 * 1024)
    .build();

let img = TerminalImage::new(path)?;
img.render_with_options(&options)?;
```

## ANSI Rendering Gotchas

### Background Gaps in Blockquotes and Inline Code

When rendering segments that share a background color (e.g. blockquote paragraphs or inline code inside a blockquote), **never** emit `\x1b[0m` (hard SGR reset) between words or spaces. Hard reset clears *all* attributes including the background color, which creates tiny one-cell gaps where the terminal's default background shows through. In blockquotes every word is a separate segment, so the gaps accumulate into visible artifacts.

**Prefer a soft reset** that only clears style attributes while preserving the background:

```rust
// Hard reset - DON'T USE between adjacent background segments
"\x1b[48;2;50;54;62mtext\x1b[0m"           // background is lost

// Soft reset - preserves background across word boundaries
"\x1b[48;2;50;54;62mtext\x1b[22;23;24;25;27;28;29;39m"
```

The soft reset sequence `\x1b[22;23;24;25;27;28;29;39m` clears:
- `22` normal intensity, `23` italic off, `24` underline off
- `25` blink off, `27` inverse off, `28` conceal off, `29` strikethrough off
- `39` default foreground

But **preserves** `48` (background color). This is implemented in:
- `push_prose_text()` for blockquote prose words
- `push_inline_code_with_bg()` for inline code inside blockquotes
- `LineWrapper::clear_blockquote()` emits a final hard reset after padding

**Reference issue**: Background gaps were visible as missing background on blank characters and commas inside inline code and blockquotes (2026-05-06).

## Mermaid Diagrams

Terminal (via biscuit-terminal):

```rust
use biscuit_terminal::components::mermaid::MermaidRenderer;

let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
match renderer.render_for_terminal() {
    Ok(()) => {},
    Err(_) => println!("{}", renderer.fallback_code_block()),
}
```

HTML (via darkmatter):

```rust
use darkmatter::mermaid::{Mermaid, MermaidTheme};

let diagram = Mermaid::new("flowchart LR\n    A --> B")
    .with_title("My Flowchart")
    .with_footer("Generated 2026-01-29");

let html = diagram.render_for_html();
println!("<head>{}</head><body>{}</body>", html.head, html.body);
```
