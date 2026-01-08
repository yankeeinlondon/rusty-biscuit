# Testing Utilities

The `testing` module provides utilities for testing terminal output, including ANSI escape sequence handling and controlled terminal environments.

## ANSI Code Stripping

```rust
use shared::testing::strip_ansi_codes;

// Remove ANSI escape sequences for plain-text assertions
let colored_output = "\x1b[31mError:\x1b[0m Something went wrong";
let plain = strip_ansi_codes(colored_output);
assert_eq!(plain, "Error: Something went wrong");

// Works with complex sequences
let complex = "\x1b[1;32mSuccess!\x1b[0m \x1b[38;2;255;127;0mWarning\x1b[0m";
let plain = strip_ansi_codes(complex);
assert_eq!(plain, "Success! Warning");
```

## Test Terminal

The `TestTerminal` provides a controlled environment for testing terminal-aware code:

```rust
use shared::testing::TestTerminal;
use shared::terminal::ColorCapability;

#[test]
fn test_color_output() {
    let mut terminal = TestTerminal::new();

    // Configure capabilities
    terminal.set_color_capability(ColorCapability::Basic);
    terminal.set_width(80);
    terminal.set_height(24);

    // Run code in controlled environment
    terminal.run(|term| {
        term.push_str("\x1b[31mRed text\x1b[0m");
    });

    // Assert on output
    terminal.assert_contains("Red text");
    terminal.assert_has_color("\x1b[31m");
}
```

### Configuration Options

```rust
let mut terminal = TestTerminal::new()
    .with_color(ColorCapability::TrueColor)
    .with_dimensions(120, 40)
    .with_hyperlinks(true)
    .with_unicode(true);
```

### Output Assertions

```rust
// Content assertions
terminal.assert_contains("expected text");
terminal.assert_not_contains("unwanted text");
terminal.assert_line_count(5);
terminal.assert_empty();

// ANSI code assertions
terminal.assert_has_color("\x1b[31m");      // Has red
terminal.assert_no_ansi();                  // No escape codes
terminal.assert_has_style("\x1b[1m");       // Has bold

// Get raw output
let output = terminal.output();
let lines = terminal.lines();
```

## Testing Strategies

### Strategy 1: Strip and Compare

Best for testing content regardless of formatting:

```rust
#[test]
fn test_error_message() {
    let output = format_error("File not found");
    let plain = strip_ansi_codes(&output);

    assert!(plain.contains("Error:"));
    assert!(plain.contains("File not found"));
}
```

### Strategy 2: Controlled Environment

Best for testing adaptive rendering:

```rust
#[test]
fn test_adaptive_rendering() {
    // Test with colors
    let mut term = TestTerminal::new()
        .with_color(ColorCapability::TrueColor);

    term.run(|t| render_output(t));
    term.assert_has_color("\x1b[38;2;"); // RGB color

    // Test without colors
    let mut term = TestTerminal::new()
        .with_color(ColorCapability::None);

    term.run(|t| render_output(t));
    term.assert_no_ansi();
}
```

### Strategy 3: Snapshot Testing

Best for complex output with formatting:

```rust
use insta::assert_snapshot;

#[test]
fn test_table_output() {
    let mut term = TestTerminal::new()
        .with_color(ColorCapability::Basic)
        .with_dimensions(80, 24);

    term.run(|t| render_table(t, &data));

    // Snapshot includes ANSI codes
    assert_snapshot!(term.output());
}
```

## Mock Terminal Capabilities

```rust
use shared::testing::{MockCapabilities, with_mock_terminal};

#[test]
fn test_with_mock_capabilities() {
    let caps = MockCapabilities {
        color: ColorCapability::EightBit,
        width: Some(100),
        height: Some(30),
        hyperlinks: true,
        unicode: true,
    };

    with_mock_terminal(caps, || {
        // Code here sees mocked capabilities
        assert_eq!(terminal_size(), Some(TerminalSize { width: 100, height: 30 }));
        assert_eq!(ColorCapability::detect(), ColorCapability::EightBit);
    });
}
```

## Environment Isolation

```rust
use shared::testing::isolated_env;

#[test]
fn test_environment_variables() {
    isolated_env(|| {
        // Changes to env vars don't leak
        std::env::set_var("NO_COLOR", "1");
        assert_eq!(ColorCapability::detect(), ColorCapability::None);
    });

    // Original environment restored
}
```

## Common Patterns

### Testing Progress Indicators

```rust
#[test]
fn test_progress_bar() {
    let mut term = TestTerminal::new();

    term.run(|t| {
        for i in 0..=100 {
            t.clear_line();
            t.push_str(&format!("Progress: {}%", i));
            if i < 100 {
                t.carriage_return();
            }
        }
    });

    // Final output should show 100%
    assert!(term.output().contains("Progress: 100%"));
}
```

### Testing Interactive Menus

```rust
#[test]
fn test_menu_rendering() {
    let mut term = TestTerminal::new()
        .with_dimensions(80, 10);

    term.run(|t| {
        render_menu(t, &["Option 1", "Option 2", "Option 3"], 1);
    });

    // Second option should be highlighted
    let lines = term.lines();
    assert!(lines[1].contains("\x1b[7m")); // Reverse video
}
```

### Testing Color Gradients

```rust
#[test]
fn test_gradient_rendering() {
    let mut term = TestTerminal::new()
        .with_color(ColorCapability::TrueColor);

    term.run(|t| {
        render_gradient(t, 0.0, 1.0, 10);
    });

    // Should contain RGB sequences
    term.assert_has_color("\x1b[38;2;");

    // Test fallback
    term.set_color_capability(ColorCapability::None);
    term.clear();
    term.run(|t| {
        render_gradient(t, 0.0, 1.0, 10);
    });
    term.assert_no_ansi();
}
```

## Performance Testing

```rust
use shared::testing::measure_render_time;

#[test]
fn test_render_performance() {
    let duration = measure_render_time(|| {
        let mut term = TestTerminal::new();
        term.run(|t| {
            for _ in 0..1000 {
                render_complex_output(t);
            }
        });
    });

    // Assert reasonable performance
    assert!(duration.as_millis() < 100);
}
```

## Integration with Other Crates

### With `insta`

```rust
use insta::assert_snapshot;

#[test]
fn test_formatted_output() {
    let mut term = TestTerminal::new();
    term.run(|t| render_report(t, &data));

    // Snapshot with settings
    let output = strip_ansi_codes(&term.output());
    assert_snapshot!(output);
}
```

### With `proptest`

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_strip_preserves_content(
        content in "[a-zA-Z0-9 ]+",
        color in prop::sample::select(vec![31, 32, 33, 34])
    ) {
        let colored = format!("\x1b[{}m{}\x1b[0m", color, content);
        let stripped = strip_ansi_codes(&colored);
        prop_assert_eq!(stripped, content);
    }
}
```

## Debugging Tips

```rust
// Enable debug output
let mut term = TestTerminal::new()
    .with_debug(true);

term.run(|t| {
    // Operations logged to stderr
    t.push_str("Hello");
});

// Dump raw bytes
println!("Raw output: {:?}", term.output().as_bytes());

// Inspect ANSI sequences
for sequence in term.ansi_sequences() {
    println!("Found sequence: {:?}", sequence);
}
```