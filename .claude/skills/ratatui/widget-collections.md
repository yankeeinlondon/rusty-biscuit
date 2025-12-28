# Widget Collections

Community-maintained widget collections extend Ratatui with specialized components.

## tui-widgets

Meta-package bundling several high-quality widgets:

```toml
[dependencies]
tui-widgets = { version = "0.2", features = ["popup", "big-text"] }
```

### tui-popup

Simplifies centered popup/modal rendering:

```rust
use tui_popup::Popup;

fn render_popup(f: &mut Frame) {
    let text = Text::from("Confirm deletion? [y/n]");

    let popup = Popup::new(text)
        .title(" Warning ")
        .style(Style::default().bg(Color::Red))
        .border_set(symbols::border::ROUNDED);

    f.render_widget(popup, f.area());  // Auto-centers
}
```

### tui-big-text

Renders large ASCII art text:

```rust
use tui_big_text::{BigText, PixelSize};

let big_text = BigText::builder()
    .pixel_size(PixelSize::Full)
    .style(Style::default().fg(Color::Cyan))
    .lines(vec![
        "12:45".into(),
        "SYSTEM OK".into(),
    ])
    .build();

f.render_widget(big_text, area);
```

### tui-scrollview

Provides scrollable container for content larger than screen:

```rust
use tui_scrollview::{ScrollView, ScrollViewState};

struct App {
    scroll_state: ScrollViewState,
}

fn render(f: &mut Frame, app: &mut App) {
    let scroll_view = ScrollView::new(Size::new(100, 200));

    f.render_stateful_widget(
        scroll_view,
        area,
        &mut app.scroll_state
    );
}
```

## Other Widget Crates

### ratatui-widgets

Official experimental widgets:

```toml
[dependencies]
ratatui-widgets = "0.1"
```

- **Advanced List** - Enhanced list with filtering
- **Tree** - Hierarchical tree view
- **Calendar** - Date picker widget

### edtui

Vim-inspired editor widget:

```toml
[dependencies]
edtui = "0.6"
```

```rust
use edtui::{EditorView, EditorState};

let mut state = EditorState::new();
let view = EditorView::new(&mut state);

f.render_widget(view, area);
```

### tui-logger

Logging widget with filters:

```toml
[dependencies]
tui-logger = "0.11"
```

```rust
use tui_logger::{TuiLoggerWidget, TuiLoggerLevelOutput};

let logger = TuiLoggerWidget::default()
    .style_error(Style::default().fg(Color::Red))
    .style_warn(Style::default().fg(Color::Yellow))
    .output_separator(':')
    .output_level(Some(TuiLoggerLevelOutput::Abbreviated));

f.render_widget(logger, area);
```

## Common Gotchas

### Z-Index and Overlays

**Problem**: Widgets render in order, later widgets cover earlier ones

**Solution**: Always render popups/modals last:
```rust
// ✓ CORRECT order
f.render_widget(background, area);
f.render_widget(main_content, area);
if app.show_popup {
    f.render_widget(popup, area);  // Last = on top
}
```

### Scrollview and Layout Constraints

**Problem**: Passing oversized Rect to scrollview causes panics

**Solution**: Use scrollview's provided context:
```rust
// Use ScrollView::render_widget, not f.render_widget
scroll_view.render_widget(widget, buf);
```

### Dependency Bloat

**Problem**: `tui-widgets` pulls in many sub-dependencies

**Solution**: Use feature flags to only include what you need:
```toml
[dependencies]
tui-widgets = {
    version = "0.2",
    default-features = false,
    features = ["popup", "big-text"]
}
```

### State Persistence

**Problem**: Widget state recreated every frame loses user position

**Solution**: Store widget state in App struct:
```rust
struct App {
    scroll_state: ScrollViewState,  // ✓ Persist
    popup_visible: bool,
}

// ✗ Don't create new state in draw
fn draw() {
    let mut state = ScrollViewState::default();  // WRONG
}
```

## Complete Example

```rust
use tui_big_text::{BigText, PixelSize};
use tui_popup::Popup;

struct App {
    show_popup: bool,
    counter: i32,
}

fn ui(f: &mut Frame, app: &App) {
    // Background layer
    let big_text = BigText::builder()
        .pixel_size(PixelSize::Full)
        .style(Style::default().fg(Color::Yellow))
        .lines(vec![format!("COUNT: {}", app.counter).into()])
        .build();

    f.render_widget(big_text, f.area());

    // Foreground layer - popup
    if app.show_popup {
        let block = Block::bordered()
            .title(" Notification ")
            .border_style(Style::default().fg(Color::Cyan));

        let popup = Popup::new("Press any key to continue")
            .block(block);

        f.render_widget(popup, f.area());
    }
}
```

## Best Practices

1. **Use feature flags** - Only include widgets you need
2. **Layer widgets carefully** - Render background to foreground
3. **Store state at App level** - Widget states must persist
4. **Test widget combinations** - Some widgets may conflict
5. **Check documentation** - Widget APIs may vary from core Ratatui
