# Event Handling

The `event` module provides comprehensive input event handling including keyboard, mouse, terminal resize, focus, and paste events.

## Key Concepts

Event handling in crossterm follows a poll-then-read pattern to avoid blocking. Events must be explicitly enabled for mouse and focus tracking.

## Event Types

```rust
use crossterm::event::Event;

pub enum Event {
    Key(KeyEvent),       // Keyboard input
    Mouse(MouseEvent),   // Mouse input (requires EnableMouseCapture)
    Resize(u16, u16),    // Terminal resized (cols, rows)
    FocusGained,         // Terminal gained focus (requires EnableFocusChange)
    FocusLost,           // Terminal lost focus (requires EnableFocusChange)
    Paste(String),       // Pasted text (requires bracketed-paste feature)
}
```

## Basic Event Loop

```rust
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::time::Duration;

loop {
    // ALWAYS poll before reading to avoid blocking
    if event::poll(Duration::from_millis(100))? {
        match event::read()? {
            Event::Key(key) => {
                println!("Key: {:?}", key);

                // Exit on Ctrl+C or 'q'
                match (key.code, key.modifiers) {
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                    (KeyCode::Char('q'), KeyModifiers::NONE) => break,
                    _ => {}
                }
            }
            Event::Resize(w, h) => {
                println!("Terminal resized to {}x{}", w, h);
            }
            _ => {}
        }
    }
}
```

**Critical:** Always use `poll()` before `read()`. Direct calls to `read()` block indefinitely.

## Keyboard Events

```rust
use crossterm::event::{KeyEvent, KeyCode, KeyModifiers, KeyEventKind};

match event::read()? {
    Event::Key(KeyEvent {
        code,
        modifiers,
        kind,
        state,
    }) => {
        // kind distinguishes press, release, repeat
        if kind == KeyEventKind::Press {
            match code {
                KeyCode::Char(c) => println!("Character: {}", c),
                KeyCode::Enter => println!("Enter pressed"),
                KeyCode::Esc => println!("Escape pressed"),
                KeyCode::Backspace => println!("Backspace pressed"),
                KeyCode::Tab => println!("Tab pressed"),
                KeyCode::F(n) => println!("F{} pressed", n),
                KeyCode::Up => println!("Up arrow"),
                KeyCode::Down => println!("Down arrow"),
                KeyCode::Left => println!("Left arrow"),
                KeyCode::Right => println!("Right arrow"),
                _ => {}
            }

            // Check modifiers
            if modifiers.contains(KeyModifiers::CONTROL) {
                println!("Ctrl is held");
            }
            if modifiers.contains(KeyModifiers::SHIFT) {
                println!("Shift is held");
            }
            if modifiers.contains(KeyModifiers::ALT) {
                println!("Alt is held");
            }
        }
    }
    _ => {}
}
```

### Key Event Kinds

```rust
use crossterm::event::KeyEventKind;

match kind {
    KeyEventKind::Press => {
        // Key was pressed down
    }
    KeyEventKind::Release => {
        // Key was released (not widely supported)
    }
    KeyEventKind::Repeat => {
        // Key is being held (auto-repeat)
    }
}
```

## Mouse Events

Mouse events require explicit enablement:

```rust
use crossterm::{
    execute,
    event::{
        EnableMouseCapture, DisableMouseCapture,
        MouseEvent, MouseEventKind, MouseButton,
    },
};
use std::io::stdout;

// Enable mouse capture
execute!(stdout(), EnableMouseCapture)?;

// Event loop
loop {
    if event::poll(Duration::from_millis(100))? {
        if let Event::Mouse(MouseEvent {
            kind,
            column,
            row,
            modifiers,
        }) = event::read()? {
            match kind {
                MouseEventKind::Down(button) => {
                    match button {
                        MouseButton::Left => println!("Left click at ({}, {})", column, row),
                        MouseButton::Right => println!("Right click at ({}, {})", column, row),
                        MouseButton::Middle => println!("Middle click at ({}, {})", column, row),
                    }
                }
                MouseEventKind::Up(button) => {
                    println!("Released {:?} at ({}, {})", button, column, row);
                }
                MouseEventKind::Drag(button) => {
                    println!("Dragging {:?} to ({}, {})", button, column, row);
                }
                MouseEventKind::Moved => {
                    println!("Mouse moved to ({}, {})", column, row);
                }
                MouseEventKind::ScrollUp => {
                    println!("Scrolled up at ({}, {})", column, row);
                }
                MouseEventKind::ScrollDown => {
                    println!("Scrolled down at ({}, {})", column, row);
                }
            }
        }
    }
}

// CRITICAL: Always disable when done
execute!(stdout(), DisableMouseCapture)?;
```

