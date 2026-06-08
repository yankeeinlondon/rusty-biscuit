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
use renderable::layout::{Alignment, Edges, Layout, Length, TargetValue, Width};
use renderable::style::{Border, Opacity, PaintColor, PerMode, Style, TextEmphasis};
use renderable::tree::{
    CodeRenderHints, ColumnAlign, ColumnsHints, Document, DocumentMetadata, HeadingDepth,
    HintNamespace, ListMarkerPolicy, ListRenderHints, NodeAttrs, ProgressHints, RenderNode,
    SourceRegistry, TableCellHints, TableColumnHints, TableTerminalHints, TaskHints, TaskState,
    TextLayoutHints, TextOverflow, hint_accesses, reset_hint_accesses,
};

use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_document};

/// Builds a styled corpus exercising every first-class hint branch the terminal
/// renderer reads: layout, inherited block style, an inline styled span with a
/// nested styled span (inline style inheritance), a progress widget, a
/// two-column widget, a list (marker policy + list hints), a task item, a table
/// (column + title + terminal striping hints + a typed data cell), a code
/// block, and one opaque package-local extension hint.
///
/// Per the closeout spec's performance-corpus list (spec section 6), it also
/// carries the full CSS box vocabulary so the fold reads every typed `Layout`
/// and `Style` field: alpha foreground+background paint (degraded to opaque by
/// the terminal), a box combining `padding`, `border`, `Width::Fixed`,
/// `max_width`, and center `alignment`, a separate `Width::FitContent` block, an
/// ordered list, and a link + image carrying width-dependent `TextLayoutHints`.
/// First-class presentation rides on typed fields; only the extension hint
/// touches the bag, so a behaving fold touches `data` zero times for
/// `renderable.*`.
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

    // List with a marker policy and list hints; one task item carries an opaque
    // package-local extension hint to populate the bag.
    let mut task_item = RenderNode::list_item(
        Some(false),
        vec![RenderNode::paragraph(vec![RenderNode::text("todo")])],
    );
    task_item.attrs.set_task_hints(&TaskHints {
        state: TaskState::InProgress,
    });
    task_item
        .attrs
        .set_hint(HintNamespace("myapp.custom"), "kind", serde_json::json!("solid"));
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

    // A box exercising the whole CSS box vocabulary in one node: alpha
    // foreground + background (the terminal degrades both to opaque), a border,
    // padding, a fixed content width, a max-width cap, and center alignment.
    let alpha_fg =
        PaintColor::new(Color::Tailwind(Tailwind::Slate200)).with_opacity(Opacity::new(128));
    let alpha_bg =
        PaintColor::new(Color::Tailwind(Tailwind::Red500)).with_opacity(Opacity::new(64));
    let mut boxed = RenderNode::block_quote(vec![RenderNode::paragraph(vec![RenderNode::text(
        "boxed",
    )])]);
    boxed.attrs.set_layout(&Layout {
        margin: Edges::all(Length::ch(1)),
        padding: Edges::all(Length::ch(2)),
        width: Width::Fixed(TargetValue::universal(Length::ch(40))),
        max_width: Some(TargetValue::universal(Length::ch(60))),
        alignment: Alignment::Center,
        ..Layout::default()
    });
    boxed.attrs.set_style(&Style {
        color: Some(TargetValue::universal(PerMode::universal(alpha_fg))),
        background: Some(TargetValue::universal(PerMode::universal(alpha_bg))),
        border: Some(Border::default()),
        ..Style::default()
    });

    // A fit-content block (the orthogonal width mode to the box's Fixed) under a
    // max-width cap, so both `Width` arms are read by the fold.
    let mut fit = RenderNode::paragraph(vec![RenderNode::text("fit-content")]);
    fit.attrs.set_layout(&Layout {
        width: Width::FitContent,
        max_width: Some(TargetValue::universal(Length::ch(30))),
        ..Layout::default()
    });

    // An ordered list (the unordered list above is marker-policy driven).
    let ordered = RenderNode::list(
        true,
        Some(1),
        vec![RenderNode::list_item(
            None,
            vec![RenderNode::paragraph(vec![RenderNode::text("first")])],
        )],
    );

    // A link with an exact-width text-layout field and an image with a
    // max-width-capped placeholder, so the width-dependent `TextLayoutHints`
    // branch is read for both inline kinds.
    let mut link = RenderNode::link("https://example.com", None, vec![RenderNode::text("link")]);
    link.attrs.set_text_layout(&TextLayoutHints {
        width: Some(TargetValue::universal(Length::ch(20))),
        alignment: Alignment::Right,
        overflow: TextOverflow::Truncate,
        ..Default::default()
    });
    let mut image = RenderNode::image("img.png", None, "an image alt");
    image.attrs.set_text_layout(&TextLayoutHints {
        max_width: Some(TargetValue::universal(Length::ch(10))),
        overflow: TextOverflow::Truncate,
        ..Default::default()
    });
    let link_para = RenderNode::paragraph(vec![link, RenderNode::text(" "), image]);

    Document {
        sources: SourceRegistry::default(),
        metadata: DocumentMetadata::default(),
        root: RenderNode::root(vec![
            heading, styled_para, progress, columns, list, table, code, boxed, fit, ordered,
            link_para,
        ]),
    }
}

/// Folding the styled corpus to the terminal must not round-trip any
/// renderable-owned hint through `NodeAttrs::data`. Every first-class hint is a
/// typed field, so the renderable-owned counter stays at zero; the corpus's
/// lone opaque package-local extension hint is permitted and does not affect it.
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
