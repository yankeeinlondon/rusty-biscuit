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
fn terminal_tree_render_matches_render_optimistic() {
    // `Progress::render_optimistic` and the direct tree render of the
    // projected node must agree (both route through the canonical tree path).
    let bar = Progress::new(0.5).with_label("Loading");
    let via_optimistic = strip_ansi(&bar.render_optimistic(Some(80)));

    let node = bar.render_tree_node().expect("tree node");
    let direct = strip_ansi(&render_tree(&node, 80));

    assert_eq!(
        via_optimistic.trim(),
        direct.trim(),
        "render_optimistic and direct tree render agree"
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

// ---------------------------------------------------------------------------
// TreeRenderable canonical adoption
// ---------------------------------------------------------------------------

#[test]
fn tree_renderable_and_render_tree_node_share_one_projection() {
    // The migration pattern: the canonical `TreeRenderable::render_tree`
    // entry point and the terminal compatibility
    // `TerminalRenderable::render_tree_node` hook MUST share one private
    // projection helper so they cannot drift.
    use renderable::tree::TreeRenderable;
    let bar = Progress::new(0.5).with_label("Sync");
    let canonical = <Progress as TreeRenderable>::render_tree(&bar);
    let compat = bar.render_tree_node().expect("tree node");
    assert_eq!(
        serde_json::to_value(&canonical).unwrap(),
        serde_json::to_value(&compat).unwrap(),
        "canonical and compatibility projections must serialize identically"
    );
}

// ---------------------------------------------------------------------------
// Layout flows through the tree path
// ---------------------------------------------------------------------------

#[test]
fn left_margin_is_honored_through_tree_path() {
    use biscuit_terminal::utils::layout::{Length, TargetValue};
    let bar = Progress::new(0.5).left_margin(TargetValue::universal(Length::ch(4)));
    let term = test_terminal(80);
    let out = strip_ansi(&bar.render(&term));
    assert!(
        out.starts_with("    "),
        "left margin of 4 spaces applied through tree: {out:?}"
    );
}

#[test]
fn percentage_alignment_preserved_through_tree_path() {
    let term = test_terminal(80);
    // 0% -> "  0%" (two leading spaces)
    assert!(strip_ansi(&Progress::new(0.0).render(&term)).contains("  0%"));
    // 75% -> " 75%" (one leading space)
    assert!(strip_ansi(&Progress::new(0.75).render(&term)).contains(" 75%"));
    // 100% -> "100%" (no leading space)
    assert!(strip_ansi(&Progress::new(1.0).render(&term)).contains("100%"));
}

// ---------------------------------------------------------------------------
// Markdown (portable): label + percentage only
// ---------------------------------------------------------------------------

#[test]
fn markdown_renderable_unlabeled_emits_percentage_text() {
    use renderable::markdown::MarkdownRenderable;
    let md = Progress::new(0.5).render_markdown();
    assert_eq!(md.trim(), "50%");
}

#[test]
fn markdown_renderable_labeled_emits_label_and_percentage() {
    use renderable::markdown::MarkdownRenderable;
    let md = Progress::new(0.5).with_label("Loading").render_markdown();
    assert_eq!(md.trim(), "Loading 50%");
}

#[test]
fn markdown_renderable_drops_colors_glyphs_layout() {
    use biscuit_terminal::utils::layout::{Length, TargetValue};
    use renderable::color::{BasicColor, Color};
    use renderable::markdown::MarkdownRenderable;
    let bar = Progress::new(0.5)
        .with_label("Sync")
        .with_fill_char('#')
        .with_empty_char('-')
        .with_brackets('(', ')')
        .with_filled_color(Color::BasicColor(BasicColor::Green))
        .left_margin(TargetValue::universal(Length::ch(4)));
    let md = bar.render_markdown();
    let trimmed = md.trim();
    assert_eq!(
        trimmed, "Sync 50%",
        "portable Markdown drops glyphs/colors/layout, keeps label+percentage"
    );
}

// ---------------------------------------------------------------------------
// MarkdownPlus: inline progress HTML
// ---------------------------------------------------------------------------

#[test]
fn markdown_plus_emits_progress_html_shape() {
    use renderable::markdown::MarkdownRenderable;
    let md = Progress::new(0.75)
        .with_label("Loading")
        .render_markdown_plus();
    assert!(
        md.contains(r#"role="progressbar""#),
        "MarkdownPlus carries semantic progress widget: {md:?}"
    );
    assert!(
        md.contains(r#"aria-valuenow="75""#),
        "ARIA value reflects completion: {md:?}"
    );
    assert!(md.contains("Loading"), "label survives: {md:?}");
    assert!(md.contains("75%"), "percentage survives: {md:?}");
}

#[test]
fn markdown_plus_unlabeled_omits_label_span() {
    use renderable::markdown::MarkdownRenderable;
    let md = Progress::new(0.5).render_markdown_plus();
    assert!(
        md.contains(r#"role="progressbar""#),
        "MarkdownPlus carries semantic widget: {md:?}"
    );
    // Markdown-plus without a label should NOT carry a `progress-label` span.
    assert!(
        !md.contains("progress-label"),
        "unlabeled progress emits no label span: {md:?}"
    );
}

#[test]
fn markdown_falls_through_for_plain_paragraph() {
    // A plain Paragraph (no ProgressHints) renders as ordinary Markdown.
    use renderable::tree::render::MarkdownRenderOptions;
    use renderable::tree::{RenderNode, render_markdown_node};
    let para = RenderNode::paragraph(vec![RenderNode::text("Plain text.")]);
    let out = render_markdown_node(&para, &MarkdownRenderOptions::default()).expect("markdown ok");
    assert_eq!(out.output.trim(), "Plain text.");
}

// ---------------------------------------------------------------------------
// Browser
// ---------------------------------------------------------------------------

#[test]
fn browser_renderable_emits_semantic_progress() {
    use biscuit_terminal::components::renderable::BrowserRenderable;
    let bar = Progress::new(0.5).with_label("Sync");
    let html = bar.render_html_fragment().render();
    assert!(
        html.contains(r#"role="progressbar""#),
        "BrowserRenderable emits semantic widget: {html:?}"
    );
    assert!(
        html.contains(r#"aria-valuenow="50""#),
        "ARIA value reflects completion: {html:?}"
    );
    assert!(html.contains("Sync"), "label preserved: {html:?}");
    assert!(html.contains("50%"), "percentage preserved: {html:?}");
}

#[test]
fn browser_renders_at_zero_and_full() {
    use biscuit_terminal::components::renderable::BrowserRenderable;
    let zero = Progress::new(0.0).render_html_fragment().render();
    let full = Progress::new(1.0).render_html_fragment().render();
    assert!(
        zero.contains(r#"aria-valuenow="0""#),
        "0% -> ARIA value 0: {zero:?}"
    );
    assert!(
        full.contains(r#"aria-valuenow="100""#),
        "100% -> ARIA value 100: {full:?}"
    );
}

#[test]
fn browser_escapes_label_text() {
    // Label content with HTML-special characters must be escaped.
    use biscuit_terminal::components::renderable::BrowserRenderable;
    let bar = Progress::new(0.5).with_label("<script>");
    let html = bar.render_html_fragment().render();
    assert!(
        !html.contains("<script>"),
        "raw <script> must not appear in HTML: {html:?}"
    );
    assert!(
        html.contains("&lt;script&gt;") || html.contains("&lt;script"),
        "label is HTML-escaped: {html:?}"
    );
}

// ---------------------------------------------------------------------------
// SGR / ColorDepth degradation in the tree path
// ---------------------------------------------------------------------------

#[test]
fn tree_render_skips_sgr_at_color_depth_none() {
    use biscuit_terminal::discovery::detection::ColorDepth;
    use biscuit_terminal::terminal::Terminal;
    use renderable::color::{BasicColor, Color};
    let bar = Progress::new(0.5).with_filled_color(Color::BasicColor(BasicColor::Green));
    let mut term = Terminal::new_optimistic(80);
    term.color_depth = ColorDepth::None;
    let out = bar.render(&term);
    assert!(
        !out.contains('\x1b'),
        "no ANSI escapes when terminal lacks color support: {out:?}"
    );
}

// ---------------------------------------------------------------------------
// Width-mode slack sink (style-everywhere Phase 2, Task 2.7)
//
// Progress is an internal-layout component. The shared render-tree fold
// resolves the outer box from `Layout::width`; the bar track renders at its
// explicit `bar_width` (default 20). The bar track is the documented slack
// sink (spec D2): when the box is too narrow for `label + brackets + bar +
// percentage`, the bar gives way; the label, brackets, and percentage stay
// at their natural widths.
// ---------------------------------------------------------------------------

#[test]
fn width_auto_outer_box_fills_available() {
    // Width::Auto is the default. The bar renders at bar_width (default 20),
    // the outer box is the available width, and alignment applies. With no
    // alignment the rendered line is left-aligned at the available width.
    use biscuit_terminal::utils::layout::{Alignment, Layout};
    let mut bar = Progress::new(0.5).with_label("Loading");
    bar.layout_mut().width = biscuit_terminal::utils::layout::Width::Auto;
    bar.layout_mut().alignment = Alignment::Center;
    bar.layout_mut().margin = Default::default();
    let _ = Layout::default();
    let term = test_terminal(80);
    let out = strip_ansi(&bar.render(&term));
    // The bar track (20 cells) + label + brackets + percentage is well under
    // 80 columns; under center alignment the line is padded symmetrically.
    assert!(
        out.lines().any(|l| l.starts_with(' ') && l.contains('[')),
        "Width::Auto + center alignment centers the bar within the available width: {out:?}"
    );
}

#[test]
fn width_fixed_percent_50_does_not_double_apply() {
    // Width::Fixed(50%) resolves the outer box to 50% of available. The bar
    // still renders at its full bar_width — the box does NOT re-resolve the
    // percentage against itself. A bar width of 20 + the label + brackets +
    // percentage (~30 cells total) fits inside the 40-cell box.
    use biscuit_terminal::utils::layout::{Length, TargetValue, Width};
    let mut bar = Progress::new(0.5).with_label("Loading");
    bar.layout_mut().width =
        Width::Fixed(TargetValue::universal(Length::Percent(50.0)));
    bar.layout_mut().margin = Default::default();
    let node = bar.render_tree_node().expect("tree node");
    let out = render_tree(&node, 80);
    let stripped = strip_ansi(&out);
    let widest = stripped
        .lines()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    assert!(
        widest <= 40,
        "Fixed(50%) caps the outer box at 40 cells: widest={widest}"
    );
    // The bar's 10 fill chars (50% of bar_width 20) and 10 empty chars must
    // still be present — they were not re-resolved to 5+5 (which would be
    // the double-application bug).
    assert!(
        stripped.contains(&"█".repeat(10)),
        "bar fill is the full 10 chars (50% of bar_width 20, not 50% of 50%): {stripped:?}"
    );
}

#[test]
fn width_fit_content_hugs_bar_track() {
    // FitContent hugs the natural width. The natural width is the bar track
    // (20) + label + brackets + percentage; the rendered line is well below
    // the 80-column available width and is not padded.
    use biscuit_terminal::utils::layout::Width;
    let mut bar = Progress::new(0.5).with_label("Loading");
    bar.layout_mut().width = Width::FitContent;
    bar.layout_mut().margin = Default::default();
    let node = bar.render_tree_node().expect("tree node");
    let out = render_tree(&node, 80);
    let stripped = strip_ansi(&out);
    let widest = stripped
        .lines()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    assert!(
        widest < 80,
        "FitContent hugs the bar track natural width and does not pad to full available: widest={widest}"
    );
    assert!(
        widest > 20,
        "FitContent does not collapse the bar: widest={widest}"
    );
}

#[test]
fn bar_track_is_the_slack_sink_when_box_is_narrow() {
    // D2 slack sink: when the outer box (Layout::width Fixed small) is
    // narrower than the natural `label + brackets + bar + percentage` width,
    // the rendered output clamps inside the box. The label, brackets, and
    // percentage are fixed-width; the bar is the conceptual slack absorber.
    use biscuit_terminal::utils::layout::{Length, TargetValue, Width};
    let mut bar = Progress::new(0.5).with_label("Loading");
    bar.layout_mut().width = Width::Fixed(TargetValue::universal(Length::ch(60)));
    bar.layout_mut().margin = Default::default();
    let node = bar.render_tree_node().expect("tree node");
    let out = render_tree(&node, 80);
    let stripped = strip_ansi(&out);
    let widest = stripped
        .lines()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    assert!(
        widest <= 60,
        "Fixed(60ch) caps the outer box at 60 cells: widest={widest}"
    );
    // The fixed-width chrome (label "Loading", brackets `[` `]`, percentage
    // ` 50%`) all survive — they are not the slack sink.
    assert!(stripped.contains("Loading"), "label is fixed: {stripped:?}");
    assert!(stripped.contains('['), "left bracket is fixed: {stripped:?}");
    assert!(stripped.contains(']'), "right bracket is fixed: {stripped:?}");
    assert!(stripped.contains("50%"), "percentage is fixed: {stripped:?}");
}
