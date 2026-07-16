# Table

Columnar data display component with support for typed cell content, automatic column width calculation, alignment, word wrapping, and ANSI-aware rendering. Handles text, integers (with thousands separators), floats, currency values, and inline [Prose](./prose.md) content.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;

// Basic table with typed cell content
let table = Table::new()
    .with_columns(vec![
        TableColumn::new("Name").with_type(ColumnType::Text),
        TableColumn::new("Count").with_type(ColumnType::Integer),
        TableColumn::new("Price").with_type(ColumnType::Currency(Currency::USD)),
    ])
    .with_data(vec![
        vec!["Widget".into(), 1234567i64.into(), TableCellContent::Currency(Currency::USD, 19.99)],
        vec!["Gadget".into(), 42i64.into(), TableCellContent::Currency(Currency::USD, 149.50)],
    ]);

// A styled cell with capability-aware inline content
let styled = Table::new()
    .with_columns(vec![TableColumn::new("Feature"), TableColumn::new("Status")])
    .with_data(vec![vec![
        Prose::new("**Bold** feature").into(),
        Prose::new("[docs](https://example.com) — _ready_").into(),
    ]]);

let term = Terminal::default();
println!("{}", table.display(&term));
```

### Cell Content Types

| Type | Example Input | Rendered Output |
|------|--------------|-----------------|
| `Text(String)` | `"Hello World"` | `Hello World` (terminal rendering uses `Terminal::tab_width`) |
| `Integer(i64)` | `1234567` | `1,234,567` |
| `Float(f64)` | `12345.678` | `12,345.68` |
| `Currency(Currency, f64)` | `(USD, 1234.56)` | `$1,234.56` |
| `StyledProse(Box<Prose>)` | `Prose::new("<b>Bold</b>")` | `Bold` (styled) |

Convenience `From` implementations allow using `&str`, `String`, `i64`, `f64`, and `Prose` directly. A `StyledProse` cell embeds capability-aware inline Prose; the cell hint records `kind == "styled_prose"` with a null `raw_value`. The tree path projects Prose's semantic inline nodes (Terminal/Browser/Markdown); the terminal bespoke path resolves every styled cell to `Text(prose.render(term))` exactly once before width planning. Prose's own `Layout` is intentionally not applied — the table owns cell geometry. See [Prose in table cells](./prose.md#prose-in-table-cells) for details.

### Key API

| Method | Description |
|--------|-------------|
| `Table::new()` | Create an empty table |
| `.with_columns(Vec<TableColumn>)` | Define column definitions |
| `.with_data(Vec<Vec<TableCellContent>>)` | Set row data |
| `.with_headers(bool)` | Toggle header row display |

### Column Configuration

Columns support type hints (`ColumnType::Text`, `Integer`, `Float`, `Currency`), alignment (`VerticalAlign`), and automatic width measurement that accounts for ANSI escape codes.

## CLI

`bt table` renders a table through the render tree (`render_terminal_node`),
so the typed `TableStyle` striping and header/body slot styling are applied by
the terminal tree renderer.

```bash
bt table --columns "Name,Score" --row "Ann,90" --row "Bob,75"
bt table --columns "Name,Score" --row "Ann,90" --row "Bob,75" --striped
bt table --columns "Name,Score" --row "Ann,90" --row "Bob,75" --striped --stripe-bg blue
bt table --columns "Name,Score" --row "Ann,90" --bold-header --body-color cyan
```

Options:

- `--columns`: Comma-separated column headers (required)
- `--row`: Comma-separated cell values (repeatable — one per data row)
- `--striped`: Apply an alternating background stripe to even data rows
- `--stripe-bg`: Explicit stripe background color (named or `#rrggbb`)
- `--stripe-text`: Explicit stripe text color (named or `#rrggbb`)
- `--bold-header`: Render every column header in bold
- `--header-color`: Header text color (named or `#rrggbb`)
- `--body-color`: Body (data cell) text color (named or `#rrggbb`)

Tables are also used programmatically and rendered by other tools (e.g.,
`darkmatter` for GFM table rendering).
