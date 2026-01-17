---
name: crossterm
description: Comprehensive guide to the crossterm crate for Rust terminal manipulation - cross-platform TUI development with type-safe APIs
created: 2025-12-24
last_updated: 2025-12-26T00:00:00Z
hash: cbd5ecca7410d4e5
tags:
  - rust
  - terminal
  - tui
  - cli
  - crossterm
  - cross-platform
  - ecosystem
  - ratatui
  - inquire
---

# Crossterm: Cross-Platform Terminal Manipulation in Rust

**crossterm** is a pure-Rust, cross-platform terminal manipulation library that enables developers to create sophisticated text-based interfaces (TUIs) and command-line applications. It provides a type-safe, performant API for controlling terminal behavior across different operating systems while abstracting away platform-specific differences.

## Table of Contents

- [Crossterm: Cross-Platform Terminal Manipulation in Rust](#crossterm-cross-platform-terminal-manipulation-in-rust)
    - [Table of Contents](#table-of-contents)
    - [Overview](#overview)
        - [Key Characteristics](#key-characteristics)
        - [Key Use Cases](#key-use-cases)
    - [Installation and Configuration](#installation-and-configuration)
        - [Basic Installation](#basic-installation)
        - [Feature Flags Configuration](#feature-flags-configuration)
    - [Core Functionality](#core-functionality)
        - [Terminal Manipulation](#terminal-manipulation)
        - [Cursor Control](#cursor-control)
        - [Styling and Coloring](#styling-and-coloring)
        - [Event Handling](#event-handling)
        - [Clipboard Operations](#clipboard-operations)
    - [Advanced Usage Patterns](#advanced-usage-patterns)
        - [Command API and Queueing](#command-api-and-queueing)
        - [Async Event Stream](#async-event-stream)
        - [Raw Mode for Interactive Applications](#raw-mode-for-interactive-applications)
    - [Ecosystem Integration](#ecosystem-integration)
        - [UI Frameworks](#ui-frameworks)
        - [Interactive Prompt Libraries](#interactive-prompt-libraries)
            - [Inquire (Highly Recommended)](#inquire-highly-recommended)
            - [Dialoguer](#dialoguer)
            - [Cliclack](#cliclack)
            - [Requestty](#requestty)
        - [Supporting Crates](#supporting-crates)
    - [Gotchas and Common Pitfalls](#gotchas-and-common-pitfalls)
        - [Platform-Specific Issues](#platform-specific-issues)
            - [macOS /dev/tty Problem](#macos-devtty-problem)
            - [Windows Key Hold Detection](#windows-key-hold-detection)
            - [Terminal Compatibility](#terminal-compatibility)
        - [Event Handling Pitfalls](#event-handling-pitfalls)
            - [Blocking Event Reads](#blocking-event-reads)
            - [Mouse and Focus Event Enabling](#mouse-and-focus-event-enabling)
        - [Performance Considerations](#performance-considerations)
            - [Command Flushing Overhead](#command-flushing-overhead)
            - [Binary Size](#binary-size)
        - [Async Runtime Integration](#async-runtime-integration)
        - [Deprecated Functionality](#deprecated-functionality)
    - [Comparison with Alternatives](#comparison-with-alternatives)
        - [When to Use crossterm](#when-to-use-crossterm)
        - [When to Consider Alternatives](#when-to-consider-alternatives)
    - [Best Practices](#best-practices)
    - [Resources](#resources)

## Overview

### Key Characteristics

- **Cross-platform support**: Works seamlessly on all UNIX systems and Windows (down to Windows 7), though not all terminals have been tested
- **Zero-cost abstractions**: Leverages Rust's type system to provide efficient terminal operations without runtime overhead
- **Memory safety**: Designed with Rust's ownership principles to prevent common memory errors
- **Production-ready**: With over 80 million downloads, it's battle-tested and used by prominent projects like Broot, Cursive, and Ratatui
- **Flexible API**: Offers both function-based and macro-based interfaces for different coding preferences

### Key Use Cases

crossterm is the "assembly language" of terminal manipulation, providing low-level control that makes it ideal for several categories of applications:

**1. Building Rich Terminal User Interfaces (TUIs)**

The most common use case is as a backend for TUI libraries, frequently paired with **Ratatui** (formerly tui-rs):

- **Dashboards**: Real-time monitoring tools for system resources, stock prices, or server health
- **File Managers**: Visual navigation tools (like broot or ranger clones) that allow users to browse directories using arrow keys
- **Data Visualization**: Rendering charts, sparklines, and tables directly in the terminal buffer

**2. Interactive Command-Line Tools**

Beyond simple static output, crossterm enables CLI tools to feel like applications:

- **Custom Prompts**: Interactive menus where users select options from a list rather than typing commands
- **Progress Indicators**: Advanced progress bars that stay at the bottom of the screen while other logs scroll past
- **Syntax Highlighting**: Terminal-based text editors or pagers that need to colorize text and manage cursor movement dynamically

**3. Terminal Gaming**

Low-level control over raw mode, input polling, and the terminal buffer makes crossterm suitable for terminal-based games (Snake, Tetris, Roguelikes):

- **Raw Mode**: Read every keystroke immediately (including Ctrl+C or arrow keys) without requiring Enter
- **Double Buffering**: Write to an alternate screen buffer to prevent flickering common in older terminal programs

**4. Cross-Platform System Utilities**

Before crossterm became standard, Windows support for terminal styling was notoriously difficult. Developers use crossterm when they need:

- **Consistent Styling**: Colors, bold text, and underlining that look the same on Windows Command Prompt as on Linux xterm
- **Terminal Manipulation**: Clearing the screen, moving the cursor to specific coordinates, or hiding the cursor during long-running processes

**Key Technical Capabilities**

| Feature | Description |
|---------|-------------|
| **Cursor Movement** | Move to specific coordinates, save/restore position, or hide/show |
| **Styling** | Set foreground/background colors and text attributes (bold, italic) |
| **Terminal Control** | Switch to the alternate screen, resize, or clear |
| **Event Handling** | Poll for keyboard, mouse, and resize events asynchronously |

## Installation and Configuration

### Basic Installation

Add crossterm to your `Cargo.toml`:

```toml
[dependencies]
crossterm = "0.29.0"
```

Or use cargo-add:

```bash
cargo add crossterm
```

### Feature Flags Configuration

crossterm provides several optional features that can be enabled to reduce dependency footprint or enable additional functionality:

```toml
[dependencies.crossterm]
version = "0.29.0"
features = [
    "bracketed-paste", # Enables bracketed paste mode
    "event-stream",    # Enables async event stream
    "events",          # Enables input/event reading (default)
    "filedescriptor",  # Uses raw file descriptor instead of mio
    "serde",           # Enables serialization/deserialization of events
    "osc52",           # Enables clipboard support via OSC52 escape sequence
    "derive-more"      # Adds helper functions for event types
]
```

**Available Feature Flags:**

| Feature | Description | Default |
|---------|-------------|---------|
| `bracketed-paste` | Enables bracketed paste mode for better paste handling | No |
| `event-stream` | Provides futures Stream for async event handling | No |
| `events` | Enables reading input/system events | Yes |
| `filedescriptor` | Uses raw file descriptor instead of mio dependency | No |
| `serde` | Enables serialization/deserialization of events | No |
| `osc52` | Enables clipboard support via OSC52 escape sequence | No |
| `derive-more` | Adds `is_*` helper functions for event types | Yes |

## Core Functionality

### Terminal Manipulation

The `terminal` module provides comprehensive control over terminal properties:

```rust
use std::io::{stdout, Write};
use crossterm::{
    execute,
    terminal::{Clear, ClearType, SetSize, SetTitle, ScrollUp, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

fn main() -> std::io::Result<()> {
    // Clear the entire screen
    execute!(stdout(), Clear(ClearType::All))?;

    // Set terminal title
    execute!(stdout(), SetTitle("My Terminal Application"))?;

    // Resize terminal to 40 columns x 20 rows
    execute!(stdout(), SetSize(40, 20))?;

    // Scroll up 5 lines
    execute!(stdout(), ScrollUp(5))?;

    // Enter alternate screen (for TUI applications)
    execute!(stdout(), EnterAlternateScreen)?;

    // ... application code ...

    // Leave alternate screen when done
    execute!(stdout(), LeaveAlternateScreen)?;

    Ok(())
}
```

### Cursor Control

The `cursor` module offers precise cursor positioning and appearance management:

```rust
use crossterm::{
    cursor::{MoveTo, MoveRight, MoveDown, Show, Hide, SavePosition, RestorePosition},
    execute,
};

fn main() -> std::io::Result<()> {
    let mut stdout = stdout();

    // Move cursor to column 10, row 5
    execute!(stdout, MoveTo(10, 5))?;

    // Move cursor right by 3 columns
    execute!(stdout, MoveRight(3))?;

    // Move cursor down by 2 rows
    execute!(stdout, MoveDown(2))?;

    // Hide cursor
    execute!(stdout, Hide)?;

    // ... operations with hidden cursor ...

    // Show cursor again
    execute!(stdout, Show)?;

    // Save current cursor position
    execute!(stdout, SavePosition)?;

    // ... move cursor around ...

    // Restore saved cursor position
    execute!(stdout, RestorePosition)?;

    Ok(())
}
```

### Styling and Coloring

The `style` module enables text and background color manipulation along with text attributes:

```rust
use crossterm::{
    style::{
        Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor,
        SetAttribute, Attribute, PrintStyledContent, Stylize
    },
    execute,
};

fn main() -> std::io::Result<()> {
    let mut stdout = stdout();

    // Basic text coloring
    execute!(
        stdout,
        SetForegroundColor(Color::Blue),
        SetBackgroundColor(Color::Red),
        Print("Blue text on red background"),
        ResetColor
    )?;

    // Using styled content with convenience methods
    execute!(
        stdout,
        PrintStyledContent("Bold text".bold()),
        PrintStyledContent("Italic text".italic()),
        PrintStyledContent("Underlined text".underlined())
    )?;

    // Multiple attributes at once
    execute!(
        stdout,
        SetAttribute(Attribute::Bold),
        SetAttribute(Attribute::Italic),
        Print("Bold and italic text"),
        ResetColor
    )?;

    // Named colors
    execute!(
        stdout,
        SetForegroundColor(Color::Green),
        Print("Green text using named color")
    )?;

    // RGB colors
    execute!(
        stdout,
        SetForegroundColor(Color::Rgb {
            r: 255, g: 128, b: 0
        }),
        Print("Orange text using RGB color")
    )?;

    Ok(())
}
```

### Event Handling

The `event` module provides comprehensive input event handling, including keyboard, mouse, and terminal resize events:

```rust
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent,
        KeyModifiers, MouseEvent, MouseEventKind, EnableFocusChange, DisableFocusChange
    },
    execute,
};

fn main() -> std::io::Result<()> {
    execute!(
        std::io::stdout(),
        EnableMouseCapture,
        EnableFocusChange
    )?;

    loop {
        // Poll for events with a timeout of 500ms
        if event::poll(std::time::Duration::from_millis(500))? {
            match event::read()? {
                Event::Key(KeyEvent {
                    code,
                    modifiers,
                    kind,
                    state,
                }) => {
                    println!("Key event: code={:?}, modifiers={:?}, kind={:?}, state={:?}",
                             code, modifiers, kind, state);

                    // Handle specific key combinations
                    match (code, modifiers) {
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            println!("Ctrl+C pressed, exiting...");
                            break;
                        }
                        (KeyCode::Char('q'), KeyModifiers::NONE) => {
                            println!("q pressed, exiting...");
                            break;
                        }
                        _ => {}
                    }
                }
                Event::Mouse(MouseEvent {
                    kind,
                    column,
                    row,
                    modifiers,
                }) => {
                    println!("Mouse event: kind={:?}, column={}, row={}, modifiers={:?}",
                             kind, column, row, modifiers);

                    // Handle mouse clicks
                    match kind {
                        MouseEventKind::Down(button) => {
                            println!("Mouse button {:?} pressed at ({}, {})", button, column, row);
                        }
                        MouseEventKind::Up(button) => {
                            println!("Mouse button {:?} released at ({}, {})", button, column, row);
                        }
                        MouseEventKind::Drag(button) => {
                            println!("Mouse button {:?} dragged to ({}, {})", button, column, row);
                        }
                        MouseEventKind::Moved => {
                            println!("Mouse moved to ({}, {})", column, row);
                        }
                        MouseEventKind::ScrollUp => {
                            println!("Mouse scrolled up at ({}, {})", column, row);
                        }
                        MouseEventKind::ScrollDown => {
                            println!("Mouse scrolled down at ({}, {})", column, row);
                        }
                        _ => {}
                    }
                }
                Event::Resize(width, height) => {
                    println!("Terminal resized to {}x{}", width, height);
                }
                Event::FocusGained => {
                    println!("Focus gained");
                }
                Event::FocusLost => {
                    println!("Focus lost");
                }
                Event::Paste(data) => {
                    println!("Pasted: {}", data);
                }
            }
        }
    }

    execute!(
        std::io::stdout(),
        DisableMouseCapture,
        DisableFocusChange
    )?;

    Ok(())
}
```

### Clipboard Operations

With the `osc52` feature enabled, crossterm provides clipboard functionality:

```rust
use crossterm::{
    clipboard::{CopyToClipboard, PasteFromClipboard},
    execute,
};

#[cfg(feature = "osc52")]
fn main() -> std::io::Result<()> {
    let mut stdout = stdout();

    // Copy text to clipboard
    execute!(stdout, CopyToClipboard("Hello, Clipboard!"))?;
    println!("Text copied to clipboard");

    // Paste from clipboard (not supported in all terminals)
    // execute!(stdout, PasteFromClipboard)?;

    Ok(())
}

#[cfg(not(feature = "osc52"))]
fn main() {
    println!("This example requires the 'osc52' feature to be enabled");
}
```

## Advanced Usage Patterns

### Command API and Queueing

The Command API allows batching terminal operations for better performance:

```rust
use crossterm::{
    cursor::{MoveTo, MoveRight},
    style::{Print, SetForegroundColor},
    queue,
};
use std::io::{stdout, Write};

fn main() -> std::io::Result<()> {
    let mut stdout = stdout();

    // Queue multiple commands without flushing
    queue!(
        stdout,
        MoveTo(10, 5),
        SetForegroundColor(crossterm::style::Color::Blue),
        Print(" positioned text"),
        MoveRight(5),
        Print(" shifted text")
    )?;

    // Flush all queued commands at once
    stdout.flush()?;

    Ok(())
}
```

### Async Event Stream

For applications requiring asynchronous event handling, use the `event-stream` feature:

```rust
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream},
    execute,
};
use futures_util::StreamExt;
use std::time::Duration;

#[cfg(feature = "event-stream")]
#[tokio::main]
async fn main() -> std::io::Result<()> {
    execute!(std::io::stdout(), EnableMouseCapture)?;

    let mut reader = EventStream::new();

    loop {
        match reader.next().await {
            Some(Ok(event)) => {
                println!("Event: {:?}", event);

                if let Event::Key(key_event) = event {
                    if key_event.code == crossterm::event::KeyCode::Char('q') {
                        break;
                    }
                }
            }
            Some(Err(e)) => {
                println!("Error reading event: {:?}", e);
                break;
            }
            None => {
                println!("Event stream ended");
                break;
            }
        }
    }

    execute!(std::io::stdout(), DisableMouseCapture)?;
    Ok(())
}

#[cfg(not(feature = "event-stream"))]
fn main() {
    println!("This example requires the 'event-stream' feature to be enabled");
}
```

### Raw Mode for Interactive Applications

For applications that need to process individual keystrokes (like text editors), enable raw mode:

```rust
use crossterm::{
    event::{self, KeyCode},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;

fn main() -> io::Result<()> {
    // Enable raw mode to process individual keystrokes
    terminal::enable_raw_mode()?;

    // Enter alternate screen for full control
    execute!(io::stdout(), EnterAlternateScreen)?;

    println!("Press 'q' to quit");

    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(event) => {
                    if event.code == KeyCode::Char('q') {
                        break;
                    } else {
                        println!("Key pressed: {:?}", event.code);
                    }
                }
                _ => {}
            }
        }
    }

    // Clean up
    execute!(io::stdout(), LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    Ok(())
}
```

## Ecosystem Integration

crossterm is rarely used in isolation. It serves as the foundation for a rich ecosystem of higher-level libraries that build on its terminal manipulation capabilities. Most developers don't use crossterm directly but instead leverage it through complementary crates that add structure, design, or higher-level logic.

### UI Frameworks

**Ratatui** (successor to tui-rs) is the gold standard for building full terminal user interfaces:

- Uses crossterm as its default backend for drawing and input handling
- Provides widgets, layouts, and styling abstractions for building dashboards, editors, and complex TUIs
- Handles the complexity of buffer management, event loops, and rendering
- Ideal for applications requiring structured UI components rather than raw terminal control

### Interactive Prompt Libraries

For building surveys, questionnaires, or interactive CLI tools without manually tracking cursor positions and keypresses, several specialized libraries build on crossterm:

#### Inquire (Highly Recommended)

The most popular and feature-rich library for interactive prompts, using crossterm for cross-platform support:

```rust
use inquire::{Confirm, MultiSelect, Select, Text};

fn main() {
    // Simple text input
    let name = Text::new("What is your name?")
        .with_placeholder("e.g. Ferris")
        .prompt();

    // Single selection
    let languages = vec!["Rust", "Python", "Go", "C++"];
    let favorite = Select::new("What is your favorite language?", languages).prompt();

    // Multi-selection with checkboxes
    let features = vec!["Speed", "Memory Safety", "Ease of Use", "Package Management"];
    let choices = MultiSelect::new("Which features do you value most?", features).prompt();

    // Confirmation prompt
    let confirm = Confirm::new("Do you want to save these results?")
        .with_default(true)
        .prompt();

    match confirm {
        Ok(true) => println!("\nThanks! We've recorded your preferences."),
        _ => println!("\nSurvey cancelled."),
    }
}
```

**Features:**

- Multiple prompt types: text input, passwords (hidden), confirmation (y/n), single select, multi-select, date picker
- Built-in validation with custom error messages
- Selectable derive macros for enums
- Best for professional-grade CLI tools requiring robust user input

**Validation Example:**

```rust
let age = Text::new("How old are you?")
    .with_validator(|input: &str| {
        if input.parse::<u32>().is_ok() {
            Ok(inquire::validator::Validation::Valid)
        } else {
            Ok(inquire::validator::Validation::Invalid("Please enter a number".into()))
        }
    })
    .prompt();
```

#### Dialoguer

A long-standing staple in the ecosystem, providing similar functionality with a simpler API:

- Provides Select, Confirm, Input, and other prompt types
- Uses the console crate by default, but often used alongside crossterm applications
- Praised for simplicity and ease of use
- Good choice for straightforward prompt requirements

#### Cliclack

Modern aesthetic inspired by JavaScript CLI tools (the "Clack" package):

- Very clean and minimal design with beautiful vertical lines and symbols
- Makes surveys feel like modern applications
- Opinionated styling that provides a polished look out of the box
- Best for applications prioritizing visual appeal

#### Requestty

Inspired by Node.js's inquirer library:

- Widget-based system for rendering complex questions
- Comprehensive feature set similar to inquire
- Good choice for teams familiar with JavaScript tooling

**Comparison of Prompt Libraries:**

| Feature | Inquire | Dialoguer | Cliclack |
|---------|---------|-----------|----------|
| **Backend** | Crossterm | Console | Crossterm |
| **Complexity** | High (Powerful) | Medium (Simple) | Low (Aesthetic) |
| **Multi-select** | Yes | Yes | Yes |
| **Customization** | Extensive | Moderate | Opinionated |

**How These Work with crossterm:**

When you run an interactive prompt using these libraries, they leverage crossterm's capabilities:

- **Raw Mode**: Puts the terminal into raw mode to detect every keypress (arrow keys, Enter, etc.) instantly without requiring Enter
- **Terminal Cleanup**: Handles Ctrl+C gracefully, cleaning up terminal state so the command line doesn't stay "broken" after program exit
- **ANSI Escaping**: Uses crossterm style commands to color question marks, dim help text, and style selections

### Supporting Crates

Several complementary crates enhance crossterm-based applications:

**CLI Argument Parsing:**

- **Clap**: Almost every crossterm application needs to handle flags and arguments (e.g., `my_app --debug`)

**Error Handling:**

- **Anyhow / Thiserror**: Essential for managing the many `io::Error` results that terminal manipulation generates

**Styling & Tables:**

- **Comfy-table**: While crossterm has a style module, comfy-table builds complex, auto-wrapping tables on top of it
- **Owo-colors**: Popular alternative for super-simple ANSI text coloring

## Gotchas and Common Pitfalls

### Platform-Specific Issues

#### macOS /dev/tty Problem

**Issue**: On macOS, crossterm doesn't support reading from `/dev/tty`, which can cause applications to hang when input is piped.

**Workaround**:

- Use alternative libraries like termion for macOS-specific applications
- Avoid piping input when using crossterm on macOS
- Consider using `filedescriptor` feature flag for more direct terminal access

#### Windows Key Hold Detection

**Issue**: Windows doesn't properly detect when a key is being held down, leading to duplicate key events instead of a single hold event.

**Workaround**:

- Implement your own key repeat detection logic
- Check for both press and release events
- Use alternative input libraries for Windows-specific applications that require precise key hold detection

#### Terminal Compatibility

**Issue**: Not all terminals support all features, especially older terminals on Windows (pre-Windows 10) and some less common UNIX terminals.

**Workaround**:

- Test your application on target terminals
- Provide fallback implementations for unsupported features
- Document terminal requirements for your application

### Event Handling Pitfalls

#### Blocking Event Reads

**Issue**: Using `event::read()` without polling can cause your application to hang indefinitely when no events are available.

**Solution**: Always use `event::poll()` with a timeout before calling `event::read()`:

```rust
// Bad - will block indefinitely
// let event = event::read()?;

// Good - checks for events with timeout
if event::poll(std::time::Duration::from_millis(100))? {
    let event = event::read()?;
    // Process event
}
```

#### Mouse and Focus Event Enabling

**Issue**: Mouse and focus events are not enabled by default and must be explicitly enabled.

**Solution**: Always enable the events you need:

```rust
execute!(
    stdout(),
    EnableMouseCapture,
    EnableFocusChange
)?;

// Remember to disable when done
execute!(
    stdout(),
    DisableMouseCapture,
    DisableFocusChange
)?;
```

### Performance Considerations

#### Command Flushing Overhead

**Issue**: Flushing terminal commands too frequently can cause performance issues.

**Solution**: Use the `queue!` macro to batch commands and flush once:

```rust
// Bad - flushes after each command
execute!(stdout(), MoveTo(10, 5))?;
execute!(stdout(), Print("text"))?;
execute!(stdout(), MoveRight(5))?;

// Good - batches commands and flushes once
queue!(
    stdout(),
    MoveTo(10, 5),
    Print("text"),
    MoveRight(5)
)?;
stdout.flush()?;
```

#### Binary Size

**Issue**: crossterm can add significant size to your binary due to its dependencies (approximately 4600 lines of code plus up to 20,000 lines in dependencies).

**Solutions**:

- Disable default features you don't need
- Use `filedescriptor` feature to avoid mio dependency
- Consider alternative libraries for very size-constrained applications

### Async Runtime Integration

**Issue**: The `event-stream` feature requires an async runtime like Tokio, which can complicate applications that don't otherwise need async.

**Solution**: Only use `event-stream` if you specifically need async event handling. For most applications, the synchronous `poll()`/`read()` API is sufficient:

```rust
// For most applications, use synchronous API
if event::poll(Duration::from_millis(100))? {
    let event = event::read()?;
    // Handle event
}

// Only use event-stream if you specifically need async
#[cfg(feature = "event-stream")]
let mut reader = EventStream::new();
while let Some(Ok(event)) = reader.next().await {
    // Handle event
}
```

### Deprecated Functionality

**Issue**: The `crossterm_input` crate is deprecated and no longer maintained.

**Solution**: Use the integrated event handling in the main crossterm crate instead:

```rust
// Old deprecated approach (DO NOT USE)
// use crossterm_input::{input, TerminalInput};

// New correct approach
use crossterm::event;
```

## Comparison with Alternatives

| Feature | crossterm | termion | ncurses |
|---------|-----------|---------|---------|
| **Cross-platform** | Yes (Windows + UNIX) | No (Linux only) | Yes (with limitations) |
| **Pure Rust** | Yes | Yes | No (C wrapper) |
| **Async Support** | Yes (with feature) | No | No |
| **Mouse Support** | Yes | Yes | Yes |
| **Clipboard Support** | Yes (with feature) | No | No |
| **Performance** | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| **Ease of Use** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ |
| **Documentation** | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ |
| **Binary Size** | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐ |

### When to Use crossterm

- **Cross-platform applications** that need to work on both Windows and UNIX
- **Rust-native projects** that prefer pure Rust implementations
- **Applications requiring async** event handling
- **Projects needing clipboard support**
- **Beginner-friendly projects** that prioritize ease of use

### When to Consider Alternatives

- **Linux-only applications** where termion's performance is critical
- **Existing ncurses applications** being ported to Rust
- **Size-constrained applications** where binary size is paramount
- **Applications requiring very specialized terminal features** not supported by crossterm

## Best Practices

1. **Always enable raw mode** for interactive applications that process individual keystrokes
2. **Use queueing** for batch operations to improve performance
3. **Handle cleanup properly** by disabling features when done (e.g., `DisableMouseCapture`)
4. **Test on target platforms** early and often to catch platform-specific issues
5. **Disable unused features** to reduce binary size and compilation time
6. **Use appropriate event handling** for your use case (synchronous vs. asynchronous)
7. **Provide fallback implementations** for features not supported on all terminals
8. **Document terminal requirements** for your application
9. **Handle errors gracefully** using `?` operator or proper error handling
10. **Consider using higher-level TUI frameworks** like ratatui for complex interfaces

## Resources

- **Official Repository**: [https://github.com/crossterm-rs/crossterm](https://github.com/crossterm-rs/crossterm)
- **Documentation**: [https://docs.rs/crossterm](https://docs.rs/crossterm)
- **Crates.io**: [https://crates.io/crates/crossterm](https://crates.io/crates/crossterm)
- **Examples**: Available in the repository's examples directory
- **Community**: Active development with over 80 million downloads

---

**crossterm** is a powerful, flexible, and well-maintained terminal manipulation library that provides excellent cross-platform support for Rust applications. Its type-safe API, comprehensive feature set, and strong community make it an excellent choice for both simple CLI tools and complex TUI applications. For most Rust developers building terminal applications, crossterm represents an excellent balance of features, performance, and ease of use, making it a top choice in the ecosystem.
