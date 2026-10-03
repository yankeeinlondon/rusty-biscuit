//! Verification tests for `StyledInlineProse` table cells: a table cell is
//! phrasing content, so it embeds an `InlineProse`.
//!
//! Covers every acceptance criterion from
//! `renderable/features/2026-06-08-prose-cells/spec.md` and the table half of
//! the fenced-code-in-`InlineProse` rule.

#[allow(clippy::duplicate_mod)]
#[path = "parity_helpers.rs"]
mod parity_helpers;

use biscuit_terminal::components::prose::{InlineProse, LineBreaks};
use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use biscuit_terminal::components::table::types::{ColumnType, Currency};
use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Length, TargetValue};
use renderable::markdown::MarkdownRenderable;
use renderable::tree::{
    NodeKind, RenderNode, RenderStrictness, TreeRenderable, ValidationMode, validate,
};

use parity_helpers::{assert_contains_tokens, strip_ansi, test_terminal};

fn prose_table() -> Table {
    Table::new()
        .with_columns(vec![TableColumn::new("Status"), TableColumn::new("Owner")])
        .with_data(vec![vec![
            TableCellContent::from(InlineProse::new("<dim>inactive</dim>")),
            TableCellContent::from(InlineProse::new("<b>Alice</b>")),
        ]])
}

fn render_tree_node(node: &RenderNode, width: u32) -> String {
    let term = test_terminal(width);
    let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
    render_terminal_node(node, &opts)
        .expect("tree render should succeed")
        .output
}

fn first_data_cell(node: &RenderNode) -> &RenderNode {
    let NodeKind::Table { children, .. } = &node.kind else {
        panic!("expected Table node");
    };
    let NodeKind::TableRow { children: cells } = &children[1].kind else {
        panic!("expected data row at index 1");
    };
    &cells[0]
}

// ---------------------------------------------------------------------------
// API
// ---------------------------------------------------------------------------

#[test]
fn inline_prose_into_produces_styled_inline_prose() {
    let cell: TableCellContent = InlineProse::new("hello").into();
    assert!(
        matches!(cell, TableCellContent::StyledInlineProse(_)),
        "InlineProse::into() must produce StyledInlineProse, got {cell:?}"
    );
}

#[test]
fn inline_prose_into_boxed_correctly() {
    let prose = InlineProse::new("<b>bold</b>");
    let cell: TableCellContent = prose.into();
    let TableCellContent::StyledInlineProse(boxed) = &cell else {
        panic!("expected StyledInlineProse");
    };
    assert_eq!(boxed.content(), "<b>bold</b>");
}

// ---------------------------------------------------------------------------
// Projection — semantic children
// ---------------------------------------------------------------------------

#[test]
fn bold_prose_projects_strong_child() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Col")])
        .with_data(vec![vec![TableCellContent::from(InlineProse::new(
            "<b>bold</b>",
        ))]]);
    let node = table.render_tree_node().expect("tree node");
    let cell = first_data_cell(&node);
    let NodeKind::TableCell { children } = &cell.kind else {
        panic!("expected TableCell");
    };
    assert!(
        matches!(&children[0].kind, NodeKind::Strong { .. }),
        "bold Prose must project Strong, got {:?}",
        children[0].kind
    );
}

#[test]
fn italic_prose_projects_emphasis_child() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Col")])
        .with_data(vec![vec![TableCellContent::from(InlineProse::new(
            "<i>italic</i>",
        ))]]);
    let node = table.render_tree_node().expect("tree node");
    let cell = first_data_cell(&node);
    let NodeKind::TableCell { children } = &cell.kind else {
        panic!("expected TableCell");
    };
    assert!(
        matches!(&children[0].kind, NodeKind::Emphasis { .. }),
        "italic Prose must project Emphasis, got {:?}",
        children[0].kind
    );
}

#[test]
fn strikethrough_prose_projects_delete_child() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Col")])
        .with_data(vec![vec![TableCellContent::from(InlineProse::new(
            "<~>strike</~>",
        ))]]);
    let node = table.render_tree_node().expect("tree node");
    let cell = first_data_cell(&node);
    let NodeKind::TableCell { children } = &cell.kind else {
        panic!("expected TableCell");
    };
    assert!(
        matches!(&children[0].kind, NodeKind::Delete { .. }),
        "strikethrough Prose must project Delete, got {:?}",
        children[0].kind
    );
}

