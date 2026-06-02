---
last_updated: "2026-05-16"
---

# Challenges of Migrating the `Table` Component to the Tree Rendering Architecture

## Functional and Design Goals

The `Table` component provides a rich, terminal-aware tabular data renderer with Unicode
box-drawing borders, multi-line cell content, conditional column visibility, and
sophisticated width planning. It was created because CLI programs in the rusty-biscuit
monorepo needed to present structured data (configuration, device status, hook
descriptions, model inventories) in a readable, width-adaptive format that goes far
beyond what simple space-padded columns can achieve.

### Why Table was created

Before `Table`, each CLI program built ad-hoc columnar output by manually padding
strings, guessing widths, and hoping the terminal was wide enough. `Table` packages all
of that complexity into a single `TerminalRenderable` component so that:

- **Box-drawing borders** (`┌`, `┬`, `┐`, `│`, `├`, `┼`, `┤`, `└`, `┴`, `┘`) render
  correctly regardless of terminal width.
- **Typed cell content** (`Text`, `Integer`, `Float`, `Currency`) formats
  automatically with locale-appropriate thousands separators and currency symbols.
- **Width planning** measures all cells and headers, respects fixed/min/max width
  constraints, and shrinks or wraps columns to fit the available terminal width.
- **Conditional column visibility** lets columns appear or hide based on terminal
  width (`Conditional::WidthGreaterThan(80)`), enabling responsive tables.
- **Column dropping** allows non-essential columns to be silently dropped (with an
  optional footnote) when the table cannot fit even after wrapping.
- **Row striping** applies subtle alternating background or text colors for
  readability on true-color terminals.
- **Cursor-based alignment** uses ANSI cursor positioning (`\x1b[{n}G`) instead of
  space padding to ensure borders align correctly even when glyphs render narrower
  than their computed Unicode width.
- **Multi-line cells** with word wrapping and vertical alignment (`Top`, `Middle`,
  `Bottom`) let cells span multiple visual rows while maintaining grid alignment.
- **Uniform alignment** ensures cells with mixed-width characters (emoji, symbols)
  align vertically within a column.

### Where Table is used today

| Consumer | Crate | Usage pattern |
|----------|-------|---------------|
| Hook listing / descriptions | `claudine-cli` | `Table::new()` with `prefer_cursor_alignment()` and `alternate_background_color()` for event tables |
| Hook variables, support, capture-method | `claudine-cli` | Various `Table` instances with typed columns (currency, integer) |
| Provider listing, actions | `claudine-cli` | Tables with styled Prose cells |
| AV device status, inputs, zones | `homelab-cli` | Heavily used for receiver/amplifier status display with many columns |
| Filesystem trees, language detection | `sniff-cli` | Tables for file listings and language statistics |
| Program detection output | `sniff-cli` | Tables for detected programs |
| Model inventory, scanner output | `model-citizen-cli` | Tables with cursor alignment |
| Messenger provider info | `messenger-cli` | Tables for provider capability display |
| Drift scripts | `scripts/` | Tables for structured output |
| Render tree terminal renderer | `biscuit-terminal` | `render_terminal_node` creates `Table` instances when rendering `NodeKind::Table` nodes |

### Example usage

**Claudine hook list table** (`claudine/cli/src/commands/hooks/list.rs`):

```rust
let columns = vec![
    TableColumn::new(bold("Event")),
    TableColumn::new(bold("Support")),
    TableColumn::new(bold("Actions")),
];
let mut table = Table::new()
    .with_columns(columns)
    .prefer_cursor_alignment()
    .alternate_background_color();
table.layout_mut().left_margin = Margin::Chars(1);

for (event, actions) in &event_rows {
    let support_cell: TableCellContent = match support_level {
        EventSupportLevel::Hook { .. } => "hook".into(),
        EventSupportLevel::Acp { .. } => {
            Prose::new("{{cyan}}acp{{reset}}").render(&term).into()
        }
        EventSupportLevel::NotSupported => {
            Prose::new("{{dim}}-{{reset}}").render(&term).into()
        }
    };
    table.add_row(vec![
        event.as_pascal_case().into(),
        support_cell,
        actions_cell,
    ]);
}

let rendered = table.render(&term);
```

**Homelab AV device status** (`homelab/cli/src/main.rs`):

```rust
let mut table = Table::new()
    .with_columns(vec![
        TableColumn::new("Zone"),
        TableColumn::new("Source"),
        TableColumn::new("Volume").with_type(ColumnType::Integer),
    ])
    .prefer_cursor_alignment()
    .alternate_text_color();
```

