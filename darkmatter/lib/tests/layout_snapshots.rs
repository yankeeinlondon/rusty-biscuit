//! Snapshot tests for the DarkmatterPage end-to-end example from the layout spec.
//!
//! These tests capture the full terminal output for the worked example described
//! in `darkmatter/features/2026-05-08-darkmatter-layout/spec.md`.

// `DarkmatterPage::use_alignment` only accepts the deprecated `PageAlignment`;
// exercising the builder here legitimately references it, matching the
// `#![allow(deprecated)]` on the `layout` source modules.
#![allow(deprecated)]

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

/// Zero-config page must match the default `as_terminal` render exactly.
///
/// Post tree-cutover both routes go through the render-tree terminal document
/// renderer, so a zero-config page adds nothing on top of the public terminal
/// render. This equivalence holds regardless of the test process's ambient
/// terminal state, because both sides resolve the same default terminal.
#[test]
fn zero_config_prose_snapshot() {
    use darkmatter::markdown::output::terminal::{ColorDepth, TerminalOptions};

    let term = Terminal::new_optimistic(120);
    let page = DarkmatterPage::new(&term);
    let md: Markdown = "# Hello World\n\nSome prose here.\n".into();

    let page_out = page.render(&md).unwrap();
    let direct_out = md.as_terminal(TerminalOptions::default()).unwrap();
    assert_eq!(page_out, direct_out);

    // Snapshot a width- and color-pinned render so the bytes are deterministic:
    // the zero-config render above resolves the *ambient* terminal, whose color
    // depth and tty state vary between a real terminal and a piped test process.
    let mut pinned_opts = TerminalOptions::default();
    pinned_opts.max_width = Some(120);
    pinned_opts.color_depth = Some(ColorDepth::TrueColor);
    let pinned = md.as_terminal(pinned_opts).unwrap();
    insta::assert_snapshot!(pinned);
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
