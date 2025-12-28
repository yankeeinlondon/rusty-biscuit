---
name: ratatui
description: Expert knowledge for building terminal user interfaces (TUIs) with Ratatui, a Rust library for creating rich, cross-platform terminal applications with immediate-mode rendering, layout constraints, widgets, and async integration
last_updated: 2025-12-24T12:00:00Z
hash: 984cf841655dd773
---

# Ratatui TUI Development

Ratatui is the industry-standard Rust crate for building Terminal User Interfaces (TUIs). It's a community-driven fork of `tui-rs` with an immediate-mode rendering philosophy and comprehensive widget ecosystem.

## Core Principles

- **Immediate-mode rendering**: Redraw the entire UI each frame based on current state
- **Backend abstraction**: Choose from crossterm (default, cross-platform), termion (Unix), or termwiz
- **Buffer-based diffing**: Maintains two buffers and only sends changed characters to terminal
- **Constraint-based layouts**: Define responsive layouts using percentage, length, min/max, ratio, or fill
- **Separation of concerns**: Widgets (view) are separate from state (data)
- **Terminal state cleanup**: Always restore terminal state with panic hooks

## Quick Start Pattern

```rust
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, Paragraph},
    layout::{Layout, Constraint, Direction},
    Terminal,
};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
                .split(f.area());

            let title = Paragraph::new("Hello Ratatui!")
                .block(Block::default().borders(Borders::ALL).title("Header"));
            f.render_widget(title, chunks[0]);
        })?;

        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') { break; }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
```

## Essential Patterns

### Terminal State Management with Panic Hook

```rust
use std::panic;

fn setup_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));
}
```

### Stateful Widgets

```rust
use ratatui::widgets::{List, ListItem, ListState};

struct App {
    list_state: ListState,
    items: Vec<String>,
}

impl App {
    fn scroll_down(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => if i >= self.items.len() - 1 { 0 } else { i + 1 },
            None => 0,
        };
        self.list_state.select(Some(i));
    }
}

// In draw loop:
f.render_stateful_widget(list, area, &mut app.list_state);
```

### Async Integration

```rust
use tokio::sync::mpsc;

enum Message {
    UserPrompt(String),
    BotResponse(String),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx_to_main, mut rx_from_background) = mpsc::unbounded_channel::<Message>();
    let (tx_to_background, mut rx_from_main) = mpsc::unbounded_channel::<String>();

    // Background worker
    tokio::spawn(async move {
        while let Some(prompt) = rx_from_main.recv().await {
            // Simulate API call
            tokio::time::sleep(Duration::from_secs(1)).await;
            let _ = tx_to_main.send(Message::BotResponse(format!("Response to: {}", prompt)));
        }
    });

    // UI loop
    loop {
        terminal.draw(|f| ui(f, &app))?;

        // Non-blocking check for background messages
        while let Ok(msg) = rx_from_background.try_recv() {
            match msg {
                Message::BotResponse(text) => app.messages.push(text),
                _ => {}
            }
        }

        // Handle input...
    }
}
```

## Topics

### Core Architecture

- [Layout System](./layout-system.md) - Constraints, nested layouts, and responsive design
- [Widgets](./widgets.md) - Built-in widgets (Paragraph, List, Table, Block, Gauge, etc.)
- [Styling](./styling.md) - Colors, modifiers, themes, and terminal compatibility
- [Backend System](./backend-system.md) - Crossterm, Termion, Termwiz comparison

### Advanced Patterns

- [Async Integration](./async-integration.md) - Tokio channels, non-blocking patterns, background workers
- [Scrolling](./scrolling.md) - Vertical scrolling, scrollbars, auto-scroll, ListState management
- [Event Handling](./event-handling.md) - Keyboard, mouse, resize events, input routing

### Specialized Features

- [Markdown Rendering](./markdown.md) - Using pulldown-cmark and tui-markdown for rich text
- [Chat Applications](./chat-applications.md) - Bubbles, alignment, streaming responses
- [Code Editor](./code-editor.md) - Tree-sitter syntax highlighting, file I/O, search
- [Images](./images.md) - Terminal graphics protocols (Sixel, Kitty, iTerm2, halfblocks)
- [Prompts and Forms](./prompts.md) - Text input, multi-field forms, validation, autocomplete

### Ecosystem

- [Macros](./macros.md) - ratatui-macros for simplified layout and styling syntax
- [Widget Collections](./widget-collections.md) - tui-widgets, tui-big-text, tui-popup
- [Web Deployment](./web-deployment.md) - Ratzilla for WASM/WebAssembly TUIs

## Common Gotchas

### High CPU Usage

**Problem**: Loop runs at 100% CPU redrawing static screens

**Solution**: Use `event::poll` with timeout or rate limiting:
```rust
if event::poll(Duration::from_millis(16))? { // ~60 FPS max
    // Handle events
}
```

### Terminal State Corruption on Panic

**Problem**: Panic leaves terminal in raw mode with hidden cursor

**Solution**: Always set panic hook (see Essential Patterns above)

### Widget Ownership in Immediate Mode

**Problem**: Trying to store widgets in App struct

**Solution**: Only store data and state. Widgets are created on-the-fly in `draw`:
```rust
struct App {
    data: Vec<String>,        // ✓ Store data
    list_state: ListState,    // ✓ Store state
    // list_widget: List,     // ✗ Never store widgets
}
```

### Layout Overflow

**Problem**: Widgets overflow allocated areas causing visual glitches

**Solution**: Use `Wrap` for text and proper constraint sizing:
```rust
let paragraph = Paragraph::new("Long text")
    .wrap(Wrap { trim: true });
```

### Color Inconsistencies Across Terminals

**Problem**: RGB colors look different or glitch on some terminals

**Solution**: Provide fallbacks and use indexed colors for compatibility:
```rust
let color = if cfg!(target_os = "macos") {
    Color::Indexed(208)  // Use indexed color
} else {
    Color::Rgb(255, 128, 0)  // Use RGB elsewhere
};
```

## Architecture Patterns

### Elm/MVU Pattern

```rust
struct Model {
    counter: i32,
    items: Vec<String>,
}

enum Msg {
    Increment,
    Decrement,
    AddItem(String),
}

fn update(msg: Msg, model: &mut Model) {
    match msg {
        Msg::Increment => model.counter += 1,
        Msg::Decrement => model.counter -= 1,
        Msg::AddItem(item) => model.items.push(item),
    }
}

fn view(model: &Model) -> impl Widget {
    // Build UI based on model state
}
```

### Component Pattern

```rust
enum CurrentScreen {
    Main,
    Fetching,
    Error(String),
}

struct App {
    current_screen: CurrentScreen,
    data: Option<String>,
}

fn ui(f: &mut Frame, app: &App) {
    match &app.current_screen {
        CurrentScreen::Main => render_main(f, app),
        CurrentScreen::Fetching => render_loading(f),
        CurrentScreen::Error(msg) => render_error(f, msg),
    }
}
```

## Resources

- [Official Docs](https://docs.rs/ratatui)
- [GitHub](https://github.com/ratatui-org/ratatui)
- [Awesome Ratatui](https://github.com/ratatui-org/awesome-ratatui) - Curated ecosystem
- [Ratatui Book](https://ratatui.rs) - Comprehensive guide
