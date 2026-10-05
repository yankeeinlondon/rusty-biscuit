//! Literal backslashes in `Prose` and `InlineProse` Markdown output.
//!
//! Each case renders Markdown and MarkdownPlus, then reads the output back
//! with `pulldown-cmark`, an independent CommonMark reader, so the assertion
//! is about what a Markdown consumer sees: the literal text and the kind of
//! each break, not the escaping bytes chosen to get there.

use biscuit_terminal::components::prose::{InlineProse, LineBreaks, Prose};
use pulldown_cmark::{Event, Parser, Tag};
use renderable::markdown::MarkdownRenderable;

/// Visible text with `{SB}` per soft break, `{HB}` per hard break, and `¶`
/// before every paragraph after the first.
fn read_back(markdown: &str) -> String {
    let mut out = String::new();
    let mut paragraphs = 0;
    for event in Parser::new(markdown) {
        match event {
            Event::Text(text) => out.push_str(&text),
            Event::SoftBreak => out.push_str("{SB}"),
            Event::HardBreak => out.push_str("{HB}"),
            Event::Start(Tag::Paragraph) => {
                if paragraphs > 0 {
                    out.push('¶');
                }
                paragraphs += 1;
            }
            _ => {}
        }
    }
    out
}

/// Markdown and MarkdownPlus output for both components in one line-break mode.
fn outputs(source: &str, mode: LineBreaks) -> Vec<(&'static str, String)> {
    let prose = Prose::new(source).with_line_breaks(mode);
    let inline = InlineProse::new(source).with_line_breaks(mode);
    vec![
        ("Prose/Markdown", prose.render_markdown()),
        ("Prose/MarkdownPlus", prose.render_markdown_plus()),
        ("InlineProse/Markdown", inline.render_markdown()),
        ("InlineProse/MarkdownPlus", inline.render_markdown_plus()),
    ]
}

fn assert_reads_back(source: &str, mode: LineBreaks, expected: &str) {
    for (label, markdown) in outputs(source, mode) {
        assert_eq!(
            read_back(&markdown),
            expected,
            "{label} {mode:?} source {source:?}\nMarkdown: {markdown:?}"
        );
    }
}

#[test]
fn escaped_backslash_before_newline_keeps_backslash_and_soft_break() {
    assert_reads_back("a\\\\\nb", LineBreaks::Soft, "a\\{SB}b");
}

#[test]
fn escaped_backslash_before_newline_keeps_backslash_and_hard_break() {
    assert_reads_back("a\\\\\nb", LineBreaks::Hard, "a\\{HB}b");
}

#[test]
fn three_backslashes_before_newline_are_literal_backslash_plus_explicit_hard_break() {
    for mode in [LineBreaks::Soft, LineBreaks::Hard] {
        assert_reads_back("a\\\\\\\nb", mode, "a\\{HB}b");
    }
}

#[test]
fn prose_escaped_backslash_before_blank_line_stays_literal() {
    for mode in [LineBreaks::Soft, LineBreaks::Hard] {
        let prose = Prose::new("a\\\\\n\nb").with_line_breaks(mode);
        for markdown in [prose.render_markdown(), prose.render_markdown_plus()] {
            assert_eq!(read_back(&markdown), "a\\¶b", "{mode:?} {markdown:?}");
        }
    }
}

#[test]
fn escaped_backslash_inside_bold_keeps_emphasis_before_newline() {
    for (mode, marker) in [(LineBreaks::Soft, "{SB}"), (LineBreaks::Hard, "{HB}")] {
        for (label, markdown) in outputs("**a\\\\**\nb", mode) {
            let strong = Parser::new(&markdown)
                .filter(|event| matches!(event, Event::Start(Tag::Strong)))
                .count();
            assert_eq!(strong, 1, "{label} {mode:?} {markdown:?}");
            assert_eq!(
                read_back(&markdown),
                format!("a\\{marker}b"),
                "{label} {mode:?} {markdown:?}"
            );
        }
    }
}

#[test]
fn ordinary_breaks_and_trailing_backslash_controls_are_unchanged() {
    assert_reads_back("a\nb", LineBreaks::Soft, "a{SB}b");
    assert_reads_back("a\nb", LineBreaks::Hard, "a{HB}b");
    assert_reads_back("a\\\nb", LineBreaks::Soft, "a{HB}b");
    assert_reads_back("a\\\\", LineBreaks::Soft, "a\\");
}

/// The text inside each `Strong` run of `markdown`, read back.
fn strong_runs(markdown: &str) -> Vec<String> {
    let mut runs = Vec::new();
    let mut current: Option<String> = None;
    for event in Parser::new(markdown) {
        match event {
            Event::Start(Tag::Strong) => current = Some(String::new()),
            Event::End(pulldown_cmark::TagEnd::Strong) => runs.extend(current.take()),
            Event::Text(text) => {
                if let Some(run) = current.as_mut() {
                    run.push_str(&text);
                }
            }
            _ => {}
        }
    }
    runs
}

