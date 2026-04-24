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

    // Terminal rendering is reachable via the Renderable trait (also prelude).
    let term = Terminal::default();
    let out = rule.render(&term);
    assert!(!out.is_empty(), "expected non-empty terminal output");
}

#[test]
fn prelude_exports_browser_renderable_trait() {
    // `BrowserRenderable` must be importable from the prelude so generic
    // code can be written against it without reaching into submodules.
    fn takes_browser_renderable<T: BrowserRenderable>(r: &T) -> String {
        r.render_to_browser()
    }

    let rule = HorizontalRule::new();
    let svg = takes_browser_renderable(&rule);
    assert!(svg.contains("<svg"), "expected SVG output: {svg}");
}
