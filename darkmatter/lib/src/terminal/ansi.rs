//! ANSI escape code builder for terminal output.
//!
//! This module provides a composable builder pattern for generating ANSI escape
//! sequences, reducing code duplication and improving consistency across terminal
//! rendering code.
//!
//! ## Examples
//!
//! ```
//! use darkmatter_lib::terminal::ansi::AnsiBuilder;
//! use syntect::highlighting::Color;
//!
//! // Simple foreground color
//! let text = AnsiBuilder::new().fg_rgb(255, 0, 0).wrap("Red text");
//! assert_eq!(text, "\x1b[38;2;255;0;0mRed text\x1b[0m");
//!
//! // Combined styles
//! let styled = AnsiBuilder::new()
//!     .bold()
//!     .fg_rgb(0, 255, 0)
//!     .bg_rgb(40, 40, 40)
//!     .wrap("Bold green on dark");
//!
//! // Using Color struct
//! let fg = Color { r: 200, g: 200, b: 200, a: 255 };
//! let text = AnsiBuilder::new().fg_color(fg).wrap("Gray text");
//! ```

use syntect::highlighting::Color;

/// Builder for composing ANSI escape sequences.
///
/// Provides a fluent API for combining multiple ANSI formatting codes
/// (colors, styles) into a single escape sequence.
///
/// ## Notes
///
/// - Uses 24-bit true color sequences (`38;2;R;G;B` for foreground, `48;2;R;G;B` for background)
/// - Multiple codes are semicolon-separated per ANSI standard
/// - All sequences are terminated with reset (`\x1b[0m`) via `wrap()`
#[derive(Debug, Clone, Default)]
pub struct AnsiBuilder {
    codes: Vec<String>,
}

impl AnsiBuilder {
    /// Creates a new empty ANSI builder.
    #[inline]
    pub fn new() -> Self {
        Self { codes: Vec::new() }
    }

    /// Sets the foreground color using RGB values.
    #[inline]
    pub fn fg_rgb(mut self, r: u8, g: u8, b: u8) -> Self {
        self.codes.push(format!("38;2;{};{};{}", r, g, b));
        self
    }

    /// Sets the foreground color from a syntect Color.
    #[inline]
    pub fn fg_color(self, color: Color) -> Self {
        self.fg_rgb(color.r, color.g, color.b)
    }

    /// Sets the background color using RGB values.
    #[inline]
    pub fn bg_rgb(mut self, r: u8, g: u8, b: u8) -> Self {
        self.codes.push(format!("48;2;{};{};{}", r, g, b));
        self
    }

    /// Sets the background color from a syntect Color.
    #[inline]
    pub fn bg_color(self, color: Color) -> Self {
        self.bg_rgb(color.r, color.g, color.b)
    }

    /// Adds bold formatting (SGR code 1).
    #[inline]
    pub fn bold(mut self) -> Self {
        self.codes.push("1".into());
        self
    }

    /// Adds italic formatting (SGR code 3).
    #[inline]
    pub fn italic(mut self) -> Self {
        self.codes.push("3".into());
        self
    }

    /// Adds underline formatting (SGR code 4).
    #[inline]
    pub fn underline(mut self) -> Self {
        self.codes.push("4".into());
        self
    }

    /// Adds strikethrough formatting (SGR code 9).
    #[inline]
    pub fn strikethrough(mut self) -> Self {
        self.codes.push("9".into());
        self
    }

    /// Clears the line from cursor to end (used with backgrounds).
    ///
    /// Note: This is an escape sequence that should be emitted separately,
    /// not combined with SGR codes. Use `wrap_with_clear()` instead.
    #[inline]
    pub fn clear_to_eol(mut self) -> Self {
        // Store as marker - handled specially in wrap_with_clear
        self.codes.push("CLEAR_EOL".into());
        self
    }

    /// Wraps the given text with ANSI escape codes and reset.
    ///
    /// If no codes have been added, returns the text unchanged.
    ///
    /// ## Format
    ///
    /// ```text
    /// \x1b[CODE1;CODE2;...m<text>\x1b[0m
    /// ```
    pub fn wrap(self, text: &str) -> String {
        if self.codes.is_empty() {
            text.to_string()
        } else {
            // Filter out special markers
            let codes: Vec<&str> = self
                .codes
                .iter()
                .filter(|c| *c != "CLEAR_EOL")
                .map(|s| s.as_str())
                .collect();

            if codes.is_empty() {
                text.to_string()
            } else {
                format!("\x1b[{}m{}\x1b[0m", codes.join(";"), text)
            }
        }
    }

