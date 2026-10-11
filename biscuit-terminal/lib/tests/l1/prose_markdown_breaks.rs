//! Soft and hard breaks in `Prose` and `InlineProse` Markdown output.
//!
//! Each case renders a component to Markdown and MarkdownPlus, reads the
//! output back with `pulldown-cmark`, an independent CommonMark reader, and
//! compares it with the component's own render tree and HTML: the same text,
//! the same breaks, the same emphasis and links, and the same paragraphs.
//!
//! A soft break and a space both read as a space, and a hard break and an
//! inline `<br>` both read as a line break; everything else must match.

use biscuit_terminal::components::prose::{InlineProse, LineBreaks, Prose};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use renderable::browser::BrowserRenderable;
use renderable::markdown::MarkdownRenderable;
use renderable::tree::{NodeKind, RenderNode, TreeRenderable};

/// A hard break, in every reading.
const HB: char = '⏎';

/// The nine wrapper categories. Prose has no `mark` tag; a background color
/// is its highlight.
const WRAPPERS: &[(&str, &str)] = &[
    ("", ""),
    ("<b>", "</b>"),
    ("<i>", "</i>"),
    ("<strikethrough>", "</strikethrough>"),
    ("<red>", "</red>"),
    ("<u>", "</u>"),
    ("<dim>", "</dim>"),
    ("<bg-coral>", "</bg-coral>"),
    ("<clipboard>", "</clipboard>"),
    ("<a href=\"https://e.io\">", "</a>"),
];

/// Break shapes as written, with `{o}` and `{c}` for the wrapper's tags.
const SHAPES: &[&str] = &[
    "{o}a\nb{c}",
    "{o}\na{c}",
    "{o}a\n{c}",
    "{o}\n{c}",
    "{o}a\n\n{c}",
    "{o}a\n\nb{c}",
    "{o}a\\\n{c}",
    "{o}a\\\nb{c}",
    "{o}\n**x**{c}",
    "x {o}a\n{c}",
    "{o}a\n{c} y",
    "{o}a\n{c}\n\nnext",
    "{o}a\\\n{c}\n\nnext",
];

/// What the tree means: text (a literal line feed is whitespace), a space
/// per soft break, [`HB`] per hard break, `¶` per paragraph, and emphasis
/// and links as tags. Styled spans are left out on both sides; a raw HTML
/// block in place of one shows up as `{html block}` in [`markdown_reading`].
/// Emphasis edges are written outside the delimiters, as the writer's
/// documented delimiter-edge policy does.
fn tree_meaning(node: &RenderNode, out: &mut String) {
    let wrap = |open: &str, close: &str, children: &[RenderNode], out: &mut String| {
        let mut inner = String::new();
        for child in children {
            tree_meaning(child, &mut inner);
        }
        out.push_str(&wrapped(open, &inner, close));
    };
    match &node.kind {
        NodeKind::Text { value } => out.push_str(&value.replace('\n', " ")),
        NodeKind::SoftBreak => out.push(' '),
        NodeKind::HardBreak => out.push(HB),
        NodeKind::Strong { children } => wrap("<strong>", "</strong>", children, out),
        NodeKind::Emphasis { children } => wrap("<em>", "</em>", children, out),
        NodeKind::Delete { children } => wrap("<del>", "</del>", children, out),
        NodeKind::Link { children, .. } => {
            out.push_str("<a>");
            for child in children {
                tree_meaning(child, out);
            }
            out.push_str("</a>");
        }
        NodeKind::Paragraph { children } => {
            out.push('¶');
            for child in children {
                tree_meaning(child, out);
            }
        }
        _ => {
            for child in node.children() {
                tree_meaning(child, out);
            }
        }
    }
}

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

/// What a reader sees, in the notation of [`tree_meaning`]. Inline `<br>`
/// is a hard break; an HTML fallback for emphasis is its tag. A raw HTML
/// block holding only `<br>` is `{lone br}`: no Markdown paragraph holds
/// only a hard break, so the writer's documented spelling for one is that
/// line break on its own.
fn markdown_reading(markdown: &str) -> String {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_FOOTNOTES;
    let mut out = String::new();
    for event in Parser::new_ext(markdown, options) {
        match event {
            Event::Text(text) => out.push_str(&text.replace('\n', " ")),
            Event::SoftBreak => out.push(' '),
            Event::HardBreak => out.push(HB),
            Event::InlineHtml(html) => {
                let tag = html.trim().to_ascii_lowercase();
                match tag.as_str() {
                    "<br>" => out.push(HB),
                    "<strong>" | "</strong>" | "<em>" | "</em>" | "<del>" | "</del>" => {
                        out.push_str(&tag);
                    }
                    _ if tag.starts_with("<span") || tag == "</span>" => {}
                    _ => out.push_str(&format!("{{html {html}}}")),
                }
            }
            Event::Html(html) if html.trim() == "<br>" => out.push_str("{lone br}"),
            Event::Html(html) => out.push_str(&format!("{{html block {html}}}")),
            Event::Start(Tag::Paragraph) => out.push('¶'),
            Event::Start(Tag::Strong) => out.push_str("<strong>"),
            Event::Start(Tag::Emphasis) => out.push_str("<em>"),
            Event::Start(Tag::Strikethrough) => out.push_str("<del>"),
            Event::Start(Tag::Link { .. }) => out.push_str("<a>"),
            Event::End(TagEnd::Strong) => out.push_str("</strong>"),
            Event::End(TagEnd::Emphasis) => out.push_str("</em>"),
            Event::End(TagEnd::Strikethrough) => out.push_str("</del>"),
            Event::End(TagEnd::Link) => out.push_str("</a>"),
            Event::Start(Tag::HtmlBlock) | Event::End(_) => {}
            other => out.push_str(&format!("{{{other:?}}}")),
        }
    }
    out
}

