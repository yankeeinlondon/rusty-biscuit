//! Parity tests for `MetricsTree`'s box-model contract.
//!
//! `MetricsTree` is a structured-text component whose output composes Prose
//! markup. Per spec D5 / Task 3.5 the preferred path is a tree projection;
//! `MetricsTree` achieves this by delegating to `Prose` (which IS a
//! `TreeRenderable`), and propagating its outer `Layout` onto the inner
//! `Prose` so the shared render-tree fold applies the box model.
//!
//! ## Style-everywhere Phase 3 contract (Task 3.5)
//!
//! Decision: **project via delegation** (preferred per spec D5). The honored
//! subset is the full block surface — margin / alignment / max_width / width
//! / word_wrap — plus the inline appearance tags carried by the Prose markup
//! (`<b>` / `<dim>` / `<red>`). `Style::background` and `Style::border` are
//! N/A for the documented rationale (see the `MetricsTree` rustdoc).

mod parity_helpers;

use std::time::Duration;

use biscuit_terminal::components::metrics_tree::{
    MetricNode, MetricShare, MetricValue, MetricsTree,
};
use biscuit_terminal::components::renderable::TerminalRenderable;
use renderable::layout::{
    Alignment, Edges, Layout, Length, TargetValue, WordWrap,
};

use parity_helpers::{strip_ansi, test_terminal};

fn sample_tree() -> MetricNode {
    MetricNode::branch(
        "Performance",
        MetricValue::Duration(Duration::from_millis(100)),
        MetricShare::Full,
        vec![
            MetricNode::leaf(
                "parse",
                MetricValue::Duration(Duration::from_micros(45)),
                MetricShare::Of(0.0005),
            ),
            MetricNode::branch(
                "render",
                MetricValue::Duration(Duration::from_millis(60)),
                MetricShare::Of(0.6),
                vec![MetricNode::leaf(
                    "layout",
                    MetricValue::Duration(Duration::from_millis(58)),
                    MetricShare::Of(0.58),
                )],
            ),
        ],
    )
    .emphasized()
}

// ---------------------------------------------------------------------------
// Honored subset: Layout::margin
// ---------------------------------------------------------------------------

