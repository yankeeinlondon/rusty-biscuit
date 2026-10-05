//! `TwoColumn` MarkdownPlus output read back by an independent reader.
//!
//! MarkdownPlus writes the columns as one raw HTML block, which a CommonMark
//! reader passes through without parsing it as Markdown. The column bodies
//! must therefore be HTML: code an escaped `<code>` element, bold a
//! `<strong>`, and a link an `<a>`, exactly as the browser renders the same
//! component. Each case reads the output with `pulldown-cmark` and then reads
//! the HTML a browser would see.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::two_column::TwoColumn;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use renderable::browser::BrowserRenderable;
use renderable::markdown::MarkdownRenderable;

/// The raw HTML of a document that a reader takes as exactly one HTML block
/// and nothing else.
fn single_html_block(markdown: &str) -> String {
    let mut html = String::new();
    let mut blocks = 0;
    for event in Parser::new_ext(markdown, Options::all()) {
        match event {
            Event::Start(Tag::HtmlBlock) => blocks += 1,
            Event::End(TagEnd::HtmlBlock) => {}
            Event::Html(line) => html.push_str(&line),
            other => panic!("{other:?} outside the HTML block in:\n{markdown}"),
        }
    }
    assert_eq!(blocks, 1, "one HTML block expected in:\n{markdown}");
    html
}

fn decode(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#10;", "\n")
        .replace("&amp;", "&")
}

/// The decoded text of every `<code>` element in `html`.
fn code_texts(html: &str) -> Vec<String> {
    html.split("<code>")
        .skip(1)
        .filter_map(|rest| rest.split_once("</code>"))
        .map(|(body, _)| decode(body))
        .collect()
}

fn markdown_plus(left: &str, right: &str) -> (String, String) {
    let columns = TwoColumn::new(Prose::new(left), Prose::new(right));
    let markdown = columns.render_markdown_plus();
    let browser = columns.render_html_fragment().render();
    (markdown, browser)
}

#[test]
fn code_in_a_column_stays_literal_code() {
    for value in ["<em>x</em>", "&copy;", "**x**", "[x](u)"] {
        let (markdown, browser) = markdown_plus(&format!("`{value}`"), "right");
        let html = single_html_block(&markdown);
        assert_eq!(code_texts(&html), vec![value.to_string()], "{markdown}");
        assert!(!html.contains("<em"), "no emphasis element: {markdown}");
        assert!(!html.contains('`'), "no visible backticks: {markdown}");
        assert_eq!(html.trim_end(), browser, "reads as the browser renders it");
    }
}

#[test]
fn formatting_and_links_in_a_column_are_html_elements() {
    let (markdown, browser) = markdown_plus("**b** and [l](https://e.io)", "right");
    let html = single_html_block(&markdown);
    assert!(html.contains("<strong>b</strong>"), "{markdown}");
    assert!(html.contains(r#"<a href="https://e.io">l</a>"#), "{markdown}");
    assert!(!html.contains("**") && !html.contains("]("), "{markdown}");
    assert_eq!(html.trim_end(), browser, "reads as the browser renders it");
}

#[test]
fn several_paragraphs_in_a_column_stay_inside_the_container() {
    let (markdown, browser) = markdown_plus("one\n\n`<b>two</b>`", "right");
    let html = single_html_block(&markdown);
    assert_eq!(code_texts(&html), vec!["<b>two</b>".to_string()], "{markdown}");
    assert!(html.trim_end().ends_with("</div></div>"), "{markdown}");
    assert_eq!(html.trim_end(), browser, "reads as the browser renders it");
}
