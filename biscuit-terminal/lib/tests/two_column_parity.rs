//! Parity tests for the `TwoColumn` layout's tree projection.
//!
//! `TwoColumn` has no dedicated `NodeKind`: it projects to a
//! `NodeKind::BlockQuote` container carrying [`ColumnsHints`] in its `attrs`.
//! The flat child list holds the left column's blocks followed by the right
//! column's, split at `ColumnsHints::left_count`. All three tree renderers
//! special-case the columns hint so the quote border never renders.
//!
//! A `TwoColumn` containing a `TerminalImage` cannot be projected — cursor
//! overlay rendering has no tree representation — so it projects to an
//! `Unsupported` node instead.

mod parity_helpers;

use std::io::Write;
use std::path::Path;

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use biscuit_terminal::components::terminal_image::TerminalImage;
use biscuit_terminal::components::two_column::{ColumnWidth, TwoColumn};
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::utils::layout::{Alignment, Length, TargetValue};
use renderable::markdown::MarkdownRenderable;
use renderable::tree::render::{MarkdownDialect, MarkdownRenderOptions as MdOpts};
use renderable::tree::{
    BrowserRenderOptions, ColumnWidthKind, MarkdownRenderOptions, NodeKind, RenderError,
    RenderNode, RenderStrictness, TreeRenderable, ValidationMode, render_browser_node,
    render_markdown_node, validate,
};

use parity_helpers::{strip_ansi, test_terminal};

/// Renders a render-tree node to a terminal string at the given width.
fn render_tree(node: &RenderNode, width: u32) -> String {
    let term = test_terminal(width);
    let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
    render_terminal_node(node, &opts)
        .expect("tree render should succeed")
        .output
}

/// Renders a render-tree node to a Markdown string.
fn render_md(node: &RenderNode) -> String {
    render_markdown_node(node, &MarkdownRenderOptions::default())
        .expect("markdown render should succeed")
        .output
}

/// Renders a render-tree node to an HTML string.
fn render_html(node: &RenderNode) -> String {
    render_browser_node(node, &BrowserRenderOptions::default())
        .expect("browser render should succeed")
        .output
        .render()
}

/// Builds a minimal valid PNG file and returns a `TerminalImage` for it.
///
/// The `TempDir` is returned so the file outlives the image use.
fn temp_terminal_image() -> (tempfile::TempDir, TerminalImage) {
    use image::{ImageBuffer, ImageFormat, Rgb};

    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("pixel.png");
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_fn(2, 2, |_, _| Rgb([0u8, 128u8, 255u8]));
    let mut buffer = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buffer, ImageFormat::Png).expect("encode");
    std::fs::File::create(&path)
        .expect("create file")
        .write_all(&buffer.into_inner())
        .expect("write png");

    let image = TerminalImage::new(Path::new(&path)).expect("load image");
    (dir, image)
}

// ---------------------------------------------------------------------------
// Structural snapshot
// ---------------------------------------------------------------------------

#[test]
fn two_column_projects_to_block_quote_with_columns_hints() {
    let cols = TwoColumn::new("Left content", "Right content");
    let node = cols.render_tree_node().expect("two-column tree node");

    assert!(
        matches!(node.kind, NodeKind::BlockQuote { .. }),
        "TwoColumn projects to a BlockQuote carrier, not a dedicated kind"
    );

    let hints = node
        .attrs
        .columns_hints()
        .expect("block quote carries columns hints");
    assert_eq!(hints.gap, 3);
    assert!(hints.stack_below);
}

#[test]
fn projected_children_preserve_both_column_regions() {
    let cols = TwoColumn::new("Left content", "Right content");
    let node = cols.render_tree_node().expect("tree node");

    let NodeKind::BlockQuote { children } = &node.kind else {
        panic!("expected block quote");
    };
    let hints = node.attrs.columns_hints().expect("columns hints");

    // `left_count` records where the left group ends.
    assert!(
        hints.left_count >= 1,
        "left column produced at least one node"
    );
    assert!(
        hints.left_count < children.len(),
        "right column produced at least one node"
    );

    let (left, right) = children.split_at(hints.left_count);
    assert!(!left.is_empty(), "left region preserved");
    assert!(!right.is_empty(), "right region preserved");
}

