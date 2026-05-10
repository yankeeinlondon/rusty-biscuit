# Table Component

Terminal-aware table renderer with box-drawing borders, auto-sized columns, and layout integration.

## Module Structure

```
table/
├── mod.rs      # Module re-exports
├── table.rs    # Table struct, TableColumn, TableCellContent, Conditional, Renderable impl
└── types.rs    # ColumnType, Currency, VerticalAlign
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

println!("{}", table.render_optimistic(Some(120)));
```

**Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `title` | `Option<String>` | Optional heading printed above the header row |
| `columns` | `Vec<TableColumn>` | Column definitions (header text + width constraints) |
| `data` | `Vec<Vec<TableCellContent>>` | Row data as a 2D grid of cell values |
| `layout` | `Layout` | Controls margins, alignment, word-wrap, row-fill |
| `prefer_cursor_alignment` | `bool` | Use ANSI cursor positioning instead of space-based padding for cell alignment (helps when glyphs render narrower than computed Unicode width) |
| `alternate_background_color` | `bool` | Apply subtle background color to even data rows (requires true color) |
| `alternate_text_color` | `bool` | Apply subtle text color shift to even data rows (requires true color) |

**Builder Methods:**

| Method | Description |
|--------|-------------|
| `with_title(title)` | Set an optional title line above the table |
| `with_columns(cols)` | Set column definitions |
| `with_data(rows)` | Set all data rows at once |
| `add_row(row)` | Append a single data row (mutates in place, returns `()`) |
| `prefer_cursor_alignment()` | Enable ANSI cursor-based cell alignment |
| `alternate_background_color()` | Enable row striping via background color |
| `alternate_text_color()` | Enable row striping via text color |

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
| `fixed_width` | `Option<usize>` | Exact width (overrides header/data widths and min/max) |
| `min_width` | `Option<usize>` | Floor for the column width |
| `max_width` | `Option<usize>` | Ceiling for the column width |
| `column_type` | `ColumnType` | Data type driving default alignment and word wrap |
| `alignment` | `Option<Alignment>` | Explicit horizontal alignment override (None = use `column_type` default) |
| `word_wrap` | `Option<WordWrap>` | Word wrap override (ignored for numeric column types) |
| `vertical_align` | `VerticalAlign` | Vertical alignment for multi-line cells (default: `Top`) |
| `uniform_alignment` | `bool` | Align all cells at the same position regardless of content width |
| `when` | `Conditional` | Controls column visibility based on terminal width (default: `Always`) |
| `drop_when_space_is_limited(..)` | builder | Marks a column as auto-droppable and optionally records a note |

### `TableCellContent`

Enum representing cell values with four active variants:

| Variant | Wraps | Formatting |
|---------|-------|------------|
| `Text(String)` | `String` | As-is (supports ANSI escape codes) |
| `Integer(i64)` | `i64` | Thousands separators (e.g., `1,234`) |
| `Float(f64)` | `f64` | Two decimal places with thousands separators (e.g., `1,234.56`) |
| `Currency(Currency, f64)` | `Currency` + `f64` | Symbol prefix with thousands separators (e.g., `$1,234.56`) |

`From` impls are provided for `String`, `&str`, `i64`, and `f64`:

```rust
vec!["Alice".into(), 30i64.into(), TableCellContent::Currency(Currency::USD, 99.95)]
```

### `Conditional`

Controls whether a table column is visible based on terminal width. Set on `TableColumn` via `with_when()`.

| Variant | Behavior |
|---------|----------|
| `Always` (default) | Column is always visible |
| `WidthGreaterThan(u32)` | Visible only when renderable width exceeds the threshold |
| `LessThanOrEqual(u32)` | Visible only when renderable width is at or below the threshold |

The width checked is the **renderable width** (terminal width minus layout margins).

### `Currency`

Enum with `USD`, `GBP`, and `EUR` variants. Defined in `types.rs` with a `symbol()` method returning `$`, `£`, or `€`.

## Width Planning

Table rendering now goes through an explicit planning phase before any rows are rendered:

1. Resolve renderable width from `Layout`.
2. Filter columns by `Conditional`.
3. Measure headers and cells line-by-line, not as one raw string.
4. Classify columns as fixed, non-wrapping, or shrinkable.
5. Compute a natural break width for shrinkable columns.
6. Reuse the same `TableWidthPlan` in both normal and cursor-positioned renderers.
7. Optionally drop rightmost droppable columns and append their notes after the table.

Public planning APIs are available on `Table`:

```rust
use biscuit_terminal::components::table::{Table, TableColumn};

let table = Table::new().with_columns(vec![TableColumn::new("Name")]);
let measurements = table.measure_widths(80)?;
let plan = table.plan_widths(80)?;
assert_eq!(measurements.available_render_width, 80);
assert_eq!(plan.visible_column_indices, vec![0]);
# Ok::<(), biscuit_terminal::components::table::TableWidthError>(())
```

The number of columns is still derived from whichever is larger: the column definitions count or the widest data row. Extra cells participate in measurement and rendering with default table-column semantics when no explicit `TableColumn` exists for that index.

## Rendering Pipeline

### Box-Drawing Output

The renderer produces Unicode box-drawing output with top/header/separator/data/bottom borders:

```
Users
┌───────┬─────┐
│ Name  │ Age │
├───────┼─────┤
│ Alice │  30 │
│ Bob   │  25 │
└───────┴─────┘
```

Cell alignment follows each column's effective alignment (for example: text defaults to left, numeric types default to right), with space padding to the computed column width.

### Renderable Trait Integration

`Table` implements `Renderable`, which provides two rendering paths:

| Method | When to Use |
|--------|-------------|
| `render(term_width)` | Optimistic path -- assumes full terminal capabilities. Falls back to 80 columns when `term_width` is `None`. |
| `render(term)` | Conservative path -- receives a `Terminal` reference for capability-aware decisions. Uses `term.width()` for sizing. |

Both paths call `render_content()` to produce raw table text, then pass it through `Layout::apply_layout()` which applies:

- Left/right margin resolution (chars, percent, or nested offset)
- Word wrapping (if configured)
- Text alignment (left, center, right)
- Row-fill padding (for opaque backgrounds)

### Block-Level Behavior

`is_block_level()` returns `true`, signaling to composition systems (like `Compose`) that the table occupies full width and should not be placed inline with other components.

## Types (types.rs)

`types.rs` defines the following fully implemented types:

### `ColumnType`

```rust
pub enum ColumnType {
    String,            // Default alignment: Left, word wrap: WrapProse
    Integer,           // Default alignment: Right, word wrap: None
    Float,             // Default alignment: Right, word wrap: None
    Currency(Currency), // Default alignment: Right, word wrap: None
}
```

Provides `default_alignment()`, `default_word_wrap()`, and `allows_word_wrap_override()` methods. Numeric types disallow word wrap overrides to preserve formatting.

### `VerticalAlign`

```rust
pub enum VerticalAlign { Top, Middle, Bottom }
```

Controls vertical positioning of multi-line cell content. Default is `Top`.

## Relationship to Layout System

The table delegates all spatial concerns to `Layout`:

- **Margins** can be absolute (`Chars`), relative (`Percent`), or composed (`Offset`) for nesting inside parent layouts.
- **Word wrapping** is applied per cell using each column's effective wrap strategy (for example, text columns can wrap while numeric columns force `WordWrap::None`).
- **`with_parent_layout(parent, left, right)`** (from `Renderable`) lets the table inherit and extend a parent's margins for nested rendering contexts.