## Focus Events

Focus events require explicit enablement:

```rust
use crossterm::event::{EnableFocusChange, DisableFocusChange};

execute!(stdout(), EnableFocusChange)?;

// Event loop
match event::read()? {
    Event::FocusGained => {
        println!("Application gained focus");
    }
    Event::FocusLost => {
        println!("Application lost focus");
    }
    _ => {}
}

execute!(stdout(), DisableFocusChange)?;
```

## Paste Events

Paste events require the `bracketed-paste` feature:

```rust
// Cargo.toml
// [dependencies.crossterm]
// features = ["bracketed-paste"]

match event::read()? {
    Event::Paste(data) => {
        println!("Pasted: {}", data);
    }
    _ => {}
}
```

## Common Patterns

### Text Input Handler

```rust
use crossterm::event::{KeyCode, KeyModifiers};

fn handle_text_input(input: &mut String, key: KeyEvent) {
    match (key.code, key.modifiers) {
        (KeyCode::Char(c), KeyModifiers::NONE) => {
            input.push(c);
        }
        (KeyCode::Char(c), KeyModifiers::SHIFT) => {
            input.push(c.to_ascii_uppercase());
        }
        (KeyCode::Backspace, _) => {
            input.pop();
        }
        (KeyCode::Enter, _) => {
            // Submit input
        }
        _ => {}
    }
}
```

### Clickable Regions

```rust
struct ClickableRegion {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl ClickableRegion {
    fn contains(&self, col: u16, row: u16) -> bool {
        col >= self.x && col < self.x + self.width &&
        row >= self.y && row < self.y + self.height
    }
}

// Usage
let button = ClickableRegion { x: 10, y: 5, width: 15, height: 3 };

if let Event::Mouse(mouse) = event::read()? {
    if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
        if button.contains(mouse.column, mouse.row) {
            println!("Button clicked!");
        }
    }
}
```

### Event Filter

```rust
fn wait_for_key(target: KeyCode) -> io::Result<()> {
    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == target {
                    break;
                }
            }
        }
    }
    Ok(())
}

// Usage: wait for Enter key
wait_for_key(KeyCode::Enter)?;
```

## Gotchas

### Platform Differences

**Windows Key Hold Issue:**
- Windows doesn't properly detect key holds
- You get multiple Press events instead of Press + Repeat + Release
- Workaround: Implement custom key repeat detection

**macOS /dev/tty Issue:**
- crossterm doesn't support reading from `/dev/tty` on macOS
- Can cause hangs when input is piped
- Workaround: Use alternative libraries like termion for macOS-specific apps

### Event Enablement

**Issue:** Mouse and focus events aren't enabled by default

**Solution:** Always enable before use and disable when done:

```rust
// Setup
execute!(stdout(), EnableMouseCapture, EnableFocusChange)?;

// Use events

// Cleanup (even on error!)
execute!(stdout(), DisableMouseCapture, DisableFocusChange)?;
```

### Blocking Reads

**Issue:** Calling `read()` without `poll()` blocks indefinitely

```rust
// BAD - blocks forever if no events
let event = event::read()?;

// GOOD - checks for events first
if event::poll(Duration::from_millis(100))? {
    let event = event::read()?;
}
```

## Related

- [Raw Mode](./raw-mode.md) - Essential for capturing individual keystrokes
- [Async Patterns](./async.md) - Using EventStream for async event handling
- [Platform Issues](./platform-issues.md) - Platform-specific event handling gotchas