    /// Wraps the text and adds clear-to-end-of-line before reset.
    ///
    /// Useful for backgrounds that should extend to terminal edge.
    ///
    /// ## Format
    ///
    /// ```text
    /// \x1b[CODE1;CODE2;...m<text>\x1b[K\x1b[0m
    /// ```
    pub fn wrap_with_clear(self, text: &str) -> String {
        if self.codes.is_empty() {
            format!("{}\x1b[K", text)
        } else {
            let codes: Vec<&str> = self
                .codes
                .iter()
                .filter(|c| *c != "CLEAR_EOL")
                .map(|s| s.as_str())
                .collect();

            format!("\x1b[{}m{}\x1b[K\x1b[0m", codes.join(";"), text)
        }
    }

    /// Builds just the opening escape sequence without text.
    ///
    /// Useful when you need to set styling that persists across multiple writes.
    pub fn start_sequence(&self) -> String {
        if self.codes.is_empty() {
            String::new()
        } else {
            let codes: Vec<&str> = self
                .codes
                .iter()
                .filter(|c| *c != "CLEAR_EOL")
                .map(|s| s.as_str())
                .collect();

            format!("\x1b[{}m", codes.join(";"))
        }
    }

    /// Returns the reset sequence.
    #[inline]
    pub fn reset() -> &'static str {
        "\x1b[0m"
    }

    /// Returns the clear-to-end-of-line sequence.
    #[inline]
    pub fn clear_eol() -> &'static str {
        "\x1b[K"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_builder_returns_unchanged() {
        let result = AnsiBuilder::new().wrap("hello");
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_foreground_color() {
        let result = AnsiBuilder::new().fg_rgb(255, 128, 0).wrap("orange");
        assert_eq!(result, "\x1b[38;2;255;128;0morange\x1b[0m");
    }

    #[test]
    fn test_background_color() {
        let result = AnsiBuilder::new().bg_rgb(40, 40, 40).wrap("dark bg");
        assert_eq!(result, "\x1b[48;2;40;40;40mdark bg\x1b[0m");
    }

    #[test]
    fn test_combined_fg_bg() {
        let result = AnsiBuilder::new()
            .fg_rgb(255, 255, 255)
            .bg_rgb(0, 0, 0)
            .wrap("white on black");
        assert_eq!(
            result,
            "\x1b[38;2;255;255;255;48;2;0;0;0mwhite on black\x1b[0m"
        );
    }

    #[test]
    fn test_bold() {
        let result = AnsiBuilder::new().bold().wrap("bold text");
        assert_eq!(result, "\x1b[1mbold text\x1b[0m");
    }

    #[test]
    fn test_italic() {
        let result = AnsiBuilder::new().italic().wrap("italic text");
        assert_eq!(result, "\x1b[3mitalic text\x1b[0m");
    }

    #[test]
    fn test_underline() {
        let result = AnsiBuilder::new().underline().wrap("underlined");
        assert_eq!(result, "\x1b[4munderlined\x1b[0m");
    }

    #[test]
    fn test_strikethrough() {
        let result = AnsiBuilder::new().strikethrough().wrap("struck");
        assert_eq!(result, "\x1b[9mstruck\x1b[0m");
    }

    #[test]
    fn test_complex_combination() {
        let result = AnsiBuilder::new()
            .bold()
            .italic()
            .fg_rgb(200, 100, 50)
            .bg_rgb(30, 30, 30)
            .wrap("styled");
        assert_eq!(
            result,
            "\x1b[1;3;38;2;200;100;50;48;2;30;30;30mstyled\x1b[0m"
        );
    }

    #[test]
    fn test_color_struct() {
        let color = Color {
            r: 128,
            g: 64,
            b: 32,
            a: 255,
        };
        let result = AnsiBuilder::new().fg_color(color).wrap("colored");
        assert_eq!(result, "\x1b[38;2;128;64;32mcolored\x1b[0m");
    }

    #[test]
    fn test_wrap_with_clear() {
        let result = AnsiBuilder::new().bg_rgb(50, 50, 50).wrap_with_clear("");
        assert_eq!(result, "\x1b[48;2;50;50;50m\x1b[K\x1b[0m");
    }

    #[test]
    fn test_start_sequence() {
        let result = AnsiBuilder::new().bold().fg_rgb(255, 0, 0).start_sequence();
        assert_eq!(result, "\x1b[1;38;2;255;0;0m");
    }

    #[test]
    fn test_reset_constant() {
        assert_eq!(AnsiBuilder::reset(), "\x1b[0m");
    }

    #[test]
    fn test_clear_eol_constant() {
        assert_eq!(AnsiBuilder::clear_eol(), "\x1b[K");
    }
}
