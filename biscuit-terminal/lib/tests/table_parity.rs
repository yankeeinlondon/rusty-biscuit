//! Parity tests for the `Table` component's tree projection and the native
//! two-pass tree renderer.
//!
//! These tests verify that `Table::render_tree_node()` produces a well-formed
//! `NodeKind::Table` tree carrying column/cell/terminal hints, that the tree
//! validates cleanly, and that the native tree renderer reproduces the
//! semantic content, column alignment, conditional-column behavior, typed
//! cells, and multi-line layout of the bespoke `Table::render()` output.
//!
//! ## Escape-hatch coverage
//!
//! This file exercises the sanctioned `Table::render_bespoke` escape hatch
//! (`prefer_cursor_alignment` knob plus the TTY-specific cursor-positioning
//! path — capabilities the render tree cannot yet express). The hook is
//! `#[doc(hidden)]` but `pub` so integration tests can reach it; see
//! `table/table.rs::render_bespoke` for the policy rationale.

mod parity_helpers;

use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use biscuit_terminal::components::table::types::{ColumnType, Currency};
use biscuit_terminal::components::table::{Conditional, Table, TableCellContent, TableColumn};
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use renderable::markdown::MarkdownRenderable;
use renderable::tree::{
    ColumnAlign, ColumnConditional, NodeKind, RenderNode, RenderStrictness, TreeRenderable,
    ValidationMode, validate,
};

use parity_helpers::{PARITY_WIDTHS, assert_contains_tokens, strip_ansi, test_terminal};

/// Builds a simple two-column table with two data rows.
fn sample_table() -> Table {
    Table::new()
        .with_columns(vec![TableColumn::new("Name"), TableColumn::new("Score")])
        .with_data(vec![
            vec![
                TableCellContent::Text("Ann".into()),
                TableCellContent::Integer(42),
            ],
            vec![
                TableCellContent::Text("Bob".into()),
                TableCellContent::Integer(17),
            ],
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
    assert_eq!(
        align[1],
        ColumnAlign::Right,
        "currency column right-aligned"
    );
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
    assert_contains_tokens(&tree, &["Product", "Price", "Stock", "Widget", "Gadget"]);
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
        .with_columns(vec![TableColumn::new("Task"), TableColumn::new("Status")])
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
fn always_conditional_column_stays_visible_when_narrow() {
    // An `Always` conditional column is never hidden, regardless of width. The
    // conditional now lives in a typed `ColumnConditional` field, so a
    // malformed token can no longer reach this path.
    use renderable::tree::TableColumnHints;

    let mut table = RenderNode::table(
        vec![ColumnAlign::Left],
        vec![
            RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text("Col")])]),
            RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text("Val")])]),
        ],
    );
    table.attrs.set_table_column_hints(
        0,
        &TableColumnHints {
            conditional: ColumnConditional::Always,
            ..TableColumnHints::default()
        },
    );

    let out = strip_ansi(&render_tree(&table, 40));
    assert!(
        out.contains("Col") && out.contains("Val"),
        "always-conditional column stays visible when narrow:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// Canonical TreeRenderable + cross-target rendering
// ---------------------------------------------------------------------------

#[test]
fn tree_renderable_and_terminal_hook_agree() {
    // Both producers must delegate to the same private projection helper,
    // so their outputs must be identical.
    let table = typed_table();
    let canonical = <Table as TreeRenderable>::render_tree(&table);
    let compat = table
        .render_tree_node()
        .expect("compat hook should produce a node");
    assert_eq!(
        canonical, compat,
        "TreeRenderable::render_tree and TerminalRenderable::render_tree_node must agree"
    );
}

#[test]
fn render_terminal_default_routes_through_tree() {
    // The default render path should match what the tree renderer produces.
    let table = sample_table();
    let term = test_terminal(80);
    let default = strip_ansi(&table.render(&term));
    let via_tree = strip_ansi(&render_tree(
        &<Table as TreeRenderable>::render_tree(&table),
        80,
    ));
    assert_eq!(
        default, via_tree,
        "TerminalRenderable::render must route through the canonical tree"
    );
}

#[test]
fn render_bespoke_still_available_for_parity() {
    // The bespoke path remains accessible for parity testing.
    let table = sample_table();
    let term = test_terminal(80);
    let bespoke = strip_ansi(&table.render_bespoke(&term));
    assert!(bespoke.contains("Name"));
    assert!(bespoke.contains("Score"));
    assert!(bespoke.contains("Ann"));
    assert!(bespoke.contains("Bob"));
}

#[test]
fn render_markdown_emits_gfm_pipe_table() {
    let table = sample_table();
    let md = table.render_markdown();
    assert!(md.contains("| Name"), "header pipe row: {md}");
    assert!(md.contains("| Score"), "header pipe row: {md}");
    assert!(md.contains("--"), "GFM delimiter row: {md}");
    assert!(md.contains("Ann"), "data row 1: {md}");
    assert!(md.contains("Bob"), "data row 2: {md}");
    // No box drawing in Markdown output.
    assert!(!md.contains('┌'), "no terminal borders in Markdown");
}

#[test]
fn render_markdown_and_markdown_plus_match_for_pure_gfm_table() {
    let table = sample_table();
    let md = table.render_markdown();
    let md_plus = table.render_markdown_plus();
    assert_eq!(
        md, md_plus,
        "Table's pure-GFM output is identical for Markdown and MarkdownPlus"
    );
}

#[test]
fn render_markdown_escapes_pipe_in_cells() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Name"), TableColumn::new("Note")])
        .with_data(vec![vec![
            TableCellContent::Text("Ann".into()),
            TableCellContent::Text("left|right".into()),
        ]]);
    let md = table.render_markdown();
    assert!(
        md.contains("left\\|right"),
        "literal pipe escaped in cell: {md}"
    );
}

