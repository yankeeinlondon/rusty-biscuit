# Platform Issues

crossterm aims for cross-platform compatibility, but some platform-specific limitations and gotchas exist. Understanding these helps you build robust applications.

## macOS Issues

### /dev/tty Read Not Supported

**Problem:** crossterm doesn't support reading from `/dev/tty` on macOS. This can cause applications to hang when input is piped.

```rust
// This hangs on macOS when input is piped
let event = event::read()?;
```

**Workarounds:**

1. **Detect piped input and skip event reading:**

   ```rust
   use std::io::IsTerminal;

   if std::io::stdin().is_terminal() {
       // Safe to read events
       let event = event::read()?;
   } else {
       // Input is piped, skip event reading
       println!("Cannot read terminal events from piped input");
   }
   ```

2. **Use alternative library for macOS:**

   ```rust
   #[cfg(target_os = "macos")]
   use termion; // Supports /dev/tty on macOS

   #[cfg(not(target_os = "macos"))]
   use crossterm;
   ```

3. **Document limitation and avoid piped input:**

   Document that your application requires direct terminal access on macOS.

### Terminal Capability Detection

Not all macOS terminals support all features:

```rust
// Test if terminal supports true color
fn supports_true_color() -> bool {
    std::env::var("COLORTERM")
        .map(|v| v == "truecolor" || v == "24bit")
        .unwrap_or(false)
}

// Use with fallback
if supports_true_color() {
    execute!(stdout(), SetForegroundColor(Color::Rgb { r: 255, g: 128, b: 0 }))?;
} else {
    execute!(stdout(), SetForegroundColor(Color::Yellow))?;
}
```

## Windows Issues

### Key Hold Detection

**Problem:** Windows doesn't properly detect when a key is being held. Instead of Press -> Repeat -> Release, you get multiple Press events.

```rust
// Windows behavior
Event::Key(KeyEvent { kind: KeyEventKind::Press, .. })  // Initial press
Event::Key(KeyEvent { kind: KeyEventKind::Press, .. })  // Not Repeat!
Event::Key(KeyEvent { kind: KeyEventKind::Press, .. })  // Not Repeat!
```

**Workaround - Manual Repeat Detection:**

```rust
use std::time::{Duration, Instant};
use std::collections::HashMap;

struct KeyRepeatTracker {
    last_press: HashMap<KeyCode, Instant>,
    repeat_delay: Duration,
}

impl KeyRepeatTracker {
    fn new() -> Self {
        Self {
            last_press: HashMap::new(),
            repeat_delay: Duration::from_millis(50),
        }
    }

    fn is_repeat(&mut self, code: KeyCode) -> bool {
        let now = Instant::now();

        if let Some(&last) = self.last_press.get(&code) {
            let is_repeat = now.duration_since(last) < self.repeat_delay;
            self.last_press.insert(code, now);
            is_repeat
        } else {
            self.last_press.insert(code, now);
            false
        }
    }

    fn clear(&mut self, code: &KeyCode) {
        self.last_press.remove(code);
    }
}

// Usage
let mut tracker = KeyRepeatTracker::new();

if let Event::Key(key) = event::read()? {
    if tracker.is_repeat(key.code) {
        println!("Key repeat: {:?}", key.code);
    } else {
        println!("Key press: {:?}", key.code);
    }
}
```

### Console API Limitations

Windows 7/8 have limited support for ANSI escape sequences. Windows 10+ has much better support.

**Detection:**

```rust
fn is_windows_10_or_later() -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        if let Ok(output) = Command::new("cmd")
            .args(&["/c", "ver"])
            .output()
        {
            let version = String::from_utf8_lossy(&output.stdout);
            return version.contains("10.") || version.contains("11.");
        }
    }
    false
}

// Use with fallback
if is_windows_10_or_later() {
    // Use full color support
} else {
    // Use basic 16-color mode
}
```

### Mouse Coordinate Limitations

On older Windows terminals, mouse coordinates may be limited to 255x255.

**Workaround:**

```rust
fn clamp_mouse_coords(col: u16, row: u16, max_col: u16, max_row: u16) -> (u16, u16) {
    (col.min(max_col), row.min(max_row))
}
```

## Linux/Unix Issues

### Terminal Emulator Variations

Different terminals support different features:

```rust
fn detect_terminal() -> String {
    std::env::var("TERM")
        .unwrap_or_else(|_| "unknown".to_string())
}

// Adjust features based on terminal
match detect_terminal().as_str() {
    "xterm-256color" => {
        // Full 256-color support
    }
    "xterm" => {
        // Basic 16-color support
    }
    "linux" | "vt100" => {
        // Very basic support
    }
    _ => {
        // Conservative defaults
    }
}
```

### SSH Session Detection

Some features behave differently over SSH:

```rust
fn is_ssh_session() -> bool {
    std::env::var("SSH_CONNECTION").is_ok() ||
    std::env::var("SSH_CLIENT").is_ok() ||
    std::env::var("SSH_TTY").is_ok()
}

// Disable mouse capture over SSH (often problematic)
if !is_ssh_session() {
    execute!(stdout(), EnableMouseCapture)?;
}
```

### Clipboard over SSH