#[test]
fn link_prose_projects_link_child() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Col")])
        .with_data(vec![vec![TableCellContent::from(InlineProse::new(
            r#"<a href="https://example.com">go</a>"#,
        ))]]);
    let node = table.render_tree_node().expect("tree node");
    let cell = first_data_cell(&node);
    let NodeKind::TableCell { children } = &cell.kind else {
        panic!("expected TableCell");
    };
    match &children[0].kind {
        NodeKind::Link { url, .. } => assert_eq!(url, "https://example.com"),
        other => panic!("link Prose must project Link, got {other:?}"),
    }
}

#[test]
fn colored_prose_projects_span_child() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Col")])
        .with_data(vec![vec![TableCellContent::from(InlineProse::new(
            "<red>warn</red>",
        ))]]);
    let node = table.render_tree_node().expect("tree node");
    let cell = first_data_cell(&node);
    let NodeKind::TableCell { children } = &cell.kind else {
        panic!("expected TableCell");
    };
    assert!(
        matches!(&children[0].kind, NodeKind::Span { .. }),
        "colored Prose must project Span, got {:?}",
        children[0].kind
    );
    let style = children[0].attrs.style();
    assert!(style.is_some(), "Span must carry a Style attribute");
}

#[test]
fn styled_inline_prose_does_not_project_flat_text() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Col")])
        .with_data(vec![vec![TableCellContent::from(InlineProse::new(
            "<b>bold</b> plain",
        ))]]);
    let node = table.render_tree_node().expect("tree node");
    let cell = first_data_cell(&node);
    let NodeKind::TableCell { children } = &cell.kind else {
        panic!("expected TableCell");
    };
    assert!(
        children.len() >= 2,
        "styled Prose must project multiple children, got {:?}",
        children.len()
    );
    assert!(
        matches!(&children[0].kind, NodeKind::Strong { .. }),
        "first child must be Strong"
    );
    assert!(
        matches!(&children[1].kind, NodeKind::Text { .. }),
        "second child must be Text"
    );
}

// ---------------------------------------------------------------------------
// Hint
// ---------------------------------------------------------------------------

#[test]
fn styled_inline_prose_cell_hints() {
    let table = prose_table();
    let node = table.render_tree_node().expect("tree node");
    let cell = first_data_cell(&node);
    let hints = cell
        .attrs
        .table_cell_hints()
        .expect("cell must carry hints");
    assert_eq!(hints.kind, "styled_inline_prose");
    assert!(hints.raw_value.is_null(), "raw_value must be null");
}

// ---------------------------------------------------------------------------
// Fenced code in a cell becomes inline code
// ---------------------------------------------------------------------------

