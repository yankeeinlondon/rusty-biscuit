# Raw Mode

Raw mode is essential for interactive terminal applications that need immediate access to user input without line buffering or special character processing.

## What Raw Mode Does

### Without Raw Mode (Canonical/Cooked Mode)

- Input is **line-buffered** - user must press Enter before program sees input
- Special characters are processed (Ctrl+C terminates, Ctrl+Z suspends)
- Input is **echoed** to the screen automatically
- Backspace edits the input line

### With Raw Mode

- Input is **immediate** - program sees each keystroke instantly
- Special characters are passed as normal input (Ctrl+C becomes a key event)
- Input is **NOT echoed** - program must handle display
- Raw keystrokes without interpretation

## Basic Usage

```rust
use crossterm::terminal::{self, enable_raw_mode, disable_raw_mode};
use crossterm::event::{self, Event, KeyCode};
use std::io;
use std::time::Duration;

fn main() -> io::Result<()> {
    // Enable raw mode
    enable_raw_mode()?;

    println!("Press 'q' to quit");

    loop {
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.code == KeyCode::Char('q') {
                        break;
                    }
                    println!("Key pressed: {:?}", key.code);
                }
                _ => {}
            }
        }
    }

    // CRITICAL: Always disable before exit
    disable_raw_mode()?;

    Ok(())
}
```

## Critical: Always Cleanup

**Problem:** If your application panics or exits without disabling raw mode, the user's terminal becomes unusable (no echo, no Ctrl+C).

**Solution:** Use a guard struct with Drop implementation:

```rust
struct RawModeGuard;

impl RawModeGuard {
    fn new() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        Ok(RawModeGuard)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        // Cleanup even on panic
        let _ = terminal::disable_raw_mode();
    }
}

fn main() -> io::Result<()> {
    let _guard = RawModeGuard::new()?;

    // Your application logic
    // Raw mode automatically disabled when _guard drops

    Ok(())
}
```

## Common Patterns

### Text Input with Echo

Since raw mode doesn't echo input, you must handle it:

```rust
use crossterm::{
    cursor::{MoveToColumn, MoveTo},
    execute, queue,
    style::Print,
    terminal::{self, Clear, ClearType},
};
use std::io::{stdout, Write};

fn read_line() -> io::Result<String> {
    terminal::enable_raw_mode()?;
    let mut input = String::new();
    let mut stdout = stdout();

    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char(c) => {
                        input.push(c);
                        queue!(stdout, Print(c))?;
                        stdout.flush()?;
                    }
                    KeyCode::Backspace => {
                        if input.pop().is_some() {
                            queue!(
                                stdout,
                                cursor::MoveLeft(1),
                                Print(' '),
                                cursor::MoveLeft(1)
                            )?;
                            stdout.flush()?;
                        }
                    }
                    KeyCode::Enter => {
                        println!();
                        break;
                    }
                    KeyCode::Esc => {
                        input.clear();
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    terminal::disable_raw_mode()?;
    Ok(input)
}
```

### Arrow Key Navigation

```rust
fn handle_navigation(key: KeyCode) -> Option<Direction> {
    match key {
        KeyCode::Up => Some(Direction::Up),
        KeyCode::Down => Some(Direction::Down),
        KeyCode::Left => Some(Direction::Left),
        KeyCode::Right => Some(Direction::Right),
        _ => None,
    }
}

enum Direction {
    Up,
    Down,
    Left,
    Right,
}
```

### Ctrl+C Handling

In raw mode, Ctrl+C is just another key event. You must handle it explicitly:

```rust
use crossterm::event::{KeyCode, KeyModifiers};

if let Event::Key(key) = event::read()? {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        println!("\nInterrupted!");
        break;
    }
}
```

## TUI Application Template

```rust
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, stdout};
use std::time::Duration;

struct App {
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        Self { should_quit: false }
    }

    fn run(&mut self) -> io::Result<()> {
        // Setup
        terminal::enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;

        // Main loop
        while !self.should_quit {
            self.render()?;
            self.handle_events()?;
        }

        // Cleanup
        execute!(stdout(), LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;

        Ok(())
    }

    fn render(&self) -> io::Result<()> {
        // Draw your UI
        Ok(())
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    match (key.code, key.modifiers) {
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            self.should_quit = true;
                        }
                        (KeyCode::Char('q'), KeyModifiers::NONE) => {
                            self.should_quit = true;
                        }
                        _ => {
                            self.handle_key(key.code)?;
                        }
                    }
                }
                Event::Resize(_, _) => {
                    // Handle resize
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, code: KeyCode) -> io::Result<()> {
        // Handle specific keys
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let mut app = App::new();
    app.run()
}
```

## When to Use Raw Mode

**Always use for:**
- Text editors
- Interactive menus and prompts
- Games
- Full-screen TUI applications
- REPLs

**Don't use for:**
- Simple output-only tools
- Applications that should respect Ctrl+C
- Standard CLI argument processing

## Gotchas

### Terminal State Corruption

**Problem:** If your app crashes without cleanup, terminal is unusable.

**Solution:** Use Drop guard or panic hook:

```rust
use std::panic;

fn main() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // Your app
}
```

### Signal Handling

**Problem:** SIGTERM and SIGINT don't automatically cleanup.

**Solution:** Use signal handlers to cleanup before exit (requires `signal-hook` crate):

```rust
use signal_hook::{consts::TERM_SIGNALS, iterator::Signals};

let mut signals = Signals::new(TERM_SIGNALS)?;
for signal in signals.forever() {
    terminal::disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    std::process::exit(0);
}
```

### Platform Differences

**Windows:** Raw mode works well, but some key combinations are captured by Windows (e.g., Alt+Tab).

**macOS/Linux:** Full control over key events, but some terminal emulators may intercept certain keys.

## Related

- [Event Handling](./events.md) - Processing events in raw mode
- [Terminal Control](./terminal.md) - Alternate screen and other terminal operations
- [Platform Issues](./platform-issues.md) - Platform-specific raw mode gotchas
