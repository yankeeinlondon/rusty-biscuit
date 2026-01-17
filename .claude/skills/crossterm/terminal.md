# Terminal Control

The `terminal` module provides comprehensive control over terminal properties including screen management, sizing, and scrolling.

## Key Concepts

Terminal control operations manipulate the terminal window itself rather than cursor position or text styling. These operations are essential for creating full-screen TUI applications.

## Common Operations

### Screen Clearing

```rust
use crossterm::{
    execute,
    terminal::{Clear, ClearType},
};
use std::io::stdout;

// Clear entire screen
execute!(stdout(), Clear(ClearType::All))?;

// Clear from cursor to end of screen
execute!(stdout(), Clear(ClearType::FromCursorDown))?;

// Clear from cursor to start of screen
execute!(stdout(), Clear(ClearType::FromCursorUp))?;

// Clear current line
execute!(stdout(), Clear(ClearType::CurrentLine))?;
```

### Alternate Screen

The alternate screen buffer is essential for TUI applications. It preserves the user's terminal content and restores it when your app exits.

```rust
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};

// Enter alternate screen (saves current screen)
execute!(stdout(), EnterAlternateScreen)?;

// Your TUI application runs here

// Leave alternate screen (restores original screen)
execute!(stdout(), LeaveAlternateScreen)?;
```

**When to use:**
- Full-screen TUI applications
- Applications that need to preserve user's terminal state
- Applications that modify the entire screen

### Terminal Sizing

```rust
use crossterm::{
    execute,
    terminal::{SetSize, size},
};

// Get current terminal size
let (cols, rows) = size()?;
println!("Terminal is {} columns by {} rows", cols, rows);

// Resize terminal (may not work in all terminals)
execute!(stdout(), SetSize(80, 24))?;
```

**Gotcha:** Not all terminals support programmatic resizing. Always check your target terminal's capabilities.

### Scrolling

```rust
use crossterm::{
    execute,
    terminal::{ScrollUp, ScrollDown},
};

// Scroll up 5 lines
execute!(stdout(), ScrollUp(5))?;

// Scroll down 3 lines
execute!(stdout(), ScrollDown(3))?;
```

### Terminal Title

```rust
use crossterm::{
    execute,
    terminal::SetTitle,
};

execute!(stdout(), SetTitle("My Application v1.0"))?;
```

## Raw Mode

Raw mode is critical for interactive applications. It disables line buffering and special character processing.

```rust
use crossterm::terminal::{self, enable_raw_mode, disable_raw_mode};

// Enable raw mode
enable_raw_mode()?;

// Your interactive application logic
// - Individual keystrokes are captured immediately
// - Ctrl+C doesn't terminate the process
// - Enter doesn't echo newline

// Always disable when done
disable_raw_mode()?;
```

**What raw mode does:**
- Disables line buffering (get input immediately)
- Disables echoing input to screen
- Disables special character processing (Ctrl+C, Ctrl+Z, etc.)
- Allows reading individual keystrokes

**When to use:**
- Text editors
- Interactive prompts
- Games
- Any application that needs immediate keystroke access

**Critical:** Always disable raw mode before exiting, or the user's terminal will be unusable. Use Drop implementations or defer patterns:

```rust
struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

fn main() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let _guard = RawModeGuard; // Ensures cleanup on panic

    // Your application logic

    Ok(())
}
```

## Related

- [Cursor Control](./cursor.md) - For positioning content on screen
- [Event Handling](./events.md) - For processing user input in raw mode
- [Raw Mode](./raw-mode.md) - Detailed raw mode patterns
