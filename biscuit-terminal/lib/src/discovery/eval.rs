//! Escape-code-aware text evaluation utilities.
//!
//! This module provides functions for analyzing terminal text that may contain
//! ANSI escape sequences, including:
//! - Calculating visual line widths after stripping escape codes
//! - Detecting presence of escape sequences
//! - Identifying OSC8 hyperlinks

use std::sync::LazyLock;

use regex::Regex;
use unicode_width::UnicodeWidthStr;

/// Regex pattern for ANSI escape sequences.
///
/// Matches:
/// - CSI sequences: `\x1b[` followed by parameter bytes, intermediate bytes, and final byte
/// - OSC sequences: `\x1b]` followed by content until BEL (\x07) or ST (\x1b\\)
/// - Other escape sequences: `\x1b` followed by single character
static ANSI_ESCAPE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"\x1b\[[\x30-\x3f]*[\x20-\x2f]*[\x40-\x7e]", // CSI sequences
        r"|\x1b\].*?(?:\x07|\x1b\\)",                  // OSC sequences (BEL or ST terminator)
        r"|\x1b[\x20-\x2f]*[\x40-\x5f]",               // Other escape sequences (Fe)
    ))
    .expect("Invalid ANSI escape regex")
});

/// Regex pattern for OSC8 hyperlinks.
///
/// OSC8 format: `\x1b]8;params;URL` followed by BEL (\x07) or ST (\x1b\\)
/// - params can be empty or contain key=value pairs separated by colons
/// - URL is the hyperlink target
static OSC8_LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\x1b\]8;[^;]*;[^\x07\x1b]*(?:\x07|\x1b\\)")
        .expect("Invalid OSC8 link regex")
});

/// Strip all ANSI escape sequences from text.
///
/// Removes:
/// - CSI sequences (Control Sequence Introducer): `\x1b[...`
/// - OSC sequences (Operating System Command): `\x1b]...`
/// - Other escape sequences
fn strip_ansi_codes(text: &str) -> String {
    ANSI_ESCAPE_RE.replace_all(text, "").into_owned()
}

/// Returns the visual width of each line after stripping escape codes.
///
/// Uses Unicode width calculation to properly handle:
/// - Wide characters (CJK, emoji)
/// - Zero-width characters
/// - ANSI escape sequences (stripped before measurement)
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::eval::line_widths;
///
/// // Plain text
/// assert_eq!(line_widths("hello"), vec![5]);
///
/// // With escape codes (stripped)
/// assert_eq!(line_widths("\x1b[31mred\x1b[0m"), vec![3]);
///
/// // Multiple lines
/// assert_eq!(line_widths("foo\nbar"), vec![3, 3]);
/// ```
pub fn line_widths<T: Into<String>>(content: T) -> Vec<u16> {
    let content = content.into();
    let stripped = strip_ansi_codes(&content);

    // Handle empty string case - return vec with single 0 width
    if stripped.is_empty() {
        return vec![0];
    }

    stripped
        .lines()
        .map(|line| UnicodeWidthStr::width(line) as u16)
        .collect()
}

/// Detects if the content contains any ANSI escape sequences.
///
/// Detects:
/// - CSI sequences (Control Sequence Introducer): `\x1b[...`
/// - OSC sequences (Operating System Command): `\x1b]...`
/// - SGR (Select Graphic Rendition): color/style codes
/// - Other escape sequences: `\x1b` followed by various characters
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::eval::has_escape_codes;
///
/// assert!(!has_escape_codes("plain text"));
/// assert!(has_escape_codes("\x1b[31mred\x1b[0m"));
/// assert!(has_escape_codes("\x1b]8;;http://example.com\x07link\x1b]8;;\x07"));
/// ```
pub fn has_escape_codes<T: Into<String>>(content: T) -> bool {
    let content = content.into();
    ANSI_ESCAPE_RE.is_match(&content)
}

