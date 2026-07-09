//! Parity tests for `TerminalImage`'s box-model contract.
//!
//! `TerminalImage` is a bespoke image-protocol component (spec C5/D5): the
//! Kitty / iTerm2 / Sixel escape sequences are irreducible, so the component
//! emits the protocol bytes directly through `TerminalRenderable::render` and
//! does not project to a `RenderNode`. The shared fold-style box model does
//! not apply.
//!
//! ## Style-everywhere Phase 3 contract (Task 3.1)
//!
//! The honored subset (spec C5) is the outer placement:
//!
//! - `Layout::margin` is **Honored**: reduces `available_width` and seeds
//!   `x_offset` via `resolve_dimensions`.
//! - `Layout::alignment` is **Honored**: selects left / center / right
//!   placement of the image canvas within the slack.
//! - `Layout::max_width`, `Layout::width`, `Layout::padding`,
//!   `Layout::word_wrap`, and every `Style` field are **N/A**: the image
//!   protocol has no box, padding, or text for the fold to paint. Each
//!   N/A cell is documented on the `TerminalImage` rustdoc and pinned below
//!   with a rationale + test.
//!
//! `TerminalImage::resolve_dimensions_for` is the single width/margin
//! calculator; testing it directly is the narrowest way to pin the contract
//! without depending on a real terminal's image-protocol support.

mod parity_helpers;

use biscuit_terminal::components::terminal_image::{ImageWidth, TerminalImage};
use renderable::layout::{
    Alignment, Edges, Layout, Length, TargetValue, Width, WordWrap,
};

// ---------------------------------------------------------------------------
// Honored subset: Layout::margin reduces available_width and seeds x_offset
// ---------------------------------------------------------------------------

#[test]
fn margin_left_shrinks_available_width_and_seeds_x_offset() {
    let layout = Layout {
        margin: Edges {
            left: TargetValue::universal(Length::ch(8)),
            ..Edges::default()
        },
        ..Layout::default()
    };
    let dims = TerminalImage::resolve_dimensions_for(
        &ImageWidth::Fill,
        &layout,
        80,
    );
    assert_eq!(
        dims.available_width, 72,
        "left margin of 8 cells shrinks the 80-cell terminal to 72 available: {:?}",
        dims,
    );
    assert_eq!(
        dims.left_margin, 8,
        "left_margin is the resolved left edge: {:?}",
        dims,
    );
    // Fill absorbs all the slack, so image_width equals available_width and
    // x_offset is the left margin under left alignment (default).
    assert_eq!(dims.image_width, 72);
    assert_eq!(dims.x_offset, 8);
}

#[test]
fn margin_right_shrinks_available_width_without_changing_x_offset() {
    let layout = Layout {
        margin: Edges {
            right: TargetValue::universal(Length::ch(10)),
            ..Edges::default()
        },
        ..Layout::default()
    };
    let dims = TerminalImage::resolve_dimensions_for(
        &ImageWidth::Fill,
        &layout,
        80,
    );
    assert_eq!(
        dims.available_width, 70,
        "right margin of 10 cells shrinks the 80-cell terminal to 70 available: {:?}",
        dims,
    );
    assert_eq!(dims.right_margin, 10);
    assert_eq!(dims.x_offset, 0, "left alignment keeps x_offset at 0");
}

#[test]
fn margin_percentage_resolves_against_terminal_width() {
    // A 10% margin on an 80-cell terminal yields 8 cells per side.
    let layout = Layout {
        margin: Edges {
            left: TargetValue::universal(Length::Percent(10.0)),
            right: TargetValue::universal(Length::Percent(10.0)),
            ..Edges::default()
        },
        ..Layout::default()
    };
    let dims = TerminalImage::resolve_dimensions_for(
        &ImageWidth::Fill,
        &layout,
        80,
    );
    assert_eq!(dims.left_margin, 8);
    assert_eq!(dims.right_margin, 8);
    assert_eq!(dims.available_width, 64);
}

