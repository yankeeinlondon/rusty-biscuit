# Event Handling

Ratatui doesn't handle input events directly but is designed to work with event handling libraries like crossterm.

## Basic Event Loop

```rust
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use std::time::Duration;

loop {
    terminal.draw(|f| ui(f, &app))?;

    if event::poll(Duration::from_millis(16))? {  // ~60 FPS
        match event::read()? {
            Event::Key(key) => handle_key(key, &mut app),
            Event::Mouse(mouse) => handle_mouse(mouse, &mut app),
            Event::Resize(width, height) => handle_resize(width, height, &mut app, &mut terminal),
            _ => {}
        }
    }
}
```

## Keyboard Events

```rust
fn handle_key(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        KeyCode::Up => app.scroll_up(),
        KeyCode::Down => app.scroll_down(),
        KeyCode::Enter => app.submit(),
        KeyCode::Esc => app.cancel(),
        KeyCode::Char(c) => app.input.push(c),
        KeyCode::Backspace => { app.input.pop(); }
        _ => {}
    }
}
```

### Key Modifiers

```rust
use crossterm::event::KeyModifiers;

if key.modifiers.contains(KeyModifiers::CONTROL) {
    // Ctrl+Key
}
if key.modifiers.contains(KeyModifiers::SHIFT) {
    // Shift+Key
}
if key.modifiers.contains(KeyModifiers::ALT) {
    // Alt+Key
}
```

## Mouse Events

Enable mouse support:

```rust
use crossterm::event::{EnableMouseCapture, DisableMouseCapture, MouseEvent, MouseEventKind};

// In setup
execute!(stdout, EnableMouseCapture)?;

// In cleanup
execute!(stdout, DisableMouseCapture)?;

// In event loop
fn handle_mouse(mouse: MouseEvent, app: &mut App) {
    match mouse.kind {
        MouseEventKind::ScrollDown => app.scroll_down(),
        MouseEventKind::ScrollUp => app.scroll_up(),
        MouseEventKind::Down(MouseButton::Left) => {
            app.handle_click(mouse.column, mouse.row);
        }
        _ => {}
    }
}
```

## Window Resize

```rust
fn handle_resize(
    width: u16,
    height: u16,
    app: &mut App,
    terminal: &mut Terminal<impl Backend>
) -> io::Result<()> {
    terminal.autoresize()?;

    // Optionally adjust app state
    app.clamp_scroll_position();

    Ok(())
}
```

## Event Routing

Route events to different components based on app state:

```rust
enum AppMode {
    Normal,
    Insert,
    Search,
}

fn handle_event(event: Event, app: &mut App) {
    match app.mode {
        AppMode::Normal => handle_normal_mode(event, app),
        AppMode::Insert => handle_insert_mode(event, app),
        AppMode::Search => handle_search_mode(event, app),
    }
}

fn handle_normal_mode(event: Event, app: &mut App) {
    if let Event::Key(key) = event {
        match key.code {
            KeyCode::Char('i') => app.mode = AppMode::Insert,
            KeyCode::Char('/') => app.mode = AppMode::Search,
            KeyCode::Char('q') => app.should_quit = true,
            _ => {}
        }
    }
}

fn handle_insert_mode(event: Event, app: &mut App) {
    if let Event::Key(key) = event {
        match key.code {
            KeyCode::Esc => app.mode = AppMode::Normal,
            KeyCode::Char(c) => app.input.push(c),
            KeyCode::Backspace => { app.input.pop(); }
            _ => {}
        }
    }
}
```

## Preventing Default Browser Behavior (WASM)

For web deployments with Ratzilla:

```rust
use web_sys::KeyboardEvent;
use wasm_bindgen::JsCast;

fn setup_input_handling() {
    let window = web_sys::window().expect("no global window");

    let closure = Closure::wrap(Box::new(move |event: KeyboardEvent| {
        let key = event.key();

        // Prevent default for TUI keys
        let keys_to_prevent = ["ArrowUp", "ArrowDown", " ", "Tab"];
        if keys_to_prevent.contains(&key.as_str()) {
            event.prevent_default();
        }
    }) as Box<dyn FnMut(KeyboardEvent)>);

    window
        .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
        .expect("failed to add event listener");

    closure.forget();
}
```

## Polling vs Blocking

### Polling (Recommended)

```rust
// Non-blocking, allows for frame rate control
if event::poll(Duration::from_millis(16))? {
    if let Event::Key(key) = event::read()? {
        // Handle key
    }
}
```

### Blocking

```rust
// Blocks until event arrives, saves CPU but no frame rate control
let event = event::read()?;
match event {
    Event::Key(key) => handle_key(key, app),
    _ => {}
}
```

## Best Practices

1. **Use polling with timeout** - Balances responsiveness and CPU usage
2. **Handle all Event variants** - Gracefully ignore unknown events
3. **Validate input** - Check for valid characters, lengths, etc.
4. **Provide escape hatches** - Always have a way to quit (Ctrl+C, Esc, 'q')
5. **Route events by mode** - Different app states handle events differently