/// The visible text of the component's own HTML: tags removed, `<br>` as
/// [`HB`], and entities decoded.
fn html_text(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        let close = rest[open..].find('>').expect("closed tag") + open;
        if rest[open..=close].starts_with("<br") {
            out.push(HB);
        }
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    out.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
        .replace('\n', " ")
}

/// The visible text of a reading: its structure markers removed.
fn visible(reading: &str) -> String {
    let mut out = String::new();
    let mut rest = reading;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        rest = &rest[rest[open..].find('>').map_or(rest.len(), |close| open + close + 1)..];
    }
    out.push_str(rest);
    out.replace('¶', "").replace("{lone br}", &HB.to_string())
}

#[test]
fn breaks_keep_their_meaning_for_every_wrapper_mode_and_dialect() {
    let mut failures = Vec::new();
    let mut cases = 0;
    for (open, close) in WRAPPERS {
        for shape in SHAPES {
            let source = shape.replace("{o}", open).replace("{c}", close);
            for mode in [LineBreaks::Soft, LineBreaks::Hard] {
                let prose = Prose::new(source.as_str()).with_line_breaks(mode);
                let inline = InlineProse::new(source.as_str()).with_line_breaks(mode);
                let components = [
                    (
                        "Prose",
                        prose.render_tree(),
                        prose.render_html_fragment().render(),
                        [prose.render_markdown(), prose.render_markdown_plus()],
                    ),
                    (
                        "InlineProse",
                        inline.render_tree(),
                        inline.render_html_fragment().render(),
                        [inline.render_markdown(), inline.render_markdown_plus()],
                    ),
                ];
                for (component, tree, html, outputs) in components {
                    let mut expected = String::new();
                    tree_meaning(&tree, &mut expected);
                    // InlineProse has no paragraph node; its Markdown is one
                    // paragraph when it writes anything.
                    if component == "InlineProse" && !expected.is_empty() {
                        expected.insert(0, '¶');
                    }
                    let lone = expected.replacen(&format!("¶{HB}"), "{lone br}", 1);
                    for (dialect, markdown) in ["Markdown", "MarkdownPlus"].iter().zip(outputs) {
                        cases += 1;
                        let actual = markdown_reading(&markdown);
                        let shown = html_text(&html);
                        let tree_matches = actual == expected || actual == lone;
                        if !tree_matches || visible(&actual) != shown {
                            failures.push(format!(
                                "{component}/{dialect} {mode:?} source {source:?}\n  \
                                 markdown: {markdown:?}\n  read: {actual:?}\n  \
                                 tree: {expected:?}\n  html: {html:?}"
                            ));
                        }
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

/// The review's reproductions, each compared with the component's HTML.
#[test]
fn reported_break_reproductions_read_back_as_their_html() {
    let inline = InlineProse::new("a\n").with_line_breaks(LineBreaks::Hard);
    assert_eq!(inline.render_html_fragment().render(), "a<br>");
    assert_eq!(markdown_reading(&inline.render_markdown()), format!("¶a{HB}"));

    let prose = Prose::new("<b>a\n</b>").with_line_breaks(LineBreaks::Hard);
    assert_eq!(
        prose.render_html_fragment().render(),
        "<p><strong>a<br></strong></p>"
    );
    assert_eq!(
        markdown_reading(&prose.render_markdown()),
        format!("¶<strong>a</strong>{HB}")
    );

    for markdown in [
        Prose::new("<red>\n**a**</red>").render_markdown_plus(),
        InlineProse::new("<red>\n**a**</red>").render_markdown_plus(),
    ] {
        assert_eq!(markdown_reading(&markdown), "¶ <strong>a</strong>", "{markdown:?}");
    }

    let inline = InlineProse::new("\na");
    assert_eq!(inline.render_html_fragment().render(), " a");
    assert_eq!(markdown_reading(&inline.render_markdown()), "¶ a");
}
