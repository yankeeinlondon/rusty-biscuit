//! Structural breaks keep their meaning in every Markdown context.
//!
//! A `SoftBreak` or `HardBreak` node is spelled for its position: a line
//! ending or backslash-line-ending where content follows on the next line,
//! and a space or `<br>` where a reader would strip the line ending, read the
//! backslash literally, end the paragraph, or open a raw HTML block. Each
//! case renders a break shape in a context, reads the Markdown back with an
//! independent CommonMark/GFM reader (`pulldown-cmark`), and compares what a
//! reader shows with what the tree means.
//!
//! The comparison is on visible meaning: a soft break and a space both read
//! as a space, and a hard break and an inline `<br>` both read as a line
//! break. Text, the requested structure, and paragraph boundaries must match
//! exactly.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use renderable::color::{BasicColor, Color};
use renderable::layout::TargetValue;
use renderable::style::{PerMode, Style, TextEmphasis, UnderlineStyle};
use renderable::tree::{
    ColumnAlign, HeadingDepth, MarkdownDialect, MarkdownRenderOptions, NodeKind, RenderNode,
    RenderStrictness, SequenceJoin, render_markdown_node,
};

const DIALECTS: [MarkdownDialect; 2] = [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus];

/// A hard break, in both readings.
const HB: char = '⏎';

fn render_in(node: &RenderNode, dialect: MarkdownDialect) -> String {
    let opts = MarkdownRenderOptions {
        dialect,
        strictness: RenderStrictness::Lossy,
        style: None,
    };
    render_markdown_node(node, &opts).expect("render").output
}

/// Inline HTML read as the structure it spells: `<br>` is a hard break, and
/// the HTML fallbacks for emphasis and styled spans are their tags.
fn inline_html(html: &str) -> String {
    let tag = html.trim().to_ascii_lowercase();
    if matches!(tag.as_str(), "<br>" | "<br/>" | "<br />") {
        return HB.to_string();
    }
    for name in ["strong", "em", "del"] {
        if tag == format!("<{name}>") || tag == format!("</{name}>") {
            return tag;
        }
    }
    if tag.starts_with("<span") {
        return "<span>".to_string();
    }
    if tag == "</span>" {
        return "</span>".to_string();
    }
    format!("{{html {html}}}")
}

/// The text of a raw HTML `<summary>`, as a browser shows it: a line
/// ending, written or as `&#10;`, is whitespace there.
fn summary_html(html: &str) -> Option<String> {
    let start = html.find("<summary>")? + "<summary>".len();
    let end = html.find("</summary>")?;
    let body = html[start..end]
        .replace("&#10;", "\n")
        .replace(['\n', '\r'], " ")
        .replace("<br>", &HB.to_string())
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    Some(format!("<summary>{body}</summary>"))
}