#[test]
fn fixed_left_width_survives_projection() {
    let cols = TwoColumn::new("L", "R").with_left_width(ColumnWidth::Fixed(25));
    let node = cols.render_tree_node().expect("tree node");
    let hints = node.attrs.columns_hints().expect("columns hints");
    assert_eq!(
        hints.left_width,
        renderable::tree::ColumnWidthKind::Fixed(25)
    );
}

#[test]
fn custom_gap_survives_projection() {
    let cols = TwoColumn::new("L", "R").with_gap(7);
    let node = cols.render_tree_node().expect("tree node");
    assert_eq!(node.attrs.columns_hints().expect("hints").gap, 7);
}

// ---------------------------------------------------------------------------
// Validity
// ---------------------------------------------------------------------------

#[test]
fn projected_two_column_tree_validates() {
    let cols = TwoColumn::new("Left content", "Right content");
    let node = cols.render_tree_node().expect("tree node");
    let report = validate(&node, ValidationMode::Full);
    assert!(
        !report.has_errors(),
        "projected two-column should validate cleanly: {:?}",
        report.errors().collect::<Vec<_>>()
    );
}

#[test]
fn projected_two_column_with_empty_column_validates() {
    // An empty right column projects to no nodes; the tree must still be
    // structurally valid.
    let cols = TwoColumn::new("Only left", "");
    let node = cols.render_tree_node().expect("tree node");
    let report = validate(&node, ValidationMode::Full);
    assert!(
        !report.has_errors(),
        "two-column with empty column should validate cleanly"
    );
}

// ---------------------------------------------------------------------------
// Semantic parity
// ---------------------------------------------------------------------------

#[test]
fn terminal_render_keeps_both_columns() {
    let cols = TwoColumn::new("LeftSide", "RightSide");
    let node = cols.render_tree_node().expect("tree node");
    let plain = strip_ansi(&render_tree(&node, 80));
    assert!(
        plain.contains("LeftSide"),
        "left content survives: {plain:?}"
    );
    assert!(
        plain.contains("RightSide"),
        "right content survives: {plain:?}"
    );
}

#[test]
fn markdown_render_keeps_both_columns() {
    let cols = TwoColumn::new("LeftSide", "RightSide");
    let node = cols.render_tree_node().expect("tree node");
    let md = render_md(&node);
    assert!(md.contains("LeftSide"), "left survives in markdown: {md:?}");
    assert!(
        md.contains("RightSide"),
        "right survives in markdown: {md:?}"
    );
    // The columns fallback is sequential sections, not a `>` quote.
    assert!(!md.contains('>'), "no block-quote marker emitted: {md:?}");
}

