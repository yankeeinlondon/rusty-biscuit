use serde::{Deserialize, Serialize};

use super::{BasicColor, RgbColor, Tailwind, WEB_COLOR_LOOKUP, WebColor, basic_color_to_rgb};

/// A unified color type supporting multiple color representations.
///
/// This enum provides a single interface for specifying colors across
/// different color spaces and sources, enabling flexible color configuration
/// for terminal rendering.
///
/// ## Variants
///
/// - `BasicColor(BasicColor)` - Standard 16-color ANSI palette
/// - `Rgb(RgbColor)` - True 24-bit RGB color with ANSI fallback
/// - `Web(WebColor)` - 148 CSS named colors (e.g., "tomato", "steelblue")
/// - `Tailwind(Tailwind)` - Tailwind CSS color palette
/// - `DefaultForeground` - Terminal's default text color
/// - `DefaultBackground` - Terminal's default background color
/// - `Reset` - Reset all color attributes
///
/// ## Examples
///
/// ```
/// use renderable::color::{Color, BasicColor, RgbColor, WebColor, Tailwind};
///
/// // Basic ANSI colors
/// let red = Color::BasicColor(BasicColor::Red);
///
/// // RGB with fallback for limited terminals
/// let orange = Color::Rgb(RgbColor::new(255, 165, 0, BasicColor::Yellow));
///
/// // CSS named colors
/// let tomato = Color::Web(WebColor::Tomato);
///
/// // Tailwind palette colors
/// let blue_500 = Color::Tailwind(Tailwind::Blue500);
///
/// // Use terminal defaults
/// let default_fg = Color::DefaultForeground;
/// let default_bg = Color::DefaultBackground;
///
/// // Pattern matching on Color enum
/// match blue_500 {
///     Color::Tailwind(tw) => println!("Tailwind color: {:?}", tw),
///     Color::Web(web) => println!("Web color: {:?}", web),
///     _ => println!("Other color type"),
/// }
/// ```
///
/// ## Converting to RGB
///
/// ```
/// use renderable::color::Color;
///
/// let color = Color::Web(renderable::color::WebColor::Crimson);
/// if let Some((r, g, b)) = color.to_rgb() {
///     println!("RGB: ({}, {}, {})", r, g, b);
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Color {
    BasicColor(BasicColor),
    /// Specify a bespoke RGB color value (with a `BasicColor` as a fallback)
    Rgb(RgbColor),
    /// Use a named CSS/Web color
    Web(WebColor),
    Tailwind(Tailwind),
    /// Use the terminal's default foreground color
    DefaultForeground,
    /// Use the terminal's default background color
    DefaultBackground,
    /// Reset all colors
    Reset,
}

impl Color {
    /// Returns RGB values (r, g, b) for this color, if available.
    ///
    /// Returns `None` for `DefaultForeground`, `DefaultBackground`, and `Reset`
    /// since these don't have fixed RGB values.
    pub fn to_rgb(&self) -> Option<(u8, u8, u8)> {
        match self {
            Color::BasicColor(c) => Some(basic_color_to_rgb(*c)),
            Color::Rgb(rgb) => Some((rgb.red(), rgb.green(), rgb.blue())),
            Color::Web(web) => WEB_COLOR_LOOKUP
                .get(web)
                .map(|rgb| (rgb.red(), rgb.green(), rgb.blue())),
            Color::Tailwind(tw) => tw
                .to_hdr_color()
                .map(|hdr| (hdr.red(), hdr.green(), hdr.blue())),
            Color::DefaultForeground | Color::DefaultBackground | Color::Reset => None,
        }
    }
}
