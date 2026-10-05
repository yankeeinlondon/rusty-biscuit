//! Content embedded where a reader consumes HTML keeps its meaning.
//!
//! MarkdownPlus writes some constructs as raw HTML: the disclosure
//! `<summary>`, the columns container, and the progress widget. A reader
//! passes a raw HTML block through without parsing it as Markdown, so the
//! children written there must be HTML (code as an escaped `<code>`), and
//! the progress label must be plain text. Each case renders a tree through
//! the public Markdown writer, reads the result with an independent
//! CommonMark/GFM reader (`pulldown-cmark`) followed by the HTML a browser
//! would see, and compares that reading with the public browser renderer's
//! HTML for the same tree.
//!
//! Ordinary paragraphs, disclosure bodies, and MarkdownPlus styled spans are
//! Markdown-parsed contexts and serve as controls. Mark and dim are
//! extension syntax there, not CommonMark, so they are compared only where
//! the writer produces HTML.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use renderable::color::{BasicColor, Color};
use renderable::layout::TargetValue;
use renderable::style::{PerMode, Style, TextEmphasis, UnderlineStyle};
use renderable::tree::{
    BrowserRenderOptions, ColumnsHints, MarkdownDialect, MarkdownRenderOptions, ProgressHints,
    RenderNode, RenderStrictness, render_browser_node, render_markdown_node,
};

const DIALECTS: [MarkdownDialect; 2] = [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus];

/// Code and text values whose meaning changes if a reader takes them as
/// HTML or as Markdown.
const CONTROLS: [&str; 6] = [
    "<em>x</em>",
    "&copy;",
    "[x](u)",
    "**x**",
    "a`b",
    "plain",
];

/// One item of what a reader shows.
#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Open(String),
    Close(String),
    Void(String),
    Text(String),
}

/// Element names a reading keeps; every other element is transparent.
fn kept(name: &str) -> Option<&'static str> {
    Some(match name {
        "p" => "p",
        "strong" | "b" => "strong",
        "em" | "i" => "em",
        "del" | "s" => "del",
        "code" => "code",
        "pre" => "pre",
        "mark" => "mark",
        "a" => "a",
        "summary" => "summary",
        "ul" => "ul",
        "ol" => "ol",
        "li" => "li",
        _ => return None,
    })
}

fn decode_entities(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(end) = rest.find(';').filter(|end| *end < 12) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let name = &rest[1..end];
        let decoded = match name {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "copy" => Some('©'),
            "nbsp" => Some('\u{a0}'),
            _ if name.starts_with("#x") || name.starts_with("#X") => {
                u32::from_str_radix(&name[2..], 16).ok().and_then(char::from_u32)
            }
            _ if name.starts_with('#') => name[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The value of attribute `name` in the tag body `tag` (`a href="u"`).
fn attribute(tag: &str, name: &str) -> Option<String> {
    let mut rest = tag;
    while let Some(at) = rest.find(name) {
        let after = &rest[at + name.len()..];
        let boundary = rest[..at].ends_with(char::is_whitespace);
        if boundary && let Some(value) = after.strip_prefix('=') {
            let quote = value.chars().next()?;
            let value = &value[1..];
            let end = value.find(quote)?;
            return Some(decode_entities(&value[..end]));
        }
        rest = after;
    }
    None
}

/// The index of the `>` that closes the tag `html` starts with; a quoted
/// attribute value may hold one.
fn tag_end(html: &str) -> Option<usize> {
    let mut quote = None;
    for (at, c) in html.char_indices() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(open), _) if c == open => quote = None,
            (None, '>') => return Some(at),
            _ => {}
        }
    }
    None
}

