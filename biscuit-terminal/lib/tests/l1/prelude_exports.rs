//! Verifies that `biscuit_terminal::prelude` re-exports the `HorizontalRule`
//! component and related enums alongside the `BrowserRenderable` trait, so
//! downstream crates can pull them in with a single glob import.

use biscuit_terminal::prelude::*;

#[test]
fn prelude_exports_horizontal_rule_types() {
    // Builder-based construction compiles using only prelude imports.
    let rule = HorizontalRule::new()
        .style(RuleStyle::Dashes)
        .alignment(RuleAlignment::Full)
        .weight(RuleWeight::Medium);

    // Terminal rendering is reachable via the TerminalRenderable trait (also prelude).
    let term = Terminal::default();
    let out = rule.render(&term);
    assert!(!out.is_empty(), "expected non-empty terminal output");
}

#[test]
fn prelude_exports_browser_renderable_trait() {
    // `BrowserRenderable` must be importable from the prelude so generic
    // code can be written against it without reaching into submodules.
    fn takes_browser_renderable<T: BrowserRenderable>(r: &T) -> String {
        r.render_html_fragment().render()
    }

    let rule = HorizontalRule::new();
    let svg = takes_browser_renderable(&rule);
    assert!(svg.contains("<svg"), "expected SVG output: {svg}");
}

#[test]
fn prelude_exports_prose_components() {
    // Both prose components and their option enums come from the prelude.
    let block = Prose::new("a\nb")
        .with_line_breaks(LineBreaks::Hard)
        .with_tag(ProseTag::Div);
    assert_eq!(block.render_html_fragment().render(), "<div>a<br>b</div>");
    let inline = InlineProse::new("`x`");
    assert_eq!(inline.render_html_fragment().render(), "<code>x</code>");
}
