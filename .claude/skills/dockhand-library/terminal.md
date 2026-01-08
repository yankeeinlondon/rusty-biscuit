# Terminal Rendering

The `terminal` module provides utilities for detecting terminal capabilities and rendering content appropriately.

## Terminal Capability Detection

```rust
use shared::terminal::{ColorCapability, supports_hyperlinks};

// Detect color support
let color_support = ColorCapability::detect();

match color_support {
    ColorCapability::TrueColor => {
        // 24-bit RGB color support
        println!("\x1b[38;2;255;127;0mTrueColor text\x1b[0m");
    }
    ColorCapability::EightBit => {
        // 256 color support
        println!("\x1b[38;5;208mEightBit color\x1b[0m");
    }
    ColorCapability::Basic => {
        // 16 color support
        println!("\x1b[31mBasic red\x1b[0m");
    }
    ColorCapability::None => {
        // No color support (or disabled)
        println!("Plain text");
    }
}
```

## Environment Variables

The module respects standard environment variables:

```rust
// Force disable colors
std::env::set_var("NO_COLOR", "1");
assert_eq!(ColorCapability::detect(), ColorCapability::None);

// Force basic colors
std::env::set_var("CLICOLOR", "1");

// Force color mode
std::env::set_var("COLORTERM", "truecolor"); // 24-bit
std::env::set_var("COLORTERM", "256color");  // 8-bit
```

## Hyperlink Support

```rust
use shared::terminal::supports_hyperlinks;

if supports_hyperlinks() {
    // Terminal supports OSC 8 hyperlinks
    println!("\x1b]8;;https://rust-lang.org\x1b\\Rust\x1b]8;;\x1b\\");
} else {
    // Fallback to plain text
    println!("Rust (https://rust-lang.org)");
}
```

## Terminal Dimensions

```rust
use shared::terminal::{terminal_size, TerminalSize};

if let Some(size) = terminal_size() {
    println!("Width: {} columns", size.width);
    println!("Height: {} rows", size.height);

    // Adapt output to terminal width
    if size.width < 80 {
        // Use compact format
    } else {
        // Use full format
    }
}
```

## Adaptive Rendering

### Color-Aware Output

```rust
use shared::terminal::{ColorCapability, style_for_capability};

let capability = ColorCapability::detect();

// Get appropriate style for terminal
let error_style = style_for_capability(capability, "error");
let warning_style = style_for_capability(capability, "warning");

println!("{}Error:{} Something went wrong", error_style.prefix, error_style.suffix);
println!("{}Warning:{} Be careful", warning_style.prefix, warning_style.suffix);
```

### Progressive Enhancement

```rust
use shared::terminal::{render_table, TableOptions};

let data = vec![
    vec!["Name", "Version", "License"],
    vec!["serde", "1.0", "MIT"],
    vec!["tokio", "1.48", "MIT"],
];

// Table adapts to terminal capabilities
let options = TableOptions::default()
    .with_color(ColorCapability::detect())
    .with_width(terminal_size().map(|s| s.width));

render_table(&data, options);
```

## ANSI Escape Sequences

### Common Sequences

```rust
use shared::terminal::ansi;

// Cursor movement
print!("{}", ansi::cursor_up(2));
print!("{}", ansi::cursor_down(1));
print!("{}", ansi::cursor_forward(10));
print!("{}", ansi::cursor_position(5, 10));

// Clear operations
print!("{}", ansi::clear_line());
print!("{}", ansi::clear_screen());
print!("{}", ansi::clear_to_end());

// Styling
print!("{}", ansi::bold());
print!("{}", ansi::italic());
print!("{}", ansi::underline());
print!("{}", ansi::reset());
```

### Color Helpers

```rust
// RGB colors (24-bit)
print!("{}", ansi::rgb_fg(255, 127, 0));  // Orange foreground
print!("{}", ansi::rgb_bg(0, 0, 255));    // Blue background

// 256 colors (8-bit)
print!("{}", ansi::color_256_fg(208));    // Orange
print!("{}", ansi::color_256_bg(21));     // Blue

// Basic colors (4-bit)
print!("{}", ansi::red());
print!("{}", ansi::green());
print!("{}", ansi::blue());
```

## Terminal Images

```rust
use shared::terminal::{can_display_images, display_image};

if can_display_images() {
    // Use viuer to display image
    display_image("diagram.png")?;
} else {
    // Fall back to ASCII art or skip
    println!("[Image: diagram.png]");
}
```

## Testing Utilities

```rust
use shared::testing::{TestTerminal, strip_ansi_codes};

// Strip ANSI codes for assertions
let output = "\x1b[31mError:\x1b[0m File not found";
assert_eq!(strip_ansi_codes(output), "Error: File not found");

// Test with controlled terminal
let mut terminal = TestTerminal::new();
terminal.set_color_capability(ColorCapability::Basic);
terminal.run(|term| {
    // Your rendering code here
});
```

## Platform Considerations

### Windows

```rust
#[cfg(target_os = "windows")]
{
    // Enable ANSI on Windows 10+
    shared::terminal::enable_ansi_support();

    // Check if running in legacy console
    if shared::terminal::is_legacy_console() {
        // Use fallback rendering
    }
}
```

### CI/CD Environments

```rust
use shared::terminal::is_ci;

if is_ci() {
    // Detected CI environment (GitHub Actions, etc.)
    // Often supports color but not interactive features
}

// Common CI variables checked:
// - CI
// - CONTINUOUS_INTEGRATION
// - GITHUB_ACTIONS
// - GITLAB_CI
// - CIRCLECI
```

## Best Practices

### 1. Progressive Enhancement

```rust
// Always provide fallbacks
match ColorCapability::detect() {
    ColorCapability::TrueColor => render_with_gradients(),
    ColorCapability::EightBit => render_with_palette(),
    ColorCapability::Basic => render_with_basic_colors(),
    ColorCapability::None => render_plain_text(),
}
```

### 2. Respect User Preferences

```rust
// Check NO_COLOR first
if std::env::var("NO_COLOR").is_ok() {
    // User explicitly disabled colors
    render_plain();
    return;
}
```

### 3. Terminal Width Awareness

```rust
use shared::terminal::wrap_text;

let text = "Long text that might need wrapping...";
let width = terminal_size().map(|s| s.width).unwrap_or(80);

let wrapped = wrap_text(text, width - 4); // Leave margin
println!("{}", wrapped);
```

## Integration Examples

### With Markdown Rendering

```rust
use shared::markdown::Markdown;
use shared::terminal::ColorCapability;

let md = Markdown::from(content);

// Choose theme based on terminal
let theme = match ColorCapability::detect() {
    ColorCapability::TrueColor => "one-dark",
    ColorCapability::EightBit => "base16-ocean",
    _ => "none",
};
```

### With Progress Indicators

```rust
use shared::terminal::{supports_unicode, spinner_chars};

let spinner = if supports_unicode() {
    spinner_chars::UNICODE  // ⣾⣽⣻⢿⡿⣟⣯⣷
} else {
    spinner_chars::ASCII   // |/-\
};
```

## Performance Tips

1. **Cache capability detection**: Don't detect on every render
2. **Batch ANSI sequences**: Combine multiple style changes
3. **Use raw terminal mode sparingly**: Expensive on some systems
4. **Buffer output**: Write complete lines at once

```rust
// Cache detection
lazy_static! {
    static ref COLOR_SUPPORT: ColorCapability = ColorCapability::detect();
}

// Use cached value
if *COLOR_SUPPORT != ColorCapability::None {
    // Render with color
}
```