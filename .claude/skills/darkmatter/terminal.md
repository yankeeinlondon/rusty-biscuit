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
