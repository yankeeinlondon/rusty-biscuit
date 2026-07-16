# Terminal Struct

The `Terminal` struct is the primary entry point for accessing terminal capabilities. It aggregates all detected terminal information including the application, OS details, and capability flags.

## Creating a Terminal

```rust
use biscuit_terminal::terminal::Terminal;

// Detection happens at construction
let term = Terminal::new();

// Or use Default trait
let term = Terminal::default();
```

## Static Properties

These are detected once at construction and don't change:

```rust
pub struct Terminal {
    // Terminal identification
    pub app: TerminalApp,           // WezTerm, Kitty, iTerm2, Ghostty, etc.
    pub os: OsType,                 // MacOS, Linux, Windows, etc.
    pub distro: Option<LinuxDistro>,// Linux distribution details

    // Environment
    pub is_tty: bool,               // stdout connected to TTY
    pub is_ci: bool,                // CI environment detected
    pub remote: Connection,         // Local, SshClient, MoshClient
    pub in_repo: bool,              // current directory is in a git repo
    pub in_monorepo: bool,          // repo is a monorepo
    pub repo_root: Option<PathBuf>, // repo root path (if detected)

    // Feature support
    pub supports_italic: bool,
    pub image_support: ImageSupport,      // None, Kitty, ITerm
    pub underline_support: UnderlineSupport,
    pub osc_link_support: bool,
    pub color_depth: ColorDepth,          // None, Minimal, Basic, Enhanced, TrueColor

    // Font info (from config parsing)
    pub font: Option<String>,
    pub font_size: Option<u32>,
    pub font_ligatures: Option<Vec<FontLigature>>,
    pub is_nerd_font: Option<bool>,

    // Locale & encoding
    pub char_encoding: CharEncoding,
    pub locale: TerminalLocale,
    pub config_file: Option<PathBuf>,

    // Fixed dimensions (for testing or explicit overrides)
    pub fixed_width: Option<u32>,
    pub fixed_height: Option<u32>,
    pub tab_width: usize,          // terminfo init_tabs, default 8
}
```

## Dynamic Methods

These query current state that can change at runtime:

```rust
let term = Terminal::new();

// Terminal dimensions (recalculated on each call, or fixed if set via builder)
let width = term.width();   // Returns 80 if detection fails
let height = term.height(); // Returns 24 if detection fails
let tab_width = term.tab_width; // Detected once, defaults to 8

// Color mode (light/dark) - static method
let mode = Terminal::color_mode();
```

## Builder Pattern

Create a Terminal with overridden values (useful for testing):

```rust
let term = Terminal::builder()
    .is_tty(true)
    .image_support(ImageSupport::Kitty)
    .color_depth(ColorDepth::TrueColor)
    .width(120)   // Fixed width (overrides dynamic detection)
    .height(40)   // Fixed height (overrides dynamic detection)
    .tab_width(4) // Explicit horizontal-tab interval
    .build();
```

Available builder methods:
- `app()`, `supports_italic()`, `image_support()`, `underline_support()`
- `osc_link_support()`, `is_tty()`, `color_depth()`, `is_ci()`, `is_nerd_font()`
- `width()`, `height()` (fixed dimensions), `tab_width()`

## Key Enums

### TerminalApp

```rust
pub enum TerminalApp {
    AppleTerminal,
    Alacritty,
    Contour,
    Foot,
    GnomeTerminal,
    Ghostty,
    ITerm2,
    Kitty,
    Konsole,
    VsCode,
    Warp,
    Wast,
    Wezterm,
    Other(String),
}
```

### ImageSupport

```rust
pub enum ImageSupport {
    None,
    Kitty,  // Kitty Graphics Protocol (highest quality)
    ITerm,  // iTerm2 inline images (legacy)
}
```

### ColorDepth

```rust
pub enum ColorDepth {
    None,       // No color support
    Minimal,    // 8 colors
    Basic,      // 16 colors
    Enhanced,   // 256 colors (8-bit)
    TrueColor,  // 16 million colors (24-bit)
}
```

### UnderlineSupport

```rust
pub struct UnderlineSupport {
    pub straight: bool,  // Basic underline
    pub double: bool,    // Double underline
    pub curly: bool,     // Squiggly (LSP errors)
    pub dotted: bool,
    pub dashed: bool,
    pub colored: bool,   // Independent underline color
}
```

### Connection

```rust
pub enum Connection {
    Local,
    SshClient(SshClient),   // Parsed from SSH_CLIENT
    MoshClient(MoshClient), // Parsed from MOSH_CONNECTION
}

pub struct SshClient {
    pub host: String,
    pub source_port: u32,
    pub server_port: u32,
    pub tty_path: Option<String>,
}
```

## Usage Patterns

### Feature Gating

```rust
let term = Terminal::new();

// Only style if supported
if term.is_tty && term.supports_italic {
    println!("\x1b[3mItalic\x1b[0m");
}

// Check color depth before using RGB
if matches!(term.color_depth, ColorDepth::TrueColor) {
    println!("\x1b[38;2;255;100;0mOrange\x1b[0m");
}
```

### Remote Session Handling

```rust
match &term.remote {
    Connection::Local => {
        // Full feature set
    }
    Connection::SshClient(ssh) => {
        println!("SSH from {}", ssh.host);
        // May want to reduce bandwidth usage
    }
    Connection::MoshClient(_) => {
        // Mosh has some rendering limitations
    }
}
```

### Config File Access

```rust
if let Some(config) = &term.config_file {
    println!("Terminal config: {}", config.display());
}
```

## Related

- [Discovery Functions](./discovery.md) - Low-level detection functions
- [Image Rendering](./image-rendering.md) - Using image_support for rendering
