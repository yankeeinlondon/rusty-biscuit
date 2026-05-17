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

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::terminal_image::TerminalImage;
use biscuit_terminal::components::two_column::{ColumnWidth, TwoColumn};
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use renderable::tree::{
    BrowserRenderOptions, MarkdownRenderOptions, NodeKind, RenderError, RenderNode,
    RenderStrictness, ValidationMode, render_browser_node, render_markdown_node, validate,
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