#[test]
fn bold_with_edge_whitespace_or_a_break_stays_bold_in_markdown() {
    // A `**` run next to whitespace or a line ending neither opens nor closes
    // bold, so the whitespace and the break are written outside it.
    for (source, bold) in [("a<b> b</b>", "b"), ("<b>a </b>b", "a"), ("a <b>\nb</b>", "b")] {
        for mode in [LineBreaks::Soft, LineBreaks::Hard] {
            for (label, markdown) in outputs(source, mode) {
                assert_eq!(
                    strong_runs(&markdown),
                    vec![bold.to_string()],
                    "{label} {mode:?} source {source:?}\nMarkdown: {markdown:?}"
                );
            }
        }
    }
}

/// What a reader sees: text verbatim, `<strong>`/`<em>`/`<del>`/`<a>`
/// markers, `{html}` for inline HTML, and `<h>`/`<list>`/`<quote>` for
/// blocks, so an assertion catches literal text that turned into structure.
fn read_structure(markdown: &str) -> String {
    use pulldown_cmark::{Options, TagEnd};
    let mut out = String::new();
    for event in Parser::new_ext(markdown, Options::ENABLE_STRIKETHROUGH) {
        match event {
            Event::Text(text) => out.push_str(&text),
            Event::Code(code) => out.push_str(&format!("{{code:{code}}}")),
            Event::SoftBreak => out.push_str("{SB}"),
            Event::HardBreak => out.push_str("{HB}"),
            Event::InlineHtml(_) | Event::Html(_) => out.push_str("{html}"),
            Event::Start(Tag::Strong) => out.push_str("<strong>"),
            Event::End(TagEnd::Strong) => out.push_str("</strong>"),
            Event::Start(Tag::Emphasis) => out.push_str("<em>"),
            Event::End(TagEnd::Emphasis) => out.push_str("</em>"),
            Event::Start(Tag::Strikethrough) => out.push_str("<del>"),
            Event::End(TagEnd::Strikethrough) => out.push_str("</del>"),
            Event::Start(Tag::Link { dest_url, .. }) => out.push_str(&format!("<a {dest_url}>")),
            Event::End(TagEnd::Link) => out.push_str("</a>"),
            Event::Start(Tag::Heading { .. }) => out.push_str("<h>"),
            Event::Start(Tag::List(_)) => out.push_str("<list>"),
            Event::Start(Tag::BlockQuote(_)) => out.push_str("<quote>"),
            _ => {}
        }
    }
    out
}

/// Literal values and the Prose source that spells each one: the review's
/// authored escapes, then `Prose::escape_text` for the rest.
fn literal_sources() -> Vec<(String, String)> {
    let mut sources = vec![
        (r"\**literal\**".to_string(), "**literal**".to_string()),
        (r"\_literal\_".to_string(), "_literal_".to_string()),
        (r"\<em>literal\</em>".to_string(), "<em>literal</em>".to_string()),
        ("&copy;".to_string(), "&copy;".to_string()),
    ];
    for value in [
        "[literal](https://x.io)",
        "**literal**",
        "<em>literal</em>",
        "# literal",
        "- literal",
        "1. literal",
        "> literal",
        "a &amp; b",
    ] {
        sources.push((Prose::escape_text(value), value.to_string()));
    }
    sources
}

