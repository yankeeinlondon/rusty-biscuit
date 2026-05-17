//! Regression tests for paragraph→list spacing inside blockquotes.
//!
//! When a list immediately follows a paragraph inside a blockquote, the
//! terminal renderer must not emit a blank styled line between them.

use darkmatter::markdown::{
    Markdown,
    output::{ColorDepth, TerminalImageMode, TerminalOptions, for_terminal},
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
fn paragraph_to_list_in_blockquote_is_tight() {
    let input = "> **Note:** intro text here:\n>\n> 1. First item.\n> 2. Second item.\n";
    let md: Markdown = input.into();
    let rendered = for_terminal(&md, options()).unwrap();
    let plain = strip_ansi(&rendered);

    let intro_line = plain
        .lines()
        .find(|l| l.contains("intro text here:"))
        .expect("intro line should be present");
    let intro_idx = plain.lines().position(|l| l == intro_line).unwrap();
    let next_idx = plain
        .lines()
        .skip(intro_idx + 1)
        .position(|l| l.contains("First item."))
        .map(|p| intro_idx + 1 + p)
        .expect("first item should be present");

    assert_eq!(
        next_idx,
        intro_idx + 1,
        "expected paragraph and list to be tight (no blank line between), but got:\n{plain}",
    );
}

#[test]
fn paragraph_to_paragraph_in_blockquote_keeps_blank_line() {
    let input = "> First paragraph.\n>\n> Second paragraph.\n";
    let md: Markdown = input.into();
    let rendered = for_terminal(&md, options()).unwrap();
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