/// What a reader shows: text, a space per soft break, [`HB`] per hard
/// break, `¶` per paragraph, and structure as `<tag>` markers.
///
/// A line feed inside a text event (a `&#10;` reference) is whitespace to a
/// browser, so it reads as a space. A raw HTML block that holds only `<br>`
/// reads as `{lone br}`; see [`a_block_holding_only_a_hard_break_is_one_html_line_break`].
/// The lines of one raw HTML block are read together.
fn read_back(markdown: &str) -> String {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_FOOTNOTES;
    let mut out = String::new();
    let mut block: Option<String> = None;
    for event in Parser::new_ext(markdown, options) {
        match (&mut block, &event) {
            (_, Event::Start(Tag::HtmlBlock)) => {
                block = Some(String::new());
                continue;
            }
            (Some(html), Event::Html(line)) => {
                html.push_str(line);
                continue;
            }
            (Some(_), Event::End(TagEnd::HtmlBlock)) => {
                let html = block.take().unwrap_or_default();
                // The closing `</details>` ends the disclosure's last block.
                let rest = html.trim().trim_end_matches("</details>").trim();
                if let Some(summary) = summary_html(&html) {
                    out.push_str(&summary);
                } else if rest == "<br>" {
                    out.push_str("{lone br}");
                } else if !rest.is_empty() {
                    out.push_str(&format!("{{html block {html}}}"));
                }
                continue;
            }
            _ => {}
        }
        match event {
            Event::Text(text) => out.push_str(&text.replace(['\n', '\r'], " ")),
            Event::Code(code) => out.push_str(&format!("{{code:{code}}}")),
            Event::SoftBreak => out.push(' '),
            Event::HardBreak => out.push(HB),
            Event::InlineHtml(html) => out.push_str(&inline_html(&html)),
            Event::FootnoteReference(id) => out.push_str(&format!("{{fnref:{id}}}")),
            Event::Start(tag) => out.push_str(&match tag {
                Tag::Paragraph => "¶".to_string(),
                Tag::Strong => "<strong>".to_string(),
                Tag::Emphasis => "<em>".to_string(),
                Tag::Strikethrough => "<del>".to_string(),
                Tag::Heading { .. } => "<h>".to_string(),
                Tag::BlockQuote(_) => "<quote>".to_string(),
                Tag::List(_) => "<list>".to_string(),
                Tag::Item => "<item>".to_string(),
                Tag::TableCell => "<td>".to_string(),
                Tag::FootnoteDefinition(id) => format!("<fn {id}>"),
                Tag::Link { dest_url, .. } => format!("<a {dest_url}>"),
                _ => String::new(),
            }),
            Event::End(end) => out.push_str(match end {
                TagEnd::Strong => "</strong>",
                TagEnd::Emphasis => "</em>",
                TagEnd::Strikethrough => "</del>",
                TagEnd::Heading(_) => "</h>",
                TagEnd::BlockQuote(_) => "</quote>",
                TagEnd::List(_) => "</list>",
                TagEnd::Item => "</item>",
                TagEnd::TableCell => "</td>",
                TagEnd::FootnoteDefinition => "</fn>",
                TagEnd::Link => "</a>",
                _ => "",
            }),
            _ => {}
        }
    }
    out
}

/// What the tree means, in the reading of [`read_back`]: a soft break or a
/// literal line feed is a space, a hard break is [`HB`]. In a table cell a
/// literal line feed is a hard break, the cell's documented policy.
fn meaning(nodes: &[RenderNode], cell: bool) -> String {
    let mut out = String::new();
    for node in nodes {
        match &node.kind {
            NodeKind::Text { value } => {
                out.push_str(&value.replace('\n', if cell { "⏎" } else { " " }));
            }
            NodeKind::SoftBreak => out.push(' '),
            NodeKind::HardBreak => out.push(HB),
            NodeKind::Strong { children } => {
                out.push_str(&format!("<strong>{}</strong>", meaning(children, cell)));
            }
            other => panic!("shape node {other:?} has no reading here"),
        }
    }
    out
}

/// `open` and `close` around `value` after the delimiter-edge policy moves
/// the value's edge whitespace and breaks outside the wrapper.
fn wrapped(open: &str, value: &str, close: &str) -> String {
    let edge = |c: char| c == ' ' || c == HB;
    let core = value.trim_matches(edge);
    if core.is_empty() {
        return value.to_string();
    }
    let start = value.len() - value.trim_start_matches(edge).len();
    format!(
        "{}{open}{core}{close}{}",
        &value[..start],
        &value[start + core.len()..]
    )
}

fn text(value: &str) -> RenderNode {
    RenderNode::text(value)
}

fn para(children: Vec<RenderNode>) -> RenderNode {
    RenderNode::paragraph(children)
}

fn prepend(first: RenderNode, mut rest: Vec<RenderNode>) -> Vec<RenderNode> {
    rest.insert(0, first);
    rest
}

fn styled(style: Style, children: Vec<RenderNode>) -> RenderNode {
    let mut span = RenderNode::span(vec![], children);
    span.attrs.set_style(&style);
    span
}

fn color_style() -> Style {
    Style {
        color: Some(TargetValue::universal(PerMode::universal(
            Color::BasicColor(BasicColor::Red),
        ))),
        ..Default::default()
    }
}

