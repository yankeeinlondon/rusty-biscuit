# Layout System

Ratatui's layout system uses constraints to define how space should be allocated among widgets, creating responsive interfaces that adapt to terminal size changes.

## Constraint Types

```rust
use ratatui::layout::Constraint;

let constraints = [
    Constraint::Length(10),      // Fixed length of 10 characters
    Constraint::Percentage(30),   // 30% of available space
    Constraint::Min(5),           // At least 5 characters
    Constraint::Max(20),          // At most 20 characters
    Constraint::Ratio(1, 2),      // 1/2 of available space
    Constraint::Fill(1),          // Fill remaining space (with weight)
];
```

## Basic Layout

```rust
use ratatui::layout::{Direction, Layout};

fn create_layout(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),   // Header
            Constraint::Min(0),      // Main content
            Constraint::Length(1),   // Footer
        ])
        .split(area)
}
```

## Nested Layouts

```rust
// Create complex grid-like structures
let main_chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Min(0), Constraint::Length(3)])
    .split(area);

let sub_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
    .split(main_chunks[0]);

// Result:
// sub_chunks[0]: Left panel (30% width)
// sub_chunks[1]: Right panel (70% width)
// main_chunks[1]: Bottom status bar
```

## Using ratatui-macros

The `ratatui-macros` crate simplifies layout definitions:

```rust
use ratatui_macros::{vertical, horizontal, constraints};

// Standard way
let layout = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Percentage(10),
    ])
    .split(frame.area());

// Macro way
let [top, main, bottom] = vertical![==3, >=0, ==10%].areas(frame.area());
```

**Constraint syntax**:
- `==3`: `Constraint::Length(3)`
- `>=10`: `Constraint::Min(10)`
- `<=20`: `Constraint::Max(20)`
- `==10%`: `Constraint::Percentage(10)`
- `*=1`: `Constraint::Fill(1)`

## Layout Caching

Ratatui uses an LRU cache to optimize layout calculations:

```rust
// Initialize cache with appropriate size
Layout::init_cache(16);

// Layout calculation will use cache when possible
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints(constraints)
    .split(area);
```

## Rect Operations

```rust
use ratatui::layout::{Rect, Margin};

let rect = Rect::new(0, 0, 80, 24); // x, y, width, height

// Check validity
assert!(!rect.is_empty());
assert_eq!(rect.area(), 80 * 24);

// Get inner area with margin
let inner = rect.inner(&Margin::new(1, 1));

// Get overlapping area
let intersection = rect.intersection(&other_rect);
```

## Responsive Design

### Dynamic Height Calculation

```rust
fn get_message_height(text: &str, width: u16) -> u16 {
    if width == 0 { return 3; }
    let content_width = width.saturating_sub(2); // Subtract borders
    let lines = (text.len() as f32 / content_width as f32).ceil() as u16;
    lines + 2 // +2 for top/bottom borders
}

// In render function:
let message_constraints: Vec<Constraint> = app.messages.iter()
    .map(|m| {
        let h = get_message_height(&m.content, f.area().width);
        Constraint::Length(h)
    })
    .collect();
```

### Handling Small Screens

```rust
// Guard against extremely small terminals
if f.area().width < 10 || f.area().height < 5 {
    f.render_widget(
        Paragraph::new("Terminal too small!"),
        f.area()
    );
    return;
}
```

## Common Patterns

### Three-Panel Layout

```rust
fn three_panel_layout(area: Rect) -> [Rect; 3] {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Top
            Constraint::Min(0),     // Middle
            Constraint::Length(3),  // Bottom
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),  // Left sidebar
            Constraint::Percentage(80),  // Main content
        ])
        .split(vertical[1]);

    [vertical[0], horizontal[0], horizontal[1]]
}
```

### Dashboard Grid

```rust
fn dashboard_grid(area: Rect) -> Vec<Rect> {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50); 2])
        .split(area);

    let mut cells = Vec::new();
    for row in rows {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(33); 3])
            .split(row);
        cells.extend(cols);
    }
    cells
}
```

## Best Practices

1. **Use Constraint::Min(0) for flexible areas** - Prevents overflow on small screens
2. **Cache layouts when possible** - Improves performance for complex UIs
3. **Test across terminal sizes** - Ensure layout works from 80x24 to large screens
4. **Prefer percentages for responsive design** - Adapts better than fixed lengths
5. **Use margins sparingly** - They reduce usable space quickly on small terminals
