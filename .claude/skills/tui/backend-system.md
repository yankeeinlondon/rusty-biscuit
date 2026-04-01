# Backend System

Ratatui operates through a backend abstraction that interfaces with different terminal libraries.

## Backend Comparison

| Backend | Status | Features | Platform Support | Use Case |
|---------|--------|----------|-----------------|----------|
| **Crossterm** | Default | Cross-platform, mouse capture, clipboard | Windows, Linux, macOS | General purpose, full-featured |
| **Termion** | Optional | Lightweight, Linux-focused | Linux, Unix-like | Minimal dependencies, Linux-only |
| **Termwiz** | Optional | Advanced features, Windows conpty | Windows, Linux, macOS | Complex terminal requirements |

## Backend Selection

### In Cargo.toml

```toml
[dependencies]
# Crossterm (default)
ratatui = { version = "0.29", features = ["crossterm"] }

# Or use termion
ratatui = { version = "0.29", default-features = false, features = ["termion"] }

# Or use termwiz
ratatui = { version = "0.29", default-features = false, features = ["termwiz"] }
```

## Backend Initialization

### Crossterm (Most Common)

```rust
use ratatui::backend::CrosstermBackend;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;

fn init_crossterm() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn cleanup_crossterm(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
```

### Termion

```rust
use ratatui::backend::TermionBackend;
use termion::raw::IntoRawMode;
use std::io;

fn init_termion() -> io::Result<Terminal<TermionBackend<termion::raw::RawTerminal<io::Stdout>>>> {
    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    Terminal::new(backend)
}
```

### Termwiz

```rust
use ratatui::backend::TermwizBackend;
use termwiz::terminal::new_terminal;

fn init_termwiz() -> Result<Terminal<TermwizBackend>, Box<dyn std::error::Error>> {
    let terminal = new_terminal()?;
    let backend = TermwizBackend::new(terminal);
    Ok(Terminal::new(backend)?)
}
```

## Feature Flags

| Feature | Description | Default |
|---------|-------------|---------|
| `crossterm` | Use crossterm backend | Yes |
| `termion` | Use termion backend | No |
| `termwiz` | Use termwiz backend | No |
| `underline-color` | Enable underline color support | Yes (except Windows 7) |

## Windows 7 Compatibility

For Windows 7 compatibility, disable underline colors:

```toml
ratatui = { version = "0.29", default-features = false, features = ["crossterm"] }
```

## Common Issues

### Raw Mode Detection

```rust
use crossterm::terminal::is_raw_mode_enabled;

fn safe_enable_raw_mode() -> io::Result<()> {
    if !is_raw_mode_enabled()? {
        enable_raw_mode()?;
    }
    Ok(())
}
```

### Alternate Screen Support

Some terminals don't properly support alternate screen:

```rust
// Detect terminal capabilities
let use_alt_screen = std::env::var("TERM")
    .map(|term| !term.contains("screen"))
    .unwrap_or(true);

if use_alt_screen {
    execute!(stdout, EnterAlternateScreen)?;
}
```

## Best Practices

1. **Use crossterm by default** - Best cross-platform support
2. **Always cleanup** - Restore terminal state even on panic
3. **Handle backend errors** - Some terminals have limited capabilities
4. **Test across platforms** - Verify behavior on Windows, macOS, and Linux
5. **Document requirements** - Specify which backend your app needs
