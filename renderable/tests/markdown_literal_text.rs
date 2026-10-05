//! Literal text values survive the Markdown writer in every context.
//!
//! Formatting is carried by structural nodes, so a `Text` value, a caption,
//! an image alternative, a title, or a destination must read back from an
//! independent CommonMark/GFM reader (`pulldown-cmark`) as exactly that
//! value, with only the structure the tree asked for.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use renderable::color::{BasicColor, Color};
use renderable::layout::TargetValue;
use renderable::style::{PerMode, Style, TextEmphasis, UnderlineStyle};
use renderable::tree::{
    ColumnAlign, HeadingDepth, MarkdownDialect, MarkdownRenderOptions, RenderNode,
    RenderStrictness, SequenceJoin, render_markdown_node,
};

const DIALECTS: [MarkdownDialect; 2] = [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus];

/// The review's five values first, then other syntax a reader recognizes.
const VALUES: &[&str] = &[
    "**literal**",
    "_literal_",
    "[literal](https://x.io)",
    "<em>literal</em>",
    "&copy;",
    "&#169; and &#xA9;",
    "# literal",
    "- literal",
    "+ literal",
    "1. literal",
    "2) literal",
    "> literal",
    "---",
    "`literal`",
    "~~literal~~",
    "~literal~",
    "==literal==",
    "<https://x.io>",
    "![alt](i.png)",
    "[^1]",
    "[x]: https://x.io",
    "literal #",
    "*a* _b_ __c__",
];

/// Email-looking values: an email autolink's local part may start with a
/// digit or punctuation, not only a letter. `_` is a control the older
/// escape already covered.
fn email_values() -> Vec<String> {
    let mut values = vec![
        "<3@example.com>".to_string(),
        "<+foo@example.com>".to_string(),
        "<_foo@example.com>".to_string(),
        "I <3 you".to_string(),
    ];
    for start in ['3', '.', '!', '#', '$', '%', '+', '-', '/', '?', '^', '{', '|', '}'] {
        values.push(format!("<{start}x@example.com>"));
    }
    values
}

/// Marks where [`text`] splits a value into adjacent text children of a
/// neutral span. Builders that store a string field use [`plain`].
const SPLIT: char = '\u{1}';

/// Values a reader could take as block syntax or indentation at a line
/// start, with zero to three leading spaces, each whole and split at every
/// character boundary.
fn line_start_values() -> Vec<String> {
    let bases = [
        "# literal",
        "## literal",
        "> literal",
        "- literal",
        "+ literal",
        "1. literal",
        "12) literal",
        "---",
        "===",
    ];
    let mut values = Vec::new();
    for indent in 0..=3 {
        for base in bases {
            values.push(format!("{}{base}", " ".repeat(indent)));
        }
    }
    values.push("    literal".to_string());
    values.push("\tliteral".to_string());
    values.push(" \t literal".to_string());
    let whole = values.clone();
    for value in whole {
        let boundaries = value.char_indices().map(|(at, _)| at).skip(1);
        for at in boundaries.collect::<Vec<_>>() {
            values.push(format!("{}{SPLIT}{}", &value[..at], &value[at..]));
        }
    }
    values
}

/// The value with its split markers removed.
fn plain(value: &str) -> String {
    value.replace(SPLIT, "")
}

/// `open` and `close` around `value` after the delimiter-edge policy moves
/// the value's edge whitespace outside the wrapper.
fn wrapped(open: &str, value: &str, close: &str) -> String {
    let value = plain(value);
    let core = value.trim();
    let start = value.len() - value.trim_start().len();
    format!("{}{open}{core}{close}{}", &value[..start], &value[start + core.len()..])
}

fn render_in(node: &RenderNode, dialect: MarkdownDialect) -> String {
    let opts = MarkdownRenderOptions {
        dialect,
        strictness: RenderStrictness::Lossy,
        style: None,
    };
    render_markdown_node(node, &opts).expect("render").output
}