## Technical Implementation (current)

The Table component lives in `biscuit-terminal/lib/src/components/table/` and is split
across five focused submodules:

### Module structure

```
table/
├── mod.rs        — Re-exports all public types
├── table.rs      — Table struct, TerminalRenderable impl, rendering logic
├── column.rs     — TableColumn, Conditional, DropBehavior
├── cell.rs       — TableCellContent, number formatting, pad_cell
├── types.rs      — ColumnType, Currency, VerticalAlign
└── width.rs      — MeasuredColumn, TableWidthMeasurements, TableWidthPlan, TableWidthError
```

### Key struct fields

```text
Table {
    title: Option<String>,                        — Optional table title above borders
    columns: Vec<TableColumn>,                    — Column definitions with types/constraints
    data: Vec<Vec<TableCellContent>>,             — Row data as typed cell values
    layout: Layout,                               — Margins, alignment, row fill, word wrap
    prefer_cursor_alignment: bool,                — ANSI cursor positioning mode
    alternate_background_color: bool,             — Row stripe background toggle
    alternate_text_color: bool,                   — Row stripe foreground toggle
}
```

### Key responsibilities

1. **Width planning pipeline** — The most complex subsystem. For a given terminal width:
   - Determine visible columns via `Conditional` predicates
   - Measure each column: header width, max cell width, natural break widths
   - Compute border overhead (`4 + 3 * (column_count - 1)`)
   - Calculate content budget, fixed/non-wrapping consumption, working width
   - If content exceeds budget, shrink wrapping columns to their natural break width
   - If still too wide, iteratively drop droppable columns (with optional notes)
   - Produce a `TableWidthPlan` with resolved widths per column

2. **Two rendering paths** — `render_content()` (space-padded) and
   `render_with_cursor_positioning()` (ANSI cursor-based). Both share the same
   width planning but differ in how they lay out rows:
   - Space-padded: `layout.apply_block_layout()` adds margins/alignment
   - Cursor-based: `\x1b[{n}G` positions each cell, supporting margins, center/right
     alignment, and row fill

3. **Multi-line cell handling** — Cells are wrapped via `wrap_cell_content()`, then
   row heights are calculated as the max wrapped line count across all cells in the
   row. Shorter cells receive vertical padding (`apply_vertical_padding()`) to align
   with the tallest cell.

4. **Stripe escape management** — When row striping is active, SGR resets inside cell
   content (`\x1b[0m`, `\x1b[49m`) are patched to re-apply the stripe escape so the
   background/foreground tint survives between styled spans. Resets are also inserted
   at borders to keep `│` characters uncolored.

5. **Uniform alignment** — When `uniform_alignment` is enabled on a column, all cells
   align at the same horizontal position (using the max content width across all rows)
   rather than each cell aligning independently.

6. **Dropped column notes** — When columns are dropped due to width constraints, their
   `drop_note` messages are appended as a bullet list below the table.

## Implementation Challenges

### Width Planning Has No Tree Representation

The current `Table` performs width planning as a multi-step computation: measure all
cells, compute border overhead, resolve column widths, optionally shrink and drop
columns. The tree's `NodeKind::Table` carries only `align: Vec<ColumnAlign>` and
`children: Vec<RenderNode>`. There is no place to encode per-column width constraints
(`fixed_width`, `min_width`, `max_width`), word-wrap strategies, or conditional
visibility rules. The width plan is an emergent property of the data *and* the render
target width, not something that can be projected into the tree once and consumed by
all renderers.

**Example:** A `TableColumn` with `fixed_width(10)` and
`Conditional::WidthGreaterThan(80)` renders differently at 60 columns (hidden) vs 100
columns (visible, 10 chars wide). The tree's `Table { align, children }` cannot express
"this column only appears when width > 80."

**Suggested test:**

```rust
#[test]
fn tree_table_preserves_column_visibility_semantics() {
    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("Name"),
            TableColumn::new("Notes")
                .with_when(Conditional::WidthGreaterThan(80)),
        ])
        .with_data(vec![vec!["Alice".into(), "Some notes".into()]]);

    let tree = table.render_tree();

    // At width 60, "Notes" column should not be rendered
    let narrow = render_terminal_node(&tree, &term_60);
    assert!(!narrow.contains("Notes"));

    // At width 100, "Notes" column should appear
    let wide = render_terminal_node(&tree, &term_100);
    assert!(wide.contains("Some notes"));
}
```