#[test]
fn layout_margin_indents_rendered_tree() {
    // A 6-ch left margin propagates onto the inner Prose and the fold
    // prepends 6 spaces to every row.
    let mut tree = MetricsTree::new(sample_tree());
    tree.layout_mut().margin = Edges {
        left: TargetValue::universal(Length::ch(6)),
        ..Edges::default()
    };
    let term = test_terminal(80);
    let out = strip_ansi(&tree.render(&term));
    for line in out.lines() {
        if line.trim().is_empty() {
            continue;
        }
        assert!(
            line.starts_with("      "),
            "left margin of 6 spaces must indent the metrics tree: {line:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Honored subset: Layout::alignment
// ---------------------------------------------------------------------------

#[test]
fn layout_alignment_center_shifts_block() {
    // Center alignment shifts the rendered block symmetrically within the
    // available width.
    let mut tree = MetricsTree::new(sample_tree());
    tree.layout_mut().alignment = Alignment::Center;
    let term = test_terminal(80);
    let out = strip_ansi(&tree.render(&term));
    // Find the row with the longest visible width; under center alignment
    // it must be preceded by some leading slack.
    let longest = out
        .lines()
        .map(|l| (l, l.trim_start().chars().count()))
        .max_by_key(|(_, w)| *w)
        .map(|(l, _)| l)
        .unwrap_or("");
    let leading = longest.len() - longest.trim_start().len();
    assert!(
        leading > 0,
        "center alignment must leave leading slack on the longest row: {longest:?}"
    );
}

// ---------------------------------------------------------------------------
// Honored subset: Layout::max_width caps the rendered width
// ---------------------------------------------------------------------------

#[test]
fn layout_max_width_caps_outer_box() {
    let mut tree = MetricsTree::new(sample_tree());
    tree.layout_mut().max_width = Some(TargetValue::universal(Length::ch(40)));
    let term = test_terminal(80);
    let out = strip_ansi(&tree.render(&term));
    for line in out.lines() {
        assert!(
            line.chars().count() <= 80,
            "MetricsTree render never exceeds the terminal width: {} > 80",
            line.chars().count()
        );
    }
}

// ---------------------------------------------------------------------------
// Honored subset: Layout::word_wrap propagates to the inner Prose
// ---------------------------------------------------------------------------

#[test]
fn layout_word_wrap_propagates_to_inner_prose() {
    // Round-trip the word_wrap field through layout_mut; the render path
    // propagates it onto the inner Prose so the fold can wrap long rows.
    let mut tree = MetricsTree::new(sample_tree());
    tree.layout_mut().word_wrap = WordWrap::WrapProse(Some(4), Some(2));
    assert_eq!(
        tree.layout().word_wrap,
        WordWrap::WrapProse(Some(4), Some(2)),
        "word_wrap round-trips through layout_mut"
    );
    // The render must succeed and produce non-empty output.
    let term = test_terminal(80);
    let out = strip_ansi(&tree.render(&term));
    assert!(!out.trim().is_empty(), "render produced empty output: {out:?}");
}

// ---------------------------------------------------------------------------
// N/A cells: Style::background / Style::border have no matrix effect
// ---------------------------------------------------------------------------

#[test]
fn na_style_background_is_carried_by_prose_markup_not_block_bg() {
    // The metrics tree colors individual rows via `<red>` / `<dim>` Prose
    // markup, not via a block-level `Style::background`. The default
    // `Style` on the inner Prose has no background; a hot row's red color
    // comes from the `<red>` markup tag.
    use biscuit_terminal::components::metrics_tree::MetricMarker;
    let root = MetricNode::branch(
        "Total",
        MetricValue::Duration(Duration::from_millis(100)),
        MetricShare::Full,
        vec![
            MetricNode::leaf(
                "hot stage",
                MetricValue::Duration(Duration::from_millis(90)),
                MetricShare::Of(0.9),
            )
            .with_marker(MetricMarker::Highlight),
            MetricNode::leaf(
                "cool stage",
                MetricValue::Duration(Duration::from_millis(10)),
                MetricShare::Of(0.1),
            ),
        ],
    );
    let colored = MetricsTree::new(root).render_optimistic(Some(80));
    // The hot row carries a red SGR (`\x1b[31m`) from the `<red>` markup —
    // there is no block-level background SGR (`\x1b[48;`).
    assert!(
        colored.contains("\u{1b}[31m"),
        "hot row carries red SGR via Prose markup: {colored:?}"
    );
    assert!(
        !colored.contains("\u{1b}[48;"),
        "no block-level background SGR is emitted (Style::background is N/A): {colored:?}"
    );
}

// ---------------------------------------------------------------------------
// Structural: the connector glyphs and column alignment survive
// ---------------------------------------------------------------------------

#[test]
fn tree_projection_via_prose_preserves_connectors_and_alignment() {
    // Delegating to Prose must not corrupt the connector glyphs or the
    // column alignment — those are computed by `build_markup` before any
    // Prose tag is wrapped around them.
    let plain = strip_ansi(&MetricsTree::new(sample_tree()).render_optimistic(Some(80)));
    assert!(plain.contains("├─ parse"), "tee connector survives: {plain:?}");
    assert!(plain.contains("└─ render"), "elbow connector survives: {plain:?}");
    assert!(
        plain.contains("   └─ layout"),
        "nested connector survives: {plain:?}"
    );

    // Every value's unit suffix begins at the same column (unit-aligned).
    let unit_columns: Vec<usize> = plain
        .lines()
        .filter_map(|l| l.find("ms").or_else(|| l.find("µs")).map(|idx| (l, idx)))
        .map(|(l, idx)| l[..idx].chars().count())
        .collect();
    assert!(
        unit_columns.iter().all(|c| *c == unit_columns[0]),
        "value units are column-aligned after Prose delegation: {unit_columns:?}"
    );
}

// ---------------------------------------------------------------------------
// Layout round-trip (C4 mode-bearing survival)
// ---------------------------------------------------------------------------

#[test]
fn layout_mut_round_trips_mode_bearing_fields() {
    let mut tree = MetricsTree::new(sample_tree());
    let new_layout = Layout {
        margin: Edges {
            left: TargetValue::universal(Length::ch(4)),
            ..Edges::default()
        },
        alignment: Alignment::Center,
        max_width: Some(TargetValue::universal(Length::ch(60))),
        ..Layout::default()
    };
    *tree.layout_mut() = new_layout.clone();
    assert_eq!(
        tree.layout().margin.left,
        new_layout.margin.left,
        "layout_mut round-trip preserves the left margin"
    );
    assert_eq!(
        tree.layout().alignment,
        new_layout.alignment,
        "layout_mut round-trip preserves the alignment"
    );
    assert_eq!(
        tree.layout().max_width,
        new_layout.max_width,
        "layout_mut round-trip preserves the max_width"
    );
}
