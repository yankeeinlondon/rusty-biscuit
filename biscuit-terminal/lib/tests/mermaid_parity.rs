//! Parity tests for `MermaidDiagram`'s box-model contract.
//!
//! `MermaidDiagram` is a bespoke image-protocol component (spec C5/D5): the
//! external Mermaid → PNG rendering is irreducible, so the component emits
//! the rendered image through `TerminalImage` and does not project to a
//! `RenderNode`. The fold-style box model does not apply to the rendered
//! image itself; the box placement is shared.
//!
//! ## Style-everywhere Phase 3 contract (Task 3.2)
//!
//! The honored subset (spec C5) is the outer placement, resolved by
//! `TerminalImage::resolve_dimensions_for` (the same single width/margin
//! calculator `TerminalImage` and `GraphExpression` use):
//!
//! - `Layout::margin` is **Honored**: reduces `available_width` and seeds
//!   `x_offset` before the rendered canvas is sized.
//! - `Layout::alignment` is **Honored**: applied via `apply_block_layout`
//!   so the rendered image is positioned within the slack.
//! - `Layout::width`, `Layout::max_width`, `Layout::padding`,
//!   `Layout::word_wrap`, and every `Style` field are **N/A** (see the
//!   `MermaidDiagram` rustdoc for the rationale per cell).
//!
//! The full terminal-render pipeline depends on a working Mermaid
//! installation (the L2 tests exercise it); these unit tests pin the
//! honored subset without invoking the external renderer.

#![cfg(feature = "image")]

mod parity_helpers;

use biscuit_terminal::components::mermaid::MermaidDiagram;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::terminal_image::{ImageWidth, TerminalImage};
use biscuit_terminal::discovery::detection::{ImageSupport, TerminalApp};
use biscuit_terminal::discovery::fonts::CellSize;
use biscuit_terminal::terminal::Terminal;
use renderable::layout::{
    Alignment, Edges, Layout, Length, TargetValue, Width, WordWrap,
};

use parity_helpers::test_terminal;

// ---------------------------------------------------------------------------
// Honored subset: Layout::margin shrinks the rendered canvas
// ---------------------------------------------------------------------------

#[test]
fn margin_lowers_resolved_image_width() {
    // MermaidDiagram resolves the rendered canvas through
    // `TerminalImage::resolve_dimensions_for`. A 10-cell left margin on an
    // 80-cell terminal leaves 70 available cells; under Fill the rendered
    // canvas is 70 cells.
    let diagram = MermaidDiagram::new("flowchart LR\n    A --> B")
        .with_width(ImageWidth::Fill);
    let layout = Layout {
        margin: Edges {
            left: TargetValue::universal(Length::ch(10)),
            ..Edges::default()
        },
        ..Layout::default()
    };
    let dims = TerminalImage::resolve_dimensions_for(
        &ImageWidth::Fill,
        &layout,
        80,
    );
    assert_eq!(dims.available_width, 70);
    assert_eq!(dims.image_width, 70);
    // The diagram itself carries the same layout when applied via layout_mut.
    let mut diagram = diagram;
    *diagram.layout_mut() = layout;
    assert_eq!(
        diagram.layout().margin.left,
        TargetValue::universal(Length::ch(10)),
    );
}

// ---------------------------------------------------------------------------
// Honored subset: Layout::alignment positions the rendered block
// ---------------------------------------------------------------------------

#[test]
fn layout_alignment_round_trips_through_layout_mut() {
    let mut diagram = MermaidDiagram::new("flowchart LR\n    A --> B");
    diagram.layout_mut().alignment = Alignment::Center;
    assert_eq!(
        diagram.layout().alignment,
        Alignment::Center,
        "alignment round-trips through layout_mut (C4 mode-bearing survival)"
    );
}

