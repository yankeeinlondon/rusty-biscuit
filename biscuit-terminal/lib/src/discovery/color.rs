use std::{collections::HashMap, sync::LazyLock};

/// A single byte value (0-255) representing one RGB color channel.
///
/// This is a newtype wrapper around `u8` that provides type safety for color
/// components, ensuring values are always in the valid 0-255 range.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::terminal::color::Octet;
///
/// // From a u8 value
/// let red = Octet::new(255);
///
/// // Using From/Into
/// let green: Octet = 128u8.into();
///
/// // Get the inner value
/// assert_eq!(green.value(), 128);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Octet(u8);

impl Octet {
    /// Creates a new `Octet` from a `u8` value.
    #[inline]
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    /// Returns the inner `u8` value.
    #[inline]
    pub const fn value(self) -> u8 {
        self.0
    }
}

impl From<u8> for Octet {
    #[inline]
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<Octet> for u8 {
    #[inline]
    fn from(octet: Octet) -> Self {
        octet.0
    }
}

/// Basic 8 color mode (ANSI colors 0-7 and bright variants 8-15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// An RGB color with a fallback for terminals with limited color support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RgbColor {
    red: Octet,
    green: Octet,
    blue: Octet,
    fallback: BasicColor,
}

impl RgbColor {
    /// Creates a new RGB color with the specified channel values and fallback.
    #[inline]
    pub const fn new(red: u8, green: u8, blue: u8, fallback: BasicColor) -> Self {
        Self {
            red: Octet::new(red),
            green: Octet::new(green),
            blue: Octet::new(blue),
            fallback,
        }
    }

    /// Returns the red channel value.
    #[inline]
    pub const fn red(&self) -> u8 {
        self.red.value()
    }

    /// Returns the green channel value.
    #[inline]
    pub const fn green(&self) -> u8 {
        self.green.value()
    }

    /// Returns the blue channel value.
    #[inline]
    pub const fn blue(&self) -> u8 {
        self.blue.value()
    }

    /// Returns the fallback color for terminals with limited color support.
    #[inline]
    pub const fn fallback(&self) -> BasicColor {
        self.fallback
    }
}


/// Use any of the named CSS/Web Colors.
///
/// These correspond to the 148 named colors defined in the CSS Color Module Level 4
/// specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WebColor {
    AliceBlue,
    AntiqueWhite,
    Aqua,
    Aquamarine,
    Azure,
    Beige,
    Bisque,
    Black,
    BlanchedAlmond,
    Blue,
    BlueViolet,
    Brown,
    BurlyWood,
    CadetBlue,
    Chartreuse,
    Chocolate,
    Coral,
    CornflowerBlue,
    Cornsilk,
    Crimson,
    Cyan,
    DarkBlue,
    DarkCyan,
    DarkGoldenrod,
    DarkGray,
    DarkGrey,
    DarkGreen,
    DarkKhaki,
    DarkMagenta,
    DarkOliveGreen,
    DarkOrange,
    DarkOrchid,
    DarkRed,
    DarkSalmon,
    DarkSeaGreen,
    DarkSlateBlue,
    DarkSlateGray,
    DarkSlateGrey,
    DarkTurquoise,
    DarkViolet,
    DeepPink,
    DeepSkyBlue,
    DimGray,
    DimGrey,
    DodgerBlue,
    FireBrick,
    FloralWhite,
    ForestGreen,
    Fuchsia,
    Gainsboro,
    GhostWhite,
    Gold,
    Goldenrod,
    Gray,
    Grey,
    Green,
    GreenYellow,
    HoneyDew,
    HotPink,
    IndianRed,
    Indigo,
    Ivory,
    Khaki,
    Lavender,
    LavenderBlush,
    LawnGreen,
    LemonChiffon,
    LightBlue,
    LightCoral,
    LightCyan,
    LightGoldenrodYellow,
    LightGray,
    LightGrey,
    LightGreen,
    LightPink,
    LightSalmon,
    LightSeaGreen,
    LightSkyBlue,
    LightSlateGray,
    LightSlateGrey,
    LightSteelBlue,
    LightYellow,
    Lime,
    LimeGreen,
    Linen,
    Magenta,
    Maroon,
    MediumAquamarine,
    MediumBlue,
    MediumOrchid,
    MediumPurple,
    MediumSeaGreen,
    MediumSlateBlue,
    MediumSpringGreen,
    MediumTurquoise,
    MediumVioletRed,
    MidnightBlue,
    MintCream,
    MistyRose,
    Moccasin,
    NavajoWhite,
    Navy,
    OldLace,
    Olive,
    OliveDrab,
    Orange,
    OrangeRed,
    Orchid,
    PaleGoldenrod,
    PaleGreen,
    PaleTurquoise,
    PaleVioletRed,
    PapayaWhip,
    PeachPuff,
    Peru,
    Pink,
    Plum,
    PowderBlue,
    Purple,
    RebeccaPurple,
    Red,
    RosyBrown,
    RoyalBlue,
    SaddleBrown,
    Salmon,
    SandyBrown,
    SeaGreen,
    SeaShell,
    Sienna,
    Silver,
    SkyBlue,
    SlateBlue,
    SlateGray,
    SlateGrey,
    Snow,
    SpringGreen,
    SteelBlue,
    Tan,
    Teal,
    Thistle,
    Tomato,
    Turquoise,
    Violet,
    Wheat,
    White,
    WhiteSmoke,
    Yellow,
    YellowGreen,
}


