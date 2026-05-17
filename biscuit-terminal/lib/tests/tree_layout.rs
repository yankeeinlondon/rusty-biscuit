//! Integration tests: the terminal tree renderer applies node-level `Layout`.

use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use renderable::layout::{Layout, Length, Margin};
use renderable::tree::RenderNode;

#[test]
fn terminal_renderer_applies_left_margin_in_cells() {
    let mut para = RenderNode::paragraph(vec![RenderNode::text("hello")]);
    para.attrs.set_layout(&Layout {
        margin: Margin::x(Length::ch(4)),
        ..Layout::default()
    });
    let root = RenderNode::root(vec![para]);

    let opts = TerminalRenderOptions::default();
    let rendered = render_terminal_node(&root, &opts).unwrap();
    let first = rendered.output.lines().next().unwrap_or_default();
    let lead = first.len() - first.trim_start().len();
    assert!(lead >= 4, "expected >=4 leading cells, got {lead}: {first:?}");
}

#[test]
fn terminal_renderer_resolves_percent_margin_against_width() {
    let mut para = RenderNode::paragraph(vec![RenderNode::text("x")]);
    para.attrs.set_layout(&Layout {
        margin: Margin::x(Length::percent(10.0).unwrap()),
        ..Layout::default()
    });
    let root = RenderNode::root(vec![para]);

    let mut opts = TerminalRenderOptions::default();
    opts.context.width = 80;
    opts.context.available_width = 80;
    let rendered = render_terminal_node(&root, &opts).unwrap();
    let first = rendered.output.lines().next().unwrap_or_default();
    let lead = first.len() - first.trim_start().len();
    assert_eq!(lead, 8, "10% of 80 should resolve to 8 cells: {first:?}");
}
