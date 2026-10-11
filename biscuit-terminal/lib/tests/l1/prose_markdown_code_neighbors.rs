//! Neighboring code spans in `Prose` and `InlineProse` Markdown output.
//!
//! Two code spans with nothing visible between them, after a tag that writes
//! no Markdown of its own is removed, must not be written as touching
//! backtick fences: `` `a``b` `` is one code span holding ```` a``b ````.
//! Each case renders Markdown and MarkdownPlus and reads the output with
//! `pulldown-cmark`, an independent CommonMark reader.
//!
//! The comment that separates the spans has no visible content, so both
//! components reading their own Markdown back must produce the two code
//! values and nothing else, and the terminal and browser renderers must show
//! nothing between them. Comment-shaped text in code, escaped text, and a
//! quoted attribute stays content.

use biscuit_terminal::components::prose::{InlineProse, Prose};
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use pulldown_cmark::{Event, Parser};
use renderable::markdown::MarkdownRenderable;
use renderable::tree::render::{BrowserRenderOptions, render_browser_node};
use renderable::tree::{NodeKind, RenderNode, RenderStrictness, TreeRenderable};

/// The decoded code values and the visible text of `markdown`.
fn read_back(markdown: &str) -> (Vec<String>, String) {
    let mut codes = Vec::new();
    let mut visible = String::new();
    for event in Parser::new(markdown) {
        match event {
            Event::Code(code) => {
                visible.push_str(&code);
                codes.push(code.to_string());
            }
            Event::Text(text) => visible.push_str(&text),
            _ => {}
        }
    }
    (codes, visible)
}

fn assert_values(label: &str, markdown: &str) {
    assert_eq!(
        read_back(markdown),
        (vec!["a".to_string(), "b".to_string()], "ab".to_string()),
        "{label}\nMarkdown: {markdown:?}"
    );
}

/// `<clipboard>` and an empty `<b></b>` write no Markdown, and `<dim>` has
/// none in plain Markdown, so the two code spans meet.
const SOURCES: [&str; 3] = [
    "`a`<clipboard>`b`</clipboard>",
    "`a`<b></b>`b`",
    "`a`<dim>`b`</dim>",
];

#[test]
fn prose_keeps_neighboring_code_values() {
    for source in SOURCES {
        let prose = Prose::new(source);
        assert_values(&format!("Prose/Markdown {source:?}"), &prose.render_markdown());
        assert_values(
            &format!("Prose/MarkdownPlus {source:?}"),
            &prose.render_markdown_plus(),
        );
    }
}

#[test]
fn inline_prose_keeps_neighboring_code_values() {
    for source in SOURCES {
        let inline = InlineProse::new(source);
        assert_values(&format!("InlineProse/Markdown {source:?}"), &inline.render_markdown());
        assert_values(
            &format!("InlineProse/MarkdownPlus {source:?}"),
            &inline.render_markdown_plus(),
        );
    }
}

/// `InlineProse` normalizes an explicit code block to inline code, which
/// then meets the code span before it.
#[test]
fn inline_prose_keeps_a_normalized_code_block_beside_a_code_span() {
    let inline = InlineProse::new("`a`<code-block>b</code-block>");
    assert_values("InlineProse/Markdown", &inline.render_markdown());
    assert_values("InlineProse/MarkdownPlus", &inline.render_markdown_plus());
}

/// The boundary is an empty HTML comment, which neither dialect reports as
/// a loss.
#[test]
fn neighboring_code_spans_are_separated_by_an_empty_comment() {
    let prose = Prose::new("`a`<clipboard>`b`</clipboard>");
    assert_eq!(prose.render_markdown(), "`a`<!-- -->`b`");
    assert_eq!(prose.render_markdown_plus(), "`a`<!-- -->`b`");
}

/// The leaves of a parsed tree, in order, as `(kind, value)`.
fn leaves(node: &RenderNode, out: &mut Vec<(&'static str, String)>) {
    match &node.kind {
        NodeKind::Text { value } => out.push(("text", value.clone())),
        NodeKind::InlineCode { value } => out.push(("code", value.clone())),
        NodeKind::Code { value, .. } => out.push(("block code", value.clone())),
        NodeKind::Html { value, .. } => out.push(("html", value.clone())),
        NodeKind::SoftBreak | NodeKind::HardBreak => out.push(("break", String::new())),
        NodeKind::Link { url, .. } => {
            out.push(("link", url.clone()));
            node.children().iter().for_each(|child| leaves(child, out));
        }
        _ => node.children().iter().for_each(|child| leaves(child, out)),
    }
}

fn tree_leaves(tree: &RenderNode) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    leaves(tree, &mut out);
    out
}

const STRICTNESS: [RenderStrictness; 3] = [
    RenderStrictness::Strict,
    RenderStrictness::Warn,
    RenderStrictness::Lossy,
];

/// Renders `tree` to the terminal in every strictness and to HTML under the
/// default (escaping) raw-HTML policy, and checks that nothing shows between
/// the two code values.
fn assert_renders_only_the_values(label: &str, tree: &RenderNode) {
    let term = Terminal::new_optimistic(80);
    for strictness in STRICTNESS {
        let rendered = render_terminal_node(tree, &TerminalRenderOptions::new(&term, strictness))
            .unwrap_or_else(|error| panic!("{label} {strictness:?}: {error}"));
        let plain = strip_ansi(&rendered.output);
        assert!(!plain.contains("<!--"), "{label} {strictness:?}: {plain:?}");
        assert!(plain.contains('a') && plain.contains('b'), "{label} {strictness:?}: {plain:?}");
        assert!(rendered.diagnostics.is_empty(), "{label} {strictness:?}");
    }
    let html = render_browser_node(tree, &BrowserRenderOptions::default())
        .unwrap_or_else(|error| panic!("{label} html: {error}"));
    let html_text = html.output.render();
    assert!(html_text.contains("<code>a</code><code>b</code>"), "{label}: {html_text}");
    assert!(!html_text.contains("&lt;!--"), "{label}: {html_text}");
    assert!(html.diagnostics.is_empty(), "{label}");
}

