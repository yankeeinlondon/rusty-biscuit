# Table

Columnar data display component with support for typed cell content, automatic column width calculation, alignment, word wrapping, and ANSI-aware rendering. Handles text, integers (with thousands separators), floats, and currency values.

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

let term = Terminal::default();
println!("{}", table.display(&term));
```

### Cell Content Types

| Type | Example Input | Rendered Output |
|------|--------------|-----------------|
| `Text(String)` | `"Hello World"` | `Hello World` |
| `Integer(i64)` | `1234567` | `1,234,567` |
| `Float(f64)` | `12345.678` | `12,345.68` |
| `Currency(Currency, f64)` | `(USD, 1234.56)` | `$1,234.56` |

Convenience `From` implementations allow using `&str`, `String`, `i64`, and `f64` directly.

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

Not directly exposed as a standalone CLI command. Tables are used programmatically and rendered by other tools (e.g., `darkmatter` for GFM table rendering).