#[test]
fn multiline_fence_in_a_cell_is_one_inline_code_value() {
    let prose = InlineProse::new("before\n```rust\nfn main() {}\nlet x = 1;\n```\nafter");
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Col")])
        .with_data(vec![vec![TableCellContent::from(prose)]]);
    let node = table.render_tree_node().expect("tree node");
    let report = validate(&node, ValidationMode::Full);
    assert!(
        !report.has_errors(),
        "table with a fenced cell must validate cleanly: {:?}",
        report.errors().collect::<Vec<_>>()
    );

    let cell = first_data_cell(&node);
    let NodeKind::TableCell { children } = &cell.kind else {
        panic!("expected TableCell");
    };
    let codes: Vec<&str> = children
        .iter()
        .filter_map(|c| match &c.kind {
            NodeKind::InlineCode { value } => Some(value.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(codes, ["fn main() {} let x = 1;"], "cell children: {children:?}");
    assert!(
        !children
            .iter()
            .any(|c| matches!(c.kind, NodeKind::Code { .. } | NodeKind::HardBreak)),
        "no block and no forced line break in the cell: {children:?}"
    );

    let term = test_terminal(80);
    let terminal = strip_ansi(&table.render(&term));
    assert!(
        terminal.contains("before fn main() {} let x = 1; after"),
        "terminal keeps the cell on one line: {terminal}"
    );
    let markdown = table.render_markdown();
    assert!(
        markdown.contains("| before `fn main() {} let x = 1;` after |"),
        "markdown cell holds one code span: {markdown}"
    );
    let browser = table.render_html_fragment().render();
    assert!(
        browser.contains("<code>fn main() {} let x = 1;</code>") && !browser.contains("<pre"),
        "browser cell holds inline code: {browser}"
    );
    for (target, out) in [("terminal", terminal), ("markdown", markdown), ("browser", browser)] {
        assert!(
            !out.contains("```") && !out.contains("rust"),
            "{target} must not emit the fence or its language hint: {out}"
        );
    }
}

// ---------------------------------------------------------------------------
// Layout isolation
// ---------------------------------------------------------------------------

#[test]
fn inline_prose_layout_does_not_become_cell_layout() {
    let mut prose = InlineProse::new("<b>bold</b>");
    prose.layout_mut().margin.left = TargetValue::universal(Length::ch(4));
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Col")])
        .with_data(vec![vec![TableCellContent::from(prose)]]);
    let node = table.render_tree_node().expect("tree node");
    let cell = first_data_cell(&node);
    let cell_layout = cell.attrs.layout();
    assert!(
        cell_layout.is_none(),
        "InlineProse's layout must not bleed into the cell node attrs"
    );
}

// ---------------------------------------------------------------------------
// Terminal no-color
// ---------------------------------------------------------------------------

#[test]
fn terminal_no_color_emits_no_sgr() {
    let term = Terminal::builder()
        .color_depth(ColorDepth::None)
        .width(80)
        .build();

    // Standard tree-rendering path.
    let out = table_for_no_color().render(&term);
    assert!(
        !out.contains('\u{1b}'),
        "no-color terminal must emit no escape characters at all (standard \
         path), got: {out:?}"
    );
    assert!(out.contains("inactive") && out.contains("Alice"));

    // Cursor-alignment bespoke escape hatch must honor the same policy.
    let cursor = table_for_no_color()
        .prefer_cursor_alignment()
        .render_bespoke(&term);
    assert!(
        !cursor.contains('\u{1b}'),
        "no-color terminal must emit no escape characters at all (cursor \
         path), got: {cursor:?}"
    );
    assert!(cursor.contains("inactive") && cursor.contains("Alice"));
}

/// A single-row styled table whose only styling is dim + bold — both of which a
/// `ColorDepth::None` profile must suppress, so any escape byte in the output is
/// a no-color policy violation. (Separate from [`prose_table`] so the no-color
/// assertion stays independent of the shared fixture.)
fn table_for_no_color() -> Table {
    Table::new()
        .with_columns(vec![TableColumn::new("Status"), TableColumn::new("Owner")])
        .with_data(vec![vec![
            TableCellContent::from(InlineProse::new("<dim>inactive</dim>")),
            TableCellContent::from(InlineProse::new("<b>Alice</b>")),
        ]])
}

// ---------------------------------------------------------------------------
// Terminal multiline/wrap
// ---------------------------------------------------------------------------

#[test]
fn multiline_styled_cell_does_not_bleed_into_borders() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Msg")])
        .with_data(vec![vec![TableCellContent::from(InlineProse::new(
            "<b>line one\nline two</b>",
        )
        .with_line_breaks(LineBreaks::Hard))]]);
    let term = test_terminal(80);
    let out = table.render(&term);
    let plain = strip_ansi(&out);
    assert!(plain.contains("line one"), "first line present");
    assert!(plain.contains("line two"), "second line present");
    for line in out.lines() {
        let stripped = strip_ansi(line);
        if stripped.contains("line one") || stripped.contains("line two") {
            assert!(
                stripped.starts_with('│') || stripped.starts_with('┌') || stripped.starts_with('└') || stripped.starts_with('├'),
                "border must be present: {stripped:?}"
            );
            assert!(
                stripped.ends_with('│') || stripped.ends_with('┐') || stripped.ends_with('┘') || stripped.ends_with('┤'),
                "right border must be present: {stripped:?}"
            );
        }
    }
}

#[test]
fn multiline_styled_cell_balances_sgr_per_line() {
    // Regression guard for the per-line SGR containment fix: a bold run that
    // spans an explicit newline must be reset at the end of each visual line and
    // re-opened on the next, so it cannot bleed into the cell padding, the
    // border glyph, or the following row. The ANSI-stripping sibling test cannot
    // observe this — assert on the raw escapes here.
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Msg")])
        .with_data(vec![vec![TableCellContent::from(InlineProse::new(
            "<b>line one\nline two</b>",
        )
        .with_line_breaks(LineBreaks::Hard))]]);
    let term = test_terminal(80);
    let out = table.render(&term);
    for raw_line in out.lines() {
        let plain = strip_ansi(raw_line);
        if !(plain.contains("line one") || plain.contains("line two")) {
            continue;
        }
        // Each styled content line opens the bold run AND closes it on the same
        // line (no carry across the newline into the border).
        assert!(
            raw_line.contains("\u{1b}[1m"),
            "styled content line must open its bold run: {raw_line:?}"
        );
        assert!(
            raw_line.contains("\u{1b}[0m"),
            "styled content line must reset before the border (no bleed): {raw_line:?}"
        );
        // The reset must precede the trailing border glyph: nothing styled
        // survives past the last reset to the cell edge.
        let last_reset = raw_line.rfind("\u{1b}[0m").unwrap();
        let tail = &raw_line[last_reset + "\u{1b}[0m".len()..];
        assert!(
            !tail.contains("\u{1b}[1m"),
            "no bold may be re-opened after the final reset on the line: {raw_line:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Mixed-type row
// ---------------------------------------------------------------------------

#[test]
fn mixed_type_row_retains_formatting_and_alignment() {
    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("Status"),
            TableColumn::new("Count").with_type(ColumnType::Integer),
            TableColumn::new("Rate").with_type(ColumnType::Float),
            TableColumn::new("Price").with_type(ColumnType::Currency(Currency::USD)),
        ])
        .with_data(vec![vec![
            TableCellContent::from(InlineProse::new("<b>active</b>")),
            TableCellContent::Integer(1234567),
            TableCellContent::Float(3.5),
            TableCellContent::Currency(Currency::USD, 99.99),
        ]]);
    let node = table.render_tree_node().expect("tree node");
    let plain = strip_ansi(&render_tree_node(&node, 80));

    assert!(plain.contains("active"), "prose text present");
    assert!(plain.contains("1,234,567"), "integer formatted");
    assert!(plain.contains("3.50"), "float formatted");
    assert!(plain.contains("$99.99"), "currency formatted");

    let NodeKind::Table { children, .. } = &node.kind else {
        panic!("expected Table");
    };
    let NodeKind::TableRow { children: cells } = &children[1].kind else {
        panic!("expected data row");
    };
    let status_hints = cells[0].attrs.table_cell_hints().expect("hints");
    assert_eq!(status_hints.kind, "styled_inline_prose");
    assert_eq!(status_hints.alignment, "left");

    let count_hints = cells[1].attrs.table_cell_hints().expect("hints");
    assert_eq!(count_hints.kind, "integer");
    assert_eq!(count_hints.alignment, "right");
}

// ---------------------------------------------------------------------------
// Both bespoke paths
// ---------------------------------------------------------------------------

#[test]
fn bespoke_standard_and_cursor_paths_produce_same_visible_content() {
    let table = prose_table();
    let term = test_terminal(80);

    let standard = strip_ansi(&table.render(&term));

    let cursor_table = table.clone().prefer_cursor_alignment();
    let term_tty = test_terminal(80);
    let cursor = strip_ansi(&cursor_table.render_bespoke(&term_tty));

    assert_contains_tokens(&standard, &["inactive", "Alice", "Status", "Owner"]);
    assert_contains_tokens(&cursor, &["inactive", "Alice", "Status", "Owner"]);
}

// ---------------------------------------------------------------------------
// Single resolution
// ---------------------------------------------------------------------------

#[test]
fn bespoke_resolves_prose_once() {
    // Three StyledInlineProse cells across two rows; the rest are typed/text cells
    // that must not count as Prose resolutions.
    let table = Table::new()
        .with_columns(vec![TableColumn::new("A"), TableColumn::new("B")])
        .with_data(vec![
            vec![
                TableCellContent::from(InlineProse::new("<b>one</b>")),
                TableCellContent::Integer(42),
            ],
            vec![
                TableCellContent::from(InlineProse::new("<i>two</i>")),
                TableCellContent::from(InlineProse::new("<red>three</red>")),
            ],
        ])
        .prefer_cursor_alignment();
    let term = test_terminal(80);

    // The instrumented entry point returns the resolution count from the same
    // single up-front pass the real render uses — so this measures resolutions
    // directly rather than counting words in the final output.
    let (out, resolved) = table.render_bespoke_instrumented(&term);
    assert_eq!(
        resolved, 3,
        "each of the 3 StyledInlineProse cells must be resolved exactly once before \
         planning (typed/text cells excluded)"
    );
    let plain = strip_ansi(&out);
    assert!(plain.contains("one") && plain.contains("two") && plain.contains("three"));
}

// ---------------------------------------------------------------------------
// Browser render
// ---------------------------------------------------------------------------

#[test]
fn html_preserves_semantic_emphasis_in_td() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Col")])
        .with_data(vec![vec![TableCellContent::from(InlineProse::new(
            "<b>bold</b> and <i>italic</i>",
        ))]]);
    let html = table.render_html_fragment().render();
    assert!(html.contains("<td"), "cell element: {html}");
    assert!(
        html.contains("<strong>bold</strong>"),
        "bold preserved as <strong>: {html}"
    );
    assert!(
        html.contains("<em>italic</em>"),
        "italic preserved as <em>: {html}"
    );
}

#[test]
fn html_preserves_link_in_td() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Link")])
        .with_data(vec![vec![TableCellContent::from(InlineProse::new(
            r#"<a href="https://example.com">click</a>"#,
        ))]]);
    let html = table.render_html_fragment().render();
    assert!(
        html.contains(r#"href="https://example.com""#),
        "link href preserved: {html}"
    );
    assert!(
        html.contains(">click<"),
        "link text preserved: {html}"
    );
}

// ---------------------------------------------------------------------------
// Markdown render
// ---------------------------------------------------------------------------

#[test]
fn markdown_preserves_bold_italic_strikethrough() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Col")])
        .with_data(vec![vec![TableCellContent::from(InlineProse::new(
            "<b>bold</b> <i>italic</i> <~>strike</~>",
        ))]]);
    let md = table.render_markdown();
    assert!(md.contains("**bold**"), "bold in markdown: {md}");
    assert!(md.contains("_italic_"), "italic in markdown: {md}");
    assert!(md.contains("~~strike~~"), "strikethrough in markdown: {md}");
}

#[test]
fn markdown_preserves_links() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Col")])
        .with_data(vec![vec![TableCellContent::from(InlineProse::new(
            r#"<a href="https://example.com">go</a>"#,
        ))]]);
    let md = table.render_markdown();
    assert!(
        md.contains("[go](https://example.com)"),
        "link in markdown: {md}"
    );
}