fn strip_ansi(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.next_if_eq(&'[').is_some() {
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else if chars.next_if_eq(&']').is_some() {
                while let Some(c) = chars.next() {
                    if c == '\u{7}' || (c == '\u{1b}' && chars.next_if_eq(&'\\').is_some()) {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

fn two_values() -> Vec<(&'static str, String)> {
    vec![("code", "a".to_string()), ("code", "b".to_string())]
}

/// `Prose` reads the Markdown it writes back as the two code values: the
/// separator comment contributes no node, and no target shows it.
#[test]
fn prose_reads_its_own_separated_code_spans() {
    for source in SOURCES {
        let prose = Prose::new(source);
        for (dialect, markdown) in [
            ("Markdown", prose.render_markdown()),
            ("MarkdownPlus", prose.render_markdown_plus()),
        ] {
            assert!(markdown.contains("<!-- -->"), "{source:?} {dialect}: {markdown:?}");
            let label = format!("Prose/{dialect} {source:?} -> {markdown:?}");
            let tree = Prose::new(markdown.as_str()).render_tree();
            assert_eq!(tree_leaves(&tree), two_values(), "{label}");
            assert_renders_only_the_values(&label, &tree);
        }
    }
}

#[test]
fn inline_prose_reads_its_own_separated_code_spans() {
    let sources = SOURCES.iter().copied().chain(["`a`<code-block>b</code-block>"]);
    for source in sources {
        let inline = InlineProse::new(source);
        for (dialect, markdown) in [
            ("Markdown", inline.render_markdown()),
            ("MarkdownPlus", inline.render_markdown_plus()),
        ] {
            assert!(markdown.contains("<!-- -->"), "{source:?} {dialect}: {markdown:?}");
            let label = format!("InlineProse/{dialect} {source:?} -> {markdown:?}");
            let tree = InlineProse::new(markdown.as_str()).render_tree();
            assert_eq!(tree_leaves(&tree), two_values(), "{label}");
            assert_renders_only_the_values(&label, &tree);
        }
    }
}

/// A comment contributes no node anywhere outside code, and a paragraph of
/// only comments is no paragraph.
#[test]
fn comments_contribute_no_content() {
    let text = |value: &str| ("text", value.to_string());
    let cases: [(&str, Vec<(&'static str, String)>); 5] = [
        ("x<!-- c -->y", vec![text("xy")]),
        ("x<!---->y<!-->z<!--->w", vec![text("xyzw")]),
        ("**b<!-- c -->**", vec![text("b")]),
        ("one\n\n<!-- only -->\n\ntwo", vec![text("one"), text("two")]),
        ("<!-- `a` **b** -->c", vec![text("c")]),
    ];
    for (source, expected) in cases {
        assert_eq!(tree_leaves(&Prose::new(source).render_tree()), expected, "Prose {source:?}");
        if !source.contains("\n\n") {
            assert_eq!(
                tree_leaves(&InlineProse::new(source).render_tree()),
                expected,
                "InlineProse {source:?}"
            );
        }
    }
    let paragraphs = |tree: &RenderNode| {
        tree.children().iter().filter(|n| matches!(n.kind, NodeKind::Paragraph { .. })).count()
    };
    assert_eq!(paragraphs(&Prose::new("one\n\n<!-- only -->\n\ntwo").render_tree()), 2);
}

/// Comment-shaped text that is not an HTML comment in the grammar stays
/// content: code spans, fenced code, escaped text, a recognized tag's quoted
/// attribute, an unclosed comment, and one interrupted by a blank line.
#[test]
fn comment_shaped_content_is_kept() {
    let escaped = Prose::escape_text("<!-- x -->");
    let cases: Vec<(String, Vec<(&'static str, String)>)> = vec![
        ("`<!-- -->`".into(), vec![("code", "<!-- -->".into())]),
        (escaped.clone(), vec![("text", "<!-- x -->".into())]),
        (
            "<a href=\"x<!-- -->y\">t</a>".into(),
            vec![("link", "x<!-- -->y".into()), ("text", "t".into())],
        ),
        ("a <!-- open".into(), vec![("text", "a <!-- open".into())]),
    ];
    for (source, expected) in cases {
        assert_eq!(tree_leaves(&Prose::new(source.as_str()).render_tree()), expected, "Prose {source:?}");
        assert_eq!(
            tree_leaves(&InlineProse::new(source.as_str()).render_tree()),
            expected,
            "InlineProse {source:?}"
        );
    }
    assert_eq!(
        tree_leaves(&Prose::new("```\n<!-- kept -->\n```").render_tree()),
        [("block code", "<!-- kept -->".to_string())]
    );
    assert_eq!(
        tree_leaves(&Prose::new("a <!-- x\n\ny --> b").render_tree()),
        [("text", "a <!-- x".to_string()), ("text", "y --> b".to_string())]
    );
    // Escaped comment text survives a Markdown round trip as text.
    let markdown = Prose::new(escaped.as_str()).render_markdown();
    assert_eq!(
        tree_leaves(&Prose::new(markdown.as_str()).render_tree()),
        [("text", "<!-- x -->".to_string())],
        "{markdown:?}"
    );
}
