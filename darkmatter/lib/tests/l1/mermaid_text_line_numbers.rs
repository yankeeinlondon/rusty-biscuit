//! A Mermaid text drawing laid out by Darkmatter's code renderer must fit the
//! fence's columns once the renderer has added its line-number gutter.
//!
//! The diagram hook judges the drawing's fit before the code renderer
//! decorates it, and the gutter grows with the row count (digits plus three
//! cells), so the generic renderer measures the finished rows and shows the
//! fence as code when one is wider than the fence. These tests install the
//! real code renderer and the real hook on explicit capabilities (TrueColor,
//! no images), so the host terminal plays no part.

// The raw `CodeRenderer` hook is the subject: it is what a numbered gutter
// comes from, and the page path would read the host terminal.
#![allow(deprecated)]

use std::rc::Rc;

use biscuit_terminal::discovery::detection::{ColorDepth, ImageSupport};
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::block_constraint::visible_width;
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use biscuit_visualized::components::{MermaidFallbackPolicy, TerminalDiagramRenderer, TextFallback};
use darkmatter::markdown::TerminalCodeRenderer;
use renderable::tree::{GraphicsMode, RenderNode, RenderStrictness, TerminalMermaidMode};

/// Canonically 66 cells wide, compactly 58.
const CHAIN: &str = "flowchart LR\n    A[Alpha] --> B[Beta] --> C[Gamma] --> D[Delta] --> E[Epsilon]";

/// Canonical only, 12 cells wide and more than ten rows tall, so a numbered
/// gutter takes two digits.
const STATE: &str = "stateDiagram-v2\n    [*] --> Idle\n    Idle --> Running : start\n    Running --> Finished : done\n    Finished --> [*]";

fn options(width: u32, line_numbers: bool) -> TerminalRenderOptions {
    let term = Terminal::builder()
        .width(width)
        .color_depth(ColorDepth::TrueColor)
        .image_support(ImageSupport::None)
        .supports_unicode(true)
        .is_tty(false)
        .build();
    // An explicit policy, so `MERMAID_FALLBACK` in the environment is not read.
    let policy = MermaidFallbackPolicy::default()
        .with_text_fallback(TextFallback::Allowed)
        .with_show_partial_note(true);
    let mut opts = TerminalRenderOptions::new(&term, RenderStrictness::Strict)
        .with_code_renderer(Rc::new(TerminalCodeRenderer::new()))
        .with_diagram_renderer(Rc::new(TerminalDiagramRenderer::default().with_fallback_policy(policy)));
    opts.context.graphics_mode = GraphicsMode::Vector;
    opts.context.mermaid_mode = TerminalMermaidMode::Automatic;
    opts.context.line_numbers = line_numbers;
    opts
}

fn fence(source: &str) -> RenderNode {
    RenderNode::code(Some("mermaid".to_string()), None, source)
}

fn in_list_item(node: RenderNode) -> RenderNode {
    RenderNode::list(
        false,
        None,
        vec![RenderNode::list_item(None, vec![RenderNode::paragraph(vec![RenderNode::text("step")]), node])],
    )
}

/// The rendered output and its widest row in display cells.
fn render(node: &RenderNode, opts: &TerminalRenderOptions) -> (String, u32) {
    let rendered = render_terminal_node(node, opts).expect("render");
    assert!(rendered.diagnostics.is_empty(), "a decline is silent: {:?}", rendered.diagnostics);
    let widest = rendered.output.split('\n').map(visible_width).max().unwrap_or(0);
    (strip_escape_codes(&rendered.output), widest)
}

/// Whether `plain` shows a drawing: its labels, and none of its source.
fn drawn(plain: &str) -> bool {
    let labels = plain.contains("Epsilon") || plain.contains("Running");
    labels && !plain.contains("A[Alpha]") && !plain.contains("Idle --> Running")
}

#[test]
fn unnumbered_chain_fits_sixty_columns_as_text() {
    let (plain, widest) = render(&fence(CHAIN), &options(60, false));

    assert!(drawn(&plain), "{plain}");
    assert_eq!(widest, 60, "{plain}");
}

#[test]
fn a_one_digit_gutter_that_overflows_sixty_columns_shows_the_source() {
    let opts = options(60, true);

    let (plain, _) = render(&fence(CHAIN), &opts);

    assert!(plain.contains("A[Alpha] --> B[Beta]"), "the drawing would be 62 cells: {plain}");
    let mut as_code = opts.clone();
    as_code.context.mermaid_mode = TerminalMermaidMode::Code;
    let (code, _) = render(&fence(CHAIN), &as_code);
    assert_eq!(plain, code, "the ordinary code block, numbered as any other");
}

#[test]
fn a_one_digit_gutter_with_room_keeps_the_text() {
    for width in [62, 64] {
        let (plain, widest) = render(&fence(CHAIN), &options(width, true));

        assert!(drawn(&plain), "{width}: {plain}");
        assert!(widest <= width, "{width}: widest row {widest}: {plain}");
    }
}

#[test]
fn a_list_item_measures_the_numbered_drawing_against_its_own_width() {
    let (unnumbered, widest) = render(&in_list_item(fence(CHAIN)), &options(62, false));
    assert!(drawn(&unnumbered), "{unnumbered}");
    assert!(widest <= 62, "widest row {widest}: {unnumbered}");

    let (numbered, _) = render(&in_list_item(fence(CHAIN)), &options(62, true));
    assert!(numbered.contains("step"), "{numbered}");
    assert!(numbered.contains("A[Alpha] --> B[Beta]"), "source inside the item: {numbered}");
    assert!(!drawn(&numbered), "{numbered}");
    let indent = numbered.lines().find_map(|row| row.find("step")).expect("item row");
    let source_row = numbered.lines().find(|row| row.contains("A[Alpha]")).expect("source row");
    assert!(source_row.starts_with(&" ".repeat(indent)), "the source stays inside the item: {numbered}");
}

#[test]
fn a_two_digit_gutter_on_the_state_drawing_shows_the_source() {
    // Unnumbered, the 12-cell drawing fits 14 columns.
    let (unnumbered, widest) = render(&fence(STATE), &options(14, false));
    assert!(drawn(&unnumbered), "{unnumbered}");
    assert!(widest <= 14, "widest row {widest}: {unnumbered}");
    assert!(unnumbered.lines().count() >= 10, "the gutter needs two digits: {unnumbered}");

    // Numbered, each row would be 12 + 2 + 3 = 17 cells.
    let (numbered, _) = render(&fence(STATE), &options(14, true));
    assert!(numbered.contains("Idle --> Running"), "{numbered}");
}

#[test]
fn the_partial_note_wraps_within_a_narrow_fence() {
    // `click` is a feature the text drawing drops, so the drawing is partial
    // and the note follows it. The note wraps (breaking long words), so it
    // never makes the fragment wider than the fence.
    let partial = "flowchart TB\n    A[Write] --> B{Ok?}\n    B -- yes --> C[Plan]\n    click C \"https://example.com\"";

    let (plain, widest) = render(&fence(partial), &options(12, false));

    assert!(plain.contains("Write") && !plain.contains("click C"), "{plain}");
    assert!(plain.contains("Note:") && plain.contains("warning]."), "{plain}");
    assert!(widest <= 12, "widest row {widest}: {plain}");
}