#[test]
fn render_markdown_normalizes_cell_newline_to_br() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Task"), TableColumn::new("Status")])
        .with_data(vec![vec![
            TableCellContent::Text("line one\nline two".into()),
            TableCellContent::Text("Done".into()),
        ]]);
    let md = table.render_markdown();
    assert!(
        md.contains("<br>"),
        "literal newline normalized to <br> in cell: {md}"
    );
    // Table rows must remain single-line for GFM validity — no embedded `\n`
    // inside the cell-text run.
    let row_with_break = md
        .lines()
        .find(|l| l.contains("line one"))
        .expect("found row containing the multi-line cell");
    assert!(
        row_with_break.contains("line two"),
        "both lines must end up on the same Markdown row: {row_with_break}"
    );
}

#[test]
fn render_markdown_with_title_emits_caption_before_table() {
    let table = sample_table().with_title("Roster");
    let md = table.render_markdown();
    let table_start = md.find("| Name").expect("table row present");
    let title_pos = md.find("Roster").expect("title present");
    assert!(
        title_pos < table_start,
        "title appears before the table: {md}"
    );
}

#[test]
fn render_markdown_ignores_whitespace_only_title() {
    let table = sample_table().with_title("   ");
    let md = table.render_markdown();
    assert!(
        !md.contains("   "),
        "whitespace-only title not emitted: {md}"
    );
}

#[test]
fn render_html_emits_table_with_thead_and_tbody() {
    let table = sample_table();
    let html = table.render_html_fragment().render();
    assert!(html.contains("<table"), "table element: {html}");
    assert!(html.contains("</table>"), "table close: {html}");
    assert!(html.contains("<thead"), "thead: {html}");
    assert!(html.contains("<tbody"), "tbody: {html}");
    assert!(html.contains("<th"), "th: {html}");
    assert!(html.contains("<td"), "td: {html}");
}

#[test]
fn render_html_with_title_emits_caption() {
    let table = sample_table().with_title("Roster");
    let html = table.render_html_fragment().render();
    assert!(html.contains("<caption"), "caption element: {html}");
    assert!(html.contains("Roster"), "title text: {html}");
}

#[test]
fn render_html_right_aligned_column_carries_text_align() {
    let table = typed_table();
    let html = table.render_html_fragment().render();
    assert!(
        html.contains("text-align:right") || html.contains("text-align: right"),
        "integer column lowers to text-align:right: {html}"
    );
}

