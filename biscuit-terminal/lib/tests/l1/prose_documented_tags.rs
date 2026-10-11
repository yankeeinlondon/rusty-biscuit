//! Background tags that the Prose documentation recommends must be accepted.
//!
//! Every `<bg-…>` declaration in the prose topic page and the styling skill
//! is rendered through `Prose` and `InlineProse` to HTML and must produce a
//! background declaration, so a doc edit that recommends a spelling the
//! resolver keeps as literal text fails here. The docs name the unsupported
//! spellings without angle brackets (`bg-blue`) so this scan skips them.

use biscuit_terminal::components::prose::{InlineProse, Prose};
use renderable::browser::BrowserRenderable;

/// Every opening `<bg-…>` declaration in `doc`, as authored.
fn background_declarations(doc: &str) -> Vec<&str> {
    doc.match_indices("<bg-")
        .filter_map(|(start, _)| doc[start..].find('>').map(|end| &doc[start..=start + end]))
        .collect()
}

/// HTML from both components for `<decl>x</name>`.
fn html_of(declaration: &str) -> [(&'static str, String); 2] {
    let name = declaration
        .trim_start_matches('<')
        .trim_end_matches('>')
        .split(' ')
        .next()
        .unwrap_or_default();
    let source = format!("{declaration}x</{name}>");
    [
        ("Prose", Prose::new(source.clone()).render_html_fragment().render()),
        ("InlineProse", InlineProse::new(source).render_html_fragment().render()),
    ]
}

fn assert_background(declaration: &str) {
    for (component, html) in html_of(declaration) {
        assert!(
            html.contains("background-color:") && !html.contains("&lt;bg-"),
            "{component} renders documented {declaration} without a background: {html}"
        );
    }
}

fn assert_literal(declaration: &str) {
    for (component, html) in html_of(declaration) {
        assert!(
            html.contains("&lt;bg-") && !html.contains("background-color"),
            "{component} styles {declaration}, which the docs say stays literal: {html}"
        );
    }
}

#[test]
fn prose_topic_page_background_examples_render_a_background() {
    let doc = include_str!("../../../docs/components/prose.md");
    let declarations = background_declarations(doc);
    assert!(!declarations.is_empty(), "expected background examples in the doc");
    for declaration in declarations {
        assert_background(declaration);
    }
}

#[test]
fn styling_skill_background_examples_render_a_background() {
    let doc = include_str!("../../../../.claude/skills/biscuit-terminal/styling.md");
    let declarations = background_declarations(doc);
    assert!(!declarations.is_empty(), "expected background examples in the doc");
    for declaration in declarations {
        assert_background(declaration);
    }
}

#[test]
fn basic_and_bright_names_do_not_take_the_background_prefix() {
    for name in ["red", "green", "yellow", "blue", "magenta", "cyan"] {
        assert_literal(&format!("<bg-{name}>"));
    }
    for name in ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"] {
        assert_literal(&format!("<bg-bright-{name}>"));
    }
    // Tailwind names, not the basic colors.
    assert_background("<bg-black>");
    assert_background("<bg-white>");
}
