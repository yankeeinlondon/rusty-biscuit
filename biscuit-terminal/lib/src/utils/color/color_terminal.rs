//! Terminal-specific ANSI emission for color data types.
//!
//! The color *data* lives in [`renderable::color`]; this module restores the
//! terminal rendering behavior (the `TermColor` impls and the `color_code`
//! helper) for those types.

use std::{borrow::Cow, collections::HashMap, sync::LazyLock};

use renderable::color::{BasicColor, FgBg, HdrColor, RgbColor, WEB_COLOR_LOOKUP, WebColor};

use crate::utils::term_color::TermColor;

const ESC: &str = "\x1b[";
/// resets foreground color to the default
const DEFAULT_FOREGROUND: &str = "\x1b[39m";
/// resets background color to the default
const DEFAULT_BACKGROUND: &str = "\x1b[49m";

static BASIC_COLOR_LOOKUP: LazyLock<HashMap<BasicColor, (&'static str, &'static str)>> =
    LazyLock::new(|| {
        let mut m = HashMap::with_capacity(25);

        m.insert(BasicColor::Black, ("30", "40"));
        m.insert(BasicColor::Red, ("31", "41"));
        m.insert(BasicColor::Green, ("32", "42"));
        m.insert(BasicColor::Yellow, ("33", "43"));
        m.insert(BasicColor::Blue, ("34", "44"));
        m.insert(BasicColor::Magenta, ("35", "45"));
        m.insert(BasicColor::Cyan, ("36", "46"));
        m.insert(BasicColor::White, ("37", "47"));

        m.insert(BasicColor::BrightBlack, ("90", "100"));
        m.insert(BasicColor::BrightRed, ("91", "101"));
        m.insert(BasicColor::BrightGreen, ("92", "102"));
        m.insert(BasicColor::BrightYellow, ("93", "103"));
        m.insert(BasicColor::BrightBlue, ("94", "104"));
        m.insert(BasicColor::BrightMagenta, ("95", "105"));
        m.insert(BasicColor::BrightCyan, ("96", "106"));
        m.insert(BasicColor::BrightWhite, ("97", "107"));

        m
    });

/// returns the escape-code to START the color coding
fn basic_start(color: BasicColor, pos: FgBg) -> String {
    let codes = BASIC_COLOR_LOOKUP.get(&color).unwrap();
    match pos {
        FgBg::Foreground => format!("{}{}m", ESC, codes.0),
        FgBg::Background => format!("{}{}m", ESC, codes.1),
    }
}

/// returns the escape-code to END the color coding
fn basic_end(pos: FgBg) -> String {
    match pos {
        FgBg::Foreground => DEFAULT_FOREGROUND.to_string(),
        FgBg::Background => DEFAULT_BACKGROUND.to_string(),
    }
}

/// Helper function to convert BasicColor to ANSI color code
pub(crate) fn color_code(color: BasicColor) -> u8 {
    match color {
        BasicColor::Black => 30,
        BasicColor::Red => 31,
        BasicColor::Green => 32,
        BasicColor::Yellow => 33,
        BasicColor::Blue => 34,
        BasicColor::Magenta => 35,
        BasicColor::Cyan => 36,
        BasicColor::White => 37,
        BasicColor::BrightBlack => 90,
        BasicColor::BrightRed => 91,
        BasicColor::BrightGreen => 92,
        BasicColor::BrightYellow => 93,
        BasicColor::BrightBlue => 94,
        BasicColor::BrightMagenta => 95,
        BasicColor::BrightCyan => 96,
        BasicColor::BrightWhite => 97,
    }
}

impl<'a> TermColor<'a> for BasicColor {
    /// wraps the content passed in with the escape-codes required
    /// to start and stop the foreground color rendering.
    fn fg(self, content: impl Into<Cow<'a, str>>) -> String {
        let content = content.into();
        format!(
            "{}{}{}",
            basic_start(self, FgBg::Foreground),
            content,
            basic_end(FgBg::Foreground)
        )
    }

    /// wraps the content passed in with the escape-codes required
    /// to start and stop the background color rendering.
    fn bg(self, content: impl Into<Cow<'a, str>>) -> String {
        let content = content.into();
        format!(
            "{}{}{}",
            basic_start(self, FgBg::Background),
            content,
            basic_end(FgBg::Background)
        )
    }
}

impl<'a> TermColor<'a> for RgbColor {
    /// wraps the content passed in with the escape-codes required
    /// to start and stop the foreground color rendering.
    fn fg(self, content: impl Into<Cow<'a, str>>) -> String {
        let content = content.into();
        format!(
            "\x1b[38;2;{};{};{}m{}\x1b[39m",
            self.red(),
            self.green(),
            self.blue(),
            content
        )
    }

    /// wraps the content passed in with the escape-codes required
    /// to start and stop the background color rendering.
    fn bg(self, content: impl Into<Cow<'a, str>>) -> String {
        let content = content.into();
        format!(
            "\x1b[48;2;{};{};{}m{}\x1b[49m",
            self.red(),
            self.green(),
            self.blue(),
            content
        )
    }
}

impl<'a> TermColor<'a> for HdrColor {
    fn fg(self, content: impl Into<Cow<'a, str>>) -> String {
        let content = content.into();
        format!(
            "\x1b[38;2;{};{};{}m{}\x1b[39m",
            self.red(),
            self.green(),
            self.blue(),
            content
        )
    }

    fn bg(self, content: impl Into<Cow<'a, str>>) -> String {
        let content = content.into();
        format!(
            "\x1b[48;2;{};{};{}m{}\x1b[49m",
            self.red(),
            self.green(),
            self.blue(),
            content
        )
    }
}

impl<'a> TermColor<'a> for WebColor {
    /// Wraps the content passed in with the escape-codes required
    /// to start and stop the foreground color rendering.
    fn fg(self, content: impl Into<Cow<'a, str>>) -> String {
        let rgb = WEB_COLOR_LOOKUP
            .get(&self)
            .expect("WebColor should have an RGB mapping in WEB_COLOR_LOOKUP");
        rgb.fg(content)
    }

    /// Wraps the content passed in with the escape-codes required
    /// to start and stop the background color rendering.
    fn bg(self, content: impl Into<Cow<'a, str>>) -> String {
        let rgb = WEB_COLOR_LOOKUP
            .get(&self)
            .expect("WebColor should have an RGB mapping in WEB_COLOR_LOOKUP");
        rgb.bg(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_color_fg_emits_ansi() {
        let red_text = BasicColor::Red.fg("This is red");
        assert!(red_text.contains("\x1b[31m"));
        assert!(red_text.contains("\x1b[39m"));
    }

    #[test]
    fn basic_color_bg_emits_ansi() {
        let blue_bg = BasicColor::Blue.bg("Blue background");
        assert!(blue_bg.contains("\x1b[44m"));
    }

    #[test]
    fn basic_color_bright_fg_emits_ansi() {
        let bright_green = BasicColor::BrightGreen.fg("High visibility");
        assert!(bright_green.contains("\x1b[92m"));
    }

    #[test]
    fn web_color_fg_emits_truecolor() {
        let colored = WebColor::Coral.fg("Warm coral text");
        assert!(colored.contains("\x1b[38;2;"));
    }
}
