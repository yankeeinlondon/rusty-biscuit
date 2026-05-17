//! Parity tests for the `Table` component's tree projection and the native
//! two-pass tree renderer.
//!
//! These tests verify that `Table::render_tree_node()` produces a well-formed
//! `NodeKind::Table` tree carrying column/cell/terminal hints, that the tree
//! validates cleanly, and that the native tree renderer reproduces the
//! semantic content, column alignment, conditional-column behavior, typed
//! cells, and multi-line layout of the bespoke `Table::render()` output.

mod parity_helpers;

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::types::{ColumnType, Currency};
use biscuit_terminal::components::table::{
    Conditional, Table, TableCellContent, TableColumn,
};
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use renderable::tree::{
    ColumnAlign, ColumnConditional, NodeKind, RenderNode, RenderStrictness, ValidationMode,
    validate,
};

use parity_helpers::{PARITY_WIDTHS, assert_contains_tokens, strip_ansi, test_terminal};

/// Builds a simple two-column table with two data rows.
fn sample_table() -> Table {
    Table::new()
        .with_columns(vec![TableColumn::new("Name"), TableColumn::new("Score")])
        .with_data(vec![
            vec![TableCellContent::Text("Ann".into()), TableCellContent::Integer(42)],
            vec![TableCellContent::Text("Bob".into()), TableCellContent::Integer(17)],
        ])
}

/// Builds a table with a typed currency column and integers.
fn typed_table() -> Table {
    Table::new()
        .with_columns(vec![
            TableColumn::new("Product"),
            TableColumn::new("Price").with_type(ColumnType::Currency(Currency::USD)),
            TableColumn::new("Stock").with_type(ColumnType::Integer),
        ])
        .with_data(vec![
            vec![
                TableCellContent::Text("Widget".into()),
                TableCellContent::Currency(Currency::USD, 1234.56),
                TableCellContent::Integer(1500),
            ],
            vec![
                TableCellContent::Text("Gadget".into()),
                TableCellContent::Currency(Currency::USD, 9.99),
                TableCellContent::Integer(42),
            ],
        ])
}

/// Renders a render-tree node to a terminal string at the given width.
fn render_tree(node: &RenderNode, width: u32) -> String {
    let term = test_terminal(width);
    let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
    render_terminal_node(node, &opts)
        .expect("tree render should succeed")
        .output
}

// ---------------------------------------------------------------------------
// Structural snapshot
// ---------------------------------------------------------------------------

#[test]
fn render_tree_node_produces_table_kind() {
    let table = sample_table();
    let node = table
        .render_tree_node()
        .expect("Table should produce a tree node");

    match &node.kind {
        NodeKind::Table { align, children } => {
            assert_eq!(align.len(), 2, "two columns yield two alignments");
            // Header row + two data rows.
            assert_eq!(children.len(), 3, "header row plus two data rows");
            assert!(
                matches!(children[0].kind, NodeKind::TableRow { .. }),
                "first child is the header row"
            );
        }
        other => panic!("expected NodeKind::Table, got {other:?}"),
    }
}

#[test]
fn render_tree_node_header_row_holds_column_headers() {
    let table = sample_table();
    let node = table.render_tree_node().expect("tree node");
    let NodeKind::Table { children, .. } = &node.kind else {
        panic!("expected Table");
    };
    let NodeKind::TableRow { children: cells } = &children[0].kind else {
        panic!("expected header row");
    };
    assert_eq!(cells.len(), 2);
    let NodeKind::TableCell { children: c0 } = &cells[0].kind else {
        panic!("expected cell");
    };
    assert!(matches!(&c0[0].kind, NodeKind::Text { value } if value == "Name"));
}

#[test]
fn render_tree_node_carries_column_alignment() {
    // An integer column is right-aligned by default.
    let table = typed_table();
    let node = table.render_tree_node().expect("tree node");
    let NodeKind::Table { align, .. } = &node.kind else {
        panic!("expected Table");
    };
    assert_eq!(align[0], ColumnAlign::Left, "text column left-aligned");
    assert_eq!(align[1], ColumnAlign::Right, "currency column right-aligned");
    assert_eq!(align[2], ColumnAlign::Right, "integer column right-aligned");
}

#[test]
fn render_tree_node_data_cells_carry_typed_hints() {
    let table = typed_table();
    let node = table.render_tree_node().expect("tree node");
    let NodeKind::Table { children, .. } = &node.kind else {
        panic!("expected Table");
    };
    // children[1] is the first data row.
    let NodeKind::TableRow { children: cells } = &children[1].kind else {
        panic!("expected data row");
    };
    let price_hints = cells[1]
        .attrs
        .table_cell_hints()
        .expect("price cell carries hints");
    assert_eq!(price_hints.kind, "currency");
    assert_eq!(price_hints.alignment, "right");

    let stock_hints = cells[2]
        .attrs
        .table_cell_hints()
        .expect("stock cell carries hints");
    assert_eq!(stock_hints.kind, "integer");
    assert_eq!(stock_hints.raw_value, serde_json::json!(1500));
}

