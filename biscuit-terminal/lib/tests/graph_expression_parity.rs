//! Parity tests for the `GraphExpression` component's box-model contract.
//!
//! `GraphExpression` is a bespoke image component (spec C5): the rendered
//! graph is rasterized by `biscuit-visualized` and emitted through a
//! terminal image protocol (Kitty / iTerm2 / Sixel). It does not project to
//! a `RenderNode`; the [`TerminalRenderable`] impl applies the configured
//! `Layout` via `apply_block_layout` and the inner `TerminalImage` resolves
//! image dimensions from `ImageWidth` (Fill / Percent / Characters).
//!
//! ## Style-everywhere Phase 2 contract (Task 2.6)
//!
//! The slack sink is the rendered graph canvas, capped by the component's
//! own graph/image constraints (spec D2):
//!
//! - `Layout::margin` is honored: it reduces the available width before the
//!   image width is resolved, so a wider margin shrinks the image canvas.
//! - `Layout::alignment` is honored: the image's horizontal offset reflects
//!   left / center / right alignment against the slack.
//! - `ImageWidth` is the explicit contract for the image canvas:
//!   - `Fill` ⇒ the image canvas absorbs **all** the slack (image_width ==
//!     available_width after margin).
//!   - `Percent(p)` ⇒ the image canvas is `p * term_width` (not `p *
//!     available_width` — the basis is the terminal width, mirroring
//!     `Length::Percent` semantics), then clamped under the available width.
//!   - `Characters(n)` ⇒ an explicit cell count, clamped under the available
//!     width.
//! - `Layout::width` is intentionally **N/A** for the image canvas:
//!   `ImageWidth` is the contract that selects Fill / Percent / Characters,
//!   and reusing `Layout::width` would create two competing width controls.
//!   This is the documented GraphExpression-specific carve-out — it is not a
//!   silent no-op because `Layout::width` is never read by the image
//!   resolver, and `Layout::margin` / `Layout::alignment` ARE honored.
//!
//! `TerminalImage::resolve_dimensions_for` is the single width/margin
//! calculator both `GraphExpression` and `TerminalImage` share; testing it
//! directly is the narrowest way to pin the contract without depending on a
//! real terminal's image-protocol support.

#![cfg(feature = "image")]

mod parity_helpers;

use biscuit_terminal::components::graph_expression::{GraphExpression, GraphInputSyntax};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::terminal_image::{ImageWidth, ResolvedDimensions, TerminalImage};
use biscuit_terminal::utils::layout::{
    Alignment, Edges, Layout, Length, TargetValue, Width,
};

use parity_helpers::test_terminal;

// ---------------------------------------------------------------------------
// resolve_dimensions_for — the single width/margin calculator
// ---------------------------------------------------------------------------

fn resolve(width: &ImageWidth, layout: &Layout, term_width: u32) -> ResolvedDimensions {
    TerminalImage::resolve_dimensions_for(width, layout, term_width)
}

#[test]
fn image_width_fill_absorbs_all_slack() {
    // ImageWidth::Fill is the documented slack sink for GraphExpression
    // (spec D2): the image canvas fills the available width after margin.
    let layout = Layout::default();
    let dims = resolve(&ImageWidth::Fill, &layout, 80);
    assert_eq!(
        dims.image_width, 80,
        "Fill absorbs all available slack: {:?}",
        dims
    );
    assert_eq!(
        dims.available_width, 80,
        "available_width is the term width under no margin: {:?}",
        dims
    );
}

#[test]
fn image_width_percent_uses_term_width_as_basis() {
    // ImageWidth::Percent(p) resolves to p * term_width, NOT p *
    // available_width. This matches `Length::Percent` basis semantics — the
    // percentage is against the outer containing width.
    let layout = Layout::default();
    let dims = resolve(&ImageWidth::Percent(0.5), &layout, 80);
    assert_eq!(
        dims.image_width, 40,
        "Percent(0.5) on term_width 80 yields 40 cells: {:?}",
        dims
    );
}

#[test]
fn image_width_characters_is_explicit() {
    let layout = Layout::default();
    let dims = resolve(&ImageWidth::Characters(15), &layout, 80);
    assert_eq!(
        dims.image_width, 15,
        "Characters(15) is the explicit cell count: {:?}",
        dims
    );
}

#[test]
fn image_width_clamps_to_available_when_margin_present() {
    // Layout::margin shrinks the available width. A `Characters(60)` image
    // against a 20-cell margin on an 80-cell terminal leaves only 60
    // available cells; the clamp keeps the image inside that 60-cell box.
    let layout = Layout {
        margin: Edges {
            left: TargetValue::universal(Length::ch(10)),
            right: TargetValue::universal(Length::ch(10)),
            ..Edges::default()
        },
        ..Layout::default()
    };
    let dims = resolve(&ImageWidth::Characters(80), &layout, 80);
    assert_eq!(
        dims.available_width, 60,
        "available_width is term_width minus horizontal margin: {:?}",
        dims
    );
    assert_eq!(
        dims.image_width, 60,
        "image_width clamps to the available_width: {:?}",
        dims
    );
}

// ---------------------------------------------------------------------------
// Layout.alignment is honored
// ---------------------------------------------------------------------------

#[test]
fn alignment_left_places_image_at_left_margin() {
    let layout = Layout {
        margin: Edges {
            left: TargetValue::universal(Length::ch(8)),
            ..Edges::default()
        },
        alignment: Alignment::Left,
        ..Layout::default()
    };
    let dims = resolve(&ImageWidth::Characters(20), &layout, 80);
    assert_eq!(
        dims.x_offset, 8,
        "left alignment places image at the left margin: {:?}",
        dims
    );
}