/// Reads HTML the way a browser shows it, keeping the elements of [`kept`].
/// A span carrying an opacity reads as `dim`.
fn read_html(html: &str, out: &mut Vec<Tok>, spans: &mut Vec<bool>) {
    let mut rest = html;
    while !rest.is_empty() {
        let Some(at) = rest.find('<') else {
            out.push(Tok::Text(decode_entities(rest)));
            break;
        };
        if at > 0 {
            out.push(Tok::Text(decode_entities(&rest[..at])));
        }
        rest = &rest[at..];
        if let Some(comment) = rest.strip_prefix("<!--") {
            rest = comment.find("-->").map_or("", |end| &comment[end + 3..]);
            continue;
        }
        let Some(end) = tag_end(rest) else {
            out.push(Tok::Text(decode_entities(rest)));
            break;
        };
        let tag = rest[1..end].trim_end_matches('/').trim();
        rest = &rest[end + 1..];
        if let Some(name) = tag.strip_prefix('/') {
            let name = name.trim().to_ascii_lowercase();
            if name == "span" {
                if spans.pop() == Some(true) {
                    out.push(Tok::Close("dim".into()));
                }
            } else if let Some(name) = kept(&name) {
                out.push(Tok::Close(name.into()));
            }
            continue;
        }
        let name = tag
            .split(|c: char| c.is_whitespace())
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        match name.as_str() {
            "br" => out.push(Tok::Void("br".into())),
            "img" => out.push(Tok::Void(format!(
                "img alt={}",
                attribute(tag, "alt").unwrap_or_default()
            ))),
            "span" => {
                let dim = attribute(tag, "style").is_some_and(|style| style.contains("opacity"));
                spans.push(dim);
                if dim {
                    out.push(Tok::Open("dim".into()));
                }
            }
            other => {
                if let Some(name) = kept(other) {
                    out.push(Tok::Open(name.into()));
                }
            }
        }
    }
}

/// What a reader shows for a Markdown document: CommonMark/GFM structure,
/// with every raw HTML run read as a browser reads it.
fn read_markdown(markdown: &str) -> Vec<Tok> {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_FOOTNOTES;
    let mut out = Vec::new();
    let mut spans = Vec::new();
    let mut html = String::new();
    let mut image_alt: Option<String> = None;
    for event in Parser::new_ext(markdown, options) {
        if let Event::Html(raw) | Event::InlineHtml(raw) = &event {
            html.push_str(raw);
            continue;
        }
        if !html.is_empty() {
            read_html(&std::mem::take(&mut html), &mut out, &mut spans);
        }
        if let Some(alt) = &mut image_alt {
            match event {
                Event::End(TagEnd::Image) => {
                    out.push(Tok::Void(format!("img alt={alt}")));
                    image_alt = None;
                }
                Event::Text(text) | Event::Code(text) => alt.push_str(&text),
                _ => {}
            }
            continue;
        }
        match event {
            Event::Text(text) => out.push(Tok::Text(text.to_string())),
            Event::Code(code) => {
                out.push(Tok::Open("code".into()));
                out.push(Tok::Text(code.to_string()));
                out.push(Tok::Close("code".into()));
            }
            Event::SoftBreak => out.push(Tok::Text(" ".into())),
            Event::HardBreak => out.push(Tok::Void("br".into())),
            Event::FootnoteReference(id) => {
                out.push(Tok::Open("a".into()));
                out.push(Tok::Text(id.to_string()));
                out.push(Tok::Close("a".into()));
            }
            Event::Start(Tag::Image { .. }) => image_alt = Some(String::new()),
            Event::Start(tag) => {
                let name = match tag {
                    Tag::Paragraph => "p",
                    Tag::Strong => "strong",
                    Tag::Emphasis => "em",
                    Tag::Strikethrough => "del",
                    Tag::Link { .. } => "a",
                    Tag::CodeBlock(_) => "pre",
                    Tag::List(Some(_)) => "ol",
                    Tag::List(None) => "ul",
                    Tag::Item => "li",
                    _ => continue,
                };
                out.push(Tok::Open(name.into()));
                if name == "pre" {
                    out.push(Tok::Open("code".into()));
                }
            }
            Event::End(end) => {
                let name = match end {
                    TagEnd::Paragraph => "p",
                    TagEnd::Strong => "strong",
                    TagEnd::Emphasis => "em",
                    TagEnd::Strikethrough => "del",
                    TagEnd::Link => "a",
                    TagEnd::CodeBlock => "pre",
                    TagEnd::List(true) => "ol",
                    TagEnd::List(false) => "ul",
                    TagEnd::Item => "li",
                    _ => continue,
                };
                if name == "pre" {
                    out.push(Tok::Close("code".into()));
                }
                out.push(Tok::Close(name.into()));
            }
            _ => {}
        }
    }
    if !html.is_empty() {
        read_html(&html, &mut out, &mut spans);
    }
    out
}

