# Detection Functions

Low-level detection functions for terminal capabilities. These are used internally by `Terminal::new()` but can be called directly for specific checks.

## Terminal Application

```rust
use biscuit_terminal::discovery::detection::get_terminal_app;

let app = get_terminal_app();
match app {
    TerminalApp::Wezterm => println!("Running in WezTerm"),
    TerminalApp::Kitty => println!("Running in Kitty"),
    TerminalApp::ITerm2 => println!("Running in iTerm2"),
    TerminalApp::Ghostty => println!("Running in Ghostty"),
    TerminalApp::Other(name) => println!("Running in: {}", name),
    _ => {}
}
```

Detection order:
1. `TERM_PROGRAM` environment variable
2. Terminal-specific env vars (`KITTY_WINDOW_ID`, `WEZTERM_PANE`, etc.)
3. `TERM` variable pattern matching

## Color Detection

### Color Depth

```rust
use biscuit_terminal::discovery::detection::color_depth;

match color_depth() {
    ColorDepth::TrueColor => println!("24-bit color (16M)"),
    ColorDepth::Enhanced => println!("256 colors"),
    ColorDepth::Basic => println!("16 colors"),
    ColorDepth::Minimal => println!("8 colors"),
    ColorDepth::None => println!("No color"),
}
```

Detection: `COLORTERM` env var → terminfo `MaxColors` → None

### Color Mode (Light/Dark)

```rust
use biscuit_terminal::discovery::detection::color_mode;

match color_mode() {
    ColorMode::Light => println!("Light background"),
    ColorMode::Dark => println!("Dark background"),
    ColorMode::Unknown => println!("Unknown"),
}
```

Detection: OSC11 query → `DARK_MODE` env → macOS `AppleInterfaceStyle` → Dark default

## Feature Support

### Image Support

```rust
use biscuit_terminal::discovery::detection::image_support;

match image_support() {
    ImageSupport::Kitty => println!("Kitty graphics protocol"),
    ImageSupport::ITerm => println!("iTerm2 inline images"),
    ImageSupport::None => println!("No image support"),
}
```

### OSC8 Hyperlinks

```rust
use biscuit_terminal::discovery::detection::osc8_link_support;

if osc8_link_support() {
    println!("\x1b]8;;https://example.com\x07Link\x1b]8;;\x07");
}
```

### Italics

```rust
use biscuit_terminal::discovery::detection::italics_support;

if italics_support() {
    println!("\x1b[3mItalic text\x1b[23m");
}
```

Detection: terminfo `EnterItalicsMode` → `TERM_PROGRAM` → `TERM` patterns

### Underline Styles

```rust
use biscuit_terminal::discovery::detection::underline_support;

let support = underline_support();

if support.curly {
    // Curly underline for LSP errors
    println!("\x1b[4:3m\x1b[58:2::255:0:0mError\x1b[0m");
} else if support.straight {
    println!("\x1b[4mUnderlined\x1b[0m");
}

// Available flags:
// support.straight, support.double, support.curly
// support.dotted, support.dashed, support.colored
```

## Terminal Dimensions

```rust
use biscuit_terminal::discovery::detection::{terminal_width, terminal_height, dimensions};

let width = terminal_width();   // Default: 80
let height = terminal_height(); // Default: 24
let (w, h) = dimensions();
```

## TTY Detection

```rust
use biscuit_terminal::discovery::detection::is_tty;

if is_tty() {
    println!("\x1b[32mColored output\x1b[0m");
} else {
    println!("Plain output (piped)");
}
```

## Multiplexing

```rust
use biscuit_terminal::discovery::detection::multiplex_support;

match multiplex_support() {
    MultiplexSupport::Tmux { split_window, session_persistence, .. } => {
        println!("tmux with persistence: {}", session_persistence);
    }
    MultiplexSupport::Zellij { floating_panes, .. } => {
        println!("Zellij with floating panes: {}", floating_panes);
    }
    MultiplexSupport::Native { split_window, .. } => {
        println!("Native multiplexing");
    }
    MultiplexSupport::None => {}
}
```

### MultiplexSupport Variants

```rust
pub enum MultiplexSupport {
    None,
    Native {
        split_window: bool,
        resize_pane: bool,
        focus_pane: bool,
        multiple_tabs: bool,
    },
    Tmux {
        split_window: bool,
        resize_pane: bool,
        focus_pane: bool,
        multiple_windows: bool,
        session_persistence: bool,
        detach_session: bool,
    },
    Zellij {
        split_window: bool,
        resize_pane: bool,
        focus_pane: bool,
        multiple_tabs: bool,
        session_resurrection: bool,
        floating_panes: bool,
        detach_session: bool,
    },
}
```

## Connection Detection

```rust
use biscuit_terminal::discovery::detection::detect_connection;

match detect_connection() {
    Connection::Local => println!("Local session"),
    Connection::SshClient(ssh) => {
        println!("SSH from {} (port {})", ssh.host, ssh.source_port);
    }
    Connection::MoshClient(mosh) => {
        println!("Mosh: {}", mosh.connection);
    }
}
```

Detection: `MOSH_CONNECTION` → `SSH_CLIENT` → Local

## OSC Queries

For querying terminal colors:

```rust
use biscuit_terminal::discovery::osc_queries::*;

// Query terminal colors (returns Option<RgbValue>)
let bg = bg_color();
let fg = text_color();
let cursor = cursor_color();

// Check support
let has_fg_query = osc10_support();
let has_bg_query = osc11_support();
let has_cursor_query = osc12_support();

// RgbValue methods
if let Some(bg) = bg_color() {
    let lum = bg.luminance();  // 0.0-1.0
    println!("Background: #{:02x}{:02x}{:02x}", bg.r, bg.g, bg.b);
}
```

## Clipboard (OSC52)

```rust
use biscuit_terminal::discovery::clipboard::*;

if osc52_support() {
    set_clipboard("text to copy")?;
    // get_clipboard() has limited support
}
```

Supported: Kitty, WezTerm, iTerm2, Ghostty, Alacritty, Foot, Contour

## Mode 2027 (Grapheme Clustering)

```rust
use biscuit_terminal::discovery::mode_2027::supports_mode_2027;

if supports_mode_2027() {
    // Terminal handles Unicode grapheme clusters correctly
}
```

## Related

- [Terminal Struct](./terminal-struct.md) - Uses these functions internally
- [OS & Environment](./os-environment.md) - OS detection, CI, fonts