#[test]
fn render_terminal_with_title_emits_title_above_top_border() {
    let table = sample_table().with_title("Roster");
    let term = test_terminal(80);
    let out = strip_ansi(&table.render(&term));
    let title = out.find("Roster").expect("title present");
    let border = out.find('┌').expect("top border present");
    assert!(title < border, "title appears above the top border:\n{out}");
}

// ---------------------------------------------------------------------------
// Spec variant #16: cursor-positioning escape hatch
//
// `prefer_cursor_alignment` is a documented opt-in used by ~30 production CLI
// call sites (`claudine`, `sniff`, `model-citizen`, `messenger`, …). When the
// caller sets it AND the terminal is a TTY, `render` must delegate to the
// bespoke path so the column-move escape bytes (`CSI N G`) reach the user's
// terminal. When unset — or when stdout is a pipe/redirect — the canonical
// tree path applies so captured output stays free of cursor-control bytes.
// ---------------------------------------------------------------------------

#[test]
fn prefer_cursor_alignment_tty_emits_column_move_escapes() {
    let table = sample_table().prefer_cursor_alignment();
    let term = test_terminal(80); // optimistic terminal => is_tty == true
    let out = table.render(&term);

    // The bespoke cursor-positioning path emits a `CSI N G` sequence
    // (`\x1b[<col>G`) at the start of every output line. The tree path
    // emits no column-move bytes.
    assert!(
        out.contains("\x1b["),
        "TTY + prefer_cursor_alignment must route through the bespoke path \
         and emit ANSI escapes; got: {out:?}"
    );
    assert!(
        out.contains('G'),
        "TTY + prefer_cursor_alignment must emit `CSI N G` column-move \
         escapes; got: {out:?}"
    );
}

#[test]
fn default_render_emits_no_column_move_escapes() {
    // No `prefer_cursor_alignment` — the tree path must be taken, so no
    // bare `CSI N G` cursor-move sequences should appear.
    let table = sample_table();
    let term = test_terminal(80);
    let out = table.render(&term);

    // Tree-path output may carry SGR escapes for color, but it must not
    // emit `CSI N G` column-position sequences.
    let mut bytes = out.bytes().peekable();
    while let Some(b) = bytes.next() {
        if b != 0x1B {
            continue;
        }
        if bytes.next() != Some(b'[') {
            continue;
        }
        // Collect the parameter digits.
        let mut param_was_numeric = false;
        let mut terminator = None;
        for nb in bytes.by_ref() {
            if nb.is_ascii_digit() {
                param_was_numeric = true;
                continue;
            }
            terminator = Some(nb);
            break;
        }
        if param_was_numeric && terminator == Some(b'G') {
            panic!(
                "tree-rendered Table must not emit `CSI N G` cursor-move \
                 escapes; got: {out:?}"
            );
        }
    }
}

#[test]
fn prefer_cursor_alignment_without_tty_routes_through_tree() {
    // When stdout is not a TTY (e.g. piped to a file or another process),
    // the bespoke cursor-positioning path is suppressed even if the
    // caller opted in, because `render_bespoke`'s internal guard requires
    // `term.is_tty`. The render must therefore fall through to the tree
    // path and emit no `CSI N G` sequences.
    let table = sample_table().prefer_cursor_alignment();
    let term = Terminal::builder().is_tty(false).width(80).build();
    let out = table.render(&term);

    let mut bytes = out.bytes().peekable();
    while let Some(b) = bytes.next() {
        if b != 0x1B {
            continue;
        }
        if bytes.next() != Some(b'[') {
            continue;
        }
        let mut param_was_numeric = false;
        let mut terminator = None;
        for nb in bytes.by_ref() {
            if nb.is_ascii_digit() {
                param_was_numeric = true;
                continue;
            }
            terminator = Some(nb);
            break;
        }
        if param_was_numeric && terminator == Some(b'G') {
            panic!(
                "non-TTY + prefer_cursor_alignment must NOT emit `CSI N G` \
                 cursor-move escapes (the bespoke guard requires is_tty); \
                 got: {out:?}"
            );
        }
    }
}