/// Lookup table mapping CSS named colors to their RGB values with ANSI fallbacks.
///
/// Fallback colors are chosen to be the closest perceptual match from the basic
/// 16-color ANSI palette for terminals that don't support true color.
pub static WEB_COLOR_LOOKUP: LazyLock<HashMap<WebColor, RgbColor>> = LazyLock::new(|| {
    use BasicColor::*;
    use WebColor::*;

    let mut m = HashMap::with_capacity(148);

    // A
    m.insert(AliceBlue, RgbColor::new(240, 248, 255, BrightCyan));
    m.insert(AntiqueWhite, RgbColor::new(250, 235, 215, BasicColor::White));
    m.insert(Aqua, RgbColor::new(0, 255, 255, BrightCyan));
    m.insert(Aquamarine, RgbColor::new(127, 255, 212, BrightCyan));
    m.insert(Azure, RgbColor::new(240, 255, 255, BrightWhite));

    // B
    m.insert(Beige, RgbColor::new(245, 245, 220, BasicColor::White));
    m.insert(Bisque, RgbColor::new(255, 228, 196, BasicColor::White));
    m.insert(WebColor::Black, RgbColor::new(0, 0, 0, BasicColor::Black));
    m.insert(BlanchedAlmond, RgbColor::new(255, 235, 205, BasicColor::White));
    m.insert(WebColor::Blue, RgbColor::new(0, 0, 255, BrightBlue));
    m.insert(BlueViolet, RgbColor::new(138, 43, 226, BasicColor::Magenta));
    m.insert(Brown, RgbColor::new(165, 42, 42, BasicColor::Red));
    m.insert(BurlyWood, RgbColor::new(222, 184, 135, BasicColor::Yellow));

    // C
    m.insert(CadetBlue, RgbColor::new(95, 158, 160, BasicColor::Cyan));
    m.insert(Chartreuse, RgbColor::new(127, 255, 0, BrightGreen));
    m.insert(Chocolate, RgbColor::new(210, 105, 30, BasicColor::Red));
    m.insert(Coral, RgbColor::new(255, 127, 80, BrightRed));
    m.insert(CornflowerBlue, RgbColor::new(100, 149, 237, BrightBlue));
    m.insert(Cornsilk, RgbColor::new(255, 248, 220, BrightWhite));
    m.insert(Crimson, RgbColor::new(220, 20, 60, BasicColor::Red));
    m.insert(WebColor::Cyan, RgbColor::new(0, 255, 255, BrightCyan));

    // D
    m.insert(DarkBlue, RgbColor::new(0, 0, 139, BasicColor::Blue));
    m.insert(DarkCyan, RgbColor::new(0, 139, 139, BasicColor::Cyan));
    m.insert(DarkGoldenrod, RgbColor::new(184, 134, 11, BasicColor::Yellow));
    m.insert(DarkGray, RgbColor::new(169, 169, 169, BrightBlack));
    m.insert(DarkGrey, RgbColor::new(169, 169, 169, BrightBlack));
    m.insert(DarkGreen, RgbColor::new(0, 100, 0, BasicColor::Green));
    m.insert(DarkKhaki, RgbColor::new(189, 183, 107, BasicColor::Yellow));
    m.insert(DarkMagenta, RgbColor::new(139, 0, 139, BasicColor::Magenta));
    m.insert(DarkOliveGreen, RgbColor::new(85, 107, 47, BasicColor::Green));
    m.insert(DarkOrange, RgbColor::new(255, 140, 0, BrightYellow));
    m.insert(DarkOrchid, RgbColor::new(153, 50, 204, BasicColor::Magenta));
    m.insert(DarkRed, RgbColor::new(139, 0, 0, BasicColor::Red));
    m.insert(DarkSalmon, RgbColor::new(233, 150, 122, BrightRed));
    m.insert(DarkSeaGreen, RgbColor::new(143, 188, 143, BasicColor::Green));
    m.insert(DarkSlateBlue, RgbColor::new(72, 61, 139, BasicColor::Blue));
    m.insert(DarkSlateGray, RgbColor::new(47, 79, 79, BrightBlack));
    m.insert(DarkSlateGrey, RgbColor::new(47, 79, 79, BrightBlack));
    m.insert(DarkTurquoise, RgbColor::new(0, 206, 209, BrightCyan));
    m.insert(DarkViolet, RgbColor::new(148, 0, 211, BasicColor::Magenta));
    m.insert(DeepPink, RgbColor::new(255, 20, 147, BrightMagenta));
    m.insert(DeepSkyBlue, RgbColor::new(0, 191, 255, BrightCyan));
    m.insert(DimGray, RgbColor::new(105, 105, 105, BrightBlack));
    m.insert(DimGrey, RgbColor::new(105, 105, 105, BrightBlack));
    m.insert(DodgerBlue, RgbColor::new(30, 144, 255, BrightBlue));

    // F
    m.insert(FireBrick, RgbColor::new(178, 34, 34, BasicColor::Red));
    m.insert(FloralWhite, RgbColor::new(255, 250, 240, BrightWhite));
    m.insert(ForestGreen, RgbColor::new(34, 139, 34, BasicColor::Green));
    m.insert(Fuchsia, RgbColor::new(255, 0, 255, BrightMagenta));

    // G
    m.insert(Gainsboro, RgbColor::new(220, 220, 220, BasicColor::White));
    m.insert(GhostWhite, RgbColor::new(248, 248, 255, BrightWhite));
    m.insert(Gold, RgbColor::new(255, 215, 0, BrightYellow));
    m.insert(Goldenrod, RgbColor::new(218, 165, 32, BasicColor::Yellow));
    m.insert(Gray, RgbColor::new(128, 128, 128, BrightBlack));
    m.insert(Grey, RgbColor::new(128, 128, 128, BrightBlack));
    m.insert(WebColor::Green, RgbColor::new(0, 128, 0, BasicColor::Green));
    m.insert(GreenYellow, RgbColor::new(173, 255, 47, BrightGreen));

    // H
    m.insert(HoneyDew, RgbColor::new(240, 255, 240, BrightWhite));
    m.insert(HotPink, RgbColor::new(255, 105, 180, BrightMagenta));

    // I
    m.insert(IndianRed, RgbColor::new(205, 92, 92, BasicColor::Red));
    m.insert(Indigo, RgbColor::new(75, 0, 130, BasicColor::Magenta));
    m.insert(Ivory, RgbColor::new(255, 255, 240, BrightWhite));

    // K
    m.insert(Khaki, RgbColor::new(240, 230, 140, BrightYellow));

    // L
    m.insert(Lavender, RgbColor::new(230, 230, 250, BasicColor::White));
    m.insert(LavenderBlush, RgbColor::new(255, 240, 245, BrightWhite));
    m.insert(LawnGreen, RgbColor::new(124, 252, 0, BrightGreen));
    m.insert(LemonChiffon, RgbColor::new(255, 250, 205, BrightYellow));
    m.insert(LightBlue, RgbColor::new(173, 216, 230, BrightCyan));
    m.insert(LightCoral, RgbColor::new(240, 128, 128, BrightRed));
    m.insert(LightCyan, RgbColor::new(224, 255, 255, BrightCyan));
    m.insert(LightGoldenrodYellow, RgbColor::new(250, 250, 210, BrightYellow));
    m.insert(LightGray, RgbColor::new(211, 211, 211, BasicColor::White));
    m.insert(LightGrey, RgbColor::new(211, 211, 211, BasicColor::White));
    m.insert(LightGreen, RgbColor::new(144, 238, 144, BrightGreen));
    m.insert(LightPink, RgbColor::new(255, 182, 193, BrightMagenta));
    m.insert(LightSalmon, RgbColor::new(255, 160, 122, BrightRed));
    m.insert(LightSeaGreen, RgbColor::new(32, 178, 170, BasicColor::Cyan));
    m.insert(LightSkyBlue, RgbColor::new(135, 206, 250, BrightCyan));
    m.insert(LightSlateGray, RgbColor::new(119, 136, 153, BrightBlack));
    m.insert(LightSlateGrey, RgbColor::new(119, 136, 153, BrightBlack));
    m.insert(LightSteelBlue, RgbColor::new(176, 196, 222, BrightBlue));
    m.insert(LightYellow, RgbColor::new(255, 255, 224, BrightYellow));
    m.insert(Lime, RgbColor::new(0, 255, 0, BrightGreen));
    m.insert(LimeGreen, RgbColor::new(50, 205, 50, BrightGreen));
    m.insert(Linen, RgbColor::new(250, 240, 230, BasicColor::White));

    // M
    m.insert(WebColor::Magenta, RgbColor::new(255, 0, 255, BrightMagenta));
    m.insert(Maroon, RgbColor::new(128, 0, 0, BasicColor::Red));
    m.insert(MediumAquamarine, RgbColor::new(102, 205, 170, BasicColor::Cyan));
    m.insert(MediumBlue, RgbColor::new(0, 0, 205, BasicColor::Blue));
    m.insert(MediumOrchid, RgbColor::new(186, 85, 211, BasicColor::Magenta));
    m.insert(MediumPurple, RgbColor::new(147, 112, 219, BasicColor::Magenta));
    m.insert(MediumSeaGreen, RgbColor::new(60, 179, 113, BasicColor::Green));
    m.insert(MediumSlateBlue, RgbColor::new(123, 104, 238, BrightBlue));
    m.insert(MediumSpringGreen, RgbColor::new(0, 250, 154, BrightGreen));
    m.insert(MediumTurquoise, RgbColor::new(72, 209, 204, BrightCyan));
    m.insert(MediumVioletRed, RgbColor::new(199, 21, 133, BasicColor::Magenta));
    m.insert(MidnightBlue, RgbColor::new(25, 25, 112, BasicColor::Blue));
    m.insert(MintCream, RgbColor::new(245, 255, 250, BasicColor::BrightWhite));
    m.insert(MistyRose, RgbColor::new(255, 228, 225, BasicColor::White));
    m.insert(Moccasin, RgbColor::new(255, 228, 181, BrightYellow));

    // N
    m.insert(NavajoWhite, RgbColor::new(255, 222, 173, BrightYellow));
    m.insert(Navy, RgbColor::new(0, 0, 128, BasicColor::Blue));

    // O
    m.insert(OldLace, RgbColor::new(253, 245, 230, BasicColor::White));
    m.insert(Olive, RgbColor::new(128, 128, 0, BasicColor::Yellow));
    m.insert(OliveDrab, RgbColor::new(107, 142, 35, BasicColor::Green));
    m.insert(Orange, RgbColor::new(255, 165, 0, BrightYellow));
    m.insert(OrangeRed, RgbColor::new(255, 69, 0, BrightRed));
    m.insert(Orchid, RgbColor::new(218, 112, 214, BrightMagenta));

    // P
    m.insert(PaleGoldenrod, RgbColor::new(238, 232, 170, BrightYellow));
    m.insert(PaleGreen, RgbColor::new(152, 251, 152, BrightGreen));
    m.insert(PaleTurquoise, RgbColor::new(175, 238, 238, BrightCyan));
    m.insert(PaleVioletRed, RgbColor::new(219, 112, 147, BasicColor::Magenta));
    m.insert(PapayaWhip, RgbColor::new(255, 239, 213, BasicColor::White));
    m.insert(PeachPuff, RgbColor::new(255, 218, 185, BrightYellow));
    m.insert(Peru, RgbColor::new(205, 133, 63, BasicColor::Yellow));
    m.insert(Pink, RgbColor::new(255, 192, 203, BrightMagenta));
    m.insert(Plum, RgbColor::new(221, 160, 221, BrightMagenta));
    m.insert(PowderBlue, RgbColor::new(176, 224, 230, BrightCyan));
    m.insert(Purple, RgbColor::new(128, 0, 128, BasicColor::Magenta));

    // R
    m.insert(RebeccaPurple, RgbColor::new(102, 51, 153, BasicColor::Magenta));
    m.insert(WebColor::Red, RgbColor::new(255, 0, 0, BrightRed));
    m.insert(RosyBrown, RgbColor::new(188, 143, 143, BasicColor::White));
    m.insert(RoyalBlue, RgbColor::new(65, 105, 225, BrightBlue));

    // S
    m.insert(SaddleBrown, RgbColor::new(139, 69, 19, BasicColor::Red));
    m.insert(Salmon, RgbColor::new(250, 128, 114, BrightRed));
    m.insert(SandyBrown, RgbColor::new(244, 164, 96, BasicColor::Yellow));
    m.insert(SeaGreen, RgbColor::new(46, 139, 87, BasicColor::Green));
    m.insert(SeaShell, RgbColor::new(255, 245, 238, BrightWhite));
    m.insert(Sienna, RgbColor::new(160, 82, 45, BasicColor::Red));
    m.insert(Silver, RgbColor::new(192, 192, 192, BasicColor::White));
    m.insert(SkyBlue, RgbColor::new(135, 206, 235, BrightCyan));
    m.insert(SlateBlue, RgbColor::new(106, 90, 205, BasicColor::Blue));
    m.insert(SlateGray, RgbColor::new(112, 128, 144, BrightBlack));
    m.insert(SlateGrey, RgbColor::new(112, 128, 144, BrightBlack));
    m.insert(Snow, RgbColor::new(255, 250, 250, BrightWhite));
    m.insert(SpringGreen, RgbColor::new(0, 255, 127, BrightGreen));
    m.insert(SteelBlue, RgbColor::new(70, 130, 180, BasicColor::Blue));

    // T
    m.insert(Tan, RgbColor::new(210, 180, 140, BasicColor::Yellow));
    m.insert(Teal, RgbColor::new(0, 128, 128, BasicColor::Cyan));
    m.insert(Thistle, RgbColor::new(216, 191, 216, BasicColor::White));
    m.insert(Tomato, RgbColor::new(255, 99, 71, BrightRed));
    m.insert(Turquoise, RgbColor::new(64, 224, 208, BrightCyan));

    // V
    m.insert(Violet, RgbColor::new(238, 130, 238, BrightMagenta));

    // W
    m.insert(Wheat, RgbColor::new(245, 222, 179, BrightYellow));
    m.insert(WebColor::White, RgbColor::new(255, 255, 255, BrightWhite));
    m.insert(WhiteSmoke, RgbColor::new(245, 245, 245, BrightWhite));

    // Y
    m.insert(WebColor::Yellow, RgbColor::new(255, 255, 0, BrightYellow));
    m.insert(YellowGreen, RgbColor::new(154, 205, 50, BrightGreen));

    m
});


pub enum Color {
    /// use a basic color which can be used in any terminal
    /// which supports color
    Basic(BasicColor),
    /// specify a bespoke RGB color value (with a `BasicColor` as a fallback)
    Rgb(RgbColor),
    Web(WebColor)
}