#[test]
fn layout_alignment_centers_rendered_block_when_pipeline_succeeds() {
    // When the Mermaid pipeline succeeds, `try_render` applies
    // `apply_block_layout` against the configured alignment. The block
    // layout math places the rendered image at slack/2 under center
    // alignment. The assertion runs through `resolve_dimensions_for` to
    // avoid depending on a working Mermaid installation in unit-test env.
    let layout = Layout {
        alignment: Alignment::Center,
        ..Layout::default()
    };
    let dims = TerminalImage::resolve_dimensions_for(
        &ImageWidth::Characters(20),
        &layout,
        80,
    );
    assert_eq!(
        dims.x_offset, 30,
        "center alignment places the rendered canvas at slack/2"
    );
}

// ---------------------------------------------------------------------------
// N/A cells (spec C5/D5): documented, tested rationale each.
// ---------------------------------------------------------------------------

#[test]
fn na_layout_width_does_not_affect_rendered_canvas() {
    // `Layout::width` is N/A: `ImageWidth` is the explicit contract. Setting
    // `Layout::width` MUST NOT change the resolved rendered width — doing
    // so would be a competing-width-controls defect.
    let image_width = ImageWidth::Percent(0.5); // MermaidDiagram's default
    let with_layout_width = Layout {
        width: Width::Fixed(TargetValue::universal(Length::Percent(50.0))),
        ..Layout::default()
    };
    let without_layout_width = Layout::default();
    let a = TerminalImage::resolve_dimensions_for(&image_width, &with_layout_width, 80);
    let b = TerminalImage::resolve_dimensions_for(&image_width, &without_layout_width, 80);
    assert_eq!(
        a.image_width, b.image_width,
        "Layout::width is N/A for the rendered canvas (ImageWidth is the contract): \
         {:?} vs {:?}",
        a, b,
    );
}

#[test]
fn na_layout_max_width_does_not_affect_rendered_canvas() {
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
        "Layout::max_width is N/A for the rendered canvas: {:?} vs {:?}",
        a, b,
    );
}

#[test]
fn na_layout_padding_does_not_affect_rendered_canvas() {
    // The rendered image has no padding box; padding cannot paint protocol
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
        "Layout::padding is N/A for the rendered image: {:?} vs {:?}",
        a, b,
    );
}

#[test]
fn na_layout_word_wrap_does_not_affect_rendered_canvas() {
    // A rasterized diagram cannot wrap. `Layout::word_wrap` MUST NOT change
    // the resolved rendered dimensions.
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
        "Layout::word_wrap is N/A for the rendered image: {:?} vs {:?}",
        a, b,
    );
}

// ---------------------------------------------------------------------------
// Image width is honored (the explicit MermaidDiagram contract)
// ---------------------------------------------------------------------------

#[test]
fn image_width_spec_drives_rendered_canvas_size() {
    // MermaidDiagram defaults to Percent(0.5); a 0.5 fraction on an 80-cell
    // terminal resolves to 40 cells.
    let dims = TerminalImage::resolve_dimensions_for(
        &ImageWidth::Percent(0.5),
        &Layout::default(),
        80,
    );
    assert_eq!(dims.image_width, 40);

    // Fill absorbs all available cells.
    let dims = TerminalImage::resolve_dimensions_for(
        &ImageWidth::Fill,
        &Layout::default(),
        80,
    );
    assert_eq!(dims.image_width, 80);

    // Characters is an explicit cell count, clamped to available.
    let dims = TerminalImage::resolve_dimensions_for(
        &ImageWidth::Characters(120),
        &Layout::default(),
        80,
    );
    assert_eq!(dims.image_width, 80);
}

// ---------------------------------------------------------------------------
// Component-level smoke: the bespoke render path returns content
// ---------------------------------------------------------------------------

#[test]
fn mermaid_diagram_renders_non_empty_at_parity_widths() {
    // The bespoke terminal render exercises `apply_block_layout` and the
    // Mermaid → PNG → terminal image pipeline. On hosts without a working
    // Mermaid installation the pipeline degrades to the fenced code-block
    // fallback, which is still non-empty (so the box-model fold has
    // something to wrap).
    const PARITY_WIDTHS: &[u32] = &[40, 80, 120];
    for &width in PARITY_WIDTHS {
        let diagram = MermaidDiagram::new("flowchart LR\n    A --> B --> C");
        let term = test_terminal(width);
        let out = diagram.render(&term);
        assert!(
            !out.is_empty(),
            "MermaidDiagram render at width {width} produced empty output"
        );
    }
}

