//! Raw `Html` payloads inside block quotes and list items keep their line
//! endings and stay inside their container.
//!
//! The Markdown writer prefixes every line of a block quote with `> ` and
//! indents every continuation line of a list item under its marker. A
//! CommonMark reader ends a line at a line feed, a carriage return, or a
//! CRLF, so a payload's lone CR must get the prefix too (or the rest of the
//! payload escapes the container), and a CRLF must keep its CR (raw bytes are
//! the author's).
//!
//! Each case is checked on the rendered bytes (the payload with the
//! container's continuation prefix after each of its line endings) and on an
//! independent `pulldown-cmark` reading that puts all of the payload's
//! content inside the container. `pulldown-cmark` 0.13 does not end a line
//! at a lone CR inside an HTML or code block (the commonmark.js reference
//! does), so it reads the output with every line ending spelled as LF, which
//! CommonMark defines as the same line structure.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use renderable::tree::{
    MarkdownDialect, MarkdownRenderOptions, RenderNode, RenderStrictness, render_markdown_node,
};

const DIALECTS: [MarkdownDialect; 2] = [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus];

/// Payloads, by name, and whether each is a block (`true`) or inline node.
fn payloads() -> Vec<(&'static str, &'static str, bool)> {
    vec![
        ("block, lone CR", "<div>\r</div>", true),
        ("block, CRLF", "<div>\r\n</div>", true),
        ("block, LF", "<div>\n</div>", true),
        ("block, CR then LF line", "<div>a\rb\nc</div>", true),
        ("inline, lone CR", "<i>a</i>\r<i>b</i>", false),
        ("inline, CRLF", "<i>a</i>\r\n<i>b</i>", false),
        ("inline, LF", "<i>a</i>\n<i>b</i>", false),
    ]
}

fn content(payload: &str, block: bool) -> RenderNode {
    if block {
        RenderNode::html(payload, true)
    } else {
        RenderNode::paragraph(vec![RenderNode::html(payload, false)])
    }
}

/// A container placement: how to build it, the prefix the writer puts on
/// each continuation line of its content, and the container tags a reader
/// must have open around every piece of that content.
struct Placement {
    name: &'static str,
    build: fn(RenderNode) -> RenderNode,
    continuation: &'static str,
    inside: fn(&Open) -> bool,
}

/// How many block quotes and list items a reader has open.
#[derive(Default)]
struct Open {
    quotes: usize,
    items: usize,
}

fn placements() -> Vec<Placement> {
    vec![
        Placement {
            name: "block quote",
            build: |c| RenderNode::block_quote(vec![c]),
            continuation: "> ",
            inside: |o| o.quotes > 0,
        },
        Placement {
            name: "unordered list item",
            build: |c| RenderNode::list(false, None, vec![RenderNode::list_item(None, vec![c])]),
            continuation: "  ",
            inside: |o| o.items > 0,
        },
        Placement {
            name: "ordered list item",
            build: |c| RenderNode::list(true, None, vec![RenderNode::list_item(None, vec![c])]),
            continuation: "   ",
            inside: |o| o.items > 0,
        },
        Placement {
            name: "block quote in list item",
            build: |c| {
                RenderNode::list(
                    false,
                    None,
                    vec![RenderNode::list_item(
                        None,
                        vec![RenderNode::block_quote(vec![c])],
                    )],
                )
            },
            continuation: "  > ",
            inside: |o| o.items > 0 && o.quotes > 0,
        },
    ]
}

/// `payload` with `prefix` after each line ending, a reader's line endings
/// being LF, CR, and CRLF.
fn with_continuation(payload: &str, prefix: &str) -> String {
    let mut out = String::new();
    let mut chars = payload.chars().peekable();
    while let Some(c) = chars.next() {
        out.push(c);
        let ends_line = c == '\n' || (c == '\r' && chars.peek() != Some(&'\n'));
        if ends_line && chars.peek().is_some() {
            out.push_str(prefix);
        }
    }
    out
}

/// CRLF and CR spelled as LF.
fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// The content a reader places inside and outside the container, with line
/// endings as LF.
fn read(markdown: &str, inside: fn(&Open) -> bool) -> (String, String) {
    let mut open = Open::default();
    let (mut kept, mut escaped) = (String::new(), String::new());
    for event in Parser::new_ext(&normalize(markdown), Options::all()) {
        let piece = match event {
            Event::Start(Tag::BlockQuote(_)) => {
                open.quotes += 1;
                continue;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                open.quotes -= 1;
                continue;
            }
            Event::Start(Tag::Item) => {
                open.items += 1;
                continue;
            }
            Event::End(TagEnd::Item) => {
                open.items -= 1;
                continue;
            }
            Event::Html(s) | Event::InlineHtml(s) | Event::Text(s) => s.to_string(),
            Event::SoftBreak | Event::HardBreak => "\n".to_string(),
            _ => continue,
        };
        if inside(&open) {
            kept.push_str(&piece);
        } else {
            escaped.push_str(&piece);
        }
    }
    (kept, escaped)
}

#[test]
fn raw_html_line_endings_survive_container_prefixes_and_stay_inside() {
    for (shape, payload, block) in payloads() {
        for placement in placements() {
            let node = (placement.build)(content(payload, block));
            for dialect in DIALECTS {
                let case = format!("{shape} / {} / {dialect:?}", placement.name);
                let options = MarkdownRenderOptions {
                    dialect,
                    strictness: RenderStrictness::Lossy,
                    style: None,
                };
                let markdown = render_markdown_node(&node, &options)
                    .unwrap_or_else(|e| panic!("{case}: render failed: {e}"))
                    .output;

                let expected = with_continuation(payload, placement.continuation);
                assert!(
                    markdown.contains(&expected),
                    "{case}: expected {expected:?} in {markdown:?}"
                );

                let (kept, escaped) = read(&markdown, placement.inside);
                assert_eq!(
                    escaped, "",
                    "{case}: content escaped the container in {markdown:?}"
                );
                assert!(
                    kept.contains(&normalize(payload)),
                    "{case}: reader kept {kept:?} from {markdown:?}"
                );
            }
        }
    }
}

#[test]
fn a_raw_crlf_ending_a_list_item_keeps_its_carriage_return() {
    // The item's body ends with the payload's CRLF; the list joins items
    // with a line feed, which completes the same CRLF.
    let list = RenderNode::list(
        false,
        None,
        vec![
            RenderNode::list_item(None, vec![RenderNode::html("<div>a</div>\r\n", true)]),
            RenderNode::list_item(
                None,
                vec![RenderNode::paragraph(vec![RenderNode::text("b")])],
            ),
        ],
    );
    for dialect in DIALECTS {
        let options = MarkdownRenderOptions {
            dialect,
            strictness: RenderStrictness::Lossy,
            style: None,
        };
        let markdown = render_markdown_node(&list, &options)
            .unwrap_or_else(|e| panic!("{dialect:?}: render failed: {e}"))
            .output;
        assert!(
            markdown.contains("- <div>a</div>\r\n- b"),
            "{dialect:?}: {markdown:?}"
        );
    }
}
