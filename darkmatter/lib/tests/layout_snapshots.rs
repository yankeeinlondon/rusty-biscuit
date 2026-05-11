//! Snapshot tests for the DarkmatterPage end-to-end example from the layout spec.
//!
//! These tests capture the full terminal output for the worked example described
//! in `darkmatter/features/2026-05-08-darkmatter-layout/spec.md`.

use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::{DarkmatterPage, PageAlignment, PageBackground, PageComponent};
use darkmatter::markdown::Markdown;

/// End-to-end example from the spec:
/// Terminal is dark mode, 120 cols wide.
/// --margin 2 --padding 1 --page-bg subtle --max-width 100 --line-numbers true --align-code-blocks center
#[test]
fn end_to_end_example_snapshot() {
    let term = Terminal::new_optimistic(120);
    let page = DarkmatterPage::new(&term)
        .with_margin(2)
        .with_padding(1)
        .with_page_background(PageBackground::Subtle)
        .with_max_width(100)
        .use_line_numbers()
        .use_alignment(PageComponent::CodeBlocks, PageAlignment::Center);
    let md: Markdown = "# Title\n\nSome prose here.\n\n```rust\nfn main() {}\n```\n".into();

    let out = page.render(&md).unwrap();
    insta::assert_snapshot!(out);
}

/// Zero-config page should match for_terminal output exactly.
#[test]
fn zero_config_prose_snapshot() {
    let term = Terminal::new_optimistic(120);
    let page = DarkmatterPage::new(&term);
    let md: Markdown = "# Hello World\n\nSome prose here.\n".into();

    let page_out = page.render(&md).unwrap();
    let direct_out = darkmatter::markdown::output::terminal::for_terminal(
        &md,
        darkmatter::markdown::output::terminal::TerminalOptions::default(),
    )
    .unwrap();

    assert_eq!(page_out, direct_out);
    insta::assert_snapshot!(page_out);
}

/// Pronounced background on dark terminal produces near-white surface.
#[test]
fn pronounced_background_snapshot() {
    let term = Terminal::new_optimistic(80);
    let page = DarkmatterPage::new(&term).with_page_background(PageBackground::Pronounced);
    let md: Markdown = "# Hello\n".into();

    let out = page.render(&md).unwrap();
    insta::assert_snapshot!(out);
}