### Multi-Layout Rendering Requires Two-Pass Resolution

The tree rendering architecture is a single-pass walk: `render_terminal_node` visits
each node once and emits output. But Table's terminal rendering is inherently two-pass:
first it measures all cells to determine column widths, then it renders each cell with
those widths. This is not just a performance concern — the column widths affect vertical
padding, border positions, stripe escape placement, and cursor coordinates. A tree
renderer that discovers column widths on-the-fly cannot produce correct output for
multi-line cells or right/center-aligned columns.

**Example:** A cell containing "Complete the\nproject documentation" needs wrapping
at width 20 but not at width 30. The wrapped height (2 lines vs 1 line) determines the
row height, which affects vertical padding for all other cells in the same row. Without
a measurement pass, the renderer cannot know how tall any row will be.

**Suggested test:**

```rust
#[test]
fn tree_table_multi_line_row_heights_are_consistent() {
    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("Task").with_max_width(15)
                .with_word_wrap(WordWrap::WrapProse(Some(3), None)),
            TableColumn::new("Status"),
        ])
        .with_data(vec![vec![
            TableCellContent::Text("Complete the project documentation".into()),
            TableCellContent::Text("Done".into()),
        ]]);

    let tree = table.render_tree();
    let output = render_terminal_node(&tree, &term_40);

    let lines: Vec<&str> = output.lines().collect();
    // The data row should span multiple lines, with "Done" vertically aligned
    assert!(lines.len() > 4, "Multi-line cell should increase row count");
    // "Done" should appear on the first data line (Top alignment)
    let first_data_line = lines.iter()
        .find(|l| l.contains("Done"))
        .expect("Done should appear in output");
    assert!(first_data_line.contains("Done"));
}
```

### Stripe Escape Management Requires Cross-Cell State

When `alternate_background_color` or `alternate_text_color` is enabled, the renderer
must track whether the current row is striped and inject/patch ANSI escape codes across
cell boundaries. Specifically:

- SGR resets inside cell content must be followed by a re-apply of the stripe escape
- Background must be reset before the right border `│` and re-applied after the left
  border `│` of the next striped row
- The tree renderer would need to thread this stripe state through its recursion, or
  the `NodeKind::Table` would need attributes to communicate the stripe policy

**Example:** A cell containing `\x1b[31mred\x1b[0m` on a striped row must emit
`\x1b[31mred\x1b[0m\x1b[48;2;30;30;34m` so the background tint continues after the
red text reset. Without this patching, the stripe background ends at `\x1b[0m` and the
rest of the cell (including trailing padding) shows the default background.

**Suggested test:**

```rust
#[test]
fn tree_table_stripe_survives_sgr_reset_in_cells() {
    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("Name"),
            TableColumn::new("Status"),
        ])
        .with_data(vec![
            vec!["Alice".into(), "Active".into()],
            vec!["Bob".into(),
                 TableCellContent::Text("\x1b[31merror\x1b[0m".into())],
        ])
        .alternate_background_color();

    let tree = table.render_tree();
    let output = render_terminal_node(&tree, &term_truecolor);

    // Row 2 (Bob) should have stripe bg that survives the SGR reset
    let bob_lines: Vec<&str> = output.lines()
        .filter(|l| l.contains("Bob"))
        .collect();
    assert!(!bob_lines.is_empty());
    // After \x1b[0m in "error", the stripe bg should be re-applied
    assert!(bob_lines[0].contains("\x1b[48;2;30;30;34m"),
        "Stripe bg should be re-applied after SGR reset");
}
```

### Cursor Positioning Cannot Be Represented in the Tree

The `prefer_cursor_alignment` mode emits `\x1b[{n}G` escape codes to position the
cursor at absolute column positions. This is a terminal-specific concern with no
equivalent in the Markdown or Browser render targets. The tree's `NodeKind::Table`
has no field to indicate "this table should use cursor positioning." The terminal
renderer would need to decide independently to use cursor positioning — but that
decision currently depends on `Table.prefer_cursor_alignment`, which is a per-instance
property set by the caller.

**Example:** A centered table in a 120-column terminal with `prefer_cursor_alignment`
emits `\x1b[38G│` to position the top-left border. The browser renderer would emit
`<table style="margin: 0 auto">` for the same logical intent. The tree cannot carry
both representations; it must carry the semantic intent and let each renderer interpret
it.