/// What a browser shows for the browser renderer's HTML.
fn read_browser(node: &RenderNode) -> Vec<Tok> {
    let html = render_browser_node(node, &BrowserRenderOptions::default())
        .expect("browser render")
        .output
        .render();
    let mut out = Vec::new();
    read_html(&html, &mut out, &mut Vec::new());
    out
}

/// A reading as one comparable string. Whitespace collapses outside `<pre>`,
/// where a code block's final line ending is dropped, and does not count
/// next to a block boundary.
fn show(tokens: &[Tok]) -> String {
    const BLOCKS: [&str; 6] = ["p", "summary", "pre", "ul", "ol", "li"];
    let mut out = String::new();
    let mut pre = 0;
    for token in tokens {
        match token {
            Tok::Open(name) => {
                pre += usize::from(name == "pre");
                out.push_str(&format!("⟨{name}⟩"));
            }
            Tok::Close(name) => {
                pre -= usize::from(name == "pre");
                if pre > 0 && name == "code" && out.ends_with('\n') {
                    out.pop();
                }
                out.push_str(&format!("⟨/{name}⟩"));
            }
            Tok::Void(name) => out.push_str(&format!("⟨{name}⟩")),
            Tok::Text(text) if pre > 0 => out.push_str(text),
            Tok::Text(text) => {
                for c in text.chars() {
                    if c.is_whitespace() && c != '\u{a0}' {
                        if !out.ends_with(' ') {
                            out.push(' ');
                        }
                    } else {
                        out.push(c);
                    }
                }
            }
        }
    }
    for block in BLOCKS {
        for marker in [format!("⟨{block}⟩"), format!("⟨/{block}⟩")] {
            out = out
                .replace(&format!(" {marker}"), &marker)
                .replace(&format!("{marker} "), &marker);
        }
    }
    // A tight list item's paragraph is not wrapped in `<p>`; that is
    // spacing, not meaning.
    out.replace("⟨li⟩⟨p⟩", "⟨li⟩")
        .replace("⟨/p⟩⟨/li⟩", "⟨/li⟩")
        .trim()
        .to_string()
}

fn render_in(node: &RenderNode, dialect: MarkdownDialect) -> String {
    let opts = MarkdownRenderOptions {
        dialect,
        strictness: RenderStrictness::Lossy,
        style: None,
    };
    render_markdown_node(node, &opts).expect("markdown render").output
}

fn text(value: &str) -> RenderNode {
    RenderNode::text(value)
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

fn columns(left: Vec<RenderNode>, right: Vec<RenderNode>) -> RenderNode {
    let left_count = left.len();
    let mut node = RenderNode::block_quote(left.into_iter().chain(right).collect());
    node.attrs.set_columns_hints(&ColumnsHints {
        left_count,
        ..Default::default()
    });
    node
}

fn progress(children: Vec<RenderNode>) -> RenderNode {
    let mut node = para(children);
    node.attrs.set_progress_hints(&ProgressHints {
        value: 0.6,
        ..Default::default()
    });
    node
}

type Wrap = fn(Vec<RenderNode>) -> RenderNode;

/// Every phrasing wrapper, by name, and whether a Markdown-parsed context
/// writes it as standard CommonMark (mark and dim are extension syntax).
fn wrappers() -> Vec<(&'static str, Wrap, bool)> {
    vec![
        ("strong", RenderNode::strong as Wrap, true),
        ("emphasis", RenderNode::emphasis, true),
        ("delete", RenderNode::delete, true),
        ("neutral span", |c| RenderNode::span(vec![], c), true),
        ("color span", |c| styled(color_style(), c), true),
        ("underline span", |c| styled(underline_style(), c), true),
        ("classed span", |c| RenderNode::span(vec!["note".into()], c), true),
        ("unknown extension", |c| RenderNode::extended("shout", c, None), true),
        ("link", |c| RenderNode::link("u", None, c), true),
        ("mark", |c| RenderNode::extended("mark", c, None), false),
        ("dim", |c| RenderNode::extended("dim", c, None), false),
    ]
}

/// Every phrasing shape: a name, its nodes, and whether a Markdown-parsed
/// context writes it as standard CommonMark.
fn phrasing() -> Vec<(String, Vec<RenderNode>, bool)> {
    let mut shapes = Vec::new();
    for control in CONTROLS {
        shapes.push((format!("text {control:?}"), vec![text(control)], true));
        shapes.push((
            format!("inline code {control:?}"),
            vec![RenderNode::inline_code(control)],
            true,
        ));
        shapes.push((
            format!("image alt {control:?}"),
            vec![RenderNode::image("i.png", None, control)],
            true,
        ));
        for (name, wrap, standard) in wrappers() {
            shapes.push((
                format!("{name} around text {control:?}"),
                vec![text("a "), wrap(vec![text(control)])],
                standard,
            ));
            shapes.push((
                format!("{name} around code {control:?}"),
                vec![text("a "), wrap(vec![RenderNode::inline_code(control)])],
                standard,
            ));
            shapes.push((
                format!("{name} around a color span around code {control:?}"),
                vec![wrap(vec![styled(
                    color_style(),
                    vec![RenderNode::inline_code(control)],
                )])],
                standard,
            ));
        }
    }
    shapes.push((
        "footnote reference".into(),
        vec![text("a"), RenderNode::footnote_reference("1")],
        true,
    ));
    shapes.push((
        "soft break".into(),
        vec![text("a"), RenderNode::soft_break(), text("b")],
        true,
    ));
    shapes.push((
        "hard break".into(),
        vec![text("a"), RenderNode::hard_break(), text("b")],
        true,
    ));
    shapes
}

/// The root a case renders: the site plus the definition its footnote
/// reference needs, so both readers resolve it.
fn document(site: RenderNode) -> RenderNode {
    RenderNode::root(vec![
        site,
        RenderNode::footnote_definition("1", vec![para(vec![text("note")])]),
    ])
}

/// The part of `reading` between `start` and `end`.
fn between<'a>(reading: &'a str, start: &str, end: &str) -> &'a str {
    let from = reading.find(start).map_or(0, |at| at + start.len());
    let to = reading[from..].find(end).map_or(reading.len(), |at| from + at);
    reading[from..to].trim()
}

