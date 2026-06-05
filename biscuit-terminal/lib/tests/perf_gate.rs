//! Structural performance gate for the terminal tree fold (Spec B AC5).
//!
//! The render tree carries layout/style/component hints as typed sparse
//! [`NodeAttrs`](renderable::tree::NodeAttrs) fields, never as JSON in the
//! `data` bag. This test folds a representative styled corpus through the
//! terminal renderer and asserts the fold performs **zero** renderable-owned
//! `data`-bag round-trips — every first-class hint is read from a typed field.
//!
//! The counter is renderable's `hint-access-counter` instrumentation, enabled
//! for this crate's test builds via a `[dev-dependencies]` feature. Folding
//! Markdown and both browser folds is gated inside `renderable` itself; this is
//! the terminal half, which lives here because `biscuit-terminal` owns the
//! terminal renderer.

use renderable::color::{Color, Tailwind};
use renderable::layout::{Layout, TargetValue};
use renderable::style::{PerMode, Style, TextEmphasis};
use renderable::tree::{
    CodeRenderHints, ColumnAlign, ColumnsHints, Document, DocumentMetadata, HeadingDepth,
    HintNamespace, ListMarkerPolicy, ListRenderHints, NodeAttrs, ProgressHints, RenderNode,
    SourceRegistry, TableCellHints, TableColumnHints, TableTerminalHints, TaskHints, TaskState,
    hint_accesses, reset_hint_accesses,
};

use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_document};

/// Builds a styled corpus exercising every first-class hint branch the terminal
/// renderer reads: layout, inherited block style, an inline styled span with a
/// nested styled span (inline style inheritance), a progress widget, a
/// two-column widget, a list (marker policy + list hints), a task item, a table
/// (column + title + terminal striping hints + a typed data cell), a code
/// block, and one `darkmatter.hr` extension hint. First-class presentation
/// rides on typed fields; only the extension hint touches the bag, so a
/// behaving fold touches `data` zero times for `renderable.*`.
fn styled_corpus_document() -> Document {
    let red = Style {
        color: Some(TargetValue::universal(PerMode::universal(Color::Tailwind(
            Tailwind::Red500,
        )))),
        emphasis: TextEmphasis {
            bold: true,
            ..Default::default()
        },
        ..Style::default()
    };
    let italic = Style {
        emphasis: TextEmphasis {
            italic: true,
            ..Default::default()
        },
        ..Style::default()
    };

    // Heading carrying layout + an inheritable style.
    let mut heading =
        RenderNode::heading(HeadingDepth::new(2).unwrap(), vec![RenderNode::text("Title")]);
    heading.attrs.set_layout(&Layout::default());
    heading.attrs.set_style(&red);

    // Paragraph carrying an inline styled span that nests a second styled span,
    // so the fold walks inline style inheritance.
    let mut inner_span = RenderNode::span(vec![], vec![RenderNode::text("inner")]);
    inner_span.attrs.set_style(&italic);
    let mut outer_span = RenderNode::span(vec![], vec![RenderNode::text("outer "), inner_span]);
    outer_span.attrs.set_style(&red);
    let styled_para = RenderNode::paragraph(vec![outer_span]);

    // Progress widget: a paragraph carrying ProgressHints.
    let mut progress = RenderNode::paragraph(vec![RenderNode::text("75%")]);
    progress.attrs.set_progress_hints(&ProgressHints {
        value: 0.75,
        ..Default::default()
    });

    // Two-column widget: a block quote carrying ColumnsHints; the first child is
    // the left column, the rest the right column.
    let mut columns = RenderNode::block_quote(vec![
        RenderNode::paragraph(vec![RenderNode::text("left")]),
        RenderNode::paragraph(vec![RenderNode::text("right")]),
    ]);
    columns.attrs.set_columns_hints(&ColumnsHints {
        left_count: 1,
        ..Default::default()
    });

    // List with a marker policy and list hints; one task item carries a
    // `darkmatter.hr` extension hint to populate the bag.
    let mut task_item = RenderNode::list_item(
        Some(false),
        vec![RenderNode::paragraph(vec![RenderNode::text("todo")])],
    );
    task_item.attrs.set_task_hints(&TaskHints {
        state: TaskState::InProgress,
    });
    task_item
        .attrs
        .set_hint(HintNamespace("darkmatter.hr"), "kind", serde_json::json!("solid"));
    let mut list = RenderNode::list(false, None, vec![task_item]);
    list.attrs.set_list_marker_policy(ListMarkerPolicy::None);
    list.attrs.set_list_hints(&ListRenderHints {
        bullet: Some("* ".into()),
        ..Default::default()
    });

    // Table with a column hint, a title, terminal striping hints, and a data
    // row whose typed cell carries cell hints.
    let header_row =
        RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text("c0")])]);
    let mut data_cell = RenderNode::table_cell(vec![RenderNode::text("42")]);
    data_cell.attrs.set_table_cell_hints(&TableCellHints {
        kind: "integer".into(),
        raw_value: serde_json::json!(42),
        alignment: "right".into(),
        vertical_alignment: "top".into(),
    });
    let data_row = RenderNode::table_row(vec![data_cell]);
    let mut table = RenderNode::table(vec![ColumnAlign::Left], vec![header_row, data_row]);
    table.attrs.set_table_column_hints(
        0,
        &TableColumnHints {
            min_width: Some(4),
            ..Default::default()
        },
    );
    table.attrs.set_table_terminal_hints(&TableTerminalHints {
        alternate_background: true,
        ..Default::default()
    });
    table.attrs.set_table_title("Totals");

    // Code block with hints.
    let mut code = RenderNode::code(Some("rust".into()), None, "let x = 1;");
    code.attrs.set_code_hints(&CodeRenderHints {
        header_row: true,
        language_label: Some("rust".into()),
        highlight: true,
    });

    Document {
        sources: SourceRegistry::default(),
        metadata: DocumentMetadata::default(),
        root: RenderNode::root(vec![
            heading, styled_para, progress, columns, list, table, code,
        ]),
    }
}

/// Folding the styled corpus to the terminal must not round-trip any
/// renderable-owned hint through `NodeAttrs::data`. Every first-class hint is a
/// typed field, so the renderable-owned counter stays at zero; the corpus's
/// lone `darkmatter.hr` extension hint is permitted and does not affect it.
#[test]
fn terminal_fold_does_zero_renderable_owned_hint_roundtrips() {
    let doc = styled_corpus_document();

    reset_hint_accesses();
    let _ = render_terminal_document(&doc, &TerminalRenderOptions::default())
        .expect("terminal fold");
    let (renderable_owned, _extension) = hint_accesses();

    assert_eq!(
        renderable_owned, 0,
        "the terminal fold must not round-trip any renderable-owned hint through `data`",
    );
}

/// Guards the gate above against vacuity: the counter must actually be live in
/// this build (the `hint-access-counter` feature on, `record_hint_access`
/// compiled in). If a future change disabled the instrumentation, the gate
/// would pass for the wrong reason — this asserts the bag-access counter
/// genuinely increments for a `renderable.*` namespace.
#[test]
fn hint_access_counter_is_live_in_this_build() {
    let mut attrs = NodeAttrs::default();
    reset_hint_accesses();
    attrs.set_hint(HintNamespace::LAYOUT, "margin_top", serde_json::json!(2));
    let (renderable_owned, _extension) = hint_accesses();
    assert_eq!(
        renderable_owned, 1,
        "the hint-access counter must be active in the perf-gate build",
    );
}
