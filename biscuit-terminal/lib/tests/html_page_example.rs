//! Representative example: assemble components into one HTML page.
//!
//! This integration test doubles as the canonical "build an HTML page from
//! components" example required by Project 3 of the renderable kickoff.

use biscuit_terminal::components::HorizontalRule;
use renderable::browser::BrowserRenderable;
use renderable::html::HtmlPage;

/// Compose a `HorizontalRule` fragment into an `HtmlPage` and render it.
#[test]
fn components_compose_into_an_html_page() {
    let rule = HorizontalRule::new();
    let fragment = rule.render_html_fragment();
    let page = HtmlPage::from(fragment);
    let html = page.render().expect("render");
    assert!(
        html.contains("<html"),
        "rendered page must have an <html> root"
    );
    assert!(html.contains("<head"), "rendered page must have a <head>");
    assert!(html.contains("<body"), "rendered page must have a <body>");
    assert!(
        html.contains("<svg"),
        "the HorizontalRule SVG must appear in the body"
    );
}

/// `render_html_page` promotes a single component to a standalone page.
#[test]
fn render_html_page_promotes_a_single_component() {
    let rule = HorizontalRule::new();
    let page = rule.render_html_page(None);
    let html = page.render().expect("render");
    assert!(
        html.contains("<html"),
        "promoted page must have an <html> root"
    );
    assert!(
        html.contains("<svg"),
        "promoted page must contain the rule SVG"
    );
}