/// One context a phrasing shape is placed in.
struct Site {
    name: &'static str,
    /// Whether MarkdownPlus writes this context as HTML.
    html: bool,
    build: fn(Vec<RenderNode>) -> RenderNode,
    /// The compared parts of the Markdown reading and the browser reading.
    compare: fn(MarkdownDialect, &str, &str) -> (String, String),
}

fn whole(_: MarkdownDialect, markdown: &str, browser: &str) -> (String, String) {
    (markdown.to_string(), browser.to_string())
}

fn sites() -> Vec<Site> {
    vec![
        Site {
            name: "disclosure summary",
            html: true,
            build: |s| RenderNode::disclosure(s, vec![para(vec![text("body")])], None),
            compare: |dialect, markdown, browser| {
                let summary = between(browser, "⟨summary⟩", "⟨/summary⟩").to_string();
                match dialect {
                    MarkdownDialect::Markdown => (
                        between(markdown, "::disclosure", "::details").to_string(),
                        summary,
                    ),
                    MarkdownDialect::MarkdownPlus => (markdown.to_string(), browser.to_string()),
                }
            },
        },
        Site {
            name: "columns",
            html: true,
            build: |s| columns(vec![para(s)], vec![para(vec![text("right")])]),
            compare: whole,
        },
        Site {
            name: "progress",
            html: true,
            build: |mut s| {
                s.push(text(" 60%"));
                progress(s)
            },
            compare: |dialect, markdown, browser| match dialect {
                // Plain Markdown writes the paragraph fallback, compared
                // with the same paragraph in the paragraph control.
                MarkdownDialect::Markdown => (String::new(), String::new()),
                MarkdownDialect::MarkdownPlus => (
                    between(markdown, "⟨p⟩", "⟨/p⟩").to_string(),
                    between(browser, "", "⟨p⟩").to_string(),
                ),
            },
        },
        Site {
            name: "paragraph (control)",
            html: false,
            build: |s| para(s),
            compare: whole,
        },
        Site {
            name: "disclosure body (control)",
            html: false,
            build: |s| RenderNode::disclosure(vec![text("S")], vec![para(s)], None),
            compare: |dialect, markdown, browser| match dialect {
                MarkdownDialect::Markdown => (
                    between(markdown, "::details", "::end-disclosure").to_string(),
                    between(browser, "⟨/summary⟩⟨p⟩", "⟨/p⟩").to_string(),
                ),
                MarkdownDialect::MarkdownPlus => (markdown.to_string(), browser.to_string()),
            },
        },
        Site {
            name: "styled span (control)",
            html: false,
            build: |s| para(vec![styled(color_style(), s)]),
            compare: whole,
        },
    ]
}

