use serde::{Deserialize, Serialize};

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
/// use renderable::color::BasicColor;
///
/// let red = BasicColor::Red;
/// let bright_green = BasicColor::BrightGreen;
/// assert_ne!(red, bright_green);
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

/// specify color target as either foreground or background
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FgBg {
    Foreground,
    Background,
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
