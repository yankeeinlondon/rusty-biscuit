//! Example: Analyze text for escape codes
//!
//! Run with: cargo run -p biscuit-terminal --example escape_analysis

use biscuit_terminal::discovery::eval::{has_escape_codes, has_osc8_link, line_widths};

fn main() {
    let examples = vec![
        ("Plain text", "Hello, world!"),
        ("Colored text", "\x1b[31mRed\x1b[0m and \x1b[32mGreen\x1b[0m"),
        ("Bold text", "\x1b[1mBold\x1b[0m"),
        ("OSC8 link", "\x1b]8;;https://rust-lang.org\x07Rust\x1b]8;;\x07"),
        ("Unicode CJK", "Hello \u{4f60}\u{597d}"),
        ("Emoji", "Hello \u{1F389}\u{1F38A}"),
    ];

    println!("=== Escape Code Analysis ===\n");

    for (name, text) in examples {
        println!("{}:", name);
        println!("  Text:         {:?}", text);
        println!("  Has escapes:  {}", has_escape_codes(text));
        println!("  Has OSC8:     {}", has_osc8_link(text));
        println!("  Line widths:  {:?}", line_widths(text));
        println!();
    }

    // Multi-line example
    println!("Multi-line example:");
    let multiline = "Line one\n\x1b[32mGreen line\x1b[0m\nLine three";
    println!("  Text: {:?}", multiline);
    println!("  Line widths: {:?}", line_widths(multiline));
}