/// Prose contexts around an escaped source `s`, with the expected reading
/// for literal value `v` per dialect (`true` for MarkdownPlus).
type ProseContext = (&'static str, fn(&str) -> String, fn(&str, bool) -> String);

fn prose_contexts() -> Vec<ProseContext> {
    vec![
        ("plain", |s| s.to_string(), |v, _| v.to_string()),
        ("after text", |s| format!("a {s}"), |v, _| format!("a {v}")),
        (
            "line after a hard break",
            |s| format!("a\\\n{s}"),
            |v, _| format!("a{{HB}}{v}"),
        ),
        (
            "bold",
            |s| format!("a <b>{s}</b> b"),
            |v, _| format!("a <strong>{v}</strong> b"),
        ),
        (
            "italic",
            |s| format!("<i>{s}</i>"),
            |v, _| format!("<em>{v}</em>"),
        ),
        (
            "strike",
            |s| format!("<~>{s}</~>"),
            |v, _| format!("<del>{v}</del>"),
        ),
        (
            "color",
            |s| format!("<red>{s}</red>"),
            |v, plus| {
                if plus {
                    format!("{{html}}{v}{{html}}")
                } else {
                    v.to_string()
                }
            },
        ),
        (
            "link label",
            |s| format!("[{s}](https://e.io)"),
            |v, _| format!("<a https://e.io>{v}</a>"),
        ),
    ]
}

#[test]
fn escaped_markdown_syntax_reads_back_as_literal_text_in_both_components_and_dialects() {
    let mut failures = Vec::new();
    for (source, value) in literal_sources() {
        for (name, wrap, expect) in prose_contexts() {
            let source = wrap(&source);
            for (label, markdown) in outputs(&source, LineBreaks::Soft) {
                let expected = expect(&value, label.ends_with("MarkdownPlus"));
                let actual = read_structure(&markdown);
                if actual != expected {
                    failures.push(format!(
                        "{label} {name} source {source:?}\n  markdown: {markdown:?}\n  read:     \
                         {actual:?}\n  expected: {expected:?}"
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn escape_text_inserts_a_literal_link_example() {
    let example = "[docs](https://x.io/a_b) and **bold**";
    let source = format!("See {} here", Prose::escape_text(example));
    for (label, markdown) in outputs(&source, LineBreaks::Soft) {
        assert_eq!(
            read_structure(&markdown),
            format!("See {example} here"),
            "{label} {markdown:?}"
        );
    }
}

#[test]
fn code_spans_and_backslash_controls_keep_their_contents() {
    for (label, markdown) in outputs(r"`**x** &copy;` and C:\dir", LineBreaks::Soft) {
        assert_eq!(
            read_structure(&markdown),
            r"{code:**x** &copy;} and C:\dir",
            "{label} {markdown:?}"
        );
    }
}

/// The literal reading of a render tree: its text values, `{SB}`/`{HB}`
/// per break, and the number of paragraphs it holds.
fn tree_reading(node: &renderable::tree::RenderNode, text: &mut String, paragraphs: &mut usize) {
    use renderable::tree::NodeKind;
    match &node.kind {
        NodeKind::Text { value } => text.push_str(value),
        NodeKind::SoftBreak => text.push_str("{SB}"),
        NodeKind::HardBreak => text.push_str("{HB}"),
        kind => {
            if matches!(kind, NodeKind::Paragraph { .. }) {
                *paragraphs += 1;
            }
            for child in node.children() {
                tree_reading(child, text, paragraphs);
            }
        }
    }
}

/// What a reader sees in `markdown`: text with `{SB}`/`{HB}`, the number of
/// paragraphs, and any structure other than paragraphs and inline HTML.
fn markdown_reading(markdown: &str) -> (String, usize, Vec<String>) {
    use pulldown_cmark::Options;
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_FOOTNOTES;
    let mut text = String::new();
    let mut paragraphs = 0;
    let mut structure = Vec::new();
    for event in Parser::new_ext(markdown, options) {
        match event {
            Event::Text(value) => text.push_str(&value),
            Event::SoftBreak => text.push_str("{SB}"),
            Event::HardBreak => text.push_str("{HB}"),
            Event::Start(Tag::Paragraph) => paragraphs += 1,
            Event::InlineHtml(_) | Event::End(_) => {}
            other => structure.push(format!("{other:?}")),
        }
    }
    (text, paragraphs, structure)
}

#[test]
fn markdown_keeps_email_looking_text_split_markers_and_indentation_literal() {
    use renderable::tree::TreeRenderable;
    let sources = [
        "<3@example.com>",
        "1<clipboard>. literal</clipboard>",
        "12<clipboard>) literal</clipboard>",
        "    literal",
        "\tliteral",
        "a\\\n    literal",
    ];
    let mut failures = Vec::new();
    for source in sources {
        let prose = Prose::new(source);
        let inline = InlineProse::new(source);
        let components = [
            ("Prose", prose.render_tree(), prose.render_markdown(), prose.render_markdown_plus()),
            (
                "InlineProse",
                inline.render_tree(),
                inline.render_markdown(),
                inline.render_markdown_plus(),
            ),
        ];
        for (component, tree, markdown, markdown_plus) in components {
            let mut expected = String::new();
            let mut paragraphs = 0;
            tree_reading(&tree, &mut expected, &mut paragraphs);
            for (dialect, output) in [("Markdown", markdown), ("MarkdownPlus", markdown_plus)] {
                let (text, read_paragraphs, structure) = markdown_reading(&output);
                if text != expected || read_paragraphs != paragraphs.max(1) || !structure.is_empty()
                {
                    failures.push(format!(
                        "{component}/{dialect} source {source:?}\n  markdown: {output:?}\n  \
                         read: {text:?} in {read_paragraphs} paragraph(s), structure {structure:?}\n  \
                         tree: {expected:?} in {paragraphs} paragraph(s)"
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
