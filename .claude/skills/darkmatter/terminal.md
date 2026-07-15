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

Terminal rendering is the `Markdown::as_terminal` method; it builds a complete
`renderable` render tree and runs one terminal fold over it. (The legacy
free-standing `for_terminal` / `write_terminal` event-stream serializers were
deleted in the tree cutover.)

```rust
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::TerminalOptions;

let md: Markdown = "# Hello\n\nWorld".into();

// Get as string
let output = md.as_terminal(TerminalOptions::default())?;
print!("{output}");
```

For page-framed terminal output (margins, padding, page background, `style:`
frontmatter), use `DarkmatterPage::render(&md)` instead — see the darkmatter
skill's *Common Entry Points*.

## Theme Pairs

Themes come in light/dark pairs with automatic mode detection:

| Theme | Light | Dark |
|-------|-------|------|
| `Github` | GitHub Light | GitHub Dark |
| `OneHalf` | One Half Light | One Half Dark |
| `Gruvbox` | Gruvbox Light | Gruvbox Dark |
| `Solarized` | Solarized Light | Solarized Dark |
| `Base16Ocean` | Base16 Ocean Light | Base16 Ocean Dark |
| `Nord` | One Half Light | Nord |
| `Dracula` | One Half Light | Dracula |
| `Monokai` | One Half Light | Monokai Extended |
| `VisualStudioDark` | GitHub Light | VS Dark |

**`ThemePair` is an abstract, mode-agnostic name.** A `ThemePair` is simply a
(light theme, dark theme) couple; `ThemePair::resolve(ColorMode)` maps the name
**plus** a mode to one of those two themes (`(Github, Dark) → GithubDark`).
**Every pair resolves to a distinct light *and* dark theme** — the light and
dark slots never collapse to one theme. Note that several pairs use the same
theme in their light slot: `Dracula`, `Nord`, and `Monokai` all use One Half
Light, and `VS-Dark` uses GitHub Light. Do not confuse the user-facing name with
a concrete light/dark theme.

### Code blocks invert for page contrast (terminal and HTML)

Code blocks resolve their theme *variant* against the **inverted** terminal mode
(`ColorMode::inverted`): a *light* code panel in a dark terminal, and vice versa.
This lifts the code panel off the page. Prose, headings, tables, and the page
background follow the terminal's real mode so body text stays readable.

- Every pair contrasts correctly because every pair has both a light and a dark
  theme: a dark terminal resolves the light theme and a light terminal the dark
  one. For `dracula`/`nord`/`monokai` the light theme is One Half Light and for
  `vs-dark` it is GitHub Light (the light slots several pairs share).
- Because no pair is mode-invariant, the same `ThemePair` produces **different**
  terminal and HTML output under dark vs light pages — do not expect identical
  bytes across modes for any theme.
- **The terminal is the source of truth for color mode** (Decision #4):
  `DarkmatterPage::with_color_mode` (and `TerminalOptions::color_mode`) only wins
  when terminal detection is `Unknown`. `Terminal::new_optimistic` reports
  `Dark`, so to render against a *light* surface in a test set
  `term.color_mode = ColorMode::Light` before `DarkmatterPage::new` — overriding
  via `with_color_mode(Light)` is ignored.
- The panel's *internal* contrast (header-pill text color, highlight-line
  background math) keys off the **resolved** theme background
  (`code_block::mode_for_background`), not the requested mode.
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

But **preserves** `48` (background color). Since the tree cutover, terminal SGR
emission lives in `biscuit-terminal`'s render-tree fold (`render_tree::render` /
`render_tree::style`), not in darkmatter — the bespoke darkmatter terminal
serializer (and its `push_prose_text` / `push_inline_code_with_bg` /
`LineWrapper` helpers) has been deleted. The soft-reset principle still applies
to any code that paints adjacent background segments by hand.

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

Browser (via darkmatter): Mermaid browser output is a page **feature**, not a
method on `Mermaid`. Render a `lang="mermaid"` fence through
`DarkmatterPage::render_to_browser` (interactive by default): the body carries
`<pre class="mermaid">`, and the fragment render reports a
`PageFeature::MermaidDiagram` request without injecting assets. The outer
`DarkmatterPage` resolves the collected request through
`DarkmatterFeatureResolver` (`darkmatter::mermaid::feature`) and injects one
inline ESM bootstrap into the page wrapper (jsDelivr primary, unpkg fallback,
exact `MERMAID_VERSION`). The Mermaid bundle is script-only: its palette is
passed through Mermaid `themeVariables`, with no Mermaid CSS block.

This request/resolution split is intentional. Low-level
`render_browser_node` output is a composable fragment and therefore carries no
bootstrap by itself; complete-document renderers resolve requests through their
installed resolver, while `DarkmatterPage` defers that work to its outer
body-fragment wrapper.

```rust
use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::Markdown;

let md = Markdown::from("```mermaid\nflowchart LR\n    A --> B\n```\n");
let term = Terminal::new_optimistic(80);
let html = DarkmatterPage::new(&term).render_to_browser(&md).unwrap();
```