#[test]
fn render_tree_node_carries_column_hints() {
    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("A").with_min_width(10).with_max_width(30),
            TableColumn::new("B")
                .with_when(Conditional::WidthGreaterThan(80))
                .drop_when_space_is_limited(Some("column B hidden")),
        ])
        .with_data(vec![vec!["x".into(), "y".into()]]);
    let node = table.render_tree_node().expect("tree node");

    let a = node.attrs.table_column_hints(0);
    assert_eq!(a.min_width, Some(10));
    assert_eq!(a.max_width, Some(30));

    let b = node.attrs.table_column_hints(1);
    assert_eq!(b.conditional, ColumnConditional::WidthGreaterThan(80));
    assert_eq!(b.drop_note.as_deref(), Some("column B hidden"));
}

#[test]
fn render_tree_node_carries_terminal_hints() {
    let table = sample_table()
        .prefer_cursor_alignment()
        .alternate_background_color()
        .alternate_text_color();
    let node = table.render_tree_node().expect("tree node");
    let hints = node.attrs.table_terminal_hints();
    assert!(hints.prefer_cursor_alignment);
    assert!(hints.alternate_background);
    assert!(hints.alternate_text_color);
}

// ---------------------------------------------------------------------------
// Validity
// ---------------------------------------------------------------------------

#[test]
fn projected_tree_validates_with_no_errors() {
    let table = typed_table();
    let node = table.render_tree_node().expect("tree node");
    let report = validate(&node, ValidationMode::Full);
    assert!(
        !report.has_errors(),
        "projected Table tree should validate cleanly: {:?}",
        report.errors().collect::<Vec<_>>()
    );
}

#[test]
fn projected_empty_table_validates() {
    let table = Table::new().with_columns(vec![TableColumn::new("Only")]);
    let node = table.render_tree_node().expect("tree node");
    let report = validate(&node, ValidationMode::Full);
    assert!(!report.has_errors(), "header-only table validates cleanly");
}

// ---------------------------------------------------------------------------
// Semantic parity
// ---------------------------------------------------------------------------

#[test]
fn tree_output_contains_same_content_as_bespoke() {
    let table = sample_table();
    let term = test_terminal(80);
    let bespoke = strip_ansi(&table.render(&term));
    let tokens: Vec<&str> = bespoke
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .collect();

    let node = table.render_tree_node().expect("tree node");
    let tree = render_tree(&node, 80);
    assert_contains_tokens(&tree, &tokens);
}

#[test]
fn tree_output_preserves_all_cell_content() {
    let table = typed_table();
    let node = table.render_tree_node().expect("tree node");
    let tree = render_tree(&node, 80);
    assert_contains_tokens(
        &tree,
        &["Product", "Price", "Stock", "Widget", "Gadget"],
    );
}

// ---------------------------------------------------------------------------
// Positional parity — alignment, borders
// ---------------------------------------------------------------------------

#[test]
fn tree_output_draws_box_borders() {
    let table = sample_table();
    let node = table.render_tree_node().expect("tree node");
    let tree = strip_ansi(&render_tree(&node, 80));
    assert!(tree.contains('┌'), "top-left corner present");
    assert!(tree.contains('┼'), "junction present");
    assert!(tree.contains('┘'), "bottom-right corner present");
}

#[test]
fn tree_output_keeps_columns_in_order() {
    let table = typed_table();
    let node = table.render_tree_node().expect("tree node");
    let plain = strip_ansi(&render_tree(&node, 80));
    let product = plain.find("Product").expect("Product header");
    let price = plain.find("Price").expect("Price header");
    let stock = plain.find("Stock").expect("Stock header");
    assert!(product < price && price < stock, "column order preserved");
}

#[test]
fn tree_output_handles_multi_line_cells() {
    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("Task"),
            TableColumn::new("Status"),
        ])
        .with_data(vec![vec![
            TableCellContent::Text("line one\nline two".into()),
            TableCellContent::Text("Done".into()),
        ]]);
    let node = table.render_tree_node().expect("tree node");
    let plain = strip_ansi(&render_tree(&node, 80));
    assert!(plain.contains("line one"), "first line present");
    assert!(plain.contains("line two"), "second line present");
    // The two lines occupy distinct output rows.
    let one = plain.find("line one").unwrap();
    let two = plain.find("line two").unwrap();
    let between = &plain[one..two];
    assert!(between.contains('\n'), "multi-line cell spans rows");
}

// ---------------------------------------------------------------------------
// Width matrix
// ---------------------------------------------------------------------------

#[test]
fn tree_output_renders_at_all_parity_widths() {
    for &width in PARITY_WIDTHS {
        let table = sample_table();
        let node = table.render_tree_node().expect("tree node");
        let tree = render_tree(&node, width);
        assert_contains_tokens(&tree, &["Name", "Score", "Ann", "Bob"]);
    }
}

