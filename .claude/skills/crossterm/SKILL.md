---
name: crossterm
description: Cross-platform Rust terminal manipulation library for building TUIs and CLI applications with cursor control, styling, event handling, and mouse support on Windows and UNIX systems
last_updated: 2025-12-26T00:00:00Z
hash: a9ab345fd1f4d478
---

# Crossterm

**crossterm** is a pure-Rust, cross-platform terminal manipulation library for creating text-based user interfaces (TUIs) and command-line applications. It provides type-safe APIs for controlling terminal behavior across Windows and UNIX systems.

## Core Principles

- Always use `event::poll()` with a timeout before calling `event::read()` to avoid indefinite blocking
- Enable raw mode for interactive applications that process individual keystrokes
- Use `queue!` macro to batch terminal operations and flush once for better performance
- Always disable enabled features when done (mouse capture, focus change, raw mode)
- Test on target platforms early to catch platform-specific issues
- Disable unused feature flags to reduce binary size and compilation time
- Handle cleanup properly even on errors using Drop implementations or defer patterns
- Prefer synchronous `poll()`/`read()` API unless you specifically need async event handling

## Installation

```toml
[dependencies]
crossterm = "0.29.0"
```

Or with specific features:

```toml
[dependencies.crossterm]
version = "0.29.0"
features = ["bracketed-paste", "event-stream", "osc52"]
```

## Quick Reference

### Basic Terminal Operations

```rust
use crossterm::{
    execute,
    terminal::{Clear, ClearType, SetTitle, EnterAlternateScreen, LeaveAlternateScreen},
    cursor::{MoveTo, Hide, Show},
    style::{Print, SetForegroundColor, Color, ResetColor},
};
use std::io::stdout;

fn main() -> std::io::Result<()> {
    let mut stdout = stdout();

    // Clear screen and set title
    execute!(stdout, Clear(ClearType::All), SetTitle("My App"))?;

    // Position cursor and print colored text
    execute!(
        stdout,
        MoveTo(10, 5),
        SetForegroundColor(Color::Blue),
        Print("Hello, World!"),
        ResetColor
    )?;

    Ok(())
}
```

### Event Handling Pattern

```rust
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::time::Duration;

loop {
    // Always poll before reading to avoid blocking
    if event::poll(Duration::from_millis(100))? {
        match event::read()? {
            Event::Key(key) => {
                match (key.code, key.modifiers) {
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                    (KeyCode::Char('q'), KeyModifiers::NONE) => break,
                    _ => {}
                }
            }
            Event::Mouse(mouse) => { /* handle mouse */ }
            Event::Resize(w, h) => { /* handle resize */ }
            _ => {}
        }
    }
}
```

### Performance Pattern (Queueing)

```rust
use crossterm::{queue, cursor::MoveTo, style::Print};
use std::io::{stdout, Write};

let mut stdout = stdout();

// Queue multiple commands (don't flush yet)
queue!(
    stdout,
    MoveTo(0, 0),
    Print("Line 1"),
    MoveTo(0, 1),
    Print("Line 2")
)?;

// Flush all at once
stdout.flush()?;
```

## Topics

### Getting Started

- [Use Cases](./use-cases.md) - Common scenarios and when to use crossterm
- [Ecosystem](./ecosystem.md) - Companion crates (Ratatui, Inquire, Clap, etc.)

### Core Modules

- [Terminal Control](./terminal.md) - Screen clearing, sizing, alternate screen, scrolling
- [Cursor Control](./cursor.md) - Positioning, visibility, save/restore operations
- [Styling](./styling.md) - Colors, attributes, RGB support, styled content
- [Event Handling](./events.md) - Keyboard, mouse, resize, focus, paste events

### Advanced Topics

- [Feature Flags](./features.md) - Optional features and when to use them
- [Raw Mode](./raw-mode.md) - Interactive applications and keystroke processing
- [Async Patterns](./async.md) - Using EventStream with async runtimes
- [Platform Issues](./platform-issues.md) - macOS, Windows, terminal compatibility gotchas

## Common Patterns

### Interactive TUI Setup

```rust
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};
use std::io;

fn main() -> io::Result<()> {
    // Setup
    terminal::enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;

    // Main loop
    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    // Cleanup
    execute!(io::stdout(), LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    Ok(())
}
```

### Mouse Event Handling

```rust
use crossterm::{
    event::{self, EnableMouseCapture, DisableMouseCapture, MouseEventKind},
    execute,
};

execute!(io::stdout(), EnableMouseCapture)?;

// In event loop
match event {
    Event::Mouse(mouse) => {
        match mouse.kind {
            MouseEventKind::Down(button) => {
                println!("Clicked at ({}, {})", mouse.column, mouse.row);
            }
            MouseEventKind::Drag(button) => {
                println!("Dragged to ({}, {})", mouse.column, mouse.row);
            }
            MouseEventKind::ScrollUp => { /* scroll handling */ }
            MouseEventKind::ScrollDown => { /* scroll handling */ }
            _ => {}
        }
    }
    _ => {}
}

// Cleanup
execute!(io::stdout(), DisableMouseCapture)?;
```

## Key Statistics

- **Downloads**: 80+ million
- **Platform Support**: Windows 7+, all UNIX systems
- **Used By**: Broot, Cursive, Ratatui
- **Code Size**: ~4,600 lines + up to 20,000 lines in dependencies

## Resources

- [Official Docs](https://docs.rs/crossterm)
- [GitHub](https://github.com/crossterm-rs/crossterm)
- [Examples](https://github.com/crossterm-rs/crossterm/tree/master/examples)