#[test]
fn git_graph_diagram_uses_source_png_aspect_even_with_stale_scaled_artifact() {
    let diagram = MermaidDiagram::new("gitGraph\n    commit id: \"abc1234\"")
        .with_width(ImageWidth::Characters(40));
    let term = wezterm_kitty_terminal();

    match diagram.try_render(&term) {
        Ok(result) => {
            let stale_scaled_path = scaled_png_sibling(&result.png_path);
            let source_image = image::open(&result.png_path).expect("source Mermaid PNG should load");
            let stale_image = image::DynamicImage::new_rgba8(
                source_image.width().max(1),
                (source_image.height() * 2).max(1),
            );
            stale_image
                .save(&stale_scaled_path)
                .expect("stale scaled image fixture should save");

            let rerendered = diagram
                .try_render(&term)
                .expect("second Mermaid render should use cached source PNG");
            let rows = kitty_rows(&rerendered.output).expect("Kitty output should include r=");
            let rendered_image =
                image::open(&rerendered.png_path).expect("rendered Mermaid PNG should load");
            let expected_rows = (rerendered.width_cells as f32
                * (rendered_image.height() as f32 / rendered_image.width() as f32)
                * (8.0 / 16.0))
                .ceil() as u32;

            assert_eq!(rerendered.png_path, result.png_path);
            assert!(!rerendered.png_path.to_string_lossy().contains("-h125.png"));
            assert_eq!(rows, expected_rows.max(1));
        }
        Err(e) => {
            eprintln!("Mermaid render unavailable in integration-test env: {e}");
        }
    }
}

#[test]
fn non_git_graph_diagram_keeps_source_aspect_kitty_row_geometry() {
    let diagram = MermaidDiagram::new("pie\n    A: 1").with_width(ImageWidth::Characters(40));
    let term = wezterm_kitty_terminal();

    match diagram.try_render(&term) {
        Ok(result) => {
            let rows = kitty_rows(&result.output).expect("Kitty output should include r=");
            let image = image::open(&result.png_path).expect("cached Mermaid PNG should load");
            let expected_rows = (result.width_cells as f32
                * (image.height() as f32 / image.width() as f32)
                * (8.0 / 16.0))
                .ceil() as u32;

            assert_eq!(rows, expected_rows.max(1));
        }
        Err(e) => {
            eprintln!("Mermaid render unavailable in integration-test env: {e}");
        }
    }
}

fn wezterm_kitty_terminal() -> Terminal {
    Terminal::builder()
        .app(TerminalApp::Wezterm)
        .is_tty(true)
        .image_support(ImageSupport::Kitty)
        .width(80)
        .cell_size(CellSize {
            width: 8,
            height: 16,
        })
        .build()
}

fn kitty_rows(output: &str) -> Option<u32> {
    output
        .split('\x1b')
        .find_map(|segment| segment.strip_prefix("_G"))
        .and_then(|header_and_data| header_and_data.split_once(';').map(|(header, _)| header))
        .and_then(|header| {
            header
                .split(',')
                .find_map(|part| part.strip_prefix("r="))
                .and_then(|rows| rows.parse().ok())
        })
}

fn scaled_png_sibling(path: &std::path::Path) -> std::path::PathBuf {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .expect("path should have a UTF-8 stem");
    path.with_file_name(format!("{stem}-h125.png"))
}

// ---------------------------------------------------------------------------
// Unbounded-width guard (spec C3)
// ---------------------------------------------------------------------------

#[test]
fn fill_hugs_on_degenerate_terminal_width() {
    let dims = TerminalImage::resolve_dimensions_for(
        &ImageWidth::Fill,
        &Layout::default(),
        1,
    );
    assert_eq!(
        dims.image_width, 1,
        "Fill collapses to 1 cell on a degenerate 1-cell terminal: {:?}",
        dims,
    );
}