**Suggested test:**

```rust
#[test]
fn tree_table_render_terminal_uses_cursor_when_preferred() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("X")])
        .with_data(vec![vec!["A".into()]])
        .prefer_cursor_alignment();

    let tree = table.render_tree();
    let output = render_terminal_node(&tree, &term_80);

    assert!(output.contains("\x1b["),
        "Terminal render should use cursor positioning for cursor-aligned table");
}

#[test]
fn tree_table_render_markdown_ignores_cursor_preference() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("X")])
        .with_data(vec![vec!["A".into()]])
        .prefer_cursor_alignment();

    let tree = table.render_tree();
    let output = render_markdown_node(&tree, &MarkdownRenderOptions::default())
        .unwrap().output;

    assert!(!output.contains("\x1b["),
        "Markdown render should not contain ANSI escapes");
    assert!(output.contains("| X |"), "Markdown should produce GFM table");
}
```

### Typed Cell Content (Currency, Integer, Float) Has No Tree Representation

`TableCellContent` is a rich enum with `Text`, `Integer(i64)`, `Float(f64)`, and
`Currency(Currency, f64)` variants. The tree's `NodeKind::TableCell` contains only
`children: Vec<RenderNode>` — inline content nodes like `Text`, `Strong`, `Emphasis`.
There is no way to express "this cell contains a formatted integer with thousands
separators" or "this cell is a USD currency value." The formatting (comma separators,
currency symbols, decimal places) would either need to happen during tree projection
(making the tree terminal-specific) or be deferred to the renderer (requiring the
renderer to understand column types that the tree cannot express).

**Example:** `TableCellContent::Currency(Currency::USD, 1234.56)` must render as
`$1,234.56` in all targets. The tree could carry the pre-formatted string as
`Text("$1,234.56")`, but then the alignment hint (right-align) is lost since alignment
comes from `ColumnType`, not from the text content itself.

**Suggested test:**

```rust
#[test]
fn tree_table_currency_cell_formats_correctly() {
    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("Item"),
            TableColumn::new("Price")
                .with_type(ColumnType::Currency(Currency::USD)),
        ])
        .with_data(vec![vec![
            "Widget".into(),
            TableCellContent::Currency(Currency::USD, 1234.56),
        ]]);

    let tree = table.render_tree();
    let output = render_terminal_node(&tree, &term_80);

    assert!(output.contains("$1,234.56"),
        "Currency should be formatted with symbol and separators");
}

#[test]
fn tree_table_currency_column_aligns_right() {
    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("Item"),
            TableColumn::new("Price")
                .with_type(ColumnType::Currency(Currency::USD)),
        ])
        .with_data(vec![
            vec!["A".into(), TableCellContent::Currency(Currency::USD, 9.99)],
            vec!["B".into(), TableCellContent::Currency(Currency::USD, 1234.56)],
        ]);

    let tree = table.render_tree();
    let output = render_terminal_node(&tree, &term_80);

    // Both prices should be right-aligned in their column
    let data_lines: Vec<&str> = output.lines()
        .skip_while(|l| !l.contains('$'))
        .collect();
    // "$9.99" and "$1,234.56" should end at the same column
    let pos_1 = data_lines[0].rfind('$').unwrap();
    let pos_2 = data_lines[1].rfind('$').unwrap();
    assert_eq!(pos_1, pos_2, "Currency values should be right-aligned");
}
```

### Column Dropping with Footnotes Is an Interactive Decision

When a table cannot fit, the width planner iteratively drops columns marked with
`drop_when_space_is_limited()` and appends their `drop_note` messages below the table.
This is a dynamic decision that depends on the resolved widths, which themselves depend
on the terminal width. The tree, being a static snapshot, cannot encode "if width < X,
drop column Y and show note Z." The terminal renderer would need to re-implement this
decision logic when walking the tree, duplicating the width planning that the Table
already does.

**Example:** A 5-column table at 60 columns might drop columns 4 and 5, appending
"- Details omitted for narrow terminals" and "- Notes available in wide mode." The
same table at 120 columns shows all 5 columns with no footnotes. The tree produced by
`render_tree()` cannot simultaneously represent both states.

**Suggested test:**

