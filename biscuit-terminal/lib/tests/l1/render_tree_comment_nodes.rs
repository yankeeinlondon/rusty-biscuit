//! The terminal renderer shows nothing for a raw HTML node that holds only
//! comments, in every strictness, with no diagnostic: a comment has no
//! visible content, and the shared Markdown writer puts one between two
//! touching code spans. Other raw HTML keeps the strictness policy (Warn
//! prints it with a diagnostic, Strict rejects, Lossy drops).

use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use renderable::tree::{RenderError, RenderNode, RenderStrictness};

const STRICTNESS: [RenderStrictness; 3] = [
    RenderStrictness::Strict,
    RenderStrictness::Warn,
    RenderStrictness::Lossy,
];

fn render(node: &RenderNode, strictness: RenderStrictness) -> Result<(String, usize), RenderError> {
    let term = Terminal::new_optimistic(80);
    render_terminal_node(node, &TerminalRenderOptions::new(&term, strictness))
        .map(|rendered| (rendered.output, rendered.diagnostics.len()))
}

fn between_text(html: RenderNode) -> RenderNode {
    RenderNode::paragraph(vec![RenderNode::text("x"), html, RenderNode::text("y")])
}

#[test]
fn comment_nodes_render_nothing_in_every_mode() {
    let nodes = [
        between_text(RenderNode::html("<!-- -->", false)),
        between_text(RenderNode::html("<!---->", false)),
        RenderNode::root(vec![
            RenderNode::paragraph(vec![RenderNode::text("x")]),
            RenderNode::html("<!-- a -->\n<!-- b -->\n", true),
            RenderNode::paragraph(vec![RenderNode::text("y")]),
        ]),
    ];
    for node in &nodes {
        for strictness in STRICTNESS {
            let (output, diagnostics) =
                render(node, strictness).unwrap_or_else(|error| panic!("{strictness:?}: {error}"));
            assert!(!output.contains("<!--"), "{strictness:?}: {output:?}");
            assert!(output.contains('x') && output.contains('y'), "{strictness:?}: {output:?}");
            assert_eq!(diagnostics, 0, "{strictness:?}: {output:?}");
        }
    }
    let (inline, _) = render(&nodes[0], RenderStrictness::Strict).expect("render");
    assert_eq!(inline, "xy");
}

/// A real tag, a comment followed by other content, and a tag whose quoted
/// attribute holds a comment are not comment-only.
#[test]
fn other_raw_html_keeps_the_strictness_policy() {
    for value in ["<b>z</b>", "<!-- a --> z", "<span title=\"<!-- -->\">"] {
        let node = between_text(RenderNode::html(value, false));
        assert!(
            matches!(render(&node, RenderStrictness::Strict), Err(RenderError::LossyRejected { .. })),
            "{value:?}"
        );
        let (warned, diagnostics) = render(&node, RenderStrictness::Warn).expect("warn renders");
        assert!(warned.contains(value), "{value:?}: {warned:?}");
        assert_eq!(diagnostics, 1, "{value:?}");
        let (lossy, _) = render(&node, RenderStrictness::Lossy).expect("lossy renders");
        assert_eq!(lossy, "xy", "{value:?}");
    }
}

/// Comment-shaped code and text are content.
#[test]
fn comment_shaped_code_and_text_stay_visible() {
    let node = RenderNode::paragraph(vec![
        RenderNode::inline_code("<!-- c -->"),
        RenderNode::text(" <!-- t -->"),
    ]);
    for strictness in STRICTNESS {
        let (output, _) = render(&node, strictness).expect("render");
        assert!(output.contains("<!-- c -->"), "{strictness:?}: {output:?}");
        assert!(output.contains("<!-- t -->"), "{strictness:?}: {output:?}");
    }
}