#[test]
fn markdown_pipe_in_prose_does_not_corrupt_gfm() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Col")])
        .with_data(vec![vec![TableCellContent::from(InlineProse::new(
            "left|right",
        ))]]);
    let md = table.render_markdown();
    assert!(
        md.contains("left\\|right"),
        "pipe must be escaped in Markdown cell: {md}"
    );
    for line in md.lines() {
        let trimmed = line.trim();
        if !trimmed.contains('|') {
            continue;
        }
        assert!(
            trimmed.starts_with('|') && trimmed.ends_with('|'),
            "GFM row must be pipe-delimited: {trimmed:?}"
        );
    }
}

#[test]
fn markdown_newline_in_prose_does_not_corrupt_gfm() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Col")])
        .with_data(vec![vec![TableCellContent::from(InlineProse::new(
            "line one\nline two",
        )
        .with_line_breaks(LineBreaks::Hard))]]);
    let md = table.render_markdown();
    let data_row = md
        .lines()
        .find(|l| l.contains("line one"))
        .expect("data row present");
    assert!(
        data_row.contains("line two"),
        "both lines on the same GFM row: {data_row}"
    );
}

// ---------------------------------------------------------------------------
// MarkdownPlus render
// ---------------------------------------------------------------------------

/// Shared corpus driving both the portable-Markdown and MarkdownPlus parity
/// tests: semantic emphasis, a link whose destination carries significant
/// characters (parentheses), literal Markdown sigils, and the richer
/// color/background/underline styles MarkdownPlus lowers to inline CSS.
const MARKDOWN_PARITY_CORPUS: &[&str] = &[
    "<b>bold</b>",
    "<i>italic</i>",
    "<~>strike</~>",
    "<b>bold</b> then <i>italic</i> then <~>gone</~>",
    r#"<a href="https://example.com/path_(v2)">spec</a>"#,
    "keep *these* _sigils_ literal",
    "<red>warn</red>",
    "<bg-coral>field</bg-coral>",
    "<u>under</u>",
    "<red>red</red> <bg-coral>coral</bg-coral> <u>under</u>",
];