#[test]
fn alignment_centers_image_in_slack_area() {
    // available_width 80, image_width 20, left_margin 0 ⇒ slack = 60.
    // Centered x_offset = left_margin + slack / 2 = 30.
    let layout = Layout {
        alignment: Alignment::Center,
        ..Layout::default()
    };
    let dims = resolve(&ImageWidth::Characters(20), &layout, 80);
    assert_eq!(
        dims.x_offset, 30,
        "center alignment places image at slack/2: {:?}",
        dims
    );
}

#[test]
fn alignment_right_pushes_image_to_slack_edge() {
    // available_width 80, image_width 20 ⇒ slack = 60. Right x_offset = 60.
    let layout = Layout {
        alignment: Alignment::Right,
        ..Layout::default()
    };
    let dims = resolve(&ImageWidth::Characters(20), &layout, 80);
    assert_eq!(
        dims.x_offset, 60,
        "right alignment pushes image to the slack edge: {:?}",
        dims
    );
}

// ---------------------------------------------------------------------------
// No double-resolution of percentages
// ---------------------------------------------------------------------------

#[test]
fn image_width_percent_does_not_double_apply_margin() {
    // ImageWidth::Percent(0.5) on term_width 100 resolves to 50 cells, then
    // is clamped under the available width (which is also reduced by the
    // margin). The percentage is NOT resolved against the narrowed
    // available width — that would be 50% of (100 - margin), the
    // double-application bug.
    let layout = Layout {
        margin: Edges {
            left: TargetValue::universal(Length::ch(20)),
            ..Edges::default()
        },
        ..Layout::default()
    };
    let dims = resolve(&ImageWidth::Percent(0.5), &layout, 100);
    assert_eq!(
        dims.image_width, 50,
        "Percent(0.5) resolves to 50 against term_width 100, not against the \
         margin-narrowed 80: {:?}",
        dims
    );
}

// ---------------------------------------------------------------------------
// Unbounded-width guard (spec C3)
// ---------------------------------------------------------------------------

#[test]
fn image_width_fill_hugs_when_term_width_is_one() {
    // The image resolver clamps on a degenerate 1-cell terminal so a
    // Fill-style width cannot blow up to `u32::MAX`. There is no finite
    // slack to absorb at width 1, so Fill collapses to the 1-cell canvas.
    let layout = Layout::default();
    let dims = resolve(&ImageWidth::Fill, &layout, 1);
    assert_eq!(
        dims.image_width, 1,
        "Fill on a degenerate 1-cell terminal collapses to 1 cell: {:?}",
        dims
    );
}

// ---------------------------------------------------------------------------
// GraphExpression carries Layout through the TerminalRenderable impl
// ---------------------------------------------------------------------------

#[test]
fn graph_expression_layout_mut_round_trips() {
    // The TerminalRenderable::layout_mut hook exposes the inner Layout so a
    // caller can apply margins / alignment via the shared builder API.
    // Round-tripping a layout through the accessor confirms C4 — the
    // mode-bearing fields survive.
    let mut graph =
        GraphExpression::parse("a -> b", GraphInputSyntax::Auto).expect("parse graph");
    let new_layout = Layout {
        margin: Edges {
            left: TargetValue::universal(Length::ch(4)),
            ..Edges::default()
        },
        alignment: Alignment::Center,
        ..Layout::default()
    };
    *graph.layout_mut() = new_layout.clone();
    assert_eq!(
        graph.layout().margin.left,
        new_layout.margin.left,
        "layout_mut round-trip preserves the left margin"
    );
    assert_eq!(
        graph.layout().alignment,
        new_layout.alignment,
        "layout_mut round-trip preserves the alignment"
    );
}

#[test]
fn graph_expression_layout_width_field_does_not_affect_image_canvas() {
    // Documented carve-out: `Layout::width` is N/A for the image canvas
    // because `ImageWidth` is the explicit contract. Setting `width` on the
    // GraphExpression layout MUST NOT change the resolved image width —
    // doing so would be a competing-width-controls defect.
    let image_width = ImageWidth::Percent(0.5); // GraphExpression's default
    let mut graph =
        GraphExpression::parse("a -> b", GraphInputSyntax::Auto).expect("parse graph");
    *graph.layout_mut() = Layout {
        width: Width::Fixed(TargetValue::universal(Length::Percent(50.0))),
        ..Layout::default()
    };
    let term = test_terminal(80);
    let with_layout_width = resolve(&image_width, graph.layout(), term.width());
    let without_layout_width = resolve(&image_width, &Layout::default(), term.width());
    assert_eq!(
        with_layout_width.image_width,
        without_layout_width.image_width,
        "Layout::width is N/A for the image canvas (ImageWidth is the contract): \
         {:?} vs {:?}",
        with_layout_width,
        without_layout_width
    );
}

// ---------------------------------------------------------------------------
// Component-level smoke: the bespoke render path returns content
// ---------------------------------------------------------------------------

#[test]
fn graph_expression_renders_non_empty_at_parity_widths() {
    // The bespoke terminal render exercises `apply_block_layout` against the
    // term width and the resolved image dimensions. The result depends on
    // image-protocol support; on a no-image terminal it falls back to a
    // fenced `graph-expression` / `dot` code block, which is still non-empty
    // (so the box-model fold has something to wrap).
    const PARITY_WIDTHS: &[u32] = &[40, 80, 120];
    for &width in PARITY_WIDTHS {
        let graph =
            GraphExpression::parse("a -> b -> c", GraphInputSyntax::Auto).expect("parse graph");
        let term = test_terminal(width);
        let out = graph.render(&term);
        assert!(
            !out.is_empty(),
            "GraphExpression render at width {width} produced empty output"
        );
    }
}
