# Widgets

Ratatui provides a rich set of built-in widgets for common TUI requirements. All widgets are highly customizable and can be combined to create complex interfaces.

## Built-in Widgets

| Widget | Description | Key Features |
|--------|-------------|--------------|
| **Paragraph** | Renders text with wrapping and alignment | Word wrapping, text alignment, style inheritance |
| **List** | Displays selectable items with highlighting | Custom item rendering, scrollable, highlight state |
| **Table** | Displays tabular data with headers and rows | Column widths, row highlighting, borders |
| **Block** | Decorative container with borders and titles | Border styles, custom symbols, titles |
| **Gauge** | Progress indicator for completion percentage | Different ratio styles, labels, colors |
| **Sparkline** | Displays data points as minimal line chart | Data aggregation, custom colors, styling |
| **Canvas** | Low-level drawing area for custom graphics | Shape primitives, coordinate system, markers |
| **Tabs** | Tab selection widget | Highlight selection, custom styling |
| **Clear** | Clears area for overlays/modals | Used for layering widgets |

## Paragraph

```rust
use ratatui::widgets::{Paragraph, Wrap, Block, Borders};
use ratatui::layout::Alignment;

let paragraph = Paragraph::new("Long text that needs wrapping")
    .block(Block::default().borders(Borders::ALL).title("Title"))
    .style(Style::default().fg(Color::White))
    .alignment(Alignment::Left)
    .wrap(Wrap { trim: true });

f.render_widget(paragraph, area);
```

## List

```rust
use ratatui::widgets::{List, ListItem, ListState};

// Create items
let items: Vec<ListItem> = vec![
    ListItem::new("Item 1"),
    ListItem::new("Item 2"),
    ListItem::new("Item 3"),
];

// Create widget
let list = List::new(items)
    .block(Block::default().title("List").borders(Borders::ALL))
    .style(Style::default().fg(Color::White))
    .highlight_style(Style::default().add_modifier(Modifier::BOLD))
    .highlight_symbol(">> ");

// Render with state
let mut list_state = ListState::default();
list_state.select(Some(0));
f.render_stateful_widget(list, area, &mut list_state);
```

## Table

```rust
use ratatui::widgets::{Table, Row, Cell};

let rows = vec![
    Row::new(vec![
        Cell::from("Col1"),
        Cell::from("Col2"),
        Cell::from("Col3"),
    ]),
    Row::new(vec![
        Cell::from("Val1"),
        Cell::from("Val2"),
        Cell::from("Val3"),
    ]),
];

let table = Table::new(rows)
    .block(Block::default().title("Table").borders(Borders::ALL))
    .widths(&[
        Constraint::Percentage(33),
        Constraint::Percentage(33),
        Constraint::Percentage(34),
    ])
    .column_spacing(1)
    .header(
        Row::new(vec!["Header1", "Header2", "Header3"])
            .style(Style::default().fg(Color::Yellow))
            .bottom_margin(1)
    );

f.render_widget(table, area);
```

## Block

```rust
use ratatui::widgets::{Block, BorderType};

let block = Block::default()
    .title(" Title ")
    .borders(Borders::ALL)
    .border_type(BorderType::Rounded)
    .border_style(Style::default().fg(Color::Cyan))
    .style(Style::default().bg(Color::Black));

f.render_widget(block, area);
```

**Border types**:
- `BorderType::Plain` - Simple lines
- `BorderType::Rounded` - Rounded corners
- `BorderType::Double` - Double lines
- `BorderType::Thick` - Thick lines

## Gauge

```rust
use ratatui::widgets::Gauge;

let gauge = Gauge::default()
    .block(Block::default().title("Progress").borders(Borders::ALL))
    .gauge_style(Style::default().fg(Color::Green))
    .percent(75)
    .label(format!("75%"));

f.render_widget(gauge, area);
```

## Tabs

```rust
use ratatui::widgets::Tabs;

let titles = vec!["Tab1", "Tab2", "Tab3"];
let tabs = Tabs::new(titles)
    .block(Block::default().borders(Borders::ALL).title("Tabs"))
    .select(1)  // Select second tab
    .style(Style::default().fg(Color::White))
    .highlight_style(Style::default().fg(Color::Yellow));

f.render_widget(tabs, area);
```

## Custom Widgets

Create custom widgets by implementing the `Widget` trait:

```rust
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

struct CustomWidget {
    content: String,
    style: Style,
}

impl Widget for CustomWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let x = area.x;
        let y = area.y;

        // Render styled text
        for (i, c) in self.content.chars().enumerate() {
            if (x + i as u16) < area.right() {
                let cell = buf.get_mut(x + i as u16, y);
                cell.set_char(c);
                cell.set_style(self.style);
            }
        }

        // Draw border
        let border_style = Style::default().fg(Color::Gray);
        for i in x..area.right() {
            buf.get_mut(i, area.top()).set_char('─').set_style(border_style);
            buf.get_mut(i, area.bottom() - 1).set_char('─').set_style(border_style);
        }
        for i in area.top()..area.bottom() {
            buf.get_mut(area.left(), i).set_char('│').set_style(border_style);
            buf.get_mut(area.right() - 1, i).set_char('│').set_style(border_style);
        }

        // Corners
        buf.get_mut(area.left(), area.top()).set_char('┌').set_style(border_style);
        buf.get_mut(area.right() - 1, area.top()).set_char('┐').set_style(border_style);
        buf.get_mut(area.left(), area.bottom() - 1).set_char('└').set_style(border_style);
        buf.get_mut(area.right() - 1, area.bottom() - 1).set_char('┘').set_style(border_style);
    }
}
```

## Styled Text

Use `Text`, `Line`, and `Span` for rich formatting:

```rust
use ratatui::text::{Text, Line, Span};

// Simple spans
let line = Line::from(vec![
    Span::styled("Hello ", Style::default().fg(Color::Cyan)),
    Span::styled("World", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
]);

// Multi-line text
let text = Text::from(vec![
    Line::from("First line"),
    Line::from(vec![
        Span::raw("Second line with "),
        Span::styled("color", Style::default().fg(Color::Red)),
    ]),
]);

let paragraph = Paragraph::new(text);
```

## Best Practices

1. **Separate widget creation from rendering** - Create widgets in the draw closure, not beforehand
2. **Use state for interactive widgets** - Store `ListState`, `TableState` in your App struct
3. **Leverage Block for consistency** - Wrap widgets in Block for uniform styling
4. **Test widget sizing** - Ensure widgets handle small areas gracefully
5. **Cache expensive computations** - Don't recreate item lists every frame if data hasn't changed
