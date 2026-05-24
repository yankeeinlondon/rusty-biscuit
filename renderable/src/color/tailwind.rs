use serde::{Deserialize, Serialize};

use super::{BasicColor, HdrColor};

/// Tailwind CSS color palette.
///
/// This enum provides access to the complete Tailwind CSS color scale,
/// from `50` (lightest) through `950` (darkest) for each color family.
///
/// ## Color Families
///
/// Each color family (Red, Orange, Amber, Yellow, Lime, Green, Emerald,
/// Teal, Cyan, Sky, Blue, Indigo, Violet, Purple, Fuchsia, Pink, Rose,
/// Slate, Gray, Zinc, Neutral) has shades from 50 to 950, plus special
/// values: `Inherit`, `Current`, `Transparent`, `Black`, `White`.
///
/// ## Examples
///
/// ```
/// use renderable::color::Tailwind;
///
/// // Access specific colors from the palette
/// let primary = Tailwind::Blue500;
/// let light_bg = Tailwind::Slate50;
/// let dark_text = Tailwind::Gray900;
///
/// // Common special values
/// let transparent = Tailwind::Transparent;
/// let black = Tailwind::Black;
/// let white = Tailwind::White;
///
/// // Color intensity scales (50 = lightest, 950 = darkest)
/// let red_light = Tailwind::Red200;
/// let red_main = Tailwind::Red500;
/// let red_dark = Tailwind::Red800;
/// ```
///
/// ## Usage with RgbColor
///
/// Convert to RGB for terminal rendering:
///
/// ```
/// use renderable::color::{Color, Tailwind};
///
/// let color = Color::Tailwind(Tailwind::Emerald600);
/// if let Some((r, g, b)) = color.to_rgb() {
///     println!("R: {}, G: {}, B: {}", r, g, b);
/// }
/// ```
///
/// ## Notes
///
/// The palette follows the official Tailwind CSS color values.
/// Shades are designed for both light and dark backgrounds:
/// - 50-200: Light backgrounds, subtle highlights
/// - 300-500: Primary interactive elements
/// - 600-700: Active states, emphasis
/// - 800-950: Dark backgrounds, heavy text
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tailwind {
    // “specials” commonly present in Tailwind palettes
    Inherit,
    Current,
    Transparent,
    Black,
    White,

    // Red
    Red50,
    Red100,
    Red200,
    Red300,
    Red400,
    Red500,
    Red600,
    Red700,
    Red800,
    Red900,
    Red950,
    // Orange
    Orange50,
    Orange100,
    Orange200,
    Orange300,
    Orange400,
    Orange500,
    Orange600,
    Orange700,
    Orange800,
    Orange900,
    Orange950,
    // Amber
    Amber50,
    Amber100,
    Amber200,
    Amber300,
    Amber400,
    Amber500,
    Amber600,
    Amber700,
    Amber800,
    Amber900,
    Amber950,
    // Yellow
    Yellow50,
    Yellow100,
    Yellow200,
    Yellow300,
    Yellow400,
    Yellow500,
    Yellow600,
    Yellow700,
    Yellow800,
    Yellow900,
    Yellow950,
    // Lime
    Lime50,
    Lime100,
    Lime200,
    Lime300,
    Lime400,
    Lime500,
    Lime600,
    Lime700,
    Lime800,
    Lime900,
    Lime950,
    // Green
    Green50,
    Green100,
    Green200,
    Green300,
    Green400,
    Green500,
    Green600,
    Green700,
    Green800,
    Green900,
    Green950,
    // Emerald
    Emerald50,
    Emerald100,
    Emerald200,
    Emerald300,
    Emerald400,
    Emerald500,
    Emerald600,
    Emerald700,
    Emerald800,
    Emerald900,
    Emerald950,
    // Teal
    Teal50,
    Teal100,
    Teal200,
    Teal300,
    Teal400,
    Teal500,
    Teal600,
    Teal700,
    Teal800,
    Teal900,
    Teal950,
    // Cyan
    Cyan50,
    Cyan100,
    Cyan200,
    Cyan300,
    Cyan400,
    Cyan500,
    Cyan600,
    Cyan700,
    Cyan800,
    Cyan900,
    Cyan950,
    // Sky
    Sky50,
    Sky100,
    Sky200,
    Sky300,
    Sky400,
    Sky500,
    Sky600,
    Sky700,
    Sky800,
    Sky900,
    Sky950,
    // Blue
    Blue50,
    Blue100,
    Blue200,
    Blue300,
    Blue400,
    Blue500,
    Blue600,
    Blue700,
    Blue800,
    Blue900,
    Blue950,
    // Indigo
    Indigo50,
    Indigo100,
    Indigo200,
    Indigo300,
    Indigo400,
    Indigo500,
    Indigo600,
    Indigo700,
    Indigo800,
    Indigo900,
    Indigo950,
    // Violet
    Violet50,
    Violet100,
    Violet200,
    Violet300,
    Violet400,
    Violet500,
    Violet600,
    Violet700,
    Violet800,
    Violet900,
    Violet950,
    // Purple
    Purple50,
    Purple100,
    Purple200,
    Purple300,
    Purple400,
    Purple500,
    Purple600,
    Purple700,
    Purple800,
    Purple900,
    Purple950,
    // Fuchsia
    Fuchsia50,
    Fuchsia100,
    Fuchsia200,
    Fuchsia300,
    Fuchsia400,
    Fuchsia500,
    Fuchsia600,
    Fuchsia700,
    Fuchsia800,
    Fuchsia900,
    Fuchsia950,
    // Pink
    Pink50,
    Pink100,
    Pink200,
    Pink300,
    Pink400,
    Pink500,
    Pink600,
    Pink700,
    Pink800,
    Pink900,
    Pink950,
    // Rose
    Rose50,
    Rose100,
    Rose200,
    Rose300,
    Rose400,
    Rose500,
    Rose600,
    Rose700,
    Rose800,
    Rose900,
    Rose950,

    // Slate
    Slate50,
    Slate100,
    Slate200,
    Slate300,
    Slate400,
    Slate500,
    Slate600,
    Slate700,
    Slate800,
    Slate900,
    Slate950,
    // Gray
    Gray50,
    Gray100,
    Gray200,
    Gray300,
    Gray400,
    Gray500,
    Gray600,
    Gray700,
    Gray800,
    Gray900,
    Gray950,
    // Zinc
    Zinc50,
    Zinc100,
    Zinc200,
    Zinc300,
    Zinc400,
    Zinc500,
    Zinc600,
    Zinc700,
    Zinc800,
    Zinc900,
    Zinc950,
    // Neutral
    Neutral50,
    Neutral100,
    Neutral200,
    Neutral300,
    Neutral400,
    Neutral500,
    Neutral600,
    Neutral700,
    Neutral800,
    Neutral900,
    Neutral950,
    // Stone
    Stone50,
    Stone100,
    Stone200,
    Stone300,
    Stone400,
    Stone500,
    Stone600,
    Stone700,
    Stone800,
    Stone900,
    Stone950,
}

// Generated Tailwind color implementations from build.rs
// Provides: to_hdr_color(), css_var(), hex()
include!(concat!(env!("OUT_DIR"), "/tailwind_colors.rs"));
