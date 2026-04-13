# Testing TUI Applications

Testing Terminal User Interfaces (TUIs) presents unique challenges compared to standard CLI or web applications. TUIs rely on complex state management, terminal-specific escape codes, and real-time event loops. This guide outlines the best practices for building testable TUI applications, with a focus on **Ratatui** for implementation details.

## 1. Architectural Foundation: Separation of Concerns

The most important step for testability is decoupling your application's logic from its presentation.

### The Model-View-Action Pattern

- **Model (State)**: A pure-Rust struct that holds the application data and state (e.g., `selected_index`, `input_text`).
- **View (Render)**: A stateless function that takes the Model and a `Frame` and renders the UI.
- **Action (Events)**: A handler that transforms input events into state changes.

By keeping the **View** stateless, you can test it in isolation by providing mock state and verifying the output buffer.

## 2. Testing with Ratatui `TestBackend`

Ratatui provides a `TestBackend` that renders to an in-memory buffer instead of a real terminal. This is essential for fast, deterministic tests that can run in headless CI environments.

### Example: Basic Buffer Assertion

```rust
use ratatui::{backend::TestBackend, Terminal, buffer::Buffer, widgets::Paragraph};

#[test]
fn test_ui_rendering() {
    // 1. Setup a small virtual terminal
    let backend = TestBackend::new(20, 1);
    let mut terminal = Terminal::new(backend).unwrap();

    // 2. Render a widget
    terminal.draw(|f| {
        f.render_widget(Paragraph::new("Hello World"), f.size());
    }).unwrap();

    // 3. Define the expected state of the buffer
    let expected = Buffer::with_lines(vec!["Hello World         "]);

    // 4. Assert equality
    terminal.backend().assert_buffer(&expected);
}
```

## 3. Snapshot Testing (Golden Records)

For complex UIs, manually constructing a `Buffer` for assertions is tedious and brittle. **Snapshot testing** allows you to capture the rendered output and compare it against a "gold" version stored in a file.

### Using `insta` for Snapshots
The `insta` crate is the industry standard for snapshot testing in Rust.

```rust
#[test]
fn test_complex_dashboard() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let app = App::mock_state();

    terminal.draw(|f| ui::render(f, &app)).unwrap();

    // Captures the debug representation of the terminal buffer
    insta::assert_debug_snapshot!(terminal.backend().buffer());
}
```

## 4. Input Simulation

To test interactivity, you must simulate input events (key presses, mouse clicks) and verify that the application state updates correctly.

### Event Injection Pattern
Design your event handler to take a generic `Event` type.

```rust
#[test]
fn test_handle_quit_key() {
    let mut app = App::default();
    
    // Simulate pressing 'q'
    let event = Event::Key(KeyCode::Char('q').into());
    app.handle_event(event);

    assert!(app.should_quit);
}
```

## 5. Advanced Testing Techniques

### Testing Styles and Colors
Beyond character content, you should verify styles (Bold, Underline) and colors. You can inspect individual cells in the `TestBackend` buffer.

```rust
let buffer = terminal.backend().buffer();
let cell = buffer.get(0, 0);
assert_eq!(cell.fg, Color::Red);
assert_eq!(cell.modifier, Modifier::BOLD);
```

### Resizing Tests
TUIs must handle terminal resizing gracefully. Always test your layout at multiple dimensions:

- **Minimum size**: (80x24) or smaller.
- **Widescreen**: (200x60).
- **Edge cases**: (0x0) or very narrow columns.

```rust
#[test]
fn test_layout_on_small_screen() {
    let backend = TestBackend::new(10, 10);
    // Verify that text wraps or truncates as expected...
}
```

### Testing Async and Side Effects
If your TUI performs network requests or disk I/O:

1. Use **traits** to mock the external dependencies.
2. Use **channels** (e.g., `tokio::sync::mpsc`) to send messages from background tasks to the TUI event loop.
3. In tests, use a synchronous mock implementation of the trait.

## 6. CI/CD Best Practices

1. **Avoid Panic on `stdout`**: Many TUI libraries try to query the terminal size on startup. In CI environments without a TTY, this can cause panics. Always use `TestBackend` or check `is_terminal()` before initialization.
2. **Deterministic Output**: Disable animations or blinking cursors in tests, as they cause flickering snapshots.
3. **Environment Isolation**: Use the `serial_test` crate if your tests modify environment variables (like `TERM` or `COLORTERM`).

## 7. Summary Checklist

- [ ] Is the UI logic decoupled from the rendering?
- [ ] Are complex views covered by snapshot tests?
- [ ] Are event handlers tested by injecting mock events?
- [ ] Does the UI handle resizing without panicking?
- [ ] Are all tests runnable in a headless (no-TTY) environment?