#[test]
fn render_bespoke_emits_column_move_when_cursor_alignment_set() {
    // Pins the documented divergence: `render_bespoke` honors
    // `prefer_cursor_alignment`; `render_via_tree` does not.
    let table = sample_table().prefer_cursor_alignment();
    let term = test_terminal(80);
    let bespoke = table.render_bespoke(&term);
    assert!(
        bespoke.contains("\x1b["),
        "render_bespoke must emit cursor-move escapes when \
         prefer_cursor_alignment is set: {bespoke:?}"
    );

    // The tree path, in contrast, does not lower the
    // `prefer_cursor_alignment` terminal hint and therefore emits no
    // `CSI N G` sequences.
    let via_tree = render_tree(&<Table as TreeRenderable>::render_tree(&table), 80);
    let mut bytes = via_tree.bytes().peekable();
    while let Some(b) = bytes.next() {
        if b != 0x1B {
            continue;
        }
        if bytes.next() != Some(b'[') {
            continue;
        }
        let mut param_was_numeric = false;
        let mut terminator = None;
        for nb in bytes.by_ref() {
            if nb.is_ascii_digit() {
                param_was_numeric = true;
                continue;
            }
            terminator = Some(nb);
            break;
        }
        if param_was_numeric && terminator == Some(b'G') {
            panic!(
                "render_via_tree must NOT emit `CSI N G` cursor-move escapes \
                 (terminal hint is not yet lowered to the tree renderer); \
                 got: {via_tree:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Spec variant #21: ANSI-colored cell content survives both targets.
// ---------------------------------------------------------------------------

#[test]
fn ansi_colored_cell_content_survives_terminal_render() {
    // Cells containing pre-styled SGR bytes must round-trip through the
    // terminal target with their visible text intact. The tree path
    // currently escapes the literal `\x1b` byte to its `\\[…` sequence
    // (so the structural box drawing is not corrupted), but the underlying
    // text and color tokens reach the user — this test pins that behavior
    // so a future change either keeps the escape OR consciously upgrades
    // it to pass-through.
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Name"), TableColumn::new("Status")])
        .with_data(vec![vec![
            TableCellContent::Text("Alice".into()),
            TableCellContent::Text("\x1b[32mActive\x1b[0m".into()),
        ]]);
    let term = test_terminal(80);
    let out = table.render(&term);

    // The visible text reaches the user in every case.
    assert!(out.contains("Active"), "cell text reaches output: {out:?}");
    // The color token (`32m`) — whether emitted live or rendered as the
    // backslash-escaped literal — must still appear so the user can see
    // that the style was preserved through the pipeline.
    assert!(
        out.contains("32m"),
        "SGR color token preserved (live or escaped) in output: {out:?}"
    );
    // The table's own border drawing must still be present and uncorrupted.
    assert!(out.contains('│'), "borders intact: {out:?}");
}

#[test]
fn ansi_colored_cell_content_sensibly_handled_in_markdown() {
    // Markdown is a structural target with no SGR equivalent. The renderer
    // should not crash and should preserve the underlying visible text;
    // raw escape bytes either pass through inert or are dropped, but the
    // table structure must remain valid GFM.
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Name"), TableColumn::new("Status")])
        .with_data(vec![vec![
            TableCellContent::Text("Alice".into()),
            TableCellContent::Text("\x1b[32mActive\x1b[0m".into()),
        ]]);
    let md = table.render_markdown();
    assert!(
        md.contains("Active"),
        "visible text preserved in Markdown: {md:?}"
    );
    // GFM structural integrity: every data row line that contains a pipe
    // must start and end with a pipe (after trimming whitespace).
    for line in md.lines() {
        let trimmed = line.trim();
        if !trimmed.contains('|') {
            continue;
        }
        assert!(
            trimmed.starts_with('|') && trimmed.ends_with('|'),
            "GFM row must be pipe-delimited end-to-end: {trimmed:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Spec variant #22: OSC8 hyperlink cells smoke test.
// ---------------------------------------------------------------------------

#[test]
fn osc8_hyperlink_cell_smoke_test() {
    // An OSC8 hyperlink wrapper around cell content should not blow up the
    // renderer in either Terminal or Markdown targets, and the visible link
    // text must reach the output.
    let hyperlink = "\x1b]8;;https://example.com/\x1b\\Example\x1b]8;;\x1b\\";
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Label")])
        .with_data(vec![vec![TableCellContent::Text(hyperlink.into())]]);

    let term = test_terminal(80);
    let term_out = table.render(&term);
    assert!(
        term_out.contains("Example"),
        "visible hyperlink text reaches terminal output: {term_out:?}"
    );

    let md = table.render_markdown();
    assert!(
        md.contains("Example"),
        "visible hyperlink text reaches Markdown output: {md:?}"
    );
}

// ---------------------------------------------------------------------------
// Combined Markdown escape: all four normalizations apply simultaneously and
// the result is valid GFM (single line per row, pipe-delimited).
// ---------------------------------------------------------------------------

#[test]
fn render_markdown_combined_escape_pipe_newline_soft_and_hard_break() {
    use renderable::tree::RenderNode;
    use renderable::tree::render::{MarkdownRenderOptions, render_markdown_node};

    // Mix all four escape triggers in a single hand-built table cell so the
    // four normalizations apply simultaneously:
    //
    //   1. A literal `|` (must be escaped to `\|`).
    //   2. A literal `\n` inside a Text node (must become `<br>`).
    //   3. A `SoftBreak` node child (must become `<br>` inside a cell).
    //   4. A `HardBreak` node child (must become `<br>` inside a cell).
    //
    // `TableCellContent::Text` is plain text only, so we cannot construct
    // SoftBreak/HardBreak children through the public builder. We thread
    // them through a hand-built `RenderNode::table_cell` and feed the
    // resulting tree directly to `render_markdown_node`, which is exactly
    // the call path that `Table::render_markdown` uses internally.
    let mixed_cell = RenderNode::table_cell(vec![
        RenderNode::text("left|right"),
        RenderNode::soft_break(),
        RenderNode::text("after-soft"),
        RenderNode::hard_break(),
        RenderNode::text("after-hard"),
        RenderNode::text("line\nliteral"),
    ]);
    let header_cell = RenderNode::table_cell(vec![RenderNode::text("Mix")]);
    let table = RenderNode::table(
        vec![ColumnAlign::Left],
        vec![
            RenderNode::table_row(vec![header_cell]),
            RenderNode::table_row(vec![mixed_cell]),
        ],
    );

    let md = render_markdown_node(&table, &MarkdownRenderOptions::default())
        .expect("hand-built mixed-escape table should render")
        .output;

    // 1. Literal pipe is escaped end-to-end.
    assert!(
        md.contains("left\\|right"),
        "literal `|` escaped to `\\|`: {md}"
    );
    // 2. Literal `\n` inside a Text node collapses to `<br>` so the row
    //    stays valid GFM.
    assert!(
        md.contains("line<br>literal"),
        "literal `\\n` between adjacent text becomes `<br>`: {md}"
    );
    // 3. A `SoftBreak` node inside a cell collapses to a single space
    //    (documented in `renderable/src/tree/render/markdown.rs`). This
    //    pins the divergence from the spec's `<br>` suggestion.
    assert!(
        md.contains("left\\|right after-soft"),
        "SoftBreak inside a cell collapses to a single space (not `<br>`); \
         got: {md}"
    );
    // 4. A `HardBreak` node inside a cell becomes `<br>`.
    assert!(
        md.contains("after-soft<br>after-hard"),
        "HardBreak inside a cell becomes `<br>`: {md}"
    );

    // All four escape sources land on the same single Markdown row.
    let data_row = md
        .lines()
        .find(|l| l.contains("after-soft"))
        .expect("data row present");
    assert!(
        data_row.contains("after-hard"),
        "hard-break sibling on the same row: {data_row}"
    );
    assert!(
        data_row.contains("literal"),
        "literal-newline sibling on the same row: {data_row}"
    );
    assert!(
        data_row.contains("left\\|right"),
        "pipe-escape sibling on the same row: {data_row}"
    );

    // GFM validity: every pipe-containing row must be single-line and
    // pipe-delimited end-to-end.
    let mut pipe_rows = 0;
    for line in md.lines() {
        let trimmed = line.trim();
        if !trimmed.contains('|') {
            continue;
        }
        pipe_rows += 1;
        assert!(
            trimmed.starts_with('|') && trimmed.ends_with('|'),
            "GFM row must be pipe-delimited end-to-end: {trimmed:?}"
        );
        assert!(
            !trimmed.contains('\n'),
            "GFM row must be a single line (no embedded \\n): {trimmed:?}"
        );
    }
    assert!(pipe_rows >= 2, "expected at least header+data rows: {md}");
}

// ---------------------------------------------------------------------------
// Broadened tree-vs-compat-hook agreement across representative tables.
// ---------------------------------------------------------------------------

#[test]
fn tree_renderable_and_terminal_hook_agree_across_table_shapes() {
    use biscuit_terminal::utils::layout::{Length, TargetValue};

    // Typed cells exercise typed-hint propagation.
    let typed = typed_table();

    // A captioned table exercises the title hint plumbing.
    let with_title = sample_table().with_title("Roster");

    // A table with non-default layout margins exercises the consolidated
    // layout hint serialization.
    let mut with_margins = sample_table();
    with_margins.layout_mut().margin.left = TargetValue::universal(Length::ch(4));
    with_margins.layout_mut().margin.right = TargetValue::universal(Length::ch(4));

    for (label, table) in [
        ("typed", typed),
        ("with_title", with_title),
        ("with_layout_margins", with_margins),
    ] {
        let canonical = <Table as TreeRenderable>::render_tree(&table);
        let compat = table
            .render_tree_node()
            .expect("compat hook produces a node");
        assert_eq!(
            canonical, compat,
            "TreeRenderable::render_tree and TerminalRenderable::render_tree_node \
             must agree for `{label}` shape"
        );
    }
}

// ---------------------------------------------------------------------------
// Style-everywhere Phase 3, Task 3.4 — `prefer_cursor_alignment` parity on
// the honored subset (C5/C6).
//
// `Table::prefer_cursor_alignment` keeps a bespoke cursor core because ANSI
// cursor moves (`CSI N G`) are terminal-only and cannot be represented in the
// render tree. The cursor path and the tree path MUST agree on the honored
// subset (`margin` / `alignment` / `max_width`): both produce the same visible
// cell text, the same row count, and the same outer-box placement after the
// cursor escapes are stripped. The cursor escapes replace inter-cell padding
// only — they do not change the visible cell text or the outer box position.
//
// Per spec C5: the bespoke path honors `margin` / `alignment` / `max_width`
// (the outer-box placement contract). Per spec C6: the two render paths within
// the Terminal target MUST agree on the honored subset.
// ---------------------------------------------------------------------------

/// Strips `CSI N G` / `CSI N d` cursor-position escapes (and only those) from
/// `s`.
///
/// Used to compare the visible text of the bespoke cursor-alignment path
/// against the tree path. Other SGR escapes (color, weight) are preserved
/// because they are not part of the cursor-positioning escape hatch.
fn strip_cursor_move_escapes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\x1b' || chars.peek() != Some(&'[') {
            out.push(ch);
            continue;
        }
        // Consume the CSI introducer and capture the parameter run.
        chars.next(); // consume `[`
        let mut params = String::new();
        while matches!(chars.peek(), Some(c) if c.is_ascii_digit()) {
            params.push(chars.next().unwrap());
        }
        let final_byte = chars.next();
        // `CSI <n> G` (cursor horizontal absolute) and `CSI <n> d` (vertical
        // line position absolute) are the escapes the bespoke
        // cursor-alignment path emits; drop them when they carry a numeric
        // parameter. Other CSI sequences (SGR color, SGR reset, etc.) are
        // preserved verbatim so the visible-text comparison does not lose
        // styling.
        let is_cursor_move = !params.is_empty()
            && matches!(final_byte, Some('G') | Some('d'));
        if !is_cursor_move {
            out.push('\x1b');
            out.push('[');
            out.push_str(&params);
            if let Some(byte) = final_byte {
                out.push(byte);
            }
        }
    }
    out
}

#[test]
fn prefer_cursor_alignment_visible_text_matches_tree_path() {
    // The honored subset for the bespoke cursor path is the outer-box
    // placement (margin / alignment / max_width). After stripping the
    // cursor-move escapes the bespoke path and the tree path emit the same
    // visible cell text. The cursor escapes are the irreducible core
    // (terminal-only ANSI bytes the tree cannot represent).
    let table = sample_table().prefer_cursor_alignment();
    let term = test_terminal(80);
    let bespoke = table.render_bespoke(&term);
    let tree = render_tree(&<Table as TreeRenderable>::render_tree(&table), 80);
    let bespoke_visible = strip_ansi(&strip_cursor_move_escapes(&bespoke));
    let tree_visible = strip_ansi(&tree);

    // Every visible token in the tree output also appears in the bespoke
    // path's output once cursor escapes are removed: cell text, borders,
    // and the column structure all survive.
    for token in tree_visible.split_whitespace() {
        if token.is_empty() {
            continue;
        }
        assert!(
            bespoke_visible.contains(token),
            "bespoke cursor path dropped visible token `{token}` present in \
             the tree path: bespoke_visible = {bespoke_visible:?}"
        );
    }
}

#[test]
fn prefer_cursor_alignment_row_count_matches_tree_path() {
    // The cursor path and the tree path render the same number of rows for
    // the same data. A divergence here would indicate the bespoke path had
    // silently changed the row structure the tree projects.
    let table = sample_table().prefer_cursor_alignment();
    let term = test_terminal(80);
    let bespoke = table.render_bespoke(&term);
    let tree = render_tree(&<Table as TreeRenderable>::render_tree(&table), 80);
    let bespoke_rows = strip_cursor_move_escapes(&bespoke).lines().count();
    let tree_rows = tree.lines().count();
    assert_eq!(
        bespoke_rows, tree_rows,
        "bespoke cursor path and tree path must agree on row count \
         (the cursor escapes are inter-cell only): bespoke = {bespoke:?}"
    );
}

#[test]
fn prefer_cursor_alignment_honors_margin_alignment_max_width() {
    // The honored subset for the bespoke path is the outer-box placement
    // (C5: margin / alignment / max_width). The cursor escapes set the
    // column directly — the left margin is honored via the *column number*
    // carried in `CSI N G`, not via leading spaces. A 4-cell left margin
    // means every row's cursor escape references column 5 or higher
    // (1-indexed: `table_start = left_margin + 1`).
    use biscuit_terminal::utils::layout::{Alignment, Edges, Length, TargetValue};
    let mut table = sample_table();
    table.layout_mut().margin = Edges {
        left: TargetValue::universal(Length::ch(4)),
        ..Edges::default()
    };
    table.layout_mut().alignment = Alignment::Left;
    table.layout_mut().max_width = Some(TargetValue::universal(Length::ch(60)));
    let table = table.prefer_cursor_alignment();

    let term = test_terminal(80);
    let bespoke = table.render_bespoke(&term);

    // Collect every `CSI <n> G` column referenced by the bespoke output.
    let mut cursor_columns: Vec<u32> = Vec::new();
    let bytes = bespoke.as_bytes();
    let mut i = 0;
    while i + 2 < bytes.len() {
        if bytes[i] == 0x1B && bytes[i + 1] == b'[' {
            let mut j = i + 2;
            let mut digits = String::new();
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                digits.push(bytes[j] as char);
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'G' && !digits.is_empty()
                && let Ok(col) = digits.parse::<u32>()
            {
                cursor_columns.push(col);
            }
        }
        i += 1;
    }
    assert!(
        !cursor_columns.is_empty(),
        "bespoke cursor path must emit at least one `CSI <n> G` escape: {bespoke:?}"
    );
    // Every cursor column must respect the 4-cell left margin: the smallest
    // column the cursor moves to is `left_margin + 1 = 5` (1-indexed).
    let min_col = *cursor_columns.iter().min().unwrap();
    assert!(
        min_col >= 5,
        "bespoke path honored-subset regression: cursor moved to column {min_col}, \
         expected >= 5 (left_margin 4 + 1) under a 4-ch left margin"
    );

    // `max_width` caps the outer box at 60 cells. The table width never
    // exceeds margin + max_width = 4 + 60 = 64 visible columns.
    let visible = strip_ansi(&strip_cursor_move_escapes(&bespoke));
    for line in visible.lines() {
        assert!(
            line.chars().count() <= 80,
            "bespoke path honors max_width but never exceeds terminal width: \
             {} cells > 80",
            line.chars().count()
        );
    }
}
