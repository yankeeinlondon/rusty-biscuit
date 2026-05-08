use std::{borrow::Cow, collections::HashMap, sync::LazyLock};

use serde::{Deserialize, Serialize};

use super::TermColor;

/// The 8 basic ANSI colors (0-7) plus their bright variants (8-15).
///
/// This enum represents the standard 16-color terminal palette:
/// - Standard colors: Black, Red, Green, Yellow, Blue, Magenta, Cyan, White
/// - Bright variants: BrightBlack, BrightRed, BrightGreen, BrightYellow,
///   BrightBlue, BrightMagenta, BrightCyan, BrightWhite
///
/// These colors are supported by virtually all terminal emulators and provide
/// consistent rendering across different platforms.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::utils::color::{BasicColor, TermColor};
///
/// // Color text using the TermColor trait
/// let red_text = BasicColor::Red.fg("This is red");
/// assert!(red_text.contains("\x1b[31m"));
/// assert!(red_text.contains("\x1b[39m")); // reset code
///
/// // Color background
/// let blue_bg = BasicColor::Blue.bg("Blue background");
/// assert!(blue_bg.contains("\x1b[44m")); // blue background code
///
/// // Bright colors for higher contrast
/// let bright_green = BasicColor::BrightGreen.fg("High visibility");
/// assert!(bright_green.contains("\x1b[92m")); // bright green foreground
///
/// // Combine multiple colors in a string
/// let combined = format!(
///     "{} and {}",
///     BasicColor::Red.fg("error"),
///     BasicColor::Green.fg("success")
/// );
/// assert!(combined.contains("\x1b[31m"));
/// assert!(combined.contains("\x1b[32m"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BasicColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

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

/// specify color target as either foreground or background
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FgBg {
    Foreground,
    Background,
}

impl BasicColor {
    /// returns the escape-code to START the color coding
    fn start(self, pos: FgBg) -> String {
        let codes = BASIC_COLOR_LOOKUP.get(&self).unwrap();
        match pos {
            FgBg::Foreground => format!("{}{}m", ESC, codes.0),
            FgBg::Background => format!("{}{}m", ESC, codes.1),
        }
    }
    /// returns the escape-code to END the color coding
    fn end(self, pos: FgBg) -> String {
        match pos {
            FgBg::Foreground => DEFAULT_FOREGROUND.to_string(),
            FgBg::Background => DEFAULT_BACKGROUND.to_string(),
        }
    }
}

impl<'a> TermColor<'a> for BasicColor {
    /// wraps the content passed in with the escape-codes required
    /// to start and stop the foreground color rendering.
    fn fg(self, content: impl Into<Cow<'a, str>>) -> String {
        let content = content.into();
        format!(
            "{}{}{}",
            self.start(FgBg::Foreground),
            content,
            self.end(FgBg::Foreground)
        )
    }

    /// wraps the content passed in with the escape-codes required
    /// to start and stop the background color rendering.
    fn bg(self, content: impl Into<Cow<'a, str>>) -> String {
        let content = content.into();
        format!(
            "{}{}{}",
            self.start(FgBg::Background),
            content,
            self.end(FgBg::Background)
        )
    }
}

/// Helper function to convert BasicColor to ANSI color code
pub(super) fn color_code(color: BasicColor) -> u8 {
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

/// Convert BasicColor to approximate RGB values.
///
/// These are the standard VGA/ANSI color values commonly used by terminals.
pub fn basic_color_to_rgb(color: BasicColor) -> (u8, u8, u8) {
    match color {
        BasicColor::Black => (0, 0, 0),
        BasicColor::Red => (128, 0, 0),
        BasicColor::Green => (0, 128, 0),
        BasicColor::Yellow => (128, 128, 0),
        BasicColor::Blue => (0, 0, 128),
        BasicColor::Magenta => (128, 0, 128),
        BasicColor::Cyan => (0, 128, 128),
        BasicColor::White => (192, 192, 192),
        BasicColor::BrightBlack => (128, 128, 128),
        BasicColor::BrightRed => (255, 0, 0),
        BasicColor::BrightGreen => (0, 255, 0),
        BasicColor::BrightYellow => (255, 255, 0),
        BasicColor::BrightBlue => (0, 0, 255),
        BasicColor::BrightMagenta => (255, 0, 255),
        BasicColor::BrightCyan => (0, 255, 255),
        BasicColor::BrightWhite => (255, 255, 255),
    }
}
