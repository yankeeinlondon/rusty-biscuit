//! Code spans, HTML comments, and `<code-block>` bodies inside a Markdown
//! link destination are destination text in `Prose` and `InlineProse`: the
//! render tree, HTML `href`, Markdown output, and terminal OSC 8 target all
//! keep the authored destination, with no parser placeholder, and the label
//! and following text are unchanged. The same syntax in a label is still
//! code or a comment.
//!
//! Markdown output is read back with `pulldown-cmark`, an independent
//! CommonMark reader.

use biscuit_terminal::components::prose::{InlineProse, Prose};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::terminal::Terminal;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use renderable::browser::BrowserRenderable;
use renderable::markdown::MarkdownRenderable;
use renderable::tree::{NodeKind, RenderNode, TreeRenderable};

/// Text after every link in the matrix; truncation would change it.
const TAIL: &str = " tail";

fn osc8_term() -> Terminal {
    Terminal::builder()
        .color_depth(ColorDepth::None)
        .osc_link_support(true)
        .width(200)
        .build()
}

/// Every link in `node`, as `(destination, label text)`.
fn links(node: &RenderNode) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let NodeKind::Link { url, .. } = &node.kind {
        out.push((url.clone(), plain_text(node)));
    }
    for child in node.children() {
        out.extend(links(child));
    }
    out
}

fn plain_text(node: &RenderNode) -> String {
    match &node.kind {
        NodeKind::Text { value } | NodeKind::InlineCode { value } => value.clone(),
        NodeKind::SoftBreak => " ".to_string(),
        _ => node.children().iter().map(plain_text).collect(),
    }
}

fn inline_code_values(node: &RenderNode) -> Vec<String> {
    let mut out = Vec::new();
    if let NodeKind::InlineCode { value } = &node.kind {
        out.push(value.clone());
    }
    for child in node.children() {
        out.extend(inline_code_values(child));
    }
    out
}

fn decode_html(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&amp;", "&")
}

/// The decoded `href` of the single anchor in `html`, and the document's
/// visible text.
fn html_link(context: &str, html: &str) -> (String, String) {
    let hrefs: Vec<&str> = html.split("href=\"").skip(1).collect();
    assert_eq!(hrefs.len(), 1, "{context}: {html:?}");
    let raw = hrefs[0].split_once('"').unwrap_or_else(|| panic!("{context}: {html:?}")).0;
    let mut visible = String::new();
    let mut in_tag = false;
    let mut in_value = false;
    for c in html.chars() {
        match c {
            '"' if in_tag => in_value = !in_value,
            '<' if !in_tag => in_tag = true,
            '>' if in_tag && !in_value => in_tag = false,
            c if !in_tag => visible.push(c),
            _ => {}
        }
    }
    (decode_html(raw), decode_html(&visible))
}

/// What an independent CommonMark reader finds in `markdown`: every link
/// as `(destination, label text)`, and the text outside links.
fn read_markdown(markdown: &str) -> (Vec<(String, String)>, String) {
    let mut found = Vec::new();
    let mut outside = String::new();
    let mut current: Option<(String, String)> = None;
    for event in Parser::new(markdown) {
        match event {
            Event::Start(Tag::Link { dest_url, .. }) => current = Some((dest_url.to_string(), String::new())),
            Event::End(TagEnd::Link) => found.extend(current.take()),
            Event::Text(text) | Event::Code(text) => match current.as_mut() {
                Some((_, label)) => label.push_str(&text),
                None => outside.push_str(&text),
            },
            _ => {}
        }
    }
    (found, outside)
}

/// Every OSC 8 destination opened in `output`, and the visible text.
fn osc8_links(output: &str) -> (Vec<String>, String) {
    let mut destinations = Vec::new();
    let mut visible = String::new();
    let mut parts = output.split("\x1b]8;;");
    visible.push_str(parts.next().unwrap_or_default());
    for part in parts {
        let (url, rest) = part.split_once("\x1b\\").unwrap_or(("", part));
        if !url.is_empty() {
            destinations.push(url.to_string());
        }
        visible.push_str(rest);
    }
    (destinations, visible)
}

