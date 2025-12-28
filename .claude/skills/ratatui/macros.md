# Ratatui Macros

The `ratatui-macros` crate provides DSL-like syntax to simplify layout and styling definitions.

## Layout Macros

Replace verbose `Layout` builder patterns:

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

// Macro way - returns array of Rects directly
let [top, main, bottom] = vertical![==3, >=0, ==10%].areas(frame.area());
```

## Constraint Syntax

Intuitive operators for constraints:

```rust
==3      // Constraint::Length(3)
>=10     // Constraint::Min(10)
<=20     // Constraint::Max(20)
==10%    // Constraint::Percentage(10)
==1/2    // Constraint::Ratio(1, 2)
*=1      // Constraint::Fill(1)
```

### Examples

```rust
// Vertical split
let [header, content, footer] = vertical![==3, >=0, ==1].areas(area);

// Horizontal split
let [sidebar, main] = horizontal![==20%, ==80%].areas(area);

// Repeated constraints
let columns = horizontal![==25%; 4].areas(area);  // Four equal 25% columns
```

## Styled Text Macros

Simplify `Span` and `Line` creation:

```rust
use ratatui_macros::{span, line, text};

let user = "Alice";

// Simple span with color
let greeting = line![
    span!(Color::Blue; "Hello "),
    span!(Modifier::BOLD | Modifier::ITALIC; "{user}"),
    " Welcome back!".yellow(),
];

// Multi-line text
let text = text![
    line!["First line"],
    line![
        span!(Color::Cyan; "Colored "),
        span!(Modifier::BOLD; "bold text")
    ],
];
```

## Table Row Macro

Simplify table row creation:

```rust
use ratatui_macros::row;

let table_rows = vec![
    row!["ID", "Name", "Status"],
    row![
        span!(Color::Cyan; "001"),
        "Root",
        span!(Color::Green; "Active")
    ],
];

let table = Table::new(table_rows)
    .widths(&[
        Constraint::Length(5),
        Constraint::Min(10),
        Constraint::Length(10),
    ]);
```

## Common Gotchas

### Type Inference Ambiguity

**Problem**: Compiler can't infer if you want `Span`, `Line`, or `String`

**Solution**: Be explicit or use `.into()`:
```rust
let line: Line = line!["hello"];
// or
let line = line!["hello"].into();
```

### Version Compatibility

**Problem**: Type mismatch between `ratatui` and `ratatui-macros` versions

**Solution**: Keep versions aligned:
```toml
[dependencies]
ratatui = "0.29"
ratatui-macros = "0.6"  # Compatible version
```

### Macro Nesting Depth

**Problem**: Deeply nested macros slow compilation

**Solution**: Extract complex components into functions:
```rust
fn render_header() -> Line {
    line![
        span!(Color::Blue; "Header"),
        span!(Modifier::BOLD; " Title")
    ]
}

// Use in draw
let header = render_header();
```

### Borrowing in Format Strings

**Problem**: `span!("{value}")` creates new String, lifetime issues

**Solution**: Only use macros in draw closure, not in stored state:
```rust
// ✓ CORRECT - in draw closure
terminal.draw(|f| {
    let text = span!("Count: {}", app.count);
    // ...
})?;

// ✗ WRONG - trying to store
struct App {
    cached_span: Span,  // Can't store format! result
}
```

## Comparison Table

| Feature | Standard Ratatui | `ratatui-macros` |
|---------|-----------------|------------------|
| **Layout** | `Layout::default()...split()` | `vertical![...].areas()` |
| **Constraints** | `Constraint::Percentage(50)` | `==50%` |
| **Formatting** | `Span::raw(format!("Hello {}", name))` | `span!("Hello {name}")` |
| **Table Rows** | `Row::new(vec![Cell::from(...)])` | `row![...]` |

## Best Practices

1. **Use for declarative layouts** - Macros shine with static constraints
2. **Keep nesting shallow** - Extract complex components to functions
3. **Check versions** - Ensure ratatui and ratatui-macros are compatible
4. **Leverage format strings** - Use `span!("{value}")` for interpolation
5. **Only in draw closures** - Don't store macro results in App state
