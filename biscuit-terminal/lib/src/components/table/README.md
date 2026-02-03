# Table Component

Terminal-aware table renderer with box-drawing borders, auto-sized columns, and layout integration.

## Module Structure

```
table/
├── mod.rs      # Module re-exports
├── table.rs    # Table struct, TableColumn, TableCellContent, Renderable impl
└── types.rs    # Future type definitions (ColumnType, ColumnAggregate, etc.)
```

## Core Types

### `Table`

The primary struct. Holds an optional title, column definitions, row data, and a `Layout` for margin/alignment/wrapping control.

```rust
let table = Table::new()
    .with_title("Users")
    .with_columns(vec![
        TableColumn::new("Name"),
        TableColumn::new("Age").with_min_width(5),
    ])
    .with_data(vec![
        vec!["Alice".into(), "30".into()],
        vec!["Bob".into(), "25".into()],
    ]);

println!("{}", table.render(Some(120)));
```

**Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `title` | `Option<String>` | Optional heading printed above the header row |
| `columns` | `Vec<TableColumn>` | Column definitions (header text + width constraints) |
| `data` | `Vec<Vec<TableCellContent>>` | Row data as a 2D grid of cell values |
| `layout` | `Layout` | Controls margins, alignment, word-wrap, row-fill |

**Builder Methods:**

| Method | Description |
|--------|-------------|
| `with_title(title)` | Set an optional title line above the table |
| `with_columns(cols)` | Set column definitions |
| `with_data(rows)` | Set all data rows at once |
| `add_row(row)` | Append a single data row (`&mut self`) |

All layout-level builders (margins, alignment, word-wrap, row-fill) are inherited from the `Renderable` trait's default implementations and operate on the owned `Layout`.

### `TableColumn`

Defines a single column's header text and optional width constraints.

```rust
TableColumn::new("Status")
    .with_min_width(8)
    .with_max_width(30)
```

| Field | Type | Description |
|-------|------|-------------|
| `header` | `String` | Column header text |
| `min_width` | `Option<usize>` | Floor for the column width |
| `max_width` | `Option<usize>` | Ceiling for the column width |

### `TableCellContent`

Enum representing cell values. Currently supports `Text(String)` with commented-out variants for `Integer`, `Float`, and `Currency`.

Any type that implements `Into<String>` converts into `TableCellContent::Text` via a blanket `From` impl, so string literals and `String` values work directly:

```rust
vec!["Alice".into(), "30".into()]
```

### `Currency`

Simple enum with `USD`, `GBP`, and `EUR` variants. Defined in `table.rs` but not yet wired into cell rendering.

## Column Width Calculation

`calculate_column_widths()` runs a three-pass algorithm:

1. **Header pass** -- Initialize each column's width to its header text length, flooring at `min_width` if set.
2. **Data pass** -- Walk every cell in every row; widen the column if the cell content exceeds the current width.
3. **Constraint pass** -- Clamp each column to `max_width` if set.

The number of columns is derived from whichever is larger: the column definitions count or the widest data row. This means data rows can exceed the defined column count without panicking (extra cells are ignored in rendering, but they influence width calculation up to `widths.len()`).

## Rendering Pipeline

### Box-Drawing Output

The renderer produces Unicode box-drawing output with `│` column separators and a `├─┼─┤` header/data divider:

```
Users
│ Name  │ Age │
├───────┼─────┤
│ Alice │ 30  │
│ Bob   │ 25  │
```

Cells are left-aligned and padded with spaces to the computed column width.

### Renderable Trait Integration

`Table` implements `Renderable`, which provides two rendering paths:

| Method | When to Use |
|--------|-------------|
| `render(term_width)` | Optimistic path -- assumes full terminal capabilities. Falls back to 80 columns when `term_width` is `None`. |
| `fallback_render(term)` | Conservative path -- receives a `Terminal` reference for capability-aware decisions. Uses `term.width()` for sizing. |

Both paths call `render_content()` to produce raw table text, then pass it through `Layout::apply_layout()` which applies:

- Left/right margin resolution (chars, percent, or nested offset)
- Word wrapping (if configured)
- Text alignment (left, center, right)
- Row-fill padding (for opaque backgrounds)

### Block-Level Behavior

`is_block_level()` returns `true`, signaling to composition systems (like `Compose`) that the table occupies full width and should not be placed inline with other components.

## Planned Types (types.rs)

`types.rs` contains forward-looking type stubs that are **not yet compiled into the module** (they reference undefined types like `CurrencyOptions`, `MetricOptions`, and `ColumnAlignment`). These sketch out a richer column model:

### `ColumnType`

```rust
pub enum ColumnType {
    String, Integer, Float,
    Currency(CurrencyOptions),
    Metric(MetricOptions),
    OptString, OptInteger, OptFloat,
    Unknown,
}
```

Intended to enable type-aware formatting and alignment per column (e.g., right-align numbers, format currency with symbols).

### `ColumnAggregate`

```rust
pub enum ColumnAggregate {
    None, Sum, Avg, Median, Min, Max, Range,
}
```

Would enable summary/footer rows showing computed aggregates.

### Planned `TableColumn` (types.rs variant)

A richer column definition with `kind: ColumnType`, `aggregate: ColumnAggregate`, and `alignment: ColumnAlignment` fields alongside an optional title.

### Planned `TableRow` / `TableCell`

Stub structs for row-level metadata (e.g., row titles) and cell-level customization.

## Relationship to Layout System

The table delegates all spatial concerns to `Layout`:

- **Margins** can be absolute (`Chars`), relative (`Percent`), or composed (`Offset`) for nesting inside parent layouts.
- **Word wrapping** applies to the entire rendered table string, not individual cells.
- **`as_child_of(parent, left, right)`** (from `Renderable`) lets the table inherit and extend a parent's margins for nested rendering contexts.