fn underline_style() -> Style {
    Style {
        emphasis: TextEmphasis {
            underline: Some(UnderlineStyle::Straight),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn table(cell: Vec<RenderNode>) -> RenderNode {
    RenderNode::table(
        vec![ColumnAlign::None],
        vec![
            RenderNode::table_row(vec![RenderNode::table_cell(vec![text("H")])]),
            RenderNode::table_row(vec![RenderNode::table_cell(cell)]),
        ],
    )
}

fn sequence(children: Vec<RenderNode>) -> RenderNode {
    let mut root = RenderNode::root(children);
    root.attrs.set_sequence_join(SequenceJoin::None);
    root
}

fn spanned(v: &str, dialect: MarkdownDialect) -> String {
    match dialect {
        MarkdownDialect::Markdown => v.to_string(),
        MarkdownDialect::MarkdownPlus => format!("<span>{v}</span>"),
    }
}

/// A break kind's name and constructor.
type BreakKind = (&'static str, fn() -> RenderNode);

/// The break shapes of `kind`, named, with the tree nodes each holds.
fn shapes(kind: fn() -> RenderNode) -> Vec<(&'static str, Vec<RenderNode>)> {
    vec![
        ("between words", vec![text("a"), kind(), text("b")]),
        ("before text", vec![kind(), text("a")]),
        ("after text", vec![text("a"), kind()]),
        ("alone", vec![kind()]),
        ("two trailing", vec![text("a"), kind(), kind()]),
        ("two between words", vec![text("a"), kind(), kind(), text("b")]),
        (
            "leading before nested bold",
            vec![kind(), RenderNode::strong(vec![text("a")])],
        ),
        ("after a line feed in text", vec![text("a\n"), kind(), text("b")]),
        ("ending after a line feed in text", vec![text("a\n"), kind()]),
    ]
}

/// A context a shape is written into, and the reading expected for the
/// shape's meaning `v` (or `cell`, its meaning inside a table cell).
type Context = (
    &'static str,
    fn(Vec<RenderNode>, &str, &str, MarkdownDialect) -> (RenderNode, String),
);

fn contexts() -> Vec<Context> {
    vec![
        ("paragraph", |s, v, _, _| (para(s), format!("¶{v}"))),
        ("paragraph before another paragraph", |s, v, _, _| {
            (
                RenderNode::root(vec![para(s), para(vec![text("next")])]),
                format!("¶{v}¶next"),
            )
        }),
        ("paragraph after text", |s, v, _, _| {
            (para(prepend(text("x "), s)), format!("¶x {v}"))
        }),
        ("after a soft break", |s, v, _, _| {
            (
                para(prepend(text("x"), prepend(RenderNode::soft_break(), s))),
                format!("¶x {v}"),
            )
        }),
        ("after a hard break", |s, v, _, _| {
            (
                para(prepend(text("x"), prepend(RenderNode::hard_break(), s))),
                format!("¶x{HB}{v}"),
            )
        }),
        ("root phrasing", |s, v, _, _| (RenderNode::root(s), format!("¶{v}"))),
        ("sequence", |s, v, _, _| (sequence(s), format!("¶{v}"))),
        ("sequence after text", |s, v, _, _| {
            (sequence(prepend(text("x "), s)), format!("¶x {v}"))
        }),
        ("block quote", |s, v, _, _| {
            (
                RenderNode::block_quote(vec![para(s)]),
                format!("<quote>¶{v}</quote>"),
            )
        }),
        ("list paragraph", |s, v, _, _| {
            (
                RenderNode::list(false, None, vec![RenderNode::list_item(None, vec![para(s)])]),
                format!("<list><item>{v}</item></list>"),
            )
        }),
        ("ordered item holding bare phrasing", |s, v, _, _| {
            (
                RenderNode::list(true, None, vec![RenderNode::list_item(None, s)]),
                format!("<list><item>{v}</item></list>"),
            )
        }),
        ("footnote body", |s, v, _, _| {
            (
                RenderNode::root(vec![
                    para(vec![text("x"), RenderNode::footnote_reference("n")]),
                    RenderNode::footnote_definition("n", vec![para(s)]),
                ]),
                format!("¶x{{fnref:n}}<fn n>¶{v}</fn>"),
            )
        }),
        ("bold", |s, v, _, _| {
            (
                para(vec![RenderNode::strong(s)]),
                format!("¶{}", wrapped("<strong>", v, "</strong>")),
            )
        }),
        ("italic", |s, v, _, _| {
            (
                para(vec![RenderNode::emphasis(s)]),
                format!("¶{}", wrapped("<em>", v, "</em>")),
            )
        }),
        ("strike", |s, v, _, _| {
            (
                para(vec![RenderNode::delete(s)]),
                format!("¶{}", wrapped("<del>", v, "</del>")),
            )
        }),
        ("bold between text", |s, v, _, _| {
            (
                para(vec![text("x"), RenderNode::strong(s), text("y")]),
                format!("¶x{}y", wrapped("<strong>", v, "</strong>")),
            )
        }),
        ("mark", |s, v, _, _| {
            (
                para(vec![RenderNode::extended("mark", s, None)]),
                format!("¶{}", wrapped("==", v, "==")),
            )
        }),
        ("dim", |s, v, _, _| {
            (
                para(vec![RenderNode::extended("dim", s, None)]),
                format!("¶{}", wrapped("\u{2304}", v, "\u{2304}")),
            )
        }),
        ("color span", |s, v, _, dialect| {
            (para(vec![styled(color_style(), s)]), format!("¶{}", spanned(v, dialect)))
        }),
        ("underline span", |s, v, _, dialect| {
            (
                para(vec![styled(underline_style(), s)]),
                format!("¶{}", spanned(v, dialect)),
            )
        }),
        ("classed span", |s, v, _, dialect| {
            (
                para(vec![RenderNode::span(vec!["note".to_string()], s)]),
                format!("¶{}", spanned(v, dialect)),
            )
        }),
        ("styled span after text", |s, v, _, dialect| {
            (
                para(vec![text("x "), styled(color_style(), s)]),
                format!("¶x {}", spanned(v, dialect)),
            )
        }),
        ("transparent span", |s, v, _, _| {
            (para(vec![RenderNode::span(vec![], s)]), format!("¶{v}"))
        }),
        ("unknown extended wrapper", |s, v, _, _| {
            (
                para(vec![RenderNode::extended("custom", s, None)]),
                format!("¶{v}"),
            )
        }),
        ("link label", |s, v, _, _| {
            (
                para(vec![RenderNode::link("https://e.io", None, s)]),
                format!("¶<a https://e.io>{v}</a>"),
            )
        }),
        ("table cell", |s, _, cell, _| {
            (table(s), format!("<td>H</td><td>{cell}</td>"))
        }),
        ("bold in a table cell", |s, _, cell, _| {
            (
                table(vec![RenderNode::strong(s)]),
                format!("<td>H</td><td>{}</td>", wrapped("<strong>", cell, "</strong>")),
            )
        }),
        ("disclosure body", |s, v, _, dialect| {
            (
                RenderNode::disclosure(vec![text("S")], vec![para(s)], None),
                match dialect {
                    MarkdownDialect::Markdown => {
                        format!("¶::disclosure S ::details {v} ::end-disclosure")
                    }
                    MarkdownDialect::MarkdownPlus => format!("<summary>S</summary>¶{v}"),
                },
            )
        }),
        ("disclosure summary", |s, v, _, dialect| {
            (
                RenderNode::disclosure(s, vec![para(vec![text("body")])], None),
                match dialect {
                    MarkdownDialect::Markdown => {
                        format!("¶::disclosure {v} ::details body ::end-disclosure")
                    }
                    MarkdownDialect::MarkdownPlus => format!("<summary>{v}</summary>¶body"),
                },
            )
        }),
        ("heading", |s, v, _, _| {
            (
                RenderNode::heading(HeadingDepth::new(2).unwrap(), s),
                format!("<h>{v}</h>"),
            )
        }),
    ]
}

/// Whether `actual` is the documented reading of a block whose only content
/// is one hard break: one raw HTML `<br>` line in place of the paragraph.
fn is_lone_break_block(expected: &str, actual: &str) -> bool {
    // A tight list item holds no paragraph, so the break itself is replaced.
    actual == expected.replacen(&format!("¶{HB}"), "{lone br}", 1)
        || actual == expected.replacen(&HB.to_string(), "{lone br}", 1)
}

#[test]
fn breaks_keep_their_meaning_in_every_context_and_dialect() {
    let kinds: [BreakKind; 2] = [
        ("soft", RenderNode::soft_break),
        ("hard", RenderNode::hard_break),
    ];
    let mut failures = Vec::new();
    let mut cases = 0;
    for (kind_name, kind) in kinds {
        for (context, build) in contexts() {
            for (shape, nodes) in shapes(kind) {
                for dialect in DIALECTS {
                    let v = meaning(&nodes, false);
                    let cell = meaning(&nodes, true);
                    let (node, expected) = build(nodes.clone(), &v, &cell, dialect);
                    let markdown = render_in(&node, dialect);
                    let actual = read_back(&markdown);
                    cases += 1;
                    let lone = shape == "alone" && kind_name == "hard";
                    if actual != expected && !(lone && is_lone_break_block(&expected, &actual)) {
                        failures.push(format!(
                            "{kind_name} break, {shape}, in {context} ({dialect:?})\n  \
                             markdown: {markdown:?}\n  read:     {actual:?}\n  \
                             expected: {expected:?}"
                        ));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {cases} cases changed meaning:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn a_block_holding_only_a_hard_break_is_one_html_line_break() {
    // No Markdown paragraph holds only a hard break: a backslash needs
    // content after it, and a line holding only `<br>` is a raw HTML block.
    // The writer uses `<br>`, which shows the same line break.
    for dialect in DIALECTS {
        let markdown = render_in(&para(vec![RenderNode::hard_break()]), dialect);
        assert_eq!(markdown, "<br>");
        assert_eq!(read_back(&markdown), "{lone br}");
    }
}

#[test]
fn breaks_with_content_on_both_sides_keep_the_line_ending_spellings() {
    for dialect in DIALECTS {
        let soft = para(vec![text("a"), RenderNode::soft_break(), text("b")]);
        let hard = para(vec![text("a"), RenderNode::hard_break(), text("b")]);
        assert_eq!(render_in(&soft, dialect), "a\nb");
        assert_eq!(render_in(&hard, dialect), "a\\\nb");
        let label = para(vec![RenderNode::link(
            "https://e.io",
            None,
            vec![text("a"), RenderNode::hard_break(), text("b")],
        )]);
        assert_eq!(render_in(&label, dialect), "[a\\\nb](https://e.io)");
    }
}

#[test]
fn heading_breaks_stay_on_the_heading_line() {
    // An ATX heading is one line: a soft break is the space it shows as, and
    // a hard break is inline `<br>`.
    for dialect in DIALECTS {
        let heading = |kind: RenderNode| {
            render_in(
                &RenderNode::heading(
                    HeadingDepth::new(2).unwrap(),
                    vec![text("a"), kind, text("b")],
                ),
                dialect,
            )
        };
        assert_eq!(heading(RenderNode::soft_break()), "## a b");
        assert_eq!(heading(RenderNode::hard_break()), "## a<br>b");
    }
}

#[test]
fn code_and_raw_html_are_not_treated_as_breaks() {
    for dialect in DIALECTS {
        let code = para(vec![RenderNode::inline_code("a\\"), RenderNode::hard_break(), text("b")]);
        assert_eq!(render_in(&code, dialect), "`a\\`\\\nb");
        let mut raw = para(vec![text("a"), RenderNode::hard_break()]);
        raw = RenderNode::root(vec![raw, RenderNode::html("<br>\n", true)]);
        assert_eq!(render_in(&raw, dialect), "a<br>\n\n<br>\n");
    }
}