/// What a reader sees: text verbatim, structure as `<tag>` markers, inline
/// HTML as `{html}`, raw HTML blocks as `{block}`, inline code as
/// `{code:…}`. Paragraph boundaries are omitted.
fn read_back(markdown: &str) -> String {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_FOOTNOTES;
    let mut out = String::new();
    for event in Parser::new_ext(markdown, options) {
        match event {
            Event::Text(text) => out.push_str(&text),
            Event::Code(code) => out.push_str(&format!("{{code:{code}}}")),
            Event::SoftBreak => out.push_str("{SB}"),
            Event::HardBreak => out.push_str("{HB}"),
            Event::InlineHtml(_) => out.push_str("{html}"),
            Event::Start(Tag::HtmlBlock) => out.push_str("{block}"),
            Event::FootnoteReference(id) => out.push_str(&format!("{{fnref:{id}}}")),
            Event::Rule => out.push_str("<hr>"),
            Event::TaskListMarker(_) => out.push_str("<task>"),
            Event::Start(tag) => out.push_str(&match tag {
                Tag::Strong => "<strong>".to_string(),
                Tag::Emphasis => "<em>".to_string(),
                Tag::Strikethrough => "<del>".to_string(),
                Tag::Heading { .. } => "<h>".to_string(),
                Tag::BlockQuote(_) => "<quote>".to_string(),
                Tag::List(_) => "<list>".to_string(),
                Tag::Item => "<item>".to_string(),
                Tag::TableCell => "<td>".to_string(),
                Tag::CodeBlock(_) => "<pre>".to_string(),
                Tag::FootnoteDefinition(id) => format!("<fn {id}>"),
                Tag::Link {
                    dest_url, title, ..
                } => format!("<a {dest_url} {title}>"),
                Tag::Image {
                    dest_url, title, ..
                } => format!("<img {dest_url} {title}>"),
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
                TagEnd::CodeBlock => "</pre>",
                TagEnd::FootnoteDefinition => "</fn>",
                TagEnd::Link => "</a>",
                TagEnd::Image => "</img>",
                _ => "",
            }),
            _ => {}
        }
    }
    out
}

/// A text value, or adjacent text children of a neutral span where the
/// value holds [`SPLIT`] markers.
fn text(value: &str) -> RenderNode {
    if value.contains(SPLIT) {
        RenderNode::span(vec![], value.split(SPLIT).map(RenderNode::text).collect())
    } else {
        RenderNode::text(value)
    }
}