```rust
#[test]
fn tree_table_drops_columns_and_appends_notes_at_narrow_width() {
    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("Name"),
            TableColumn::new("Details")
                .drop_when_space_is_limited(Some("Details available in wide mode")),
        ])
        .with_data(vec![vec![
            "Alice".into(),
            "Long details here".into(),
        ]]);

    let tree = table.render_tree();
    let narrow = render_terminal_node(&tree, &term_30);

    // Narrow terminal should drop "Details" and show the note
    assert!(!narrow.contains("Long details here"));
    assert!(narrow.contains("Details available in wide mode"));
}

#[test]
fn tree_table_shows_all_columns_at_wide_width() {
    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("Name"),
            TableColumn::new("Details")
                .drop_when_space_is_limited(Some("Details available in wide mode")),
        ])
        .with_data(vec![vec![
            "Alice".into(),
            "Long details here".into(),
        ]]);

    let tree = table.render_tree();
    let wide = render_terminal_node(&tree, &term_120);

    assert!(wide.contains("Long details here"));
    assert!(!wide.contains("Details available in wide mode"));
}
```

### Uniform Alignment Requires Cross-Row Measurement

The `uniform_alignment` feature positions all cells in a column at the same horizontal
offset, using the maximum content width across all rows as the alignment reference.
This requires scanning all rows to find the max content width before any row is rendered.
In a tree walk, the renderer encounters rows sequentially — it cannot know the max
content width of future rows while rendering the current one without a pre-scan or
two-pass approach.

**Example:** A column with cells `["✅", "⤫", "⚠️"]` has visible widths of 2, 1, and 2.
With `uniform_alignment(true)` and center alignment, all three cells must be centered
within a width of 2 (the max). Without pre-scanning, the renderer cannot know that the
first cell "✅" should use width 2 instead of its natural width 2 — but for "⤫" (width
1), it must pad to width 2. The decision for "⤫" depends on the max width across all
rows.

**Suggested test:**

```rust
#[test]
fn tree_table_uniform_alignment_positions_all_cells_consistently() {
    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("Name"),
            TableColumn::new("Status")
                .with_alignment(Alignment::Center)
                .with_uniform_alignment(true),
        ])
        .with_data(vec![
            vec!["Build".into(), TableCellContent::Text("✅".into())],
            vec!["Lint".into(), TableCellContent::Text("⤫".into())],
            vec!["Test".into(), TableCellContent::Text("⚠️".into())],
        ]);

    let tree = table.render_tree();
    let output = render_terminal_node(&tree, &term_80);

    let lines: Vec<&str> = output.lines().collect();
    let data_lines: Vec<&str> = lines.iter()
        .filter(|l| l.contains("Build") || l.contains("Lint") || l.contains("Test"))
        .collect();

    // All status symbols should start at the same column position
    let positions: Vec<usize> = data_lines.iter()
        .map(|l| l.find("✅").or_else(|| l.find("⤫")).or_else(|| l.find("⚠")))
        .map(|p| p.unwrap())
        .collect();
    assert!(positions.windows(2).all(|w| w[0] == w[1]),
        "All status symbols should align at the same column: {:?}", positions);
}
```

### Prose-Styled Headers and Cells Create Nested Content

`TableColumn` supports `header_prose: Option<Prose>` for styled headers, and data
cells can contain pre-rendered `Prose` output (ANSI-styled text, OSC8 hyperlinks).
When projecting into the tree, these styled strings need to be decomposed back into
inline nodes (`Text`, `Strong`, `Emphasis`, `Link`) or carried as raw text with
formatting loss. The BlockQuote adoption already demonstrated this loss: `Prose`
content flattens to plain text during tree projection. Table has the same problem but
with the added complexity that cell content must also preserve alignment semantics.

**Example:** A header created with `TableColumn::new_with_bold("Price")` produces a
`Prose` containing `<bold>Price</bold>`. In the tree, this should ideally become
`Strong { children: [Text("Price")] }` inside the `TableCell`. But the current
`Prose → String` rendering path has already resolved the Prose tokens into ANSI
escapes, making structural recovery difficult.

**Suggested test:**

