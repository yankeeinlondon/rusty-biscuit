//! A footnote definition keeps all of its content when read back.
//!
//! A GFM reader keeps a line inside a footnote definition only when it is
//! blank or indented by four spaces, so the writer indents every line of a
//! definition's body after the first. Without that, a second block, the
//! lines of a code block or list, or the line after a soft or hard break
//! would leave the footnote.
//!
//! Each case is read back with an independent GFM reader (`pulldown-cmark`
//! with GFM footnotes): the events inside the definition must equal the
//! events of the same blocks written at the top level between two
//! paragraphs, and the paragraphs around the definition must stay outside
//! it.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use renderable::tree::{
    MarkdownDialect, MarkdownRenderOptions, RenderNode, RenderStrictness, render_markdown_node,
};

const DIALECTS: [MarkdownDialect; 2] = [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus];

/// Line endings written inside code and raw HTML payloads.
const ENDINGS: [&str; 2] = ["\n", "\r\n"];

fn reader_options() -> Options {
    Options::ENABLE_FOOTNOTES
        | Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
}

fn render(node: &RenderNode, dialect: MarkdownDialect) -> String {
    let options = MarkdownRenderOptions {
        dialect,
        strictness: RenderStrictness::Lossy,
        style: None,
    };
    render_markdown_node(node, &options)
        .unwrap_or_else(|e| panic!("render failed: {e}"))
        .output
}

fn paragraph(text: &str) -> RenderNode {
    RenderNode::paragraph(vec![RenderNode::text(text)])
}

/// Definition bodies, by name. `eol` is the line ending inside code and raw
/// HTML payloads.
fn shapes(eol: &str) -> Vec<(&'static str, Vec<RenderNode>)> {
    let code = || {
        RenderNode::code(
            Some("rust".to_string()),
            None,
            format!("let a = 1;{eol}{eol}let b = 2;{eol}"),
        )
    };
    let list = || {
        RenderNode::list(
            false,
            None,
            vec![
                RenderNode::list_item(None, vec![paragraph("i1")]),
                RenderNode::list_item(
                    Some(true),
                    vec![
                        paragraph("i2"),
                        RenderNode::list(
                            true,
                            Some(3),
                            vec![RenderNode::list_item(None, vec![paragraph("n1")])],
                        ),
                    ],
                ),
            ],
        )
    };
    let breaks = || {
        RenderNode::paragraph(vec![
            RenderNode::text("s1"),
            RenderNode::soft_break(),
            RenderNode::text("s2"),
            RenderNode::hard_break(),
            RenderNode::text("h2"),
        ])
    };
    let html = || RenderNode::html(format!("<div>{eol}x{eol}</div>"), true);
    vec![
        ("two paragraphs", vec![paragraph("one"), paragraph("two")]),
        ("code block", vec![code()]),
        ("paragraph then code block", vec![paragraph("one"), code()]),
        ("list", vec![list()]),
        ("soft and hard breaks", vec![breaks()]),
        ("raw HTML block", vec![paragraph("one"), html()]),
        (
            "everything",
            vec![paragraph("one"), code(), list(), breaks(), html(), paragraph("two")],
        ),
    ]
}

/// `body` as a footnote definition, between a referencing paragraph and a
/// following top-level paragraph.
fn document(body: Vec<RenderNode>) -> RenderNode {
    RenderNode::root(vec![
        RenderNode::paragraph(vec![
            RenderNode::text("before"),
            RenderNode::footnote_reference("n"),
        ]),
        RenderNode::footnote_definition("n", body),
        paragraph("after"),
    ])
}

/// One reader event as compared here: adjacent text events merged (a
/// reader may split a code block's text at a blank line) and line endings
/// in text spelled as LF (a CRLF inside a code or HTML block is the same
/// line structure).
fn push_event(out: &mut Vec<String>, event: &Event<'_>) {
    if let Event::Text(text) = event {
        let text = text.replace("\r\n", "\n");
        if let Some(last) = out.last_mut().filter(|last| last.starts_with("Text:")) {
            last.push_str(&text);
        } else {
            out.push(format!("Text:{text}"));
        }
    } else {
        out.push(format!("{event:?}").replace("\\r\\n", "\\n"));
    }
}