#[test]
fn browser_render_keeps_both_columns_in_flex_container() {
    let cols = TwoColumn::new("LeftSide", "RightSide");
    let node = cols.render_tree_node().expect("tree node");
    let html = render_html(&node);
    assert!(html.contains("LeftSide"), "left survives in HTML: {html:?}");
    assert!(
        html.contains("RightSide"),
        "right survives in HTML: {html:?}"
    );
    assert!(
        html.contains(r#"class="columns""#),
        "columns container class present: {html:?}"
    );
    assert!(
        html.contains(r#"class="column""#),
        "column child class present: {html:?}"
    );
    assert!(
        !html.contains("<blockquote"),
        "no blockquote element emitted: {html:?}"
    );
}

// ---------------------------------------------------------------------------
// Positional parity: side-by-side vs stacked
// ---------------------------------------------------------------------------

#[test]
fn wide_terminal_renders_columns_side_by_side() {
    let cols = TwoColumn::new("LeftWord", "RightWord").with_gap(3);
    let node = cols.render_tree_node().expect("tree node");
    let plain = strip_ansi(&render_tree(&node, 80));

    // Side-by-side: both column words appear on the same line, separated by
    // the gap.
    let on_same_line = plain
        .lines()
        .any(|line| line.contains("LeftWord") && line.contains("RightWord"));
    assert!(
        on_same_line,
        "columns are side by side at width 80: {plain:?}"
    );
}

#[test]
fn narrow_terminal_stacks_columns() {
    let cols = TwoColumn::new("LeftWord", "RightWord").with_gap(3);
    let node = cols.render_tree_node().expect("tree node");
    // A width at or below the gap forces the stacked fallback.
    let plain = strip_ansi(&render_tree(&node, 3));

    let same_line = plain
        .lines()
        .any(|line| line.contains("LeftWord") && line.contains("RightWord"));
    assert!(
        !same_line,
        "columns are stacked at a narrow width: {plain:?}"
    );
    assert!(plain.contains("LeftWord"), "left still present: {plain:?}");
    assert!(
        plain.contains("RightWord"),
        "right still present: {plain:?}"
    );
}

// ---------------------------------------------------------------------------
// Plain block quotes are unaffected
// ---------------------------------------------------------------------------

#[test]
fn plain_block_quote_still_renders_border() {
    // A block quote without columns hints keeps its bespoke border.
    let node = RenderNode::block_quote(vec![RenderNode::paragraph(vec![RenderNode::text(
        "quoted",
    )])]);
    let out = render_tree(&node, 80);
    assert!(out.contains('│'), "plain block quote keeps its border");
}

// ---------------------------------------------------------------------------
// Strictness: a TerminalImage column is unsupported
// ---------------------------------------------------------------------------

#[test]
fn two_column_with_terminal_image_projects_to_unsupported() {
    let (_dir, image) = temp_terminal_image();
    let cols = TwoColumn::new("text left", image);
    let node = cols.render_tree_node().expect("tree node");
    assert!(
        matches!(node.kind, NodeKind::Unsupported { .. }),
        "a TwoColumn holding a TerminalImage projects to Unsupported"
    );
}

#[test]
fn unsupported_two_column_image_warns_under_warn_and_errors_under_strict() {
    let (_dir, image) = temp_terminal_image();
    let cols = TwoColumn::new(image, "text right");
    let node = RenderNode::root(vec![cols.render_tree_node().expect("tree node")]);

    // Warn: a diagnostic surfaces, no error.
    let term = test_terminal(80);
    let warn = render_terminal_node(
        &node,
        &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
    )
    .expect("warn render succeeds");
    assert!(
        !warn.diagnostics.is_empty(),
        "unsupported image column surfaces a diagnostic under Warn"
    );

    // Strict: the validation gate escalates the Unsupported warning to an error.
    let strict = render_terminal_node(
        &node,
        &TerminalRenderOptions::new(&term, RenderStrictness::Strict),
    );
    assert!(
        matches!(strict, Err(RenderError::InvalidTree { .. })),
        "unsupported image column is an error under Strict"
    );
}

// ---------------------------------------------------------------------------
// Canonical trait parity: `TreeRenderable` and `TerminalRenderable::render_tree_node`
// produce the same serialized node (spec test variant 25).
// ---------------------------------------------------------------------------

#[test]
fn tree_renderable_matches_terminal_render_tree_node() {
    let cases = vec![
        TwoColumn::new("L", "R"),
        TwoColumn::new("Left line\nLeft two", "Right").with_left_percent(0.7),
        TwoColumn::new("Label", "Value").with_left_width(ColumnWidth::Fixed(25)),
        TwoColumn::new("L", "R").with_gap(7),
    ];

    for cols in cases {
        let canonical = <TwoColumn as TreeRenderable>::render_tree(&cols);
        let compat = cols.render_tree_node().expect("compat tree node");
        assert_eq!(
            serde_json::to_value(&canonical).expect("canonical serialize"),
            serde_json::to_value(&compat).expect("compat serialize"),
            "TreeRenderable and TerminalRenderable::render_tree_node must agree"
        );
    }
}

// ---------------------------------------------------------------------------
// Tree structure: `ColumnsHints` faithfully encode the component's contract
// (spec test variants 18–23, 26).
// ---------------------------------------------------------------------------

#[test]
fn percent_left_width_survives_projection() {
    let cols = TwoColumn::new("L", "R").with_left_percent(0.7);
    let node = <TwoColumn as TreeRenderable>::render_tree(&cols);
    let hints = node.attrs.columns_hints().expect("columns hints");
    match hints.left_width {
        ColumnWidthKind::Percent(p) => assert!((p - 0.7).abs() < 1e-6, "percent preserved: {p}"),
        other => panic!("expected Percent, got {other:?}"),
    }
}

#[test]
fn layout_recorded_on_node_attrs() {
    let mut cols = TwoColumn::new("L", "R");
    cols.layout_mut().margin = biscuit_terminal::utils::layout::Margin::x(Length::ch(2));
    let node = <TwoColumn as TreeRenderable>::render_tree(&cols);
    assert!(
        node.attrs.layout().is_some(),
        "non-default Layout is recorded on the node"
    );
}

#[test]
fn default_layout_is_not_recorded() {
    let cols = TwoColumn::new("L", "R");
    let node = <TwoColumn as TreeRenderable>::render_tree(&cols);
    assert!(
        node.attrs.layout().is_none(),
        "default Layout is omitted from node attrs"
    );
}

#[test]
fn columns_hints_round_trip_validates_cleanly() {
    let cols = TwoColumn::new("L", "R")
        .with_left_width(ColumnWidth::Fixed(40))
        .with_gap(5);
    let node = <TwoColumn as TreeRenderable>::render_tree(&cols);
    let report = validate(&node, ValidationMode::Full);
    assert!(
        !report.has_errors(),
        "columns hints on the BlockQuote carrier validate cleanly: {:?}",
        report.errors().collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Bespoke vs tree parity at the public `render()` surface (spec variants 1–12,
// 15–17). The flip routes `TwoColumn::render` through the tree; the bespoke
// path remains on `render_bespoke` and image-overlay scenarios fall back to it.
// ---------------------------------------------------------------------------

#[test]
fn render_via_tree_matches_render_bespoke_default_50_50() {
    let cols = TwoColumn::new("Left", "Right");
    let term = test_terminal(80);
    let bespoke = strip_ansi(&cols.render_bespoke(&term));
    let tree = strip_ansi(&cols.render(&term));
    assert_eq!(
        bespoke.trim_end(),
        tree.trim_end(),
        "bespoke and tree default 50/50 columns agree (ANSI stripped)"
    );
}

#[test]
fn render_via_tree_matches_render_bespoke_custom_ratio() {
    let cols = TwoColumn::new("Left line\nLeft two", "Right").with_left_percent(0.7);
    let term = test_terminal(80);
    let bespoke = strip_ansi(&cols.render_bespoke(&term));
    let tree = strip_ansi(&cols.render(&term));
    // Tokens align in both paths; row alignment is asserted by `assert_eq!`
    // on the stripped output trimmed of trailing whitespace.
    assert_eq!(bespoke.trim_end(), tree.trim_end());
}

#[test]
fn render_via_tree_matches_render_bespoke_fixed_left_width() {
    let cols = TwoColumn::new("Label", "Value content").with_left_width(ColumnWidth::Fixed(20));
    let term = test_terminal(80);
    let bespoke = strip_ansi(&cols.render_bespoke(&term));
    let tree = strip_ansi(&cols.render(&term));
    assert_eq!(bespoke.trim_end(), tree.trim_end());
}

#[test]
fn render_via_tree_matches_render_bespoke_custom_gap() {
    let cols = TwoColumn::new("L", "R").with_gap(8);
    let term = test_terminal(80);
    let bespoke = strip_ansi(&cols.render_bespoke(&term));
    let tree = strip_ansi(&cols.render(&term));
    assert_eq!(bespoke.trim_end(), tree.trim_end());
}

#[test]
fn render_via_tree_stacks_at_narrow_widths_like_bespoke() {
    // A width at or below the gap forces vertical stacking. The tokens "L"
    // and "R" fit on a single character cell each, so both paths emit the
    // identical stacked output without hard-breaking.
    let cols = TwoColumn::new("L", "R").with_gap(3);
    let term = test_terminal(3);
    let bespoke = strip_ansi(&cols.render_bespoke(&term));
    let tree = strip_ansi(&cols.render(&term));
    assert_eq!(bespoke.trim_end(), tree.trim_end());
    let any_same_line = tree
        .lines()
        .any(|line| line.contains('L') && line.contains('R'));
    assert!(!any_same_line, "narrow output stacks: {tree:?}");
}

#[test]
fn render_via_tree_honors_alignment_center() {
    // `Alignment::Center` is applied as block-level padding on the layout's
    // remaining width. A bare two-column block expands to fill the available
    // width, so there is nothing to center; the contract is therefore that
    // the tree path agrees with the bespoke path on the visible output.
    let cols = TwoColumn::new("L", "R").alignment(Alignment::Center);
    let term = test_terminal(80);
    let bespoke = strip_ansi(&cols.render_bespoke(&term));
    let tree = strip_ansi(&cols.render(&term));
    assert_eq!(
        bespoke.trim_end(),
        tree.trim_end(),
        "centered alignment on a TwoColumn agrees between bespoke and tree paths"
    );
}

#[test]
fn render_via_tree_honors_left_margin() {
    let cols = TwoColumn::new("L", "R").left_margin(TargetValue::universal(Length::ch(4)));
    let term = test_terminal(80);
    let tree = strip_ansi(&cols.render(&term));
    let first_line = tree.lines().next().unwrap_or_default();
    assert!(
        first_line.starts_with("    "),
        "left margin pads the column block start: {first_line:?}"
    );
}

#[test]
fn render_via_tree_honors_right_margin() {
    // A right margin of 4 cells must constrain the column block to
    // `width - 4` visible cells. Use unique, space-free tokens so columns
    // do not collapse, and assert every emitted line is short enough.
    let cols = TwoColumn::new("LeftWord", "RightWord")
        .right_margin(TargetValue::universal(Length::ch(4)));
    let term = test_terminal(80);
    let bespoke = strip_ansi(&cols.render_bespoke(&term));
    let tree = strip_ansi(&cols.render(&term));

    // First contract: parity with the bespoke path (trimmed trailing spaces).
    assert_eq!(
        bespoke.trim_end(),
        tree.trim_end(),
        "bespoke and tree paths agree under a right margin"
    );

    // Second contract: no rendered line exceeds `width - right_margin`.
    let max = 80usize - 4;
    for line in tree.lines() {
        assert!(
            line.chars().count() <= max,
            "right margin constrains output to {max} cells; got line of {} cells: {line:?}",
            line.chars().count()
        );
    }
}

#[test]
fn render_via_tree_preserves_nested_block_content() {
    // Spec test variant 12 (relaxed): a `BlockQuote` inside a column projects
    // through `RenderableTerminalContent::to_tree_nodes`. Because non-`Prose`
    // components don't override `TerminalRenderable::render_tree_node`, the
    // current accepted contract is that nested components flatten to
    // ANSI-stripped text wrapped in a paragraph — the text survives, but the
    // structural block kind does not.
    //
    // TODO(stage-3): Teach `project_renderable_content` (or each block
    // component) to project nested `BlockQuote`/`Section` components through
    // their `TreeRenderable` impls so they appear as structural block-level
    // children of the columns carrier, rather than ANSI-stripped paragraphs.
    // See `renderable/features/2026-05-19-pushing-toward-ir/lessons-learned.md`
    // ("Nested non-`Prose` block components flatten to text") for the
    // discussion and future plan.
    let nested = BlockQuote::new("quoted inside".into(), None::<&str>);
    let cols = TwoColumn::new(nested, "Plain right");
    let node = <TwoColumn as TreeRenderable>::render_tree(&cols);

    let renderable::tree::NodeKind::BlockQuote { children } = &node.kind else {
        panic!("expected BlockQuote carrier, got {:?}", node.kind);
    };
    let hints = node.attrs.columns_hints().expect("columns hints");
    let (left, _right) = children.split_at(hints.left_count);

    // Current contract: the nested quote's rendered text reaches the left
    // region (even if flattened to a paragraph). Search the left subtree's
    // text content for the quoted phrase.
    fn collect_text(node: &renderable::tree::RenderNode, sink: &mut String) {
        use renderable::tree::NodeKind;
        match &node.kind {
            NodeKind::Text { value } => sink.push_str(value),
            NodeKind::Paragraph { children }
            | NodeKind::BlockQuote { children }
            | NodeKind::Section { children, .. }
            | NodeKind::Emphasis { children }
            | NodeKind::Strong { children }
            | NodeKind::Span { children, .. } => {
                for child in children {
                    collect_text(child, sink);
                }
            }
            _ => {}
        }
    }

    let mut left_text = String::new();
    for child in left {
        collect_text(child, &mut left_text);
    }
    assert!(
        left_text.contains("quoted inside"),
        "nested BlockQuote text reaches the left region of the columns carrier: \
         left_text={left_text:?}, left={left:?}",
    );

    // And the rendered terminal output retains the quoted content end-to-end.
    let term = test_terminal(80);
    let plain = strip_ansi(&cols.render(&term));
    assert!(
        plain.contains("quoted inside"),
        "nested BlockQuote content survives rendering: {plain:?}"
    );
}

#[test]
fn render_via_tree_with_empty_left_is_graceful() {
    let cols = TwoColumn::new("", "Only right");
    let term = test_terminal(40);
    let tree = strip_ansi(&cols.render(&term));
    assert!(
        tree.contains("Only right"),
        "right content survives: {tree:?}"
    );
}

#[test]
fn render_via_tree_with_empty_right_is_graceful() {
    let cols = TwoColumn::new("Only left", "");
    let term = test_terminal(40);
    let tree = strip_ansi(&cols.render(&term));
    assert!(
        tree.contains("Only left"),
        "left content survives: {tree:?}"
    );
}

#[test]
fn render_via_tree_preserves_unicode_content() {
    let cols = TwoColumn::new("héllo", "wörld");
    let term = test_terminal(40);
    let tree = strip_ansi(&cols.render(&term));
    assert!(tree.contains("héllo"), "unicode left survives: {tree:?}");
    assert!(tree.contains("wörld"), "unicode right survives: {tree:?}");
}

// ---------------------------------------------------------------------------
// Prose content survives projection — Prose is a `Component` content; the
// projector must walk into it so styled text reaches the tree.
// ---------------------------------------------------------------------------

#[test]
fn prose_content_in_columns_survives_tree_rendering() {
    let left = Prose::new("**bold** left");
    let right = Prose::new("plain right");
    let cols = TwoColumn::new(left, right);
    let term = test_terminal(80);
    let tree = strip_ansi(&cols.render(&term));
    assert!(
        tree.contains("bold"),
        "prose left content survives: {tree:?}"
    );
    assert!(
        tree.contains("plain right"),
        "prose right survives: {tree:?}"
    );
}

// ---------------------------------------------------------------------------
// Image-column fallback: the public `render()` path must use the bespoke
// renderer (not emit the unsupported placeholder) when a column holds an
// image. The bespoke path produces a non-empty rendering regardless of
// whether the image protocol is available, because the bespoke renderer
// falls back to alt text.
// ---------------------------------------------------------------------------

#[test]
fn render_with_image_column_uses_bespoke_path_not_unsupported_placeholder() {
    let (_dir, image) = temp_terminal_image();
    let cols = TwoColumn::new("text left", image);
    let term = test_terminal(80);
    let out = cols.render(&term);
    // The unsupported diagnostic placeholder text should not be in-band.
    assert!(
        !strip_ansi(&out).contains("[Unsupported"),
        "image overlay must not surface the unsupported placeholder in-band: {out:?}",
    );
    assert!(
        strip_ansi(&out).contains("text left"),
        "the surviving column content is still rendered through the bespoke path: {out:?}",
    );
}

#[test]
fn render_with_image_column_in_right_uses_bespoke_path() {
    let (_dir, image) = temp_terminal_image();
    let cols = TwoColumn::new(image, "text right");
    let term = test_terminal(80);
    let out = cols.render(&term);
    assert!(
        !strip_ansi(&out).contains("[Unsupported"),
        "image overlay must not surface the unsupported placeholder in-band: {out:?}",
    );
    assert!(
        strip_ansi(&out).contains("text right"),
        "the surviving column content is still rendered through the bespoke path: {out:?}",
    );
}

// ---------------------------------------------------------------------------
// Cross-target trait coverage (Browser + Markdown + MarkdownPlus).
// ---------------------------------------------------------------------------

#[test]
fn browser_renderable_emits_columns_flex_container() {
    let cols = TwoColumn::new("LeftSide", "RightSide");
    let html = cols.render_html_fragment().render();
    assert!(
        html.contains(r#"class="columns""#),
        "columns class: {html:?}"
    );
    assert!(html.contains("display:flex"), "flex display CSS: {html:?}");
    assert!(html.contains("LeftSide"), "left content: {html:?}");
    assert!(html.contains("RightSide"), "right content: {html:?}");
}

#[test]
fn browser_renderable_lowers_fixed_left_width_to_ch_css() {
    let cols = TwoColumn::new("L", "R").with_left_width(ColumnWidth::Fixed(30));
    let html = cols.render_html_fragment().render();
    assert!(
        html.contains("flex: 0 0 30ch") || html.contains("flex:0 0 30ch"),
        "fixed left width lowers to ch CSS: {html:?}"
    );
}

#[test]
fn browser_renderable_lowers_percent_left_width_to_percent_css() {
    let cols = TwoColumn::new("L", "R").with_left_percent(0.7);
    let html = cols.render_html_fragment().render();
    // 0.7 → 70%
    assert!(
        html.contains("70%"),
        "percent left width lowers to % CSS: {html:?}"
    );
}

#[test]
fn browser_renderable_lowers_custom_gap_to_ch_css() {
    let cols = TwoColumn::new("L", "R").with_gap(6);
    let html = cols.render_html_fragment().render();
    assert!(
        html.contains("gap:6ch") || html.contains("gap: 6ch"),
        "gap lowers to ch CSS: {html:?}"
    );
}

#[test]
fn markdown_renderable_collapses_columns_to_sequential_blocks() {
    let cols = TwoColumn::new("LeftBody", "RightBody");
    let md = cols.render_markdown();
    // Sequential: left text, blank line, right text.
    assert!(md.contains("LeftBody"), "left survives: {md:?}");
    assert!(md.contains("RightBody"), "right survives: {md:?}");
    // No raw HTML columns container in portable Markdown.
    assert!(
        !md.contains(r#"class="columns""#),
        "portable Markdown has no columns HTML container: {md:?}"
    );
    let left_pos = md.find("LeftBody").unwrap();
    let right_pos = md.find("RightBody").unwrap();
    assert!(left_pos < right_pos, "left precedes right");
}

#[test]
fn markdown_plus_renderable_emits_flex_html_container() {
    let cols = TwoColumn::new("LeftBody", "RightBody");
    let md = cols.render_markdown_plus();
    assert!(
        md.contains(r#"class="columns""#),
        "MarkdownPlus has flex container: {md:?}"
    );
    assert!(
        md.contains(r#"class="column""#),
        "MarkdownPlus has column children: {md:?}"
    );
    assert!(
        md.contains("display:flex"),
        "MarkdownPlus container has flex CSS: {md:?}"
    );
    assert!(md.contains("LeftBody"), "left survives in MarkdownPlus");
    assert!(md.contains("RightBody"), "right survives in MarkdownPlus");
}

#[test]
fn markdown_plus_renderable_lowers_fixed_left_width() {
    let cols = TwoColumn::new("L", "R").with_left_width(ColumnWidth::Fixed(30));
    let md = cols.render_markdown_plus();
    assert!(
        md.contains("flex: 0 0 30ch") || md.contains("flex:0 0 30ch"),
        "fixed width in MarkdownPlus: {md:?}"
    );
}

#[test]
fn markdown_plus_renderable_lowers_percent_left_width() {
    let cols = TwoColumn::new("L", "R").with_left_percent(0.7);
    let md = cols.render_markdown_plus();
    assert!(md.contains("70%"), "percent width in MarkdownPlus: {md:?}");
}

#[test]
fn markdown_plus_renderable_lowers_custom_gap() {
    let cols = TwoColumn::new("L", "R").with_gap(6);
    let md = cols.render_markdown_plus();
    assert!(
        md.contains("gap:6ch") || md.contains("gap: 6ch"),
        "gap in MarkdownPlus: {md:?}"
    );
}

#[test]
fn markdown_plus_renderable_renders_empty_columns_gracefully() {
    let cols = TwoColumn::new("", "");
    let md = cols.render_markdown_plus();
    // The container is still emitted with empty column children.
    assert!(
        md.contains(r#"class="columns""#),
        "container survives empty content: {md:?}"
    );
}

#[test]
fn markdown_handles_empty_columns() {
    let only_right = TwoColumn::new("", "Right only");
    let md = only_right.render_markdown();
    assert!(md.contains("Right only"));
    let only_left = TwoColumn::new("Left only", "");
    let md = only_left.render_markdown();
    assert!(md.contains("Left only"));
}

// ---------------------------------------------------------------------------
// Image columns under Warn and Strict for Browser and MarkdownPlus targets.
// ---------------------------------------------------------------------------

#[test]
fn browser_unsupported_image_column_under_warn_does_not_panic() {
    let (_dir, image) = temp_terminal_image();
    let cols = TwoColumn::new(image, "text right");
    // `BrowserRenderable::render_html_fragment` is infallible: failures log and
    // fall back to an empty fragment instead of panicking.
    let html = cols.render_html_fragment().render();
    // Either an unsupported diagnostic shape or empty output is acceptable;
    // the contract is "no panic, no in-band exception text".
    assert!(
        !html.contains("panic"),
        "browser fragment must not surface panic text: {html:?}",
    );
}

#[test]
fn markdown_plus_unsupported_image_column_does_not_panic() {
    let (_dir, image) = temp_terminal_image();
    let cols = TwoColumn::new("text left", image);
    let md = cols.render_markdown_plus();
    assert!(
        !md.contains("panic"),
        "MarkdownPlus must not surface panic text for an image column: {md:?}",
    );
}

#[test]
fn markdown_plus_unsupported_image_column_under_strict_yields_unsupported_diagnostic() {
    let (_dir, image) = temp_terminal_image();
    let cols = TwoColumn::new(image, "text right");
    let node = RenderNode::root(vec![<TwoColumn as TreeRenderable>::render_tree(&cols)]);
    let opts = MdOpts {
        dialect: MarkdownDialect::MarkdownPlus,
        strictness: RenderStrictness::Strict,
        ..MdOpts::default()
    };
    let strict = render_markdown_node(&node, &opts);
    assert!(
        matches!(strict, Err(RenderError::InvalidTree { .. })),
        "unsupported image column is an error under Strict in MarkdownPlus too: {strict:?}",
    );
}