```rust
#[test]
fn tree_table_bold_header_survives_as_strong_node() {
    let table = Table::new()
        .with_columns(vec![
            TableColumn::new_with_bold("Price"),
        ])
        .with_data(vec![vec![TableCellContent::Currency(Currency::USD, 9.99)]]);

    let tree = table.render_tree();

    // The header cell should contain a Strong node, not raw ANSI text
    let header_cell = &tree.kind; // Navigate to first table cell
    // After walking the tree to find the header cell:
    // assert!(contains_strong_node_with_text(&header_cell, "Price"));
}

#[test]
fn tree_table_osc8_link_in_cell_survives_as_link_node() {
    let link = "\x1b]8;;https://example.com\x07click\x1b]8;;\x07";
    let table = Table::new()
        .with_columns(vec![TableColumn::new("URL")])
        .with_data(vec![vec![TableCellContent::Text(link.into())]]);

    let tree = table.render_tree();
    let md_output = render_markdown_node(&tree, &MarkdownRenderOptions::default())
        .unwrap().output;

    assert!(md_output.contains("[click](https://example.com)"),
        "OSC8 link should survive as a Link node in the tree");
}
```

### Row Fill and Layout Composition

The `Layout` system (`left_margin`, `right_margin`, `alignment`, `row_fill_strategy`,
`page_bg_color`) controls the table's position within the terminal viewport. When
`row_fill_strategy` is `Fill` or `Auto`, each row is extended with trailing spaces to
fill the full available width (useful for background color). This layout composition
happens *after* the table content is rendered, via `layout.apply_block_layout()`. In
the tree architecture, there is no `NodeKind` for "apply this layout to this content" —
layout is an out-of-band concern handled by the `TerminalRenderable` trait's `layout()`
accessor.

**Example:** A table with `left_margin = 5`, `right_margin = 5`, `alignment = Center`,
and `row_fill_strategy = Fill` on a 100-column terminal renders each row padded to
90 characters and centered within margins. The tree cannot express "apply this layout
to this table node" without either encoding layout as node attributes or handling it
at the composition layer above the tree.

**Suggested test:**

```rust
#[test]
fn tree_table_respects_layout_margins_and_alignment() {
    let mut table = Table::new()
        .with_columns(vec![TableColumn::new("X")])
        .with_data(vec![vec!["A".into()]]);
    table.layout_mut().left_margin = Margin::Chars(4);
    table.layout_mut().alignment = Alignment::Center;

    let tree = table.render_tree();
    let output = render_terminal_node(&tree, &term_80);

    // Every line should start with 4 spaces of left margin
    for line in output.lines() {
        assert!(line.starts_with("    "),
            "Line should have 4-char left margin: {:?}", line);
    }
}

#[test]
fn tree_table_row_fill_extends_to_available_width() {
    let mut table = Table::new()
        .with_columns(vec![TableColumn::new("X")])
        .with_data(vec![vec!["A".into()]]);
    table.layout_mut().left_margin = Margin::Chars(2);
    table.layout_mut().right_margin = Margin::Chars(2);
    table.layout_mut().row_fill_strategy = RowFill::Fill;

    let tree = table.render_tree();
    let output = render_terminal_node(&tree, &term_20);

    for line in output.lines() {
        let width = visible_width(line);
        assert!(width >= 16,
            "Row fill should extend lines to available width (16), got {}: {:?}",
            width, line);
    }
}
```

### Vertical Alignment Across Heterogeneous Row Heights

When cells in the same row have different wrapped line counts, shorter cells receive
vertical padding so content aligns at the top, middle, or bottom of the row. This
requires knowing the row height (max lines across all cells) before rendering any cell.
In a sequential tree walk, the renderer would need to buffer all cells in a row, compute
the max height, then emit them with appropriate padding — breaking the simple
"visit node, emit string" pattern.

**Example:** A row with cells `["Short", "A longer text\nthat wraps\nto three lines"]`
has height 3. The "Short" cell gets 2 blank lines appended (top alignment) or prepended
(bottom alignment) or split 1+1 (middle alignment). The tree's `TableRow` node contains
all cells, but the renderer must pre-scan all children to determine the row height.

**Suggested test:**

```rust
#[test]
fn tree_table_bottom_alignment_pads_above_content() {
    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("Name"),
            TableColumn::new("Notes")
                .with_vertical_align(VerticalAlign::Bottom)
                .with_max_width(15)
                .with_word_wrap(WordWrap::WrapProse(Some(3), None)),
        ])
        .with_data(vec![vec![
            TableCellContent::Text("Short".into()),
            TableCellContent::Text("A description that is longer than fifteen chars".into()),
        ]]);

    let tree = table.render_tree();
    let output = render_terminal_node(&tree, &term_40);

    let lines: Vec<&str> = output.lines().collect();
    let data_start = lines.iter().position(|l| l.contains("Short")).unwrap();
    // "Short" should appear on the LAST data line (bottom alignment)
    let data_end = data_start + 1; // assuming height > 1
    // The last line of the row should contain "Short"
    let last_data_line = lines[data_end];
    assert!(last_data_line.contains("Short"),
        "Bottom-aligned 'Short' should appear on last line of row");
}
```