fn para(children: Vec<RenderNode>) -> RenderNode {
    RenderNode::paragraph(children)
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

fn table(caption: Option<&str>, cell: Vec<RenderNode>) -> RenderNode {
    let mut table = RenderNode::table(
        vec![ColumnAlign::None],
        vec![
            RenderNode::table_row(vec![RenderNode::table_cell(vec![text("H")])]),
            RenderNode::table_row(vec![RenderNode::table_cell(cell)]),
        ],
    );
    if let Some(caption) = caption {
        table.attrs.set_table_title(caption);
    }
    table
}

/// Every context a literal `Text` value is written into, with the reading
/// expected for value `v` in `dialect`.
type Context = (&'static str, fn(&str, MarkdownDialect) -> (RenderNode, String));

fn text_contexts() -> Vec<Context> {
    vec![
        ("paragraph", |v, _| (para(vec![text(v)]), v.to_string())),
        ("paragraph after text", |v, _| {
            (para(vec![text("a "), text(v)]), format!("a {v}"))
        }),
        ("line after a soft break", |v, _| {
            (
                para(vec![text("a"), RenderNode::soft_break(), text(v)]),
                format!("a{{SB}}{v}"),
            )
        }),
        ("line after a hard break", |v, _| {
            (
                para(vec![text("a"), RenderNode::hard_break(), text(v)]),
                format!("a{{HB}}{v}"),
            )
        }),
        ("line inside the value", |v, _| {
            (para(vec![text(&format!("a\n{v}"))]), format!("a{{SB}}{v}"))
        }),
        ("root", |v, _| (RenderNode::root(vec![text(v)]), v.to_string())),
        ("heading", |v, _| {
            (
                RenderNode::heading(HeadingDepth::new(2).unwrap(), vec![text(v)]),
                format!("<h>{v}</h>"),
            )
        }),
        ("quote", |v, _| {
            (
                RenderNode::block_quote(vec![para(vec![text(v)])]),
                format!("<quote>{v}</quote>"),
            )
        }),
        ("list body", |v, _| {
            (
                RenderNode::list(
                    false,
                    None,
                    vec![RenderNode::list_item(None, vec![para(vec![text(v)])])],
                ),
                format!("<list><item>{v}</item></list>"),
            )
        }),
        ("ordered list body holding bare text", |v, _| {
            (
                RenderNode::list(true, None, vec![RenderNode::list_item(None, vec![text(v)])]),
                format!("<list><item>{v}</item></list>"),
            )
        }),
        ("bold", |v, _| {
            (
                para(vec![text("a "), RenderNode::strong(vec![text(v)]), text(" b")]),
                format!("a {} b", wrapped("<strong>", v, "</strong>")),
            )
        }),
        ("italic", |v, _| {
            (
                para(vec![RenderNode::emphasis(vec![text(v)])]),
                wrapped("<em>", v, "</em>"),
            )
        }),
        ("strike", |v, _| {
            (
                para(vec![RenderNode::delete(vec![text(v)])]),
                wrapped("<del>", v, "</del>"),
            )
        }),
        ("color span", |v, dialect| {
            (
                para(vec![styled(color_style(), vec![text(v)])]),
                spanned(v, dialect),
            )
        }),
        ("underline span", |v, dialect| {
            (
                para(vec![styled(underline_style(), vec![text(v)])]),
                spanned(v, dialect),
            )
        }),
        ("span with styled sibling text", |v, dialect| {
            (
                para(vec![styled(color_style(), vec![text("a "), text(v)])]),
                match dialect {
                    MarkdownDialect::Markdown => format!("a {v}"),
                    MarkdownDialect::MarkdownPlus => format!("{{html}}a {v}{{html}}"),
                },
            )
        }),
        ("transparent wrapper", |v, _| {
            (para(vec![RenderNode::span(vec![], vec![text(v)])]), v.to_string())
        }),
        ("unknown extended wrapper", |v, _| {
            (
                para(vec![RenderNode::extended("custom", vec![text(v)], None)]),
                v.to_string(),
            )
        }),
        ("link label", |v, _| {
            (
                para(vec![RenderNode::link("https://e.io", None, vec![text(v)])]),
                format!("<a https://e.io >{v}</a>"),
            )
        }),
        ("table cell", |v, _| {
            (
                table(None, vec![text(v)]),
                format!("<td>H</td><td>{v}</td>"),
            )
        }),
        ("bold in a table cell", |v, _| {
            (
                table(None, vec![RenderNode::strong(vec![text(v)])]),
                format!("<td>H</td><td>{}</td>", wrapped("<strong>", v, "</strong>")),
            )
        }),
        ("footnote body", |v, _| {
            (
                RenderNode::root(vec![
                    para(vec![text("a"), RenderNode::footnote_reference("n")]),
                    RenderNode::footnote_definition("n", vec![para(vec![text(v)])]),
                ]),
                format!("a{{fnref:n}}<fn n>{v}</fn>"),
            )
        }),
        ("sequence container", |v, _| {
            let mut root = RenderNode::root(vec![text(v)]);
            root.attrs.set_sequence_join(SequenceJoin::None);
            (root, v.to_string())
        }),
        ("sequence container after text", |v, _| {
            let mut root = RenderNode::root(vec![text("a "), text(v)]);
            root.attrs.set_sequence_join(SequenceJoin::None);
            (root, format!("a {v}"))
        }),
        ("disclosure body", |v, dialect| {
            (
                RenderNode::disclosure(vec![text("S")], vec![para(vec![text(v)])], None),
                match dialect {
                    MarkdownDialect::Markdown => {
                        format!("::disclosure{{SB}}S{{SB}}::details{{SB}}{v}{{SB}}::end-disclosure")
                    }
                    MarkdownDialect::MarkdownPlus => format!("{{block}}{v}{{block}}"),
                },
            )
        }),
        // A caption is trimmed by design.
        ("table caption", |v, _| {
            let v = plain(v);
            (
                table(Some(&v), vec![text("c")]),
                format!("{}<td>H</td><td>c</td>", v.trim()),
            )
        }),
        ("image alternative", |v, _| {
            (
                para(vec![RenderNode::image("i.png", None, plain(v))]),
                format!("<img i.png >{v}</img>"),
            )
        }),
        ("line inside an image alternative", |v, _| {
            (
                para(vec![RenderNode::image("i.png", None, format!("a\n{}", plain(v)))]),
                format!("<img i.png >a{{SB}}{v}</img>"),
            )
        }),
        ("image alternative in a table cell", |v, _| {
            (
                table(None, vec![RenderNode::image("i.png", None, plain(v))]),
                format!("<td>H</td><td><img i.png >{v}</img></td>"),
            )
        }),
    ]
}

fn spanned(v: &str, dialect: MarkdownDialect) -> String {
    match dialect {
        MarkdownDialect::Markdown => v.to_string(),
        MarkdownDialect::MarkdownPlus => format!("{{html}}{v}{{html}}"),
    }
}

fn assert_contexts<S: AsRef<str>>(contexts: &[Context], values: &[S]) {
    let mut failures = Vec::new();
    for (name, build) in contexts {
        for value in values {
            let value = value.as_ref();
            for dialect in DIALECTS {
                let (node, expected) = build(value, dialect);
                let expected = plain(&expected);
                let markdown = render_in(&node, dialect);
                let actual = read_back(&markdown);
                if actual != expected {
                    failures.push(format!(
                        "{name} {dialect:?} {value:?}\n  markdown: {markdown:?}\n  read:     \
                         {actual:?}\n  expected: {expected:?}"
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn text_values_read_back_literally_in_every_context() {
    assert_contexts(&text_contexts(), VALUES);
}

#[test]
fn email_looking_values_stay_text_in_every_context() {
    assert_contexts(&text_contexts(), &email_values());
}

#[test]
fn line_start_syntax_split_across_text_values_stays_text_in_every_context() {
    assert_contexts(&text_contexts(), &line_start_values());
}

#[test]
fn whitespace_at_line_ends_reads_back_literally() {
    let contexts: Vec<Context> = vec![
        ("paragraph end", |v, _| (para(vec![text("a"), text(v)]), format!("a{v}"))),
        ("before a soft break", |v, _| {
            (
                para(vec![text(v), RenderNode::soft_break(), text("b")]),
                format!("{v}{{SB}}b"),
            )
        }),
        ("before a hard break", |v, _| {
            (
                para(vec![text(v), RenderNode::hard_break(), text("b")]),
                format!("{v}{{HB}}b"),
            )
        }),
        ("before a line inside the value", |v, _| {
            (para(vec![text(&format!("{v}\nb"))]), format!("{v}{{SB}}b"))
        }),
        ("heading", |v, _| {
            (
                RenderNode::heading(HeadingDepth::new(2).unwrap(), vec![text(v)]),
                format!("<h>{v}</h>"),
            )
        }),
        ("table cell", |v, _| {
            (
                table(None, vec![text(v)]),
                format!("<td>H</td><td>{v}</td>"),
            )
        }),
        ("disclosure summary", |v, dialect| {
            (
                RenderNode::disclosure(vec![text(v)], vec![para(vec![text("b")])], None),
                match dialect {
                    MarkdownDialect::Markdown => {
                        format!("::disclosure{{SB}}{v}{{SB}}::details{{SB}}b{{SB}}::end-disclosure")
                    }
                    MarkdownDialect::MarkdownPlus => "{block}b{block}".to_string(),
                },
            )
        }),
    ];
    assert_contexts(
        &contexts,
        &["x  ", "x\t", "x \\ ", " ", &format!("x {SPLIT} ")],
    );
}

#[test]
fn a_table_delimiter_row_split_across_text_values_stays_text() {
    let mut rows = Vec::new();
    for row in ["|-|-|", ":-- | --", "-|-", "| :-: | --: |"] {
        rows.push(row.to_string());
        for (at, _) in row.char_indices().skip(1) {
            rows.push(format!("{}{SPLIT}{}", &row[..at], &row[at..]));
        }
    }
    let contexts: Vec<Context> = vec![
        ("after a soft break", |v, _| {
            (
                para(vec![text("a | b"), RenderNode::soft_break(), text(v)]),
                format!("a | b{{SB}}{v}"),
            )
        }),
        ("after a hard break", |v, _| {
            (
                para(vec![text("a | b"), RenderNode::hard_break(), text(v)]),
                format!("a | b{{HB}}{v}"),
            )
        }),
        ("line inside the value", |v, _| {
            (para(vec![text(&format!("a | b\n{v}"))]), format!("a | b{{SB}}{v}"))
        }),
        ("quote", |v, _| {
            (
                RenderNode::block_quote(vec![para(vec![text(&format!("a | b\n{v}"))])]),
                format!("<quote>a | b{{SB}}{v}</quote>"),
            )
        }),
    ];
    assert_contexts(&contexts, &rows);
}

#[test]
fn a_line_feed_in_heading_text_stays_in_the_heading() {
    for dialect in DIALECTS {
        let node = RenderNode::heading(HeadingDepth::new(2).unwrap(), vec![text("a\n# b")]);
        let markdown = render_in(&node, dialect);
        assert_eq!(read_back(&markdown), "<h>a\n# b</h>", "{markdown:?}");
    }
}

#[test]
fn a_crlf_in_heading_text_stays_in_the_heading() {
    // A reader ends a line at the CR of a CRLF too, so both characters are
    // kept out of the heading's one line.
    for dialect in DIALECTS {
        let node = RenderNode::heading(HeadingDepth::new(2).unwrap(), vec![text("a\r\n# b")]);
        let markdown = render_in(&node, dialect);
        assert_eq!(read_back(&markdown), "<h>a\r\n# b</h>", "{markdown:?}");
    }
}

#[test]
fn a_lone_carriage_return_is_not_a_line_ending() {
    for dialect in DIALECTS {
        let markdown = render_in(&para(vec![text("a\r# b")]), dialect);
        assert_eq!(read_back(&markdown), "a\r# b", "{markdown:?}");
    }
}

#[test]
fn titles_keep_literal_punctuation_and_entity_spellings() {
    let contexts: Vec<Context> = vec![
        ("link title", |v, _| {
            (
                para(vec![RenderNode::link("https://e.io", Some(v.to_string()), vec![text("l")])]),
                format!("<a https://e.io {v}>l</a>"),
            )
        }),
        ("image title", |v, _| {
            (
                para(vec![RenderNode::image("i.png", Some(v.to_string()), "alt")]),
                format!("<img i.png {v}>alt</img>"),
            )
        }),
        ("link title in a table cell", |v, _| {
            (
                table(
                    None,
                    vec![RenderNode::link("https://e.io", Some(v.to_string()), vec![text("l")])],
                ),
                format!("<td>H</td><td><a https://e.io {v}>l</a></td>"),
            )
        }),
    ];
    assert_contexts(&contexts, VALUES);
    assert_contexts(&contexts, &email_values());
    assert_contexts(&contexts, &[r#"say "hi" \"#, r"C:\dir\*x", "    literal"]);
}

#[test]
fn destinations_stay_exact() {
    let contexts: Vec<Context> = vec![
        ("link destination", |v, _| {
            let url = format!("https://x.io/{v}");
            (
                para(vec![RenderNode::link(url.clone(), None, vec![text("l")])]),
                format!("<a {url} >l</a>"),
            )
        }),
        ("image destination", |v, _| {
            let url = format!("https://x.io/{v}");
            (
                para(vec![RenderNode::image(url.clone(), None, "alt")]),
                format!("<img {url} >alt</img>"),
            )
        }),
        ("link destination in a table cell", |v, _| {
            let url = format!("https://x.io/{v}");
            (
                table(None, vec![RenderNode::link(url.clone(), None, vec![text("l")])]),
                format!("<td>H</td><td><a {url} >l</a></td>"),
            )
        }),
        ("destination that is the value", |v, _| {
            (
                para(vec![RenderNode::link(v, None, vec![text("l")])]),
                format!("<a {v} >l</a>"),
            )
        }),
    ];
    assert_contexts(&contexts, VALUES);
    assert_contexts(&contexts, &email_values());
}

/// Summary values: the shared values, email-looking ones, and indentation.
fn summary_values() -> Vec<String> {
    let mut values: Vec<String> = VALUES.iter().map(ToString::to_string).collect();
    values.extend(email_values());
    values.extend(["    literal".to_string(), "\tliteral".to_string()]);
    values
}

#[test]
fn markdown_plus_disclosure_summary_is_html_escaped_raw_html() {
    for value in &summary_values() {
        let node = RenderNode::disclosure(vec![text(value)], vec![para(vec![text("b")])], None);
        let markdown = render_in(&node, MarkdownDialect::MarkdownPlus);
        let escaped = value
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        assert!(
            markdown.starts_with(&format!("<details><summary>{escaped}</summary>\n\n")),
            "{value:?}: {markdown:?}"
        );
        assert_eq!(read_back(&markdown), "{block}b{block}", "{value:?}");
    }
}

#[test]
fn plain_markdown_disclosure_summary_reads_back_literally() {
    for value in &summary_values() {
        let node = RenderNode::disclosure(vec![text(value)], vec![para(vec![text("b")])], None);
        let markdown = render_in(&node, MarkdownDialect::Markdown);
        assert_eq!(
            read_back(&markdown),
            format!("::disclosure{{SB}}{value}{{SB}}::details{{SB}}b{{SB}}::end-disclosure"),
            "{markdown:?}"
        );
    }
}

#[test]
fn code_contents_are_never_escaped() {
    for dialect in DIALECTS {
        let mut values: Vec<String> = VALUES.iter().map(ToString::to_string).collect();
        values.extend(email_values());
        values.extend(["    literal".to_string(), r"C:\dir\*x\".to_string()]);
        for value in &values {
            let value = value.as_str();
            let inline = render_in(&para(vec![RenderNode::inline_code(value)]), dialect);
            assert_eq!(read_back(&inline), format!("{{code:{value}}}"), "{inline:?}");
            let block = render_in(&RenderNode::code(None, None, value), dialect);
            assert_eq!(read_back(&block), format!("<pre>{value}\n</pre>"), "{block:?}");
            assert!(block.contains(value), "{block:?}");
        }
    }
}

#[test]
fn ordinary_prose_is_byte_identical() {
    for dialect in DIALECTS {
        for value in [
            "snake_case and file_name.rs",
            "2 * 3 and a - b",
            "a < b, AT&T rocks",
            r"C:\dir\file",
            "x == y? no",
            "~/projects",
            "Fish & chips; 50% off!",
            "I <3 you, x <= y, a <- b",
        ] {
            let markdown = render_in(&para(vec![text(value)]), dialect);
            let read = read_back(&markdown);
            assert_eq!(read, value, "{dialect:?} {markdown:?}");
        }
        assert_eq!(
            render_in(&para(vec![text("snake_case, 2 * 3, a < b, C:\\dir")]), dialect),
            "snake_case, 2 * 3, a < b, C:\\dir",
            "{dialect:?}"
        );
    }
}

#[test]
fn backslashes_before_breaks_and_wrapper_edges_stay_literal() {
    for dialect in DIALECTS {
        let node = para(vec![
            RenderNode::strong(vec![text(r"*a\")]),
            RenderNode::hard_break(),
            text(r"b\"),
            RenderNode::soft_break(),
            text("c"),
        ]);
        let markdown = render_in(&node, dialect);
        assert_eq!(
            read_back(&markdown),
            r"<strong>*a\</strong>{HB}b\{SB}c",
            "{dialect:?} {markdown:?}"
        );
    }
}
