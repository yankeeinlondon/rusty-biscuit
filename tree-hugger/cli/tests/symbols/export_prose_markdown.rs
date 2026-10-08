use super::*;

const SOURCE: &str = "/// [x](url) **bold** `code` <img> &copy;\nfn sample() {}\n";

#[test]
fn prose_and_markdown_are_public_formats() {
    let scratch = Scratch::new(&[("src/lib.rs", SOURCE)]);
    let prose = scratch.stdout(&["functions", "--format", "prose"]);
    assert!(prose.contains("sample"));
    let markdown = scratch.stdout(&["functions", "--format", "markdown"]);
    assert!(markdown.contains("sample"));
    assert!(!markdown.contains("\u{1b}"));
    assert!(!markdown.contains("\r"));
}

#[test]
fn markdown_source_comment_syntax_is_not_promoted_to_markup() {
    let scratch = Scratch::new(&[("src/lib.rs", SOURCE)]);
    let markdown = scratch.stdout(&["functions", "--format", "markdown", "--comments"]);
    let events: Vec<_> = pulldown_cmark::Parser::new(&markdown).collect();
    assert!(!events.iter().any(|event| matches!(event,
        pulldown_cmark::Event::Html(_) | pulldown_cmark::Event::InlineHtml(_) |
        pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link { .. }) |
        pulldown_cmark::Event::Start(pulldown_cmark::Tag::Image { .. }) |
        pulldown_cmark::Event::Code(_) |
        pulldown_cmark::Event::Start(pulldown_cmark::Tag::Emphasis) |
        pulldown_cmark::Event::Start(pulldown_cmark::Tag::Strong) |
        pulldown_cmark::Event::Start(pulldown_cmark::Tag::Heading { .. })
    )), "source comment syntax must remain text: {markdown:?}");
}

#[test]
fn prose_plain_omits_generated_style_and_link_tags() {
    let scratch = Scratch::new(&[("src/lib.rs", SOURCE)]);
    let prose = scratch.stdout(&["functions", "--format", "prose", "--plain"]);
    assert!(prose.contains("sample"));
    assert!(!prose.contains("<b>") && !prose.contains("<dim>") && !prose.contains("<a href="));
    assert!(!prose.contains("\u{1b}"));
}