### Border Construction Is Width-Dependent

The `build_border()` function constructs horizontal border lines (`┌───┬───┐`,
`├───┼───┤`, `└───┴───┘`) using the resolved column widths. These widths are not known
until the width planning phase completes. In the tree, `NodeKind::Table` does not carry
resolved widths — only `align` and child rows. The terminal renderer must therefore
perform width planning before it can emit borders, meaning the renderer needs access to
the column definitions (which live in `TableColumn`, not in the tree).

**Example:** A table with columns `["Name", "Description"]` and data
`[["Alice", "A very long description"]]` at 40 columns resolves to widths `[5, 25]`.
At 80 columns, the same table resolves to `[5, 65]`. The borders (`┌─────┬─────────────────────────┐`
vs `┌─────┬─────────────────────────────────────────────────────────────────┐`) are
completely different. The tree cannot encode both.

**Suggested test:**

```rust
#[test]
fn tree_table_borders_adapt_to_terminal_width() {
    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("Name"),
            TableColumn::new("Description")
                .with_word_wrap(WordWrap::WrapProse(Some(4), None)),
        ])
        .with_data(vec![vec![
            "Alice".into(),
            "A very long description text".into(),
        ]]);

    let tree = table.render_tree();

    let narrow = render_terminal_node(&tree, &term_40);
    let wide = render_terminal_node(&tree, &term_80);

    let narrow_border_width = visible_width(narrow.lines().next().unwrap());
    let wide_border_width = visible_width(wide.lines().next().unwrap());

    assert!(wide_border_width > narrow_border_width,
        "Borders should be wider on wider terminals");
    assert!(narrow_border_width <= 40);
    assert!(wide_border_width <= 80);
}
```

## Solution Suggestions

#### Node Attributes for Column Metadata

Extend `NodeAttrs` (or introduce a `TableColumnMeta` struct carried in `NodeAttrs.data`)
to encode per-column metadata that the tree currently cannot express: `min_width`,
`max_width`, `fixed_width`, `column_type`, `word_wrap`, `vertical_align`,
`conditional_visibility`, `drop_behavior`, and `uniform_alignment`. These attributes
would be serialized into the `data` BTreeMap under a reserved namespace (e.g.,
`"table.column.0"`, `"table.column.1"`).

**Challenges addressed:** Width Planning Has No Tree Representation, Typed Cell Content
Has No Tree Representation, Column Dropping with Footnotes, Uniform Alignment Requires
Cross-Row Measurement, Vertical Alignment Across Heterogeneous Row Heights.

**How it helps:** The terminal renderer can read these attributes during its walk and
reconstruct the width planning logic. The Markdown and Browser renderers can extract
alignment (`ColumnAlign` already exists) and ignore terminal-specific attributes. Column
types would allow the terminal renderer to apply formatting (currency symbols, thousands
separators) while other renderers would receive pre-formatted text.

**Variant:** Instead of storing metadata in `NodeAttrs.data`, extend `NodeKind::Table`
with an optional `column_meta: Vec<TableColumnMeta>` field. This is more type-safe but
requires changing the `NodeKind` enum and its serialization format.

#### Render-Hint Attributes on Table Nodes

Add a `render_hints: BTreeMap<String, serde_json::Value>` field to `NodeAttrs` (or a
dedicated field) that carries target-specific rendering hints like
`"terminal.prefer_cursor_alignment": true`, `"terminal.alternate_background": true`,
`"terminal.alternate_text_color": true`. These hints are not part of the document's
semantic structure but guide how each renderer should approach the table.

**Challenges addressed:** Cursor Positioning Cannot Be Represented in the Tree,
Row Fill and Layout Composition.

**How it helps:** The terminal renderer reads `render_hints` and decides to use cursor
positioning. The Markdown and Browser renderers ignore these hints entirely. Layout
properties (margins, alignment) can also be carried as render hints, allowing the
terminal renderer to apply them without bloating the semantic tree.

**Variant:** A separate `RenderHints` struct on `NodeAttrs` instead of using the generic
`data` map, providing typed access and documentation.

#### Pre-Formatted Content with Alignment Metadata

