# TwoColumn

A side-by-side two-column layout for terminal rendering. Arranges two pieces of content in parallel columns with configurable widths (fixed or percentage-based) and an adjustable gap. Automatically stacks vertically when the terminal is too narrow to display both columns.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;

// Basic 50/50 split (default)
let cols = TwoColumn::new("Left content", "Right content");

// 70/30 split
let cols = TwoColumn::new("Details:", "Some value here")
    .with_left_percent(0.7);

// Fixed width left column (30 chars), rest goes to right
let cols = TwoColumn::new("Label", "Value")
    .with_left_width(ColumnWidth::Fixed(30))
    .with_gap(2);  // 2-char gap instead of default 3

// Rich content in columns
let left = Prose::new("<bold>Name:</bold> Widget");
let right = Prose::new("<green>Status:</green> Active");
let cols = TwoColumn::new(left, right);

// Render
let term = Terminal::default();
println!("{}", cols.display(&term));
```

### Key API

| Method | Description |
|--------|-------------|
| `TwoColumn::new(left, right)` | Create with left and right content |
| `.with_left_width(ColumnWidth)` | Set left column width |
| `.with_left_percent(f32)` | Set left column as percentage (0.0..=1.0) |
| `.with_gap(u32)` | Set gap between columns (default: 3) |

### ColumnWidth

| Variant | Description |
|---------|-------------|
| `ColumnWidth::Fixed(30)` | Fixed width of 30 characters |
| `ColumnWidth::Percent(0.5)` | 50% of available space |

### Responsive Behavior

When the terminal is too narrow for both columns plus the gap, content automatically stacks vertically to remain readable.

## CLI

Exposed via `bt columns`:

```bash
bt columns "Left side" "Right side"
bt columns --left-width 40 "Label" "Value"
```