/// Extracts the single data cell's body from a one-column GFM table: the row
/// after the header and delimiter rows, stripped of its pipe delimiters.
fn one_cell_body(md: &str) -> String {
    let mut rows = md.lines().filter(|l| l.trim_start().starts_with('|'));
    rows.next().expect("header row");
    rows.next().expect("delimiter row");
    let data = rows.next().expect("data row");
    data.trim()
        .trim_start_matches('|')
        .trim_end_matches('|')
        .trim()
        .to_string()
}

fn one_prose_cell_table(markup: &str) -> Table {
    Table::new()
        .with_columns(vec![TableColumn::new("Col")])
        .with_data(vec![vec![TableCellContent::from(InlineProse::new(markup))]])
}

#[test]
fn portable_markdown_cell_matches_standalone_inline_prose() {
    for &item in MARKDOWN_PARITY_CORPUS {
        let standalone = InlineProse::new(item).render_markdown();
        let cell = one_cell_body(&one_prose_cell_table(item).render_markdown());
        assert_eq!(
            cell, standalone,
            "portable Markdown cell body must equal standalone InlineProse for {item:?}"
        );
    }
}

#[test]
fn markdown_plus_cell_matches_standalone_inline_prose() {
    for &item in MARKDOWN_PARITY_CORPUS {
        let standalone = InlineProse::new(item).render_markdown_plus();
        let cell = one_cell_body(&one_prose_cell_table(item).render_markdown_plus());
        assert_eq!(
            cell, standalone,
            "MarkdownPlus cell body must equal standalone InlineProse for {item:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Compatibility regression — existing Table and Prose tests pass unchanged
// ---------------------------------------------------------------------------

#[test]
fn existing_text_cells_still_work() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Name"), TableColumn::new("Score")])
        .with_data(vec![vec![
            TableCellContent::Text("Ann".into()),
            TableCellContent::Integer(42),
        ]]);
    let term = test_terminal(80);
    let out = strip_ansi(&table.render(&term));
    assert!(out.contains("Ann"));
    assert!(out.contains("42"));
}

#[test]
fn existing_typed_cells_still_format_correctly() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Price")])
        .with_data(vec![vec![TableCellContent::Currency(Currency::USD, 1234.56)]]);
    let node = table.render_tree_node().expect("tree node");
    let plain = strip_ansi(&render_tree_node(&node, 80));
    assert!(plain.contains("$1,234.56"));
}

#[test]
fn existing_table_validates_cleanly() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("A"), TableColumn::new("B")])
        .with_data(vec![vec![
            TableCellContent::Text("x".into()),
            TableCellContent::Integer(1),
        ]]);
    let node = table.render_tree_node().expect("tree node");
    let report = validate(&node, ValidationMode::Full);
    assert!(
        !report.has_errors(),
        "existing table must still validate: {:?}",
        report.errors().collect::<Vec<_>>()
    );
}