#[test]
fn html_backed_contexts_read_like_the_browser_in_both_dialects() {
    let mut failures = Vec::new();
    let mut cases = 0;
    for site in sites() {
        for (shape, nodes, standard) in phrasing() {
            for dialect in DIALECTS {
                // Mark and dim are extension syntax wherever Markdown is
                // parsed, which is every context plain Markdown writes.
                let markdown_parsed = !site.html || dialect == MarkdownDialect::Markdown;
                if markdown_parsed && !standard {
                    continue;
                }
                let tree = document((site.build)(nodes.clone()));
                let markdown = render_in(&tree, dialect);
                let read = show(&read_markdown(&markdown));
                let expected = show(&read_browser(&tree));
                let (actual, expected) = (site.compare)(dialect, &read, &expected);
                cases += 1;
                if actual != expected {
                    failures.push(format!(
                        "{shape} in {} ({dialect:?})\n  markdown: {markdown:?}\n  \
                         read:     {actual:?}\n  expected: {expected:?}",
                        site.name
                    ));
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

/// The `aria-label` value in `html`.
fn aria_label(html: &str) -> Option<String> {
    let at = html.find("aria-label=")?;
    attribute(&html[at - 1..], "aria-label")
}

#[test]
fn progress_labels_are_the_plain_text_the_browser_shows() {
    let mut failures = Vec::new();
    let mut cases = 0;
    for (shape, mut nodes, _) in phrasing() {
        nodes.push(text(" 60%"));
        let tree = progress(nodes);
        let markdown = render_in(&tree, MarkdownDialect::MarkdownPlus);
        let browser = render_browser_node(&tree, &BrowserRenderOptions::default())
            .expect("browser render")
            .output
            .render();
        let expected = aria_label(&browser);
        let accessible = aria_label(&markdown);
        // The visible label is the label element's text as a reader shows it.
        let visible = show(&read_markdown(&markdown));
        let label = between(&visible, "⟨p⟩", "60%").to_string();
        let expected_visible = expected
            .as_deref()
            .map(|label| label.split_whitespace().collect::<Vec<_>>().join(" "))
            .unwrap_or_default();
        cases += 1;
        if accessible != expected || label != expected_visible {
            failures.push(format!(
                "{shape}\n  markdown: {markdown:?}\n  aria:     {accessible:?}\n  \
                 visible:  {label:?}\n  expected: {expected:?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {cases} progress labels differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn column_block_content_reads_like_the_browser() {
    let left = vec![
        para(vec![text("one "), RenderNode::strong(vec![text("bold")])]),
        RenderNode::code(Some("html".into()), None, "<em>x</em>\n\n&copy; `t`\n"),
        RenderNode::list(
            false,
            None,
            vec![RenderNode::list_item(
                None,
                vec![para(vec![RenderNode::inline_code("<b>li</b>")])],
            )],
        ),
        para(vec![text("two")]),
    ];
    let tree = columns(left, vec![para(vec![text("right")])]);
    let expected = show(&read_browser(&tree));
    for dialect in DIALECTS {
        let markdown = render_in(&tree, dialect);
        assert_eq!(
            show(&read_markdown(&markdown)),
            expected,
            "{dialect:?}\n{markdown}"
        );
    }
}

#[test]
fn reported_reproductions_keep_code_literal() {
    let summary = RenderNode::disclosure(
        vec![RenderNode::inline_code("<em>x</em>")],
        vec![para(vec![text("body")])],
        None,
    );
    let markdown = render_in(&summary, MarkdownDialect::MarkdownPlus);
    assert_eq!(
        show(&read_markdown(&markdown)),
        "⟨summary⟩⟨code⟩<em>x</em>⟨/code⟩⟨/summary⟩⟨p⟩body⟨/p⟩",
        "{markdown}"
    );

    let entity = columns(
        vec![para(vec![RenderNode::inline_code("&copy;")])],
        vec![para(vec![text("right")])],
    );
    let markdown = render_in(&entity, MarkdownDialect::MarkdownPlus);
    assert_eq!(
        show(&read_markdown(&markdown)),
        "⟨p⟩⟨code⟩&copy;⟨/code⟩⟨/p⟩⟨p⟩right⟨/p⟩",
        "{markdown}"
    );
}