OSC52 clipboard support varies widely:

```rust
#[cfg(feature = "osc52")]
fn copy_to_clipboard(text: &str) -> io::Result<()> {
    if is_ssh_session() {
        // OSC52 may not work over SSH
        println!("Clipboard may not work over SSH");
    }

    execute!(stdout(), CopyToClipboard(text))?;
    Ok(())
}
```

## Cross-Platform Best Practices

### 1. Terminal Capability Detection

Always detect capabilities before using advanced features:

```rust
struct TerminalCapabilities {
    true_color: bool,
    mouse: bool,
    clipboard: bool,
    focus_events: bool,
}

impl TerminalCapabilities {
    fn detect() -> Self {
        Self {
            true_color: std::env::var("COLORTERM")
                .map(|v| v == "truecolor" || v == "24bit")
                .unwrap_or(false),
            mouse: !is_ssh_session(), // Conservative
            clipboard: cfg!(feature = "osc52") && modern_terminal(),
            focus_events: modern_terminal(),
        }
    }
}

fn modern_terminal() -> bool {
    matches!(
        detect_terminal().as_str(),
        "xterm-256color" | "screen-256color" | "tmux-256color"
    )
}
```

### 2. Graceful Degradation

Provide fallbacks for unsupported features:

```rust
fn set_color_with_fallback(color: Color) -> io::Result<()> {
    match color {
        Color::Rgb { r, g, b } if supports_true_color() => {
            execute!(stdout(), SetForegroundColor(Color::Rgb { r, g, b }))
        }
        Color::Rgb { .. } => {
            // Fallback to approximate ANSI color
            execute!(stdout(), SetForegroundColor(Color::Yellow))
        }
        other => {
            execute!(stdout(), SetForegroundColor(other))
        }
    }
}
```

### 3. Test on Target Platforms

Essential terminals to test on:

**Windows:**
- Windows Terminal (modern)
- PowerShell (legacy)
- cmd.exe (legacy)

**macOS:**
- Terminal.app
- iTerm2
- Alacritty

**Linux:**
- GNOME Terminal
- Konsole
- xterm
- tmux/screen

### 4. Platform-Specific Code

Use conditional compilation for platform-specific workarounds:

```rust
#[cfg(target_os = "windows")]
fn platform_init() -> io::Result<()> {
    // Windows-specific initialization
    Ok(())
}

#[cfg(target_os = "macos")]
fn platform_init() -> io::Result<()> {
    // macOS-specific initialization
    if !std::io::stdin().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Piped input not supported on macOS"
        ));
    }
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn platform_init() -> io::Result<()> {
    // Linux/Unix initialization
    Ok(())
}
```

## Common Compatibility Patterns

### Safe Feature Enablement

```rust
struct TerminalFeatures {
    _mouse: Option<()>,
    _focus: Option<()>,
}

impl TerminalFeatures {
    fn enable_all() -> io::Result<Self> {
        let mouse = if should_enable_mouse() {
            execute!(stdout(), EnableMouseCapture).ok()
        } else {
            None
        };

        let focus = if should_enable_focus() {
            execute!(stdout(), EnableFocusChange).ok()
        } else {
            None
        };

        Ok(Self {
            _mouse: mouse,
            _focus: focus,
        })
    }
}

impl Drop for TerminalFeatures {
    fn drop(&mut self) {
        if self._mouse.is_some() {
            let _ = execute!(stdout(), DisableMouseCapture);
        }
        if self._focus.is_some() {
            let _ = execute!(stdout(), DisableFocusChange);
        }
    }
}

fn should_enable_mouse() -> bool {
    !is_ssh_session() && std::io::stdin().is_terminal()
}

fn should_enable_focus() -> bool {
    std::io::stdin().is_terminal()
}
```

### Environment-Based Configuration

```rust
struct AppConfig {
    colors: ColorMode,
    mouse: bool,
    unicode: bool,
}

enum ColorMode {
    NoColor,
    Basic16,
    Ansi256,
    TrueColor,
}

impl AppConfig {
    fn from_env() -> Self {
        Self {
            colors: Self::detect_color_mode(),
            mouse: Self::should_enable_mouse(),
            unicode: Self::supports_unicode(),
        }
    }

    fn detect_color_mode() -> ColorMode {
        if std::env::var("NO_COLOR").is_ok() {
            return ColorMode::NoColor;
        }

        if let Ok(colorterm) = std::env::var("COLORTERM") {
            if colorterm == "truecolor" || colorterm == "24bit" {
                return ColorMode::TrueColor;
            }
        }

        if let Ok(term) = std::env::var("TERM") {
            if term.contains("256color") {
                return ColorMode::Ansi256;
            }
        }

        ColorMode::Basic16
    }

    fn should_enable_mouse() -> bool {
        std::env::var("TERM_PROGRAM").is_ok() && !is_ssh_session()
    }

    fn supports_unicode() -> bool {
        std::env::var("LANG")
            .unwrap_or_default()
            .contains("UTF-8")
    }
}
```

## Related

- [Feature Flags](./features.md) - Enabling platform-specific features
- [Raw Mode](./raw-mode.md) - Platform differences in raw mode
- [Event Handling](./events.md) - Platform-specific event behavior
