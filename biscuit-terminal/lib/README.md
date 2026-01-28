# biscuit-terminal

Terminal capability detection and utilities for Rust applications.

## Features

- **OS Detection**: Identify operating system and Linux distribution
- **Terminal App Detection**: Recognize 12+ terminal emulators
- **Color Support**: Query color depth, mode (light/dark), and background color
- **Escape Code Analysis**: Calculate visual line widths, detect escape codes
- **Clipboard**: OSC52 clipboard support for compatible terminals
- **Config Paths**: Find terminal configuration files

## Quick Start

```rust
use biscuit_terminal::terminal::Terminal;

fn main() {
    let term = Terminal::new();

    println!("Running in {:?}", term.app);
    println!("Terminal size: {}x{}", Terminal::width(), Terminal::height());

    if term.supports_italic {
        println!("\x1b[3mItalic text!\x1b[0m");
    }
}
```

## Modules

- `terminal` - Main `Terminal` struct with all capabilities
- `discovery::detection` - Low-level detection functions
- `discovery::os_detection` - OS and Linux distribution detection
- `discovery::config_paths` - Terminal config file paths
- `discovery::osc_queries` - Terminal color queries
- `discovery::clipboard` - OSC52 clipboard support
- `discovery::mode_2027` - Unicode grapheme cluster support
- `discovery::eval` - Escape code analysis utilities

## Terminal Detection

The library detects these terminal emulators:

| Terminal | Image Support | OSC8 Links | Italics |
|----------|--------------|------------|---------|
| WezTerm | Kitty | Yes | Yes |
| Kitty | Kitty | Yes | Yes |
| iTerm2 | Kitty | Yes | Yes |
| Ghostty | Kitty | Yes | Yes |
| Alacritty | None | Yes | Yes |
| Apple Terminal | None | No | Yes |
| GNOME Terminal | None | Yes | Yes |
| Konsole | Kitty | Yes | Yes |
| Foot | None | Yes | Yes |
| Contour | None | Yes | Yes |
| VS Code | None | Yes | Yes |
| Warp | Kitty | Yes | Yes |

## OS Detection

```rust
use biscuit_terminal::discovery::os_detection::{detect_os_type, detect_linux_distro, OsType};

let os = detect_os_type();
match os {
    OsType::Linux => {
        if let Some(distro) = detect_linux_distro() {
            println!("Running on {} ({})", distro.name, distro.family);
        }
    }
    OsType::MacOS => println!("Running on macOS"),
    OsType::Windows => println!("Running on Windows"),
    _ => println!("Running on {:?}", os),
}
```

## Escape Code Analysis

```rust
use biscuit_terminal::discovery::eval::{line_widths, has_escape_codes};

// Calculate visual width (escape codes don't count)
assert_eq!(line_widths("\x1b[31mred\x1b[0m"), vec![3]);

// Detect escape codes
assert!(has_escape_codes("\x1b[1mBold\x1b[0m"));
assert!(!has_escape_codes("plain text"));
```

## Clipboard (OSC52)

```rust
use biscuit_terminal::discovery::clipboard::{osc52_support, set_clipboard};

if osc52_support() {
    set_clipboard("Hello from terminal!").ok();
}
```

## Examples

Run the examples to see the library in action:

```bash
# Show terminal information
cargo run -p biscuit-terminal --example terminal_info

# Analyze escape codes
cargo run -p biscuit-terminal --example escape_analysis
```

## CLI

The package includes a `terminal` CLI (in the `cli` crate):

```bash
# Show basic info
terminal

# Show full metadata
terminal --meta

# Output as JSON
terminal --meta --json
```

## License

AGPL-3.0