/// Detects if the content contains OSC8 hyperlinks.
///
/// OSC8 format: `\x1b]8;;URL\x07` or `\x1b]8;;URL\x1b\\`
///
/// The OSC8 sequence consists of:
/// - `\x1b]8;` - OSC introducer with code 8
/// - Optional parameters (key=value pairs separated by colons)
/// - `;` - Parameter/URL separator
/// - URL - The hyperlink target
/// - BEL (`\x07`) or ST (`\x1b\\`) - Terminator
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::eval::has_osc8_link;
///
/// assert!(!has_osc8_link("plain text"));
/// assert!(!has_osc8_link("\x1b[31mred\x1b[0m")); // color code, not link
/// assert!(has_osc8_link("\x1b]8;;http://example.com\x07click\x1b]8;;\x07"));
/// ```
pub fn has_osc8_link<T: Into<String>>(content: T) -> bool {
    let content = content.into();
    OSC8_LINK_RE.is_match(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    // === line_widths tests ===

    #[test]
    fn test_line_widths_plain_text() {
        assert_eq!(line_widths("hello"), vec![5]);
        assert_eq!(line_widths(""), vec![0]);
    }

    #[test]
    fn test_line_widths_single_char() {
        assert_eq!(line_widths("a"), vec![1]);
        assert_eq!(line_widths(" "), vec![1]);
    }

    #[test]
    fn test_line_widths_multiple_lines() {
        assert_eq!(line_widths("foo\nbar\nbaz"), vec![3, 3, 3]);
        assert_eq!(line_widths("short\nlonger line"), vec![5, 11]);
    }

    #[test]
    fn test_line_widths_empty_lines() {
        assert_eq!(line_widths("foo\n\nbar"), vec![3, 0, 3]);
        assert_eq!(line_widths("\n\n"), vec![0, 0]);
    }

    #[test]
    fn test_line_widths_with_color_codes() {
        // Red text "red" should have width 3
        assert_eq!(line_widths("\x1b[31mred\x1b[0m"), vec![3]);
        // Bold + color
        assert_eq!(line_widths("\x1b[1;33mwarning\x1b[0m"), vec![7]);
    }

    #[test]
    fn test_line_widths_with_multiple_sgr_codes() {
        // Multiple SGR attributes
        let styled = "\x1b[1m\x1b[4m\x1b[31mtext\x1b[0m";
        assert_eq!(line_widths(styled), vec![4]);
    }

    #[test]
    fn test_line_widths_with_osc_links() {
        // OSC8 link with "click" text
        let link = "\x1b]8;;https://example.com\x07click\x1b]8;;\x07";
        assert_eq!(line_widths(link), vec![5]);
    }

    #[test]
    fn test_line_widths_with_osc_links_st_terminator() {
        // OSC8 link with ST terminator
        let link = "\x1b]8;;https://example.com\x1b\\click\x1b]8;;\x1b\\";
        assert_eq!(line_widths(link), vec![5]);
    }

    #[test]
    fn test_line_widths_unicode_cjk() {
        // CJK characters (width 2 each)
        assert_eq!(line_widths("你好"), vec![4]);
        // Mix of ASCII and CJK
        assert_eq!(line_widths("hi你好"), vec![6]);
    }

    #[test]
    fn test_line_widths_unicode_emoji() {
        // Single emoji (typically width 2)
        assert_eq!(line_widths("\u{1F389}"), vec![2]); // party popper
    }

    #[test]
    fn test_line_widths_mixed_unicode_and_escape() {
        // CJK with color codes
        let styled = "\x1b[32m你好\x1b[0m";
        assert_eq!(line_widths(styled), vec![4]);
    }

    #[test]
    fn test_line_widths_multiline_with_escapes() {
        let content = "\x1b[31mred\x1b[0m\n\x1b[32mgreen\x1b[0m\n\x1b[34mblue\x1b[0m";
        assert_eq!(line_widths(content), vec![3, 5, 4]);
    }

    // === has_escape_codes tests ===

    #[test]
    fn test_has_escape_codes_plain_text() {
        assert!(!has_escape_codes("plain text"));
        assert!(!has_escape_codes(""));
        assert!(!has_escape_codes("hello\nworld"));
    }

    #[test]
    fn test_has_escape_codes_sgr_colors() {
        assert!(has_escape_codes("\x1b[31m")); // red foreground
        assert!(has_escape_codes("\x1b[0m")); // reset
        assert!(has_escape_codes("\x1b[1;4;31m")); // bold, underline, red
        assert!(has_escape_codes("\x1b[38;5;196m")); // 256-color mode
        assert!(has_escape_codes("\x1b[38;2;255;0;0m")); // 24-bit color
    }

    #[test]
    fn test_has_escape_codes_sgr_styles() {
        assert!(has_escape_codes("\x1b[1m")); // bold
        assert!(has_escape_codes("\x1b[2m")); // dim
        assert!(has_escape_codes("\x1b[3m")); // italic
        assert!(has_escape_codes("\x1b[4m")); // underline
        assert!(has_escape_codes("\x1b[7m")); // reverse
        assert!(has_escape_codes("\x1b[9m")); // strikethrough
    }

    #[test]
    fn test_has_escape_codes_cursor() {
        assert!(has_escape_codes("\x1b[H")); // cursor home
        assert!(has_escape_codes("\x1b[2J")); // clear screen
        assert!(has_escape_codes("\x1b[5A")); // cursor up 5
        assert!(has_escape_codes("\x1b[10B")); // cursor down 10
        assert!(has_escape_codes("\x1b[3C")); // cursor forward 3
        assert!(has_escape_codes("\x1b[2D")); // cursor back 2
    }

    #[test]
    fn test_has_escape_codes_osc() {
        assert!(has_escape_codes("\x1b]0;title\x07")); // window title
        assert!(has_escape_codes("\x1b]8;;url\x07")); // hyperlink
        assert!(has_escape_codes("\x1b]52;c;base64\x07")); // clipboard
    }

    #[test]
    fn test_has_escape_codes_embedded_in_text() {
        assert!(has_escape_codes("Hello \x1b[31mworld\x1b[0m!"));
        assert!(has_escape_codes("prefix\x1b[32mtext\x1b[0msuffix"));
    }

    // === has_osc8_link tests ===

    #[test]
    fn test_has_osc8_link_plain_text() {
        assert!(!has_osc8_link("plain text"));
        assert!(!has_osc8_link(""));
    }

    #[test]
    fn test_has_osc8_link_not_osc8() {
        // Other OSC sequences are not OSC8 links
        assert!(!has_osc8_link("\x1b]0;title\x07")); // window title (OSC 0)
        assert!(!has_osc8_link("\x1b]52;c;base64\x07")); // clipboard (OSC 52)
        // Color codes are not links
        assert!(!has_osc8_link("\x1b[31mred\x1b[0m"));
    }

    #[test]
    fn test_has_osc8_link_valid_links_bel() {
        // BEL terminator
        assert!(has_osc8_link(
            "\x1b]8;;https://example.com\x07link\x1b]8;;\x07"
        ));
        // Just the opening link (no closing)
        assert!(has_osc8_link("\x1b]8;;https://example.com\x07"));
        // Just the closing link
        assert!(has_osc8_link("\x1b]8;;\x07"));
    }

    #[test]
    fn test_has_osc8_link_valid_links_st() {
        // ST terminator
        assert!(has_osc8_link(
            "\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\"
        ));
    }

    #[test]
    fn test_has_osc8_link_with_params() {
        // With id parameter
        assert!(has_osc8_link("\x1b]8;id=foo;https://example.com\x07"));
        // With multiple params
        assert!(has_osc8_link(
            "\x1b]8;id=foo:class=bar;https://example.com\x07"
        ));
    }

    #[test]
    fn test_has_osc8_link_embedded() {
        // Link embedded in colored text
        let text = "\x1b[32m\x1b]8;;http://example.com\x07click\x1b]8;;\x07\x1b[0m";
        assert!(has_osc8_link(text));
    }

    #[test]
    fn test_has_osc8_link_multiline() {
        // Links on multiple lines
        let text = "Line 1\n\x1b]8;;http://a.com\x07link\x1b]8;;\x07\nLine 3";
        assert!(has_osc8_link(text));
    }

    // === strip_ansi_codes tests ===

    #[test]
    fn test_strip_ansi_codes_plain() {
        assert_eq!(strip_ansi_codes("hello"), "hello");
        assert_eq!(strip_ansi_codes(""), "");
    }

    #[test]
    fn test_strip_ansi_codes_sgr() {
        assert_eq!(strip_ansi_codes("\x1b[31mred\x1b[0m"), "red");
        assert_eq!(
            strip_ansi_codes("\x1b[1;4;33mwarning\x1b[0m"),
            "warning"
        );
    }

    #[test]
    fn test_strip_ansi_codes_osc() {
        assert_eq!(strip_ansi_codes("\x1b]0;title\x07"), "");
        assert_eq!(
            strip_ansi_codes("\x1b]8;;https://example.com\x07click\x1b]8;;\x07"),
            "click"
        );
    }

    #[test]
    fn test_strip_ansi_codes_cursor() {
        assert_eq!(strip_ansi_codes("\x1b[Hstart"), "start");
        assert_eq!(strip_ansi_codes("pre\x1b[2Jpost"), "prepost");
    }

    #[test]
    fn test_strip_ansi_codes_preserves_unicode() {
        assert_eq!(strip_ansi_codes("\x1b[32m你好\x1b[0m"), "你好");
        assert_eq!(strip_ansi_codes("\x1b[31m\u{1F389}\x1b[0m"), "\u{1F389}");
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property: line_widths should never panic on any input
        #[test]
        fn line_widths_never_panics(s in ".*") {
            let _ = line_widths(&s);
        }

        /// Property: has_escape_codes should never panic on any input
        #[test]
        fn has_escape_codes_never_panics(s in ".*") {
            let _ = has_escape_codes(&s);
        }

        /// Property: has_osc8_link should never panic on any input
        #[test]
        fn has_osc8_link_never_panics(s in ".*") {
            let _ = has_osc8_link(&s);
        }

        /// Property: line_widths should always return non-empty vec
        #[test]
        fn line_widths_returns_non_empty(s in ".*") {
            let widths = line_widths(&s);
            prop_assert!(!widths.is_empty(), "line_widths should never return empty vec");
        }

        /// Property: plain ASCII text width should equal character count
        #[test]
        fn ascii_text_width_equals_length(s in "[a-zA-Z0-9 ]{0,100}") {
            // Filter out newlines since they create multiple lines
            if !s.contains('\n') {
                let widths = line_widths(&s);
                prop_assert_eq!(widths.len(), 1);
                prop_assert_eq!(widths[0] as usize, s.len());
            }
        }

        /// Property: if string contains ESC byte, has_escape_codes should detect it
        /// (when it's part of a valid escape sequence pattern)
        #[test]
        fn escape_codes_detection_with_csi(
            prefix in "[a-zA-Z0-9 ]{0,10}",
            param in "[0-9;]{0,5}",
            suffix in "[a-zA-Z0-9 ]{0,10}"
        ) {
            // Build a valid CSI sequence: ESC [ params final_byte
            let s = format!("{}\x1b[{}m{}", prefix, param, suffix);
            prop_assert!(has_escape_codes(&s), "CSI sequence should be detected: {:?}", s);
        }

        /// Property: newlines increase line count in output
        #[test]
        fn newlines_increase_line_count(lines in 1usize..20, content in "[a-z]{1,5}") {
            let s = std::iter::repeat(content.as_str())
                .take(lines)
                .collect::<Vec<_>>()
                .join("\n");
            let widths = line_widths(&s);
            prop_assert_eq!(widths.len(), lines);
        }

        /// Property: SGR codes should not affect visual width
        #[test]
        fn sgr_codes_dont_affect_width(text in "[a-zA-Z]{1,10}", code in 0u8..108) {
            let plain_width = line_widths(&text)[0];
            let styled = format!("\x1b[{}m{}\x1b[0m", code, text);
            let styled_width = line_widths(&styled)[0];
            prop_assert_eq!(plain_width, styled_width,
                "SGR code {} should not affect width", code);
        }
    }
}