Instead of trying to carry typed cell content through the tree, format cells during
tree projection and carry the formatted string as a `Text` node, but attach alignment
metadata (`right`, `center`, `left`) to the `TableCell` via `NodeAttrs.data`. The
terminal renderer uses the alignment metadata to position the pre-formatted text; the
Markdown renderer uses `ColumnAlign` from the `Table` node; the Browser renderer uses
CSS alignment.

**Challenges addressed:** Typed Cell Content Has No Tree Representation,
Prose-Styled Headers and Cells Create Nested Content.

**How it helps:** Avoids the need to decompose `Prose` back into inline nodes. The
projection formats everything into plain text with alignment hints. This is the same
lossy-but-pragmatic approach taken by BlockQuote's tree projection.

**Variant:** For Prose-styled headers specifically, detect the `header_prose` field and
decompose it into tree inline nodes (`Strong`, `Emphasis`, `Link`) using a lightweight
Prose-to-nodes parser, preserving structural information for non-terminal renderers.

#### Two-Pass Terminal Renderer for Tables

The terminal renderer's `NodeKind::Table` branch performs a pre-scan before rendering:
first walk all `TableRow` children to measure content widths and calculate the width
plan, then walk again to emit the actual output. This is architecturally similar to how
the current `Table.render_content()` works but adapted to the tree-walking pattern.

**Challenges addressed:** Multi-Layout Rendering Requires Two-Pass Resolution,
Uniform Alignment Requires Cross-Row Measurement, Vertical Alignment Across
Heterogeneous Row Heights, Border Construction Is Width-Dependent.

**How it helps:** The first pass collects all the information needed for width planning,
border construction, row height calculation, and uniform alignment. The second pass
emits the final output using the resolved measurements. This pattern is clean and
doesn't require changing the tree structure — it only affects the terminal renderer's
implementation.

**Variant:** Cache the measurement results as annotations on the tree nodes during the
first pass, so the second pass can read them without recomputation. This is useful if
the same table is rendered multiple times at the same width.

#### Stripe State as Renderer Context

Thread a `StripeState { bg: Option<&'static str>, fg: Option<&'static str> }` through
the terminal renderer's recursion when inside a `NodeKind::Table` node. The state
tracks whether the current row is striped and what escapes to inject. Cell content
rendering patches SGR resets with the stripe re-apply, and border rendering resets
the stripe before `│` characters.

**Challenges addressed:** Stripe Escape Management Requires Cross-Cell State.

**How it helps:** Keeps the stripe logic entirely within the terminal renderer. The
tree nodes remain clean of terminal-specific escape codes. The renderer knows the row
index (from enumerating `TableRow` children) and can decide stripe activation based on
render hints from `NodeAttrs`.

**Variant:** Instead of threading state, the terminal renderer could render each row
into a buffer, then post-process the buffer to inject stripe escapes. This separates
content rendering from stripe management but adds a post-processing pass.

#### Layout Application at the Composition Layer

Handle margins, alignment, and row fill outside the tree renderer, at the composition
layer where the `Table` is embedded in a larger document. The tree renderer produces
the raw table content (borders, cells), and the composition layer wraps it with the
appropriate layout. This mirrors how `TerminalRenderable.layout.apply_block_layout()`
works today.

**Challenges addressed:** Row Fill and Layout Composition.

**How it helps:** The tree renderer doesn't need to understand layout — it just produces
the table content at its natural width. The composition layer (which already handles
layout for all components) applies margins and alignment. This keeps the tree renderer
simple and composable.

**Variant:** Carry layout properties as render hints (see "Render-Hint Attributes")
and have the terminal renderer apply them as a final step, similar to how
`render_with_cursor_positioning()` does it today.

#### Dynamic Column Dropping During Terminal Render

The terminal renderer's first pass (see "Two-Pass Terminal Renderer") evaluates
conditional visibility and column dropping during measurement. Columns whose
`Conditional` predicates fail at the current terminal width are excluded from the
measurement, and columns marked as droppable are candidates for removal if the table
doesn't fit. Dropped-column notes are collected and appended after the table.

**Challenges addressed:** Column Dropping with Footnotes Is an Interactive Decision,
Width Planning Has No Tree Representation.

**How it helps:** The tree carries *all* columns with their metadata, and the terminal
renderer makes the dynamic decision about which to include based on the target width.
This avoids producing a different tree for every terminal width while still achieving
responsive behavior.

**Variant:** The tree could be parameterized at construction time with the target width,
producing a width-specific tree that already has dropped columns removed. This is
simpler but defeats the "build one tree, walk it per target" goal.