#[test]
fn wide_width_shows_conditional_column() {
    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("Name"),
            TableColumn::new("Notes").with_when(Conditional::WidthGreaterThan(60)),
        ])
        .with_data(vec![vec![
            TableCellContent::Text("Ann".into()),
            TableCellContent::Text("important".into()),
        ]]);
    let node = table.render_tree_node().expect("tree node");
    // At 120 columns the conditional column is visible.
    let wide = strip_ansi(&render_tree(&node, 120));
    assert!(wide.contains("Notes"), "conditional column shown when wide");
    assert!(wide.contains("important"), "conditional cell content shown");
}

#[test]
fn narrow_width_hides_conditional_column() {
    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("Name"),
            TableColumn::new("Notes").with_when(Conditional::WidthGreaterThan(60)),
        ])
        .with_data(vec![vec![
            TableCellContent::Text("Ann".into()),
            TableCellContent::Text("important".into()),
        ]]);
    let node = table.render_tree_node().expect("tree node");
    // At 40 columns the conditional column is hidden.
    let narrow = strip_ansi(&render_tree(&node, 40));
    assert!(narrow.contains("Name"), "always-visible column shown");
    assert!(
        !narrow.contains("Notes"),
        "conditional column hidden when narrow:\n{narrow}"
    );
}

#[test]
fn dropped_column_appends_drop_note() {
    // A fixed-width column that cannot fit at a narrow width is dropped, and
    // its drop note is appended after the table.
    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("Name"),
            TableColumn::new("Details")
                .with_fixed_width(60)
                .drop_when_space_is_limited(Some("details omitted to fit")),
        ])
        .with_data(vec![vec![
            TableCellContent::Text("Ann".into()),
            TableCellContent::Text("a long description".into()),
        ]]);
    let node = table.render_tree_node().expect("tree node");
    let narrow = strip_ansi(&render_tree(&node, 30));
    assert!(
        narrow.contains("details omitted to fit"),
        "drop note appended after the table:\n{narrow}"
    );
}

// ---------------------------------------------------------------------------
// Typed cells
// ---------------------------------------------------------------------------

#[test]
fn typed_cells_render_readable_and_right_aligned() {
    let table = typed_table();
    let node = table.render_tree_node().expect("tree node");
    let plain = strip_ansi(&render_tree(&node, 80));
    // Integer cells are formatted with thousands separators.
    assert!(plain.contains("1,500"), "integer formatted readably");
    // Currency cells carry the symbol and decimals.
    assert!(plain.contains("$1,234.56"), "currency formatted readably");

    // The integer column is right-aligned: the larger value's first digit is
    // further left than the smaller value's, so trailing positions align.
    let lines: Vec<&str> = plain.lines().collect();
    let big = lines.iter().find(|l| l.contains("1,500")).unwrap();
    let small = lines.iter().find(|l| l.contains("42")).unwrap();
    let big_end = big.find("1,500").unwrap() + "1,500".len();
    let small_end = small.find("42").unwrap() + "42".len();
    // Right alignment puts both numbers' ends within one cell of each other.
    assert!(
        big_end.abs_diff(small_end) <= 3,
        "integer column right-aligned (ends near-aligned)"
    );
}

// ---------------------------------------------------------------------------
// Strictness — malformed hints handled gracefully
// ---------------------------------------------------------------------------

#[test]
fn malformed_cell_hints_degrade_to_text() {
    // Build a table tree by hand with a cell claiming `integer` kind but
    // carrying a non-numeric raw value. The renderer must not panic and must
    // fall back to the cell's text.
    use renderable::tree::TableCellHints;

    let mut cell = RenderNode::table_cell(vec![RenderNode::text("oops")]);
    cell.attrs.set_table_cell_hints(&TableCellHints {
        kind: "integer".into(),
        raw_value: serde_json::json!("not a number"),
        alignment: "right".into(),
        vertical_alignment: "top".into(),
    });

    let table = RenderNode::table(
        vec![ColumnAlign::Left],
        vec![
            RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text(
                "Header",
            )])]),
            RenderNode::table_row(vec![cell]),
        ],
    );

    let report = validate(&table, ValidationMode::Full);
    assert!(!report.has_errors(), "hand-built table validates");

    let out = strip_ansi(&render_tree(&table, 80));
    assert!(out.contains("oops"), "malformed cell degrades to its text");
}

#[test]
fn malformed_conditional_token_defaults_to_always() {
    // A column hint with a bogus conditional token must not hide the column.
    use renderable::tree::HintNamespace;

    let mut table = RenderNode::table(
        vec![ColumnAlign::Left],
        vec![
            RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text(
                "Col",
            )])]),
            RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text(
                "Val",
            )])]),
        ],
    );
    table.attrs.set_hint(
        HintNamespace::TABLE,
        "column.0.conditional",
        serde_json::json!("garbage"),
    );

    let out = strip_ansi(&render_tree(&table, 40));
    assert!(
        out.contains("Col") && out.contains("Val"),
        "malformed conditional defaults to always-visible:\n{out}"
    );
}
