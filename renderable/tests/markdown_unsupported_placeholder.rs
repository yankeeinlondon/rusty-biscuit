//! The `Warn` placeholder comment for unsupported content.
//!
//! Under `Warn` an unsupported node is written as
//! `<!-- unsupported: {label} -->`. The label is authored, so it is made
//! comment-safe: a `>` is written `&gt;`, so no `-->` or `--!>` in the label
//! can end the comment early, and a line ending is written as a space, so
//! the placeholder stays on its line in a paragraph, heading, or table row.
//!
//! Each case is read back with an independent GFM reader (`pulldown-cmark`):
//! the comment must be one HTML token holding the whole placeholder, and the
//! content around it must be intact.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use renderable::tree::{
    ColumnAlign, DiagnosticKind, HeadingDepth, MarkdownDialect, MarkdownRenderOptions, RenderNode,
    RenderStrictness, render_markdown_node,
};

const DIALECTS: [MarkdownDialect; 2] = [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus];

/// Labels that could end or break the comment, and the label written.
const LABELS: [(&str, &str); 9] = [
    ("a-->b", "a--&gt;b"),
    ("a--!>b", "a--!&gt;b"),
    (">a", "&gt;a"),
    ("->a", "-&gt;a"),
    ("a-->", "a--&gt;"),
    ("a\nb", "a b"),
    ("a\r\n\r\nb", "a  b"),
    ("x|y-->z", "x|y--&gt;z"),
    ("plain", "plain"),
];

fn options(dialect: MarkdownDialect) -> MarkdownRenderOptions {
    MarkdownRenderOptions {
        dialect,
        strictness: RenderStrictness::Warn,
        style: None,
    }
}

/// What a reader finds in one paragraph, heading, HTML block, or table
/// cell: its text and other events, and its HTML tokens.
#[derive(Debug, Default, PartialEq)]
struct Reading {
    text: String,
    html: String,
}

fn read(markdown: &str) -> Vec<Reading> {
    let mut blocks = Vec::new();
    let mut current: Option<Reading> = None;
    let parser = Parser::new_ext(markdown, Options::ENABLE_TABLES);
    for event in parser {
        match event {
            Event::Start(Tag::Paragraph | Tag::Heading { .. } | Tag::TableCell | Tag::HtmlBlock) => {
                current = Some(Reading::default());
            }
            Event::End(TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::TableCell | TagEnd::HtmlBlock) => {
                blocks.push(current.take().expect("block open"));
            }
            Event::Text(text) => current.as_mut().expect("in a block").text.push_str(&text),
            Event::InlineHtml(html) | Event::Html(html) => {
                current.as_mut().expect("in a block").html.push_str(&html);
            }
            // Table structure lies outside every block read here.
            other => {
                if let Some(reading) = current.as_mut() {
                    reading.text.push_str(&format!("{other:?}"));
                }
            }
        }
    }
    blocks
}

#[test]
fn a_label_cannot_end_the_placeholder_comment_in_any_place() {
    let depth = HeadingDepth::new(2).expect("valid depth");
    for (label, written) in LABELS {
        let comment = format!("<!-- unsupported: {written} -->");
        // The cell reader keeps the `\|` escape inside an inline HTML token.
        let cell_comment = comment.replace('|', "\\|");
        let inline = |node: RenderNode| vec![RenderNode::text("p "), node, RenderNode::text(" q")];
        let unsupported = || RenderNode::unsupported(label);
        let places: Vec<(&str, RenderNode, Vec<Reading>)> = vec![
            (
                "paragraph",
                RenderNode::root(vec![RenderNode::paragraph(inline(unsupported()))]),
                vec![Reading {
                    text: "p  q".into(),
                    html: comment.clone(),
                }],
            ),
            (
                "heading",
                RenderNode::root(vec![RenderNode::heading(depth, inline(unsupported()))]),
                vec![Reading {
                    text: "p  q".into(),
                    html: comment.clone(),
                }],
            ),
            (
                "block, then a paragraph",
                RenderNode::root(vec![
                    unsupported(),
                    RenderNode::paragraph(vec![RenderNode::text("after")]),
                ]),
                vec![
                    Reading {
                        text: String::new(),
                        html: format!("{comment}\n"),
                    },
                    Reading {
                        text: "after".into(),
                        html: String::new(),
                    },
                ],
            ),
            (
                "table cell",
                RenderNode::root(vec![RenderNode::table(
                    vec![ColumnAlign::None, ColumnAlign::None],
                    vec![
                        RenderNode::table_row(vec![
                            RenderNode::table_cell(vec![RenderNode::text("H1")]),
                            RenderNode::table_cell(vec![unsupported()]),
                        ]),
                        RenderNode::table_row(vec![
                            RenderNode::table_cell(vec![unsupported()]),
                            RenderNode::table_cell(vec![RenderNode::text("z")]),
                        ]),
                    ],
                )]),
                vec![
                    Reading {
                        text: "H1".into(),
                        html: String::new(),
                    },
                    Reading {
                        text: String::new(),
                        html: cell_comment.clone(),
                    },
                    Reading {
                        text: String::new(),
                        html: cell_comment.clone(),
                    },
                    Reading {
                        text: "z".into(),
                        html: String::new(),
                    },
                ],
            ),
        ];
        for dialect in DIALECTS {
            for (place, tree, wanted) in &places {
                let rendered = render_markdown_node(tree, &options(dialect))
                    .unwrap_or_else(|error| panic!("{label:?} {place}: {error}"));
                assert_eq!(
                    rendered
                        .diagnostics
                        .iter()
                        .filter(|d| d.kind == DiagnosticKind::Unsupported)
                        .count(),
                    if *place == "table cell" { 2 } else { 1 },
                    "{label:?} {place}: {:?}",
                    rendered.diagnostics
                );
                // The HTML comment closes only at its own end: no `-->` or
                // `--!>` (the HTML reader's other closer) inside, and the
                // text does not begin with `>` or `->`.
                let body = &comment["<!--".len()..comment.len() - "-->".len()];
                assert!(
                    !body.contains("-->") && !body.contains("--!>"),
                    "{label:?}: {comment}"
                );
                assert!(!body.starts_with('>') && !body.starts_with("->"));
                assert_eq!(
                    read(&rendered.output),
                    *wanted,
                    "{label:?} {dialect:?} {place}\n{}",
                    rendered.output
                );
            }
        }
    }
}