fn assert_no_placeholder(context: &str, output: &str) {
    assert!(
        !output.contains(['\u{1}', '\u{2}']),
        "{context}: a parser placeholder leaked: {output:?}"
    );
}

/// Assert every public result of one component for `source`, which links
/// label `x` to `destination` and is followed by [`TAIL`].
fn assert_link_everywhere(component: &str, source: &str, destination: &str, outputs: [&str; 3], tree: &RenderNode) {
    let [html, markdown, terminal] = outputs;
    let context = format!("{component} {source:?}");

    assert_eq!(links(tree), [(destination.to_string(), "x".to_string())], "{context}: tree");
    assert_eq!(plain_text(tree), format!("x{TAIL}"), "{context}: tree text");
    assert!(inline_code_values(tree).is_empty(), "{context}: destination syntax became code: {tree:?}");

    let (href, visible) = html_link(&context, html);
    assert_eq!(href, destination, "{context}: HTML {html:?}");
    assert_eq!(visible, format!("x{TAIL}"), "{context}: HTML text {html:?}");

    let (read, outside) = read_markdown(markdown);
    assert_eq!(read, [(destination.to_string(), "x".to_string())], "{context}: Markdown {markdown:?}");
    assert_eq!(outside, TAIL, "{context}: Markdown text {markdown:?}");

    let (osc8, visible) = osc8_links(terminal);
    assert_eq!(osc8, [destination], "{context}: OSC 8 {terminal:?}");
    assert_eq!(visible.trim_end(), format!("x{TAIL}"), "{context}: terminal text {terminal:?}");

    for (target, output) in [("HTML", html), ("Markdown", markdown), ("terminal", terminal)] {
        assert_no_placeholder(&format!("{context}: {target}"), output);
    }
}

/// Assert both components keep `destination` for `source`.
fn assert_both_components(source: &str, destination: &str) {
    let term = osc8_term();
    let inline = InlineProse::new(source);
    assert_link_everywhere(
        "InlineProse",
        source,
        destination,
        [
            &inline.render_html_fragment().render(),
            &inline.render_markdown(),
            &inline.render(&term),
        ],
        &inline.render_tree(),
    );
    let prose = Prose::new(source);
    assert_link_everywhere(
        "Prose",
        source,
        destination,
        [
            &prose.render_html_fragment().render(),
            &prose.render_markdown(),
            &prose.render(&term),
        ],
        &prose.render_tree(),
    );
}

/// One test per payload: `$payload` replaces `ZQZ` in
/// `https://x.io/aZQZb`, through a Markdown link and through an explicit
/// anchor quoted with `Prose::quoted_attr`.
macro_rules! destination_payloads {
    ($($name:ident: $payload:expr;)*) => {
        mod markdown_link {
            $(
                #[test]
                fn $name() {
                    let destination = format!("https://x.io/a{}b", $payload);
                    super::assert_both_components(&format!("[x]({destination}){}", super::TAIL), &destination);
                }
            )*
        }

        mod explicit_anchor {
            use biscuit_terminal::components::prose::Prose;

            $(
                #[test]
                fn $name() {
                    let destination = format!("https://x.io/a{}b", $payload);
                    let source = format!("<a href={}>x</a>{}", Prose::quoted_attr(&destination), super::TAIL);
                    super::assert_both_components(&source, &destination);
                }
            )*
        }
    };
}

destination_payloads! {
    ordinary_url: "";
    single_backtick_code_span: "`y`";
    double_backtick_code_span: "``y``";
    double_backtick_run_around_a_backtick: "`` ` ``";
    spaced_comment: "<!-- -->";
    shortest_comment: "<!-->";
    short_comment: "<!--->";
    empty_comment: "<!---->";
    nonempty_comment: "<!-- note -->";
    two_comments_separated_by_text: "<!-- a -->m<!-- b -->";
    explicit_code_block_body: "<code-block>y</code-block>";
    unclosed_comment_stays_literal: "<!-- y";
    unmatched_backtick_stays_literal: "`y";
}

