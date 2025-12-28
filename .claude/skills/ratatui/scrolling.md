# Scrolling

Implementing scrolling in Ratatui requires managing state to track the current position and handling events to update that state.

## List Scrolling

The most common pattern uses `List` with `ListState`:

```rust
use ratatui::widgets::{List, ListItem, ListState};

struct App {
    messages: Vec<String>,
    scroll_state: ListState,
}

impl App {
    fn new() -> Self {
        Self {
            messages: (0..100).map(|i| format!("Message #{}", i)).collect(),
            scroll_state: ListState::default(),
        }
    }

    fn scroll_down(&mut self) {
        let i = match self.scroll_state.selected() {
            Some(i) => {
                if i >= self.messages.len() - 1 { 0 } else { i + 1 }
            }
            None => 0,
        };
        self.scroll_state.select(Some(i));
    }

    fn scroll_up(&mut self) {
        let i = match self.scroll_state.selected() {
            Some(i) => {
                if i == 0 { self.messages.len() - 1 } else { i - 1 }
            }
            None => 0,
        };
        self.scroll_state.select(Some(i));
    }

    fn scroll_to_bottom(&mut self) {
        if !self.messages.is_empty() {
            self.scroll_state.select(Some(self.messages.len() - 1));
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.messages
            .iter()
            .map(|m| ListItem::new(m.as_str()))
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Chat History"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, area, &mut self.scroll_state);
    }
}
```

## Auto-Scroll on New Messages

```rust
impl App {
    fn add_message(&mut self, msg: String) {
        self.messages.push(msg);
        // Auto-scroll to bottom
        self.scroll_state.select(Some(self.messages.len() - 1));
    }
}
```

## Scrollbar Widget

Add a visual scrollbar indicator:

```rust
use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

fn render_with_scrollbar(f: &mut Frame, area: Rect, app: &mut App) {
    // Render the list
    let items: Vec<ListItem> = app.messages
        .iter()
        .map(|m| ListItem::new(m.as_str()))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Chat"));

    f.render_stateful_widget(list, area, &mut app.list_state);

    // Render scrollbar on top
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("▲"))
        .end_symbol(Some("▼"))
        .track_symbol(Some("│"))
        .thumb_symbol("┃");

    let mut scrollbar_state = ScrollbarState::new(app.messages.len())
        .position(app.list_state.selected().unwrap_or(0));

    // Inner margin to avoid overlapping with borders
    f.render_stateful_widget(
        scrollbar,
        area.inner(&Margin { vertical: 1, horizontal: 0 }),
        &mut scrollbar_state,
    );
}
```

## Paragraph Scrolling

For scrolling large text blocks:

```rust
use ratatui::widgets::Paragraph;

struct App {
    text: String,
    scroll_offset: u16,
}

impl App {
    fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    fn render(&self, f: &mut Frame, area: Rect) {
        let paragraph = Paragraph::new(self.text.as_str())
            .block(Block::default().borders(Borders::ALL))
            .scroll((self.scroll_offset, 0))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }
}
```

## Page Scrolling

Jump multiple items at once:

```rust
impl App {
    fn page_down(&mut self, page_size: usize) {
        let i = match self.scroll_state.selected() {
            Some(i) => {
                let new_pos = i + page_size;
                if new_pos >= self.messages.len() {
                    self.messages.len() - 1
                } else {
                    new_pos
                }
            }
            None => 0,
        };
        self.scroll_state.select(Some(i));
    }

    fn page_up(&mut self, page_size: usize) {
        let i = match self.scroll_state.selected() {
            Some(i) => i.saturating_sub(page_size),
            None => 0,
        };
        self.scroll_state.select(Some(i));
    }
}
```

## Mouse Scrolling

Enable mouse support for scroll wheel:

```rust
use crossterm::event::{EnableMouseCapture, MouseEvent, MouseEventKind};

// In setup
execute!(stdout, EnableMouseCapture)?;

// In event loop
if let Event::Mouse(mouse) = event::read()? {
    match mouse.kind {
        MouseEventKind::ScrollDown => app.scroll_down(),
        MouseEventKind::ScrollUp => app.scroll_up(),
        _ => {}
    }
}

// In cleanup
execute!(stdout, DisableMouseCapture)?;
```

## Key Bindings

Common key bindings for navigation:

```rust
match key.code {
    KeyCode::Char('j') | KeyCode::Down => app.scroll_down(),
    KeyCode::Char('k') | KeyCode::Up => app.scroll_up(),
    KeyCode::Char('d') | KeyCode::PageDown => app.page_down(10),
    KeyCode::Char('u') | KeyCode::PageUp => app.page_up(10),
    KeyCode::Char('g') | KeyCode::Home => app.scroll_to_top(),
    KeyCode::Char('G') | KeyCode::End => app.scroll_to_bottom(),
    _ => {}
}
```

## Common Gotchas

### Selection vs. Scrolling

**Problem**: `ListState` is for selection, not pure scrolling. The highlight bar is always visible.

**Solution**: To hide the selection bar, use an empty highlight style:
```rust
let list = List::new(items)
    .highlight_style(Style::default());  // No visual highlight
```

Or use `Paragraph` with `.scroll()` for pixel-perfect scrolling.

### Height Calculation

**Problem**: Using fixed `Constraint::Length(3)` cuts off long messages.

**Solution**: Calculate dynamic heights based on content:
```rust
fn calculate_wrapped_height(text: &str, width: u16) -> u16 {
    if width == 0 { return 3; }
    let content_width = width.saturating_sub(2); // Borders
    let lines = (text.len() as f32 / content_width as f32).ceil() as u16;
    lines + 2  // +2 for borders
}
```

### Position Desync

**Problem**: After window resize, scroll position becomes invalid.

**Solution**: Clamp position on resize:
```rust
fn clamp_scroll_position(&mut self) {
    if let Some(selected) = self.scroll_state.selected() {
        if selected >= self.messages.len() {
            self.scroll_state.select(Some(self.messages.len().saturating_sub(1)));
        }
    }
}

// Call in Event::Resize handler
```

## Best Practices

1. **Always validate scroll position** - Clamp to valid range after updates
2. **Provide multiple navigation methods** - Arrow keys, page up/down, home/end, mouse
3. **Auto-scroll judiciously** - Only scroll to bottom for new messages if already near bottom
4. **Show scrollbar for long lists** - Helps users understand their position
5. **Handle empty lists** - Check for empty before setting selection
