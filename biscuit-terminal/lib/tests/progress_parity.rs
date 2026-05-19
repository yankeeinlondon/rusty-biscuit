//! Parity tests for the `Progress` widget's tree projection.
//!
//! `Progress` has no dedicated `NodeKind`: it projects to a
//! `NodeKind::Paragraph` carrying [`ProgressHints`] in its `attrs`. Renderers
//! that recognize the hints draw a bar; renderers that do not fall back to the
//! paragraph's plain `"{label} {percentage}%"` text.
//!
//! These tests verify the projection shape, validity, semantic parity across
//! the terminal/Markdown/browser targets, and that custom glyphs and bar
//! widths survive.

mod parity_helpers;

use biscuit_terminal::components::progress::Progress;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use renderable::tree::{
    BrowserRenderOptions, MarkdownRenderOptions, NodeKind, RenderNode, RenderStrictness,
    ValidationMode, render_browser_node, render_markdown_node, validate,
};

use parity_helpers::{PARITY_WIDTHS, assert_contains_tokens, strip_ansi, test_terminal};

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

// ---------------------------------------------------------------------------
// Structural snapshot
// ---------------------------------------------------------------------------

#[test]
fn progress_projects_to_paragraph_with_hints() {
    let bar = Progress::new(0.75).with_label("Loading");
    let node = bar.render_tree_node().expect("progress tree node");

    assert!(
        matches!(node.kind, NodeKind::Paragraph { .. }),
        "Progress projects to a Paragraph, not a dedicated kind"
    );

    let hints = node
        .attrs
        .progress_hints()
        .expect("paragraph carries progress hints");
    assert!((hints.value - 0.75).abs() < 1e-6);
    assert_eq!(hints.bar_width, 20);
    assert_eq!(hints.fill_char, '█');
    assert_eq!(hints.empty_char, '·');
    assert_eq!(hints.left_bracket, '[');
    assert_eq!(hints.right_bracket, ']');
}

#[test]
fn projected_paragraph_text_is_label_and_percentage() {
    let labeled = Progress::new(0.75).with_label("Loading");
    let node = labeled.render_tree_node().expect("tree node");
    match &node.kind {
        NodeKind::Paragraph { children } => match &children[0].kind {
            NodeKind::Text { value } => assert_eq!(value, "Loading 75%"),
            other => panic!("expected text child, got {other:?}"),
        },
        other => panic!("expected paragraph, got {other:?}"),
    }

    let unlabeled = Progress::new(0.5);
    let node = unlabeled.render_tree_node().expect("tree node");
    match &node.kind {
        NodeKind::Paragraph { children } => match &children[0].kind {
            NodeKind::Text { value } => assert_eq!(value, "50%"),
            other => panic!("expected text child, got {other:?}"),
        },
        other => panic!("expected paragraph, got {other:?}"),
    }
}