// ── the acceptance criterion's literal inputs ────────────────────────────

#[test]
fn criterion_backtick_destination_is_kept_on_every_target() {
    assert_both_components("[x](https://x.io/a`y`b) tail", "https://x.io/a`y`b");
}

#[test]
fn criterion_comment_destination_is_kept_on_every_target() {
    assert_both_components("[x](https://x.io/a<!-- -->b) tail", "https://x.io/a<!-- -->b");
}

#[test]
fn an_escaped_parenthesis_does_not_end_a_destination_holding_code() {
    assert_both_components("[x](https://x.io/a\\)`y`b) tail", "https://x.io/a)`y`b");
}

// ── the label is still prose ─────────────────────────────────────────────

#[test]
fn a_code_span_in_the_label_is_still_inline_code() {
    let source = "[`code`](https://x.io/a`y`b) tail";
    for (component, tree) in [
        ("InlineProse", InlineProse::new(source).render_tree()),
        ("Prose", Prose::new(source).render_tree()),
    ] {
        assert_eq!(inline_code_values(&tree), ["code"], "{component}: {tree:?}");
        assert_eq!(
            links(&tree),
            [("https://x.io/a`y`b".to_string(), "code".to_string())],
            "{component}"
        );
    }
}

#[test]
fn a_comment_in_the_label_is_still_a_comment() {
    let source = "[x<!-- -->y](https://x.io/a<!-- -->b) tail";
    for (component, tree) in [
        ("InlineProse", InlineProse::new(source).render_tree()),
        ("Prose", Prose::new(source).render_tree()),
    ] {
        assert_eq!(
            links(&tree),
            [("https://x.io/a<!-- -->b".to_string(), "xy".to_string())],
            "{component}"
        );
    }
}

#[test]
fn an_unclosed_comment_outside_a_link_stays_literal() {
    let source = "[x](https://x.io/ab) <!-- y";
    for (component, tree) in [
        ("InlineProse", InlineProse::new(source).render_tree()),
        ("Prose", Prose::new(source).render_tree()),
    ] {
        assert_eq!(plain_text(&tree), "x <!-- y", "{component}");
    }
}

// ── only a real destination is lifted ────────────────────────────────────

#[test]
fn a_blank_line_ends_a_prose_label_but_not_an_inline_one() {
    // `Prose` splits the paragraph first, so `b](u`y`)` is no link and the
    // code span is code; `InlineProse` reads one link across the blank line.
    let source = "[a\n\nb](u`y`)";

    let prose = Prose::new(source).render_tree();
    assert!(links(&prose).is_empty(), "{prose:?}");
    assert_eq!(inline_code_values(&prose), ["y"]);

    let inline = InlineProse::new(source).render_tree();
    assert_eq!(links(&inline), [("u`y`".to_string(), "a b".to_string())]);
    assert!(inline_code_values(&inline).is_empty(), "{inline:?}");
}

#[test]
fn a_parenthesis_without_a_label_is_still_prose() {
    let source = "a](u`y`) and \\[x](v`z`)";
    for (component, tree) in [
        ("InlineProse", InlineProse::new(source).render_tree()),
        ("Prose", Prose::new(source).render_tree()),
    ] {
        assert!(links(&tree).is_empty(), "{component}: {tree:?}");
        assert_eq!(inline_code_values(&tree), ["y", "z"], "{component}");
    }
}

#[test]
fn a_destination_no_link_reads_renders_as_its_text() {
    // The `]` inside `<x ]>` closes the label for the opaque-content scan,
    // but the link reader skips the whole declaration, so `(u)` is label
    // text and the link is `(c)`.
    let source = "[a <x ]>(u) ](c)";
    for (component, tree) in [
        ("InlineProse", InlineProse::new(source).render_tree()),
        ("Prose", Prose::new(source).render_tree()),
    ] {
        assert_eq!(links(&tree), [("c".to_string(), "a <x ]>(u) ".to_string())], "{component}");
    }
}
