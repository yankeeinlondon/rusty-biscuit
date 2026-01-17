# Cursor Control

The `cursor` module provides precise cursor positioning and appearance management for creating dynamic terminal interfaces.

## Key Concepts

Cursor operations control the terminal's text insertion point. All terminal output appears at the current cursor position.

## Positioning

### Absolute Positioning

```rust
use crossterm::{execute, cursor::MoveTo};
use std::io::stdout;

// Move to column 10, row 5 (0-indexed)
execute!(stdout(), MoveTo(10, 5))?;
```

**Note:** Coordinates are 0-indexed. (0, 0) is the top-left corner.

### Relative Positioning

```rust
use crossterm::{
    execute,
    cursor::{MoveUp, MoveDown, MoveLeft, MoveRight},
};

// Move cursor relative to current position
execute!(stdout(), MoveRight(3))?;  // Move 3 columns right
execute!(stdout(), MoveDown(2))?;   // Move 2 rows down
execute!(stdout(), MoveLeft(1))?;   // Move 1 column left
execute!(stdout(), MoveUp(1))?;     // Move 1 row up
```

### Line Navigation

```rust
use crossterm::{
    execute,
    cursor::{MoveToColumn, MoveToRow, MoveToNextLine, MoveToPreviousLine},
};

// Move to specific column in current row
execute!(stdout(), MoveToColumn(0))?;

// Move to start of next line
execute!(stdout(), MoveToNextLine(1))?;

// Move to start of previous line
execute!(stdout(), MoveToPreviousLine(1))?;
```

## Visibility

```rust
use crossterm::{execute, cursor::{Hide, Show}};

// Hide cursor (useful during updates)
execute!(stdout(), Hide)?;

// Perform screen updates

// Show cursor again
execute!(stdout(), Show)?;
```

**Best practice:** Hide the cursor during batch updates to prevent flickering, then show it again when done.

## Save and Restore Position

```rust
use crossterm::{
    execute,
    cursor::{SavePosition, RestorePosition},
};

// Save current cursor position
execute!(stdout(), SavePosition)?;

// Move cursor and do operations
execute!(stdout(), MoveTo(0, 0))?;
print!("Status message");

// Restore saved cursor position
execute!(stdout(), RestorePosition)?;
```

**Gotcha:** Not all terminals support save/restore. Test on your target terminals. Most modern terminals (xterm, Windows Terminal) support this.

## Cursor Position Query

```rust
use crossterm::cursor::position;

// Get current cursor position
let (col, row) = position()?;
println!("Cursor is at column {}, row {}", col, row);
```

**Note:** This requires reading from the terminal, which can be slow. Use sparingly in performance-critical code.

## Common Patterns

### Drawing a Box

```rust
use crossterm::{
    execute, queue,
    cursor::MoveTo,
    style::Print,
};
use std::io::{stdout, Write};

fn draw_box(x: u16, y: u16, width: u16, height: u16) -> io::Result<()> {
    let mut stdout = stdout();

    // Draw top border
    queue!(stdout, MoveTo(x, y), Print("┌"))?;
    for _ in 0..width-2 {
        queue!(stdout, Print("─"))?;
    }
    queue!(stdout, Print("┐"))?;

    // Draw sides
    for row in 1..height-1 {
        queue!(stdout, MoveTo(x, y + row), Print("│"))?;
        queue!(stdout, MoveTo(x + width - 1, y + row), Print("│"))?;
    }

    // Draw bottom border
    queue!(stdout, MoveTo(x, y + height - 1), Print("└"))?;
    for _ in 0..width-2 {
        queue!(stdout, Print("─"))?;
    }
    queue!(stdout, Print("┘"))?;

    stdout.flush()?;
    Ok(())
}
```

### Cursor Hide Guard

```rust
struct CursorGuard;

impl CursorGuard {
    fn new() -> io::Result<Self> {
        execute!(stdout(), Hide)?;
        Ok(CursorGuard)
    }
}

impl Drop for CursorGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), Show);
    }
}

fn main() -> io::Result<()> {
    let _guard = CursorGuard::new()?;

    // Cursor is hidden during this scope
    // Automatically restored on drop (even on panic)

    Ok(())
}
```

## Related

- [Terminal Control](./terminal.md) - For screen and window operations
- [Styling](./styling.md) - For coloring text at cursor positions
- [Event Handling](./events.md) - For responding to user input