#[test]
fn prose_table_validates_cleanly() {
    let table = prose_table();
    let node = table.render_tree_node().expect("tree node");
    let report = validate(&node, ValidationMode::Full);
    assert!(
        !report.has_errors(),
        "prose-cell table must validate: {:?}",
        report.errors().collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Tree vs compat hook agreement
// ---------------------------------------------------------------------------

#[test]
fn tree_renderable_and_terminal_hook_agree_with_prose_cells() {
    let table = prose_table();
    let canonical = <Table as TreeRenderable>::render_tree(&table);
    let compat = table
        .render_tree_node()
        .expect("compat hook produces a node");
    assert_eq!(
        canonical, compat,
        "TreeRenderable::render_tree and TerminalRenderable::render_tree_node must agree"
    );
}

// ---------------------------------------------------------------------------
// Tree render output matches bespoke for prose cells
// ---------------------------------------------------------------------------

#[test]
fn tree_output_contains_prose_cell_content() {
    let table = prose_table();
    let term = test_terminal(80);
    let _bespoke = strip_ansi(&table.render(&term));

    let node = table.render_tree_node().expect("tree node");
    let tree = render_tree_node(&node, 80);

    assert!(
        strip_ansi(&tree).contains("inactive"),
        "prose content must reach tree output"
    );
    assert!(
        strip_ansi(&tree).contains("Alice"),
        "prose content must reach tree output"
    );
}

// ---------------------------------------------------------------------------
// Browser tier — real headless Chrome computed-style assertions
//
// The L1 HTML tests above assert structure (semantic tags, link
// href). These prove the emitted CSS is valid and actually applied by a real
// browser to the styled run inside the `<td>`. They skip cleanly when no
// Chrome/Chromium is present (set BISCUIT_BROWSER_REQUIRED=1 to hard-fail).
// ---------------------------------------------------------------------------

#[cfg(feature = "browser-tests")]
use biscuit_browser_harness::{BrowserHarness, ChromeHarness, require_browser, wrap_fragment};

/// A one-column table whose single Prose cell carries `markup`, rendered to a
/// standalone HTML page ready for the browser harness.
#[cfg(feature = "browser-tests")]
fn browser_cell_page(markup: &str) -> String {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Col")])
        .with_data(vec![vec![TableCellContent::from(InlineProse::new(markup))]]);
    wrap_fragment(&table.render_html_fragment().render(), "#ffffff")
}

#[cfg(feature = "browser-tests")]
#[tokio::test]
#[serial_test::serial(browser)]
async fn browser_prose_cell_color_computes() {
    if !require_browser() {
        return;
    }
    let page = browser_cell_page("<red>warn</red>");
    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&page).await.expect("render html");

    // `<red>` lowers to `<td …><span style="color:rgb(128, 0, 0)">`; assert the
    // browser actually computes that color on the span inside the cell.
    let color = harness
        .computed_style("td span", "color")
        .await
        .expect("computed color");
    assert_eq!(
        color, "rgb(128, 0, 0)",
        "the colored Prose run must compute its color inside the <td>"
    );
    harness.shutdown().await;
}

#[cfg(feature = "browser-tests")]
#[tokio::test]
#[serial_test::serial(browser)]
async fn browser_prose_cell_background_computes() {
    if !require_browser() {
        return;
    }
    let page = browser_cell_page("<bg-coral>field</bg-coral>");
    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&page).await.expect("render html");

    let bg = harness
        .computed_style("td span", "background-color")
        .await
        .expect("computed background-color");
    assert_eq!(
        bg, "rgb(255, 127, 80)",
        "the background Prose run must compute its coral background inside the <td>"
    );
    harness.shutdown().await;
}

#[cfg(feature = "browser-tests")]
#[tokio::test]
#[serial_test::serial(browser)]
async fn browser_prose_cell_underline_computes() {
    if !require_browser() {
        return;
    }
    let page = browser_cell_page("<u>under</u>");
    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&page).await.expect("render html");

    let decoration = harness
        .computed_style("td span", "text-decoration")
        .await
        .expect("computed text-decoration");
    assert!(
        decoration.contains("underline"),
        "the underlined Prose run must compute an underline decoration inside \
         the <td>, got: {decoration:?}"
    );
    harness.shutdown().await;
}