// ---------------------------------------------------------------------------
// Honored subset: Layout::alignment positions the image canvas
// ---------------------------------------------------------------------------

#[test]
fn alignment_left_keeps_image_at_left_margin() {
    let layout = Layout {
        alignment: Alignment::Left,
        ..Layout::default()
    };
    let dims = TerminalImage::resolve_dimensions_for(
        &ImageWidth::Characters(20),
        &layout,
        80,
    );
    // Slack = 80 - 20 = 60; left alignment puts the image at the left edge.
    assert_eq!(dims.x_offset, 0);
}

#[test]
fn alignment_center_splits_slack_evenly() {
    let layout = Layout {
        alignment: Alignment::Center,
        ..Layout::default()
    };
    let dims = TerminalImage::resolve_dimensions_for(
        &ImageWidth::Characters(20),
        &layout,
        80,
    );
    // Slack = 60; centered x_offset = slack / 2 = 30.
    assert_eq!(dims.x_offset, 30);
}

#[test]
fn alignment_right_pushes_image_to_slack_edge() {
    let layout = Layout {
        alignment: Alignment::Right,
        ..Layout::default()
    };
    let dims = TerminalImage::resolve_dimensions_for(
        &ImageWidth::Characters(20),
        &layout,
        80,
    );
    // Slack = 60; right x_offset = 60.
    assert_eq!(dims.x_offset, 60);
}

#[test]
fn alignment_composes_with_margin() {
    // Left margin of 8 + image 20 + slack 52, centered ⇒ x_offset = 8 + 26.
    let layout = Layout {
        margin: Edges {
            left: TargetValue::universal(Length::ch(8)),
            ..Edges::default()
        },
        alignment: Alignment::Center,
        ..Layout::default()
    };
    let dims = TerminalImage::resolve_dimensions_for(
        &ImageWidth::Characters(20),
        &layout,
        80,
    );
    assert_eq!(dims.available_width, 72);
    assert_eq!(dims.x_offset, 34, "left_margin 8 + slack/2 (26) = 34");
}

// ---------------------------------------------------------------------------
// N/A cells (spec C5/D5): documented, tested rationale each.
// ---------------------------------------------------------------------------

#[test]
fn na_layout_width_does_not_affect_image_canvas() {
    // `Layout::width` is N/A: `ImageWidth` is the explicit contract. Setting
    // `Layout::width` MUST NOT change the resolved image width — doing so
    // would be a competing-width-controls defect.
    let image_width = ImageWidth::Percent(0.5);
    let with_layout_width = Layout {
        width: Width::Fixed(TargetValue::universal(Length::Percent(50.0))),
        ..Layout::default()
    };
    let without_layout_width = Layout::default();
    let a = TerminalImage::resolve_dimensions_for(&image_width, &with_layout_width, 80);
    let b = TerminalImage::resolve_dimensions_for(&image_width, &without_layout_width, 80);
    assert_eq!(
        a.image_width, b.image_width,
        "Layout::width is N/A for the image canvas (ImageWidth is the contract): \
         {:?} vs {:?}",
        a, b,
    );
}

#[test]
fn na_layout_max_width_does_not_affect_image_canvas() {
    // Same carve-out as `Layout::width`: `ImageWidth` is the contract, so
    // `Layout::max_width` is also N/A.
    let image_width = ImageWidth::Characters(40);
    let with_max_width = Layout {
        max_width: Some(TargetValue::universal(Length::ch(20))),
        ..Layout::default()
    };
    let without_max_width = Layout::default();
    let a = TerminalImage::resolve_dimensions_for(&image_width, &with_max_width, 80);
    let b = TerminalImage::resolve_dimensions_for(&image_width, &without_max_width, 80);
    assert_eq!(
        a.image_width, b.image_width,
        "Layout::max_width is N/A for the image canvas (ImageWidth is the contract): \
         {:?} vs {:?}",
        a, b,
    );
}