fn events(markdown: &str) -> Vec<String> {
    let mut out = Vec::new();
    for event in Parser::new_ext(markdown, reader_options()) {
        push_event(&mut out, &event);
    }
    out
}

/// The events inside the definition labeled `n`, the events outside it, and
/// how many definitions the reader saw.
fn split(markdown: &str) -> (Vec<String>, Vec<String>, usize) {
    let (mut inside, mut outside, mut definitions) = (Vec::new(), Vec::new(), 0);
    let mut depth = 0usize;
    for event in Parser::new_ext(markdown, reader_options()) {
        match &event {
            Event::Start(Tag::FootnoteDefinition(label)) => {
                assert_eq!(&**label, "n", "unexpected definition in {markdown:?}");
                definitions += 1;
                depth += 1;
                continue;
            }
            Event::End(TagEnd::FootnoteDefinition) => {
                depth -= 1;
                continue;
            }
            _ => {}
        }
        push_event(if depth > 0 { &mut inside } else { &mut outside }, &event);
    }
    (inside, outside, definitions)
}

/// The problems a reader finds with `body` written as the definition of
/// `[^n]` (empty when it reads back whole).
fn problems(body: Vec<RenderNode>, dialect: MarkdownDialect) -> Vec<String> {
    // The same blocks between two top-level paragraphs, so the last block
    // is followed by a line ending as it is inside the document.
    let mut blocks = vec![paragraph("before")];
    blocks.extend(body.iter().cloned());
    blocks.push(paragraph("after"));
    let standalone = render(&RenderNode::root(blocks), dialect);
    let standalone_events = events(&standalone);
    assert_eq!(
        standalone_events[..3],
        ["Start(Paragraph)", "Text:before", "End(Paragraph)"],
        "{standalone:?}"
    );
    let expected_inside = &standalone_events[3..standalone_events.len() - 3];
    let markdown = render(&document(body), dialect);

    let (inside, outside, definitions) = split(&markdown);
    let outside_text: Vec<&str> = outside
        .iter()
        .filter(|e| e.starts_with("Text:") || e.starts_with("FootnoteReference"))
        .map(String::as_str)
        .collect();
    let outside_paragraphs = outside
        .iter()
        .filter(|e| e.starts_with("Start(Paragraph"))
        .count();

    let mut found = Vec::new();
    if definitions != 1 {
        found.push(format!("{definitions} definitions in {markdown:?}"));
    }
    if inside != expected_inside {
        found.push(format!(
            "definition reads {inside:?}, expected {expected_inside:?} (as in {standalone:?}), \
             in {markdown:?}"
        ));
    }
    if outside_text != ["Text:before", "FootnoteReference(Borrowed(\"n\"))", "Text:after"]
        || outside_paragraphs != 2
    {
        found.push(format!(
            "outside the definition: {outside_text:?} in {outside_paragraphs} paragraphs, \
             in {markdown:?}"
        ));
    }
    found
}

#[test]
fn multi_line_footnote_definitions_keep_all_their_content() {
    let mut cases = 0;
    let mut failures = Vec::new();
    for eol in ENDINGS {
        for (shape, body) in shapes(eol) {
            for dialect in DIALECTS {
                cases += 1;
                for problem in problems(body.clone(), dialect) {
                    failures.push(format!("{shape} / {eol:?} / {dialect:?}: {problem}"));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} problems in {cases} cases:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn single_paragraph_footnote_definition_is_one_line() {
    for dialect in DIALECTS {
        let markdown = render(&document(vec![paragraph("only")]), dialect);
        assert!(
            markdown.contains("\n\n[^n]: only\n\n"),
            "{dialect:?}: definition line in {markdown:?}"
        );
        assert_eq!(
            problems(vec![paragraph("only")], dialect),
            Vec::<String>::new(),
            "{dialect:?}"
        );
    }
}

#[test]
fn continuation_lines_are_indented_by_four_spaces() {
    let body = vec![
        paragraph("one"),
        RenderNode::code(None, None, "a\r\nb\r\n"),
    ];
    for dialect in DIALECTS {
        let markdown = render(&document(body.clone()), dialect);
        assert!(
            markdown.contains("[^n]: one\n\n    ```\n    a\r\n    b\r\n    ```\n\nafter"),
            "{dialect:?}: {markdown:?}"
        );
    }
}
