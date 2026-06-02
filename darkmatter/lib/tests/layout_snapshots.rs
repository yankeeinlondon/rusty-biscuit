//! Snapshot tests for the DarkmatterPage end-to-end example from the layout spec.
//!
//! These tests capture the full terminal output for the worked example described
//! in `darkmatter/features/2026-05-08-darkmatter-layout/spec.md`.

// Exercises `DarkmatterPage`'s still-active `Page*` types; migration to
// `renderable::layout::Layout` is the deferred Spec A milestone. Mirrors the
// library's own allow in `darkmatter/lib/src/layout/mod.rs`.
#![allow(deprecated)]

use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::{DarkmatterPage, PageAlignment, PageBackground, PageComponent};
use darkmatter::markdown::output::terminal::{ColorDepth, TerminalOptions};
use darkmatter::markdown::Markdown;

struct ColorSnapshotEnv {
    no_color: Option<String>,
    force_color: Option<String>,
    clicolor_force: Option<String>,
}

impl ColorSnapshotEnv {
    fn force_color() -> Self {
        let env = Self {
            no_color: std::env::var("NO_COLOR").ok(),
            force_color: std::env::var("FORCE_COLOR").ok(),
            clicolor_force: std::env::var("CLICOLOR_FORCE").ok(),
        };

        unsafe {
            std::env::remove_var("NO_COLOR");
            std::env::set_var("FORCE_COLOR", "1");
            std::env::set_var("CLICOLOR_FORCE", "1");
        }

        env
    }
}

impl Drop for ColorSnapshotEnv {
    fn drop(&mut self) {
        unsafe {
            match &self.no_color {
                Some(value) => std::env::set_var("NO_COLOR", value),
                None => std::env::remove_var("NO_COLOR"),
            }
            match &self.force_color {
                Some(value) => std::env::set_var("FORCE_COLOR", value),
                None => std::env::remove_var("FORCE_COLOR"),
            }
            match &self.clicolor_force {
                Some(value) => std::env::set_var("CLICOLOR_FORCE", value),
                None => std::env::remove_var("CLICOLOR_FORCE"),
            }
        }
    }
}

/// End-to-end example from the spec:
/// Terminal is dark mode, 120 cols wide.
/// --margin 2 --padding 1 --page-bg subtle --max-width 100 --line-numbers true --align-code-blocks center
#[test]
#[serial_test::serial(layout_snapshot_env)]
fn end_to_end_example_snapshot() {
    let _env = ColorSnapshotEnv::force_color();
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
#[serial_test::serial(layout_snapshot_env)]
fn zero_config_prose_snapshot() {
    let _env = ColorSnapshotEnv::force_color();
    let term = Terminal::new_optimistic(120);
    let page = DarkmatterPage::new(&term).with_color_depth(ColorDepth::TrueColor);
    let md: Markdown = "# Hello World\n\nSome prose here.\n".into();

    let page_out = page.render(&md).unwrap();
    let mut terminal_options = TerminalOptions::default();
    terminal_options.color_depth = Some(ColorDepth::TrueColor);
    let direct_out = darkmatter::markdown::output::terminal::for_terminal(
        &md,
        terminal_options,
    )
    .unwrap();

    assert_eq!(page_out, direct_out);
    insta::assert_snapshot!(page_out);
}

/// Pronounced background on dark terminal produces near-white surface.
#[test]
#[serial_test::serial(layout_snapshot_env)]
fn pronounced_background_snapshot() {
    let _env = ColorSnapshotEnv::force_color();
    let term = Terminal::new_optimistic(80);
    let page = DarkmatterPage::new(&term).with_page_background(PageBackground::Pronounced);
    let md: Markdown = "# Hello\n".into();

    let out = page.render(&md).unwrap();
    insta::assert_snapshot!(out);
}