#[test]
fn na_layout_padding_does_not_affect_image_canvas() {
    // The image protocol has no padding box; padding cannot paint protocol
    // bytes. Padding MUST NOT change the resolved image dimensions.
    let image_width = ImageWidth::Fill;
    let with_padding = Layout {
        padding: Edges::all(Length::ch(4)),
        ..Layout::default()
    };
    let without_padding = Layout::default();
    let a = TerminalImage::resolve_dimensions_for(&image_width, &with_padding, 80);
    let b = TerminalImage::resolve_dimensions_for(&image_width, &without_padding, 80);
    assert_eq!(
        a.image_width, b.image_width,
        "Layout::padding is N/A for the image protocol: {:?} vs {:?}",
        a, b,
    );
}

#[test]
fn na_layout_word_wrap_does_not_affect_image_canvas() {
    // The image escape cannot wrap. `Layout::word_wrap` MUST NOT change the
    // resolved image dimensions.
    let image_width = ImageWidth::Fill;
    let with_wrap = Layout {
        word_wrap: WordWrap::WrapProse(None, None),
        ..Layout::default()
    };
    let without_wrap = Layout::default();
    let a = TerminalImage::resolve_dimensions_for(&image_width, &with_wrap, 80);
    let b = TerminalImage::resolve_dimensions_for(&image_width, &without_wrap, 80);
    assert_eq!(
        a.image_width, b.image_width,
        "Layout::word_wrap is N/A for the image protocol: {:?} vs {:?}",
        a, b,
    );
}

// ---------------------------------------------------------------------------
// Image width is honored (the explicit image-protocol contract)
// ---------------------------------------------------------------------------

#[test]
fn image_width_fill_absorbs_all_available_slack() {
    let dims = TerminalImage::resolve_dimensions_for(
        &ImageWidth::Fill,
        &Layout::default(),
        80,
    );
    assert_eq!(dims.image_width, 80);
}

#[test]
fn image_width_percent_resolves_against_terminal_width() {
    let dims = TerminalImage::resolve_dimensions_for(
        &ImageWidth::Percent(0.5),
        &Layout::default(),
        80,
    );
    assert_eq!(dims.image_width, 40);
}

#[test]
fn image_width_characters_is_explicit_clamped_to_available() {
    let dims = TerminalImage::resolve_dimensions_for(
        &ImageWidth::Characters(120),
        &Layout::default(),
        80,
    );
    assert_eq!(
        dims.image_width, 80,
        "Characters over the available width clamp to it: {:?}",
        dims,
    );
}

// ---------------------------------------------------------------------------
// Unbounded-width guard (spec C3): Fill cannot blow up on a degenerate
// terminal.
// ---------------------------------------------------------------------------

#[test]
fn image_width_fill_hugs_on_degenerate_width() {
    let dims = TerminalImage::resolve_dimensions_for(
        &ImageWidth::Fill,
        &Layout::default(),
        1,
    );
    assert_eq!(dims.image_width, 1);
}

// ---------------------------------------------------------------------------
// Layout accessor round-trip (TerminalRenderable::layout / layout_mut)
// ---------------------------------------------------------------------------

#[test]
fn layout_mut_round_trips_mode_bearing_fields() {
    // The `TerminalRenderable::layout_mut` hook exposes the inner Layout so
    // a caller can apply margins / alignment via the shared builder API.
    // Round-tripping a layout through the accessor confirms the mode-bearing
    // fields survive — the same contract an internal-layout component's
    // hint round-trip must satisfy (C4).
    use biscuit_terminal::components::renderable::TerminalRenderable;
    let mut image = TerminalImage::default();
    let new_layout = Layout {
        margin: Edges {
            left: TargetValue::universal(Length::ch(4)),
            ..Edges::default()
        },
        alignment: Alignment::Center,
        ..Layout::default()
    };
    *image.layout_mut() = new_layout.clone();
    assert_eq!(
        image.layout().margin.left,
        new_layout.margin.left,
        "layout_mut round-trip preserves the left margin"
    );
    assert_eq!(
        image.layout().alignment,
        new_layout.alignment,
        "layout_mut round-trip preserves the alignment"
    );
}
