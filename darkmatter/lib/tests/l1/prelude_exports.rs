//! Verifies that `darkmatter::prelude` re-exports every renderable component
//! defined in Darkmatter alongside the render traits, so downstream crates
//! can pull them in with a single glob import.
//!
//! When a new renderable component is added to Darkmatter, it must be added
//! to `darkmatter/lib/src/prelude.rs` and exercised here. A component is in
//! scope for this test if it implements `TerminalRenderable` or
//! `BrowserRenderable` directly within the Darkmatter crate.

use darkmatter::prelude::*;

#[test]
fn prelude_exports_render_traits() {
    // The render traits must be reachable so generic helpers can be written
    // against the prelude alone, without reaching into `biscuit_terminal`.
    fn takes_terminal_renderable<T: TerminalRenderable>(
        r: &T,
        term: &biscuit_terminal::terminal::Terminal,
    ) -> String {
        r.render(term)
    }

    fn takes_browser_renderable<T: BrowserRenderable>(r: &T) -> String {
        r.render_html_fragment().render()
    }

    let block = CodeBlock::rust("fn main() {}");
    let term = biscuit_terminal::terminal::Terminal::default();

    assert!(!takes_terminal_renderable(&block, &term).is_empty());
    assert!(takes_browser_renderable(&block).contains("<div"));
}

#[test]
fn prelude_exports_code_block() {
    let block = CodeBlock::yaml("name: darkmatter");
    let term = biscuit_terminal::terminal::Terminal::default();

    assert!(!block.render(&term).is_empty());
    assert!(block.render_html_fragment().render().contains("<div"));
}

#[test]
fn prelude_exports_darkmatter_page() {
    use darkmatter::markdown::Markdown;

    let term = biscuit_terminal::terminal::Terminal::new_optimistic(80);
    let md: Markdown = "# Prelude export check\n".into();
    let page = DarkmatterPage::new(&term);
    let output = page.render(&md).expect("page should render");

    assert!(output.contains("Prelude export check"));
}

#[test]
fn prelude_exports_delta_report() {
    // `MarkdownDelta::new()` is an empty delta (no changes). The prelude
    // export is exercised by constructing the terminal view and rendering it.
    let delta = darkmatter::markdown::MarkdownDelta::new();
    let report = DeltaReport::new(delta);
    let term = biscuit_terminal::terminal::Terminal::default();

    let output = report.render(&term);
    assert!(!output.is_empty(), "delta report should produce output");
}

#[test]
fn prelude_exports_toc_tree() {
    use darkmatter::markdown::Markdown;

    let md: Markdown = "# Heading\n\nBody".into();
    let toc = md.toc();
    let tree = TocTree::new(toc).with_filename("example.md");
    let term = biscuit_terminal::terminal::Terminal::default();

    let output = tree.render(&term);
    assert!(
        output.contains("Heading"),
        "toc tree should include the heading"
    );
}

#[test]
fn prelude_exports_validation_report_view() {
    use darkmatter::markdown::Markdown;
    use darkmatter::markdown::reference::validate::ReferenceValidationOptions;

    let md = Markdown::new("# Doc with no references");
    let report = md
        .validate_references(ReferenceValidationOptions::default())
        .expect("validation should succeed");
    let view = ValidationReportView::new(report);
    let term = biscuit_terminal::terminal::Terminal::default();

    // An empty validation report renders to empty output. The point of this
    // assertion is that `.render(&term)` resolves through the prelude-imported
    // `TerminalRenderable` trait.
    let output = view.render(&term);
    assert!(
        output.is_empty(),
        "empty validation report should render empty"
    );
}

#[test]
fn prelude_exports_file_tree() {
    use darkmatter::markdown::Markdown;

    // Construct from an already-loaded Markdown so the test does not depend
    // on filesystem layout outside the crate.
    let md = Markdown::new("# Heading\n\n::file ./peer.md\n\nBody");
    let tree = FileTree::from_markdown(md);
    let term = biscuit_terminal::terminal::Terminal::default();

    let output = tree.render(&term);
    assert!(!output.is_empty(), "file tree should produce output");
}

#[allow(deprecated)]
#[test]
fn prelude_exports_yaml_block() {
    // `YamlBlock` is deprecated but still a renderable component defined in
    // Darkmatter; it remains in the prelude so existing callers continue to
    // compile unchanged.
    let block = YamlBlock::new("name: darkmatter").expect("YamlBlock should construct");
    let term = biscuit_terminal::terminal::Terminal::default();

    assert!(!block.render(&term).is_empty());
    assert!(block.render_html_fragment().render().contains("<div"));
}