#[test]
fn projected_percentage_is_rounded() {
    // 0.756 -> 75.6 -> rounds to 76.
    let bar = Progress::new(0.756);
    let node = bar.render_tree_node().expect("tree node");
    match &node.kind {
        NodeKind::Paragraph { children } => match &children[0].kind {
            NodeKind::Text { value } => assert_eq!(value, "76%"),
            other => panic!("expected text child, got {other:?}"),
        },
        other => panic!("expected paragraph, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Validity
// ---------------------------------------------------------------------------

#[test]
fn projected_progress_tree_validates() {
    let bar = Progress::new(0.4).with_label("Sync");
    let node = bar.render_tree_node().expect("tree node");
    let report = validate(&node, ValidationMode::Full);
    assert!(
        !report.has_errors(),
        "projected progress should validate cleanly: {:?}",
        report.errors().collect::<Vec<_>>()
    );
}

#[test]
fn projected_progress_extremes_validate() {
    for value in [0.0, 1.0, -0.5, 1.5] {
        let node = Progress::new(value).render_tree_node().expect("tree node");
        let report = validate(&node, ValidationMode::Full);
        assert!(
            !report.has_errors(),
            "progress({value}) should validate cleanly"
        );
    }
}

// ---------------------------------------------------------------------------
// Terminal semantic parity
// ---------------------------------------------------------------------------

#[test]
fn terminal_tree_render_shows_bar_label_and_percentage() {
    let bar = Progress::new(0.5).with_label("Loading");
    let node = bar.render_tree_node().expect("tree node");
    let plain = strip_ansi(&render_tree(&node, 80));

    assert!(plain.contains("Loading"), "label survives: {plain:?}");
    assert!(plain.contains("50%"), "percentage survives: {plain:?}");
    assert!(plain.contains('['), "left bracket present: {plain:?}");
    assert!(plain.contains(']'), "right bracket present: {plain:?}");
    assert!(plain.contains('█'), "fill glyph present: {plain:?}");
    assert!(plain.contains('·'), "empty glyph present: {plain:?}");
}

#[test]
fn terminal_tree_render_matches_bespoke_bar_content() {
    // The tree-rendered bar reproduces the bespoke Progress::render output.
    let bar = Progress::new(0.5).with_label("Loading");
    let bespoke = strip_ansi(&bar.render_optimistic(Some(80)));

    let node = bar.render_tree_node().expect("tree node");
    let tree = strip_ansi(&render_tree(&node, 80));

    assert_eq!(
        bespoke.trim(),
        tree.trim(),
        "tree render reproduces bespoke bar"
    );
}

#[test]
fn terminal_tree_render_full_bar_is_all_fill() {
    let node = Progress::new(1.0).render_tree_node().expect("tree node");
    let plain = strip_ansi(&render_tree(&node, 80));
    assert!(plain.contains(&"█".repeat(20)), "20 fill chars: {plain:?}");
    assert!(!plain.contains('·'), "no empty chars at 100%: {plain:?}");
    assert!(plain.contains("100%"), "shows 100%: {plain:?}");
}

#[test]
fn terminal_tree_render_empty_bar_is_all_empty() {
    let node = Progress::new(0.0).render_tree_node().expect("tree node");
    let plain = strip_ansi(&render_tree(&node, 80));
    assert!(plain.contains(&"·".repeat(20)), "20 empty chars: {plain:?}");
    assert!(!plain.contains('█'), "no fill chars at 0%: {plain:?}");
    assert!(plain.contains("0%"), "shows 0%: {plain:?}");
}

#[test]
fn terminal_tree_render_unlabeled_omits_label_segment() {
    let node = Progress::new(0.5).render_tree_node().expect("tree node");
    let plain = strip_ansi(&render_tree(&node, 80));
    assert!(
        plain.trim_start().starts_with('['),
        "starts with bar bracket: {plain:?}"
    );
    assert!(plain.contains("50%"), "percentage present: {plain:?}");
}

// ---------------------------------------------------------------------------
// Markdown / Browser fallback parity
// ---------------------------------------------------------------------------

#[test]
fn markdown_fallback_keeps_label_and_percentage() {
    let bar = Progress::new(0.75).with_label("Loading");
    let node = bar.render_tree_node().expect("tree node");
    let md = render_md(&node);
    assert!(md.contains("Loading"), "label survives in markdown: {md:?}");
    assert!(
        md.contains("75%"),
        "percentage survives in markdown: {md:?}"
    );
}

#[test]
fn markdown_fallback_unlabeled_keeps_percentage() {
    let node = Progress::new(0.3).render_tree_node().expect("tree node");
    let md = render_md(&node);
    assert!(
        md.contains("30%"),
        "percentage survives in markdown: {md:?}"
    );
}

#[test]
fn browser_renders_semantic_progress_bar() {
    // RT-PROGRESS-001: the browser renderer now emits a semantic CSS progress
    // bar (not the old `<p>` paragraph fallback) when a paragraph carries
    // `ProgressHints`.
    let bar = Progress::new(0.75).with_label("Loading");
    let node = bar.render_tree_node().expect("tree node");
    let html = render_html(&node);
    assert!(html.contains("Loading"), "label survives in HTML: {html:?}");
    assert!(
        html.contains("75%"),
        "percentage survives in HTML: {html:?}"
    );
    assert!(
        html.contains(r#"role="progressbar""#),
        "rendered as a semantic progress bar: {html:?}"
    );
    assert!(
        html.contains(r#"aria-valuenow="75""#),
        "ARIA value reflects completion: {html:?}"
    );
}

// ---------------------------------------------------------------------------
// Custom glyphs
// ---------------------------------------------------------------------------

#[test]
fn custom_fill_and_empty_glyphs_survive_tree_render() {
    let bar = Progress::new(0.5).with_fill_char('#').with_empty_char('-');
    let node = bar.render_tree_node().expect("tree node");

    let hints = node.attrs.progress_hints().expect("hints present");
    assert_eq!(hints.fill_char, '#');
    assert_eq!(hints.empty_char, '-');

    let plain = strip_ansi(&render_tree(&node, 80));
    assert!(plain.contains('#'), "custom fill glyph honored: {plain:?}");
    assert!(plain.contains('-'), "custom empty glyph honored: {plain:?}");
    assert!(
        !plain.contains('█'),
        "default fill glyph not used: {plain:?}"
    );
}

#[test]
fn custom_bracket_glyphs_survive_tree_render() {
    let bar = Progress::new(0.5).with_brackets('(', ')');
    let node = bar.render_tree_node().expect("tree node");

    let hints = node.attrs.progress_hints().expect("hints present");
    assert_eq!(hints.left_bracket, '(');
    assert_eq!(hints.right_bracket, ')');

    let plain = strip_ansi(&render_tree(&node, 80));
    assert!(
        plain.contains('('),
        "custom left bracket honored: {plain:?}"
    );
    assert!(
        plain.contains(')'),
        "custom right bracket honored: {plain:?}"
    );
}

// ---------------------------------------------------------------------------
// Layout: bar width
// ---------------------------------------------------------------------------

#[test]
fn custom_bar_width_honored_by_tree_render() {
    let bar = Progress::new(1.0).with_bar_width(10);
    let node = bar.render_tree_node().expect("tree node");

    assert_eq!(node.attrs.progress_hints().unwrap().bar_width, 10);

    let plain = strip_ansi(&render_tree(&node, 80));
    // A full bar of width 10 has exactly 10 fill chars and no empties.
    assert!(plain.contains(&"█".repeat(10)), "10 fill chars: {plain:?}");
    assert!(
        !plain.contains(&"█".repeat(11)),
        "no more than 10 fill chars: {plain:?}"
    );
}

#[test]
fn bar_width_partial_fill_split_is_correct() {
    // Half-full bar of width 10 -> 5 fill + 5 empty.
    let bar = Progress::new(0.5).with_bar_width(10);
    let node = bar.render_tree_node().expect("tree node");
    let plain = strip_ansi(&render_tree(&node, 80));
    assert!(plain.contains(&"█".repeat(5)), "5 fill chars: {plain:?}");
    assert!(plain.contains(&"·".repeat(5)), "5 empty chars: {plain:?}");
}

// ---------------------------------------------------------------------------
// Strictness: clamped / extreme values
// ---------------------------------------------------------------------------

#[test]
fn over_range_value_is_clamped_to_full() {
    // Progress::new clamps; verify the projection and render both show 100%.
    let node = Progress::new(1.5).render_tree_node().expect("tree node");
    assert!((node.attrs.progress_hints().unwrap().value - 1.0).abs() < 1e-6);
    let plain = strip_ansi(&render_tree(&node, 80));
    assert!(plain.contains("100%"), "clamped to 100%: {plain:?}");
}

#[test]
fn under_range_value_is_clamped_to_empty() {
    let node = Progress::new(-0.5).render_tree_node().expect("tree node");
    assert!(node.attrs.progress_hints().unwrap().value.abs() < 1e-6);
    let plain = strip_ansi(&render_tree(&node, 80));
    assert!(plain.contains("0%"), "clamped to 0%: {plain:?}");
}

// ---------------------------------------------------------------------------
// Width matrix
// ---------------------------------------------------------------------------

#[test]
fn progress_renders_at_all_parity_widths() {
    for &width in PARITY_WIDTHS {
        let bar = Progress::new(0.6).with_label("Task");
        let node = bar.render_tree_node().expect("tree node");
        assert_contains_tokens(&render_tree(&node, width), &["Task", "60%"]);
    }
}
