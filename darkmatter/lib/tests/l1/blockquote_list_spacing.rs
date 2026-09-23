//! Regression tests for paragraph→list spacing inside blockquotes.
//!
//! When a list immediately follows a paragraph inside a blockquote, the
//! terminal renderer must not emit a blank styled line between them.

use darkmatter::markdown::{
    Markdown,
    output::{ColorDepth, TerminalImageMode, TerminalOptions},
};

fn options() -> TerminalOptions {
    let mut options = TerminalOptions::default();
    options.image_mode = TerminalImageMode::Never;
    options.max_width = Some(80);
    options.color_depth = Some(ColorDepth::TrueColor);
    options
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_esc = false;
    for ch in s.chars() {
        if in_esc {
            if ch.is_ascii_alphabetic() {
                in_esc = false;
            }
            continue;
        }
        if ch == '\x1b' {
            in_esc = true;
            continue;
        }
        out.push(ch);
    }
    out
}

#[test]
fn paragraph_to_paragraph_in_blockquote_keeps_blank_line() {
    let input = "> First paragraph.\n>\n> Second paragraph.\n";
    let md: Markdown = input.into();
    let rendered = md.as_terminal(options()).unwrap();
    let plain = strip_ansi(&rendered);

    let first_idx = plain
        .lines()
        .position(|l| l.contains("First paragraph."))
        .expect("first paragraph present");
    let second_idx = plain
        .lines()
        .position(|l| l.contains("Second paragraph."))
        .expect("second paragraph present");

    assert!(
        second_idx >= first_idx + 2,
        "expected a blank line between paragraphs in blockquote, but got adjacent lines:\n{plain}",
    );
}
