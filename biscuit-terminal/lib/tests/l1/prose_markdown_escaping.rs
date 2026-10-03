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
