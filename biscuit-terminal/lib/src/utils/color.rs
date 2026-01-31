use std::{collections::HashMap, sync::LazyLock};

use crate::{components::renderable::RenderableWrapper, terminal::Terminal};

/// A single byte value (0-255) representing one RGB color channel.
///
/// This is a newtype wrapper around `u8` that provides type safety for color
/// components, ensuring values are always in the valid 0-255 range.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::utils::color::Octet;
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

/// An RGB color with:
///
/// - a fallback for terminals with limited color support.
/// - OKLCH values for HDR mapping (lightness, chroma, hue)
///
/// The OKLCH color space is used for perceptually uniform color
/// manipulation. Values are stored as f32:
/// - `oklch_l`: Lightness (0.0 to 1.0)
/// - `oklch_c`: Chroma (0.0 to ~0.4 for in-gamut colors)
/// - `oklch_h`: Hue (0.0 to 360.0 degrees)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HdrColor {
    red: Octet,
    green: Octet,
    blue: Octet,
    fallback: BasicColor,
    /// OKLCH Lightness (0.0 to 1.0)
    oklch_l: f32,
    /// OKLCH Chroma (0.0 to ~0.4)
    oklch_c: f32,
    /// OKLCH Hue (0.0 to 360.0 degrees)
    oklch_h: f32,
}

impl HdrColor {
    /// Creates a new HDR color with RGB values, fallback, and OKLCH components.
    #[inline]
    pub const fn new(
        red: u8,
        green: u8,
        blue: u8,
        fallback: BasicColor,
        oklch_l: f32,
        oklch_c: f32,
        oklch_h: f32,
    ) -> Self {
        Self {
            red: Octet::new(red),
            green: Octet::new(green),
            blue: Octet::new(blue),
            fallback,
            oklch_l,
            oklch_c,
            oklch_h,
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

    /// Returns the OKLCH lightness component (0.0 to 1.0).
    #[inline]
    pub const fn oklch_l(&self) -> f32 {
        self.oklch_l
    }

    /// Returns the OKLCH chroma component (0.0 to ~0.4 for in-gamut colors).
    #[inline]
    pub const fn oklch_c(&self) -> f32 {
        self.oklch_c
    }

    /// Returns the OKLCH hue component (0.0 to 360.0 degrees).
    #[inline]
    pub const fn oklch_h(&self) -> f32 {
        self.oklch_h
    }

    /// Returns the OKLCH components as a tuple (lightness, chroma, hue).
    #[inline]
    pub const fn oklch(&self) -> (f32, f32, f32) {
        (self.oklch_l, self.oklch_c, self.oklch_h)
    }
}

/// Use any of the named CSS/Web Colors.
///
/// These correspond to the 148 named colors defined in the CSS Color Module Level 4
/// specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WebColor {
    /// Alice Blue color (240, 248, 255).
    AliceBlue,
    /// Antique White color (250, 235, 215).
    AntiqueWhite,
    /// Aqua color (0, 255, 255).
    Aqua,
    /// Aquamarine color (127, 255, 212).
    Aquamarine,
    /// Azure color (240, 255, 255).
    Azure,
    /// Beige color (245, 245, 220).
    Beige,
    /// Bisque color (255, 228, 196).
    Bisque,
    /// Black color (0, 0, 0).
    Black,
    /// Blanched Almond color (255, 235, 205).
    BlanchedAlmond,
    /// Blue color (0, 0, 255).
    Blue,
    /// Blue Violet color (138, 43, 226).
    BlueViolet,
    /// Brown color (165, 42, 42).
    Brown,
    /// Burly Wood color (222, 184, 135).
    BurlyWood,
    /// Cadet Blue color (95, 158, 160).
    CadetBlue,
    /// Chartreuse color (127, 255, 0).
    Chartreuse,
    /// Chocolate color (210, 105, 30).
    Chocolate,
    /// Coral color (255, 127, 80).
    Coral,
    /// Cornflower Blue color (100, 149, 237).
    CornflowerBlue,
    /// Cornsilk color (255, 248, 220).
    Cornsilk,
    /// Crimson color (220, 20, 60).
    Crimson,
    /// Cyan color (0, 255, 255).
    Cyan,
    /// Dark Blue color (0, 0, 139).
    DarkBlue,
    /// Dark Cyan color (0, 139, 139).
    DarkCyan,
    /// Dark Goldenrod color (184, 134, 11).
    DarkGoldenrod,
    /// Dark Gray color (169, 169, 169).
    DarkGray,
    /// Dark Grey color (169, 169, 169).
    DarkGrey,
    /// Dark Green color (0, 100, 0).
    DarkGreen,
    /// Dark Khaki color (189, 183, 107).
    DarkKhaki,
    /// Dark Magenta color (139, 0, 139).
    DarkMagenta,
    /// Dark Olive Green color (85, 107, 47).
    DarkOliveGreen,
    /// Dark Orange color (255, 140, 0).
    DarkOrange,
    /// Dark Orchid color (153, 50, 204).
    DarkOrchid,
    /// Dark Red color (139, 0, 0).
    DarkRed,
    /// Dark Salmon color (233, 150, 122).
    DarkSalmon,
    /// Dark Sea Green color (143, 188, 143).
    DarkSeaGreen,
    /// Dark Slate Blue color (72, 61, 139).
    DarkSlateBlue,
    /// Dark Slate Gray color (47, 79, 79).
    DarkSlateGray,
    /// Dark Slate Grey color (47, 79, 79).
    DarkSlateGrey,
    /// Dark Turquoise color (0, 206, 209).
    DarkTurquoise,
    /// Dark Violet color (148, 0, 211).
    DarkViolet,
    /// Deep Pink color (255, 20, 147).
    DeepPink,
    /// Deep Sky Blue color (0, 191, 255).
    DeepSkyBlue,
    /// Dim Gray color (105, 105, 105).
    DimGray,
    /// Dim Grey color (105, 105, 105).
    DimGrey,
    /// Dodger Blue color (30, 144, 255).
    DodgerBlue,
    /// Fire Brick color (178, 34, 34).
    FireBrick,
    /// Floral White color (255, 250, 240).
    FloralWhite,
    /// Forest Green color (34, 139, 34).
    ForestGreen,
    /// Fuchsia color (255, 0, 255).
    Fuchsia,
    /// Gainsboro color (220, 220, 220).
    Gainsboro,
    /// Ghost White color (248, 248, 255).
    GhostWhite,
    /// Gold color (255, 215, 0).
    Gold,
    /// Goldenrod color (218, 165, 32).
    Goldenrod,
    /// Gray color (128, 128, 128).
    Gray,
    /// Grey color (128, 128, 128).
    Grey,
    /// Green color (0, 128, 0).
    Green,
    /// Green Yellow color (173, 255, 47).
    GreenYellow,
    /// Honey Dew color (240, 255, 240).
    HoneyDew,
    /// Hot Pink color (255, 105, 180).
    HotPink,
    /// Indian Red color (205, 92, 92).
    IndianRed,
    /// Indigo color (75, 0, 130).
    Indigo,
    /// Ivory color (255, 255, 240).
    Ivory,
    /// Khaki color (240, 230, 140).
    Khaki,
    /// Lavender color (230, 230, 250).
    Lavender,
    /// Lavender Blush color (255, 240, 245).
    LavenderBlush,
    /// Lawn Green color (124, 252, 0).
    LawnGreen,
    /// Lemon Chiffon color (255, 250, 205).
    LemonChiffon,
    /// Light Blue color (173, 216, 230).
    LightBlue,
    /// Light Coral color (240, 128, 128).
    LightCoral,
    /// Light Cyan color (224, 255, 255).
    LightCyan,
    /// Light Goldenrod Yellow color (250, 250, 210).
    LightGoldenrodYellow,
    /// Light Gray color (211, 211, 211).
    LightGray,
    /// Light Grey color (211, 211, 211).
    LightGrey,
    /// Light Green color (144, 238, 144).
    LightGreen,
    /// Light Pink color (255, 182, 193).
    LightPink,
    /// Light Salmon color (255, 160, 122).
    LightSalmon,
    /// Light Sea Green color (32, 178, 170).
    LightSeaGreen,
    /// Light Sky Blue color (135, 206, 250).
    LightSkyBlue,
    /// Light Slate Gray color (119, 136, 153).
    LightSlateGray,
    /// Light Slate Grey color (119, 136, 153).
    LightSlateGrey,
    /// Light Steel Blue color (176, 196, 222).
    LightSteelBlue,
    /// Light Yellow color (255, 255, 224).
    LightYellow,
    /// Lime color (0, 255, 0).
    Lime,
    /// Lime Green color (50, 205, 50).
    LimeGreen,
    /// Linen color (250, 240, 230).
    Linen,
    /// Magenta color (255, 0, 255).
    Magenta,
    /// Maroon color (128, 0, 0).
    Maroon,
    /// Medium Aquamarine color (102, 205, 170).
    MediumAquamarine,
    /// Medium Blue color (0, 0, 205).
    MediumBlue,
    /// Medium Orchid color (186, 85, 211).
    MediumOrchid,
    /// Medium Purple color (147, 112, 219).
    MediumPurple,
    /// Medium Sea Green color (60, 179, 113).
    MediumSeaGreen,
    /// Medium Slate Blue color (123, 104, 238).
    MediumSlateBlue,
    /// Medium Spring Green color (0, 250, 154).
    MediumSpringGreen,
    /// Medium Turquoise color (72, 209, 204).
    MediumTurquoise,
    /// Medium Violet Red color (199, 21, 133).
    MediumVioletRed,
    /// Midnight Blue color (25, 25, 112).
    MidnightBlue,
    /// Mint Cream color (245, 255, 250).
    MintCream,
    /// Misty Rose color (255, 228, 225).
    MistyRose,
    /// Moccasin color (255, 228, 181).
    Moccasin,
    /// Navajo White color (255, 222, 173).
    NavajoWhite,
    /// Navy color (0, 0, 128).
    Navy,
    /// Old Lace color (253, 245, 230).
    OldLace,
    /// Olive color (128, 128, 0).
    Olive,
    /// Olive Drab color (107, 142, 35).
    OliveDrab,
    /// Orange color (255, 165, 0).
    Orange,
    /// Orange Red color (255, 69, 0).
    OrangeRed,
    /// Orchid color (218, 112, 214).
    Orchid,
    /// Pale Goldenrod color (238, 232, 170).
    PaleGoldenrod,
    /// Pale Green color (152, 251, 152).
    PaleGreen,
    /// Pale Turquoise color (175, 238, 238).
    PaleTurquoise,
    /// Pale Violet Red color (219, 112, 147).
    PaleVioletRed,
    /// Papaya Whip color (255, 239, 213).
    PapayaWhip,
    /// Peach Puff color (255, 218, 185).
    PeachPuff,
    /// Peru color (205, 133, 63).
    Peru,
    /// Pink color (255, 192, 203).
    Pink,
    /// Plum color (221, 160, 221).
    Plum,
    /// Powder Blue color (176, 224, 230).
    PowderBlue,
    /// Purple color (128, 0, 128).
    Purple,
    /// Rebecca Purple color (102, 51, 153).
    RebeccaPurple,
    /// Red color (255, 0, 0).
    Red,
    /// Rosy Brown color (188, 143, 143).
    RosyBrown,
    /// Royal Blue color (65, 105, 225).
    RoyalBlue,
    /// Saddle Brown color (139, 69, 19).
    SaddleBrown,
    /// Salmon color (250, 128, 114).
    Salmon,
    /// Sandy Brown color (244, 164, 96).
    SandyBrown,
    /// Sea Green color (46, 139, 87).
    SeaGreen,
    /// Sea Shell color (255, 245, 238).
    SeaShell,
    /// Sienna color (160, 82, 45).
    Sienna,
    /// Silver color (192, 192, 192).
    Silver,
    /// Sky Blue color (135, 206, 235).
    SkyBlue,
    /// Slate Blue color (106, 90, 205).
    SlateBlue,
    /// Slate Gray color (112, 128, 144).
    SlateGray,
    /// Slate Grey color (112, 128, 144).
    SlateGrey,
    /// Snow color (255, 250, 250).
    Snow,
    /// Spring Green color (0, 255, 127).
    SpringGreen,
    /// Steel Blue color (70, 130, 180).
    SteelBlue,
    /// Tan color (210, 180, 140).
    Tan,
    /// Teal color (0, 128, 128).
    Teal,
    /// Thistle color (216, 191, 216).
    Thistle,
    /// Tomato color (255, 99, 71).
    Tomato,
    /// Turquoise color (64, 224, 208).
    Turquoise,
    /// Violet color (238, 130, 238).
    Violet,
    /// Wheat color (245, 222, 179).
    Wheat,
    /// White color (255, 255, 255).
    White,
    /// White Smoke color (245, 245, 245).
    WhiteSmoke,
    /// Yellow color (255, 255, 0).
    Yellow,
    /// Yellow Green color (154, 205, 50).
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
    m.insert(
        AntiqueWhite,
        RgbColor::new(250, 235, 215, BasicColor::White),
    );
    m.insert(Aqua, RgbColor::new(0, 255, 255, BrightCyan));
    m.insert(Aquamarine, RgbColor::new(127, 255, 212, BrightCyan));
    m.insert(Azure, RgbColor::new(240, 255, 255, BrightWhite));

    // B
    m.insert(Beige, RgbColor::new(245, 245, 220, BasicColor::White));
    m.insert(Bisque, RgbColor::new(255, 228, 196, BasicColor::White));
    m.insert(WebColor::Black, RgbColor::new(0, 0, 0, BasicColor::Black));
    m.insert(
        BlanchedAlmond,
        RgbColor::new(255, 235, 205, BasicColor::White),
    );
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
    m.insert(
        DarkGoldenrod,
        RgbColor::new(184, 134, 11, BasicColor::Yellow),
    );
    m.insert(DarkGray, RgbColor::new(169, 169, 169, BrightBlack));
    m.insert(DarkGrey, RgbColor::new(169, 169, 169, BrightBlack));
    m.insert(DarkGreen, RgbColor::new(0, 100, 0, BasicColor::Green));
    m.insert(DarkKhaki, RgbColor::new(189, 183, 107, BasicColor::Yellow));
    m.insert(DarkMagenta, RgbColor::new(139, 0, 139, BasicColor::Magenta));
    m.insert(
        DarkOliveGreen,
        RgbColor::new(85, 107, 47, BasicColor::Green),
    );
    m.insert(DarkOrange, RgbColor::new(255, 140, 0, BrightYellow));
    m.insert(DarkOrchid, RgbColor::new(153, 50, 204, BasicColor::Magenta));
    m.insert(DarkRed, RgbColor::new(139, 0, 0, BasicColor::Red));
    m.insert(DarkSalmon, RgbColor::new(233, 150, 122, BrightRed));
    m.insert(
        DarkSeaGreen,
        RgbColor::new(143, 188, 143, BasicColor::Green),
    );
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
    m.insert(
        LightGoldenrodYellow,
        RgbColor::new(250, 250, 210, BrightYellow),
    );
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
    m.insert(
        MediumAquamarine,
        RgbColor::new(102, 205, 170, BasicColor::Cyan),
    );
    m.insert(MediumBlue, RgbColor::new(0, 0, 205, BasicColor::Blue));
    m.insert(
        MediumOrchid,
        RgbColor::new(186, 85, 211, BasicColor::Magenta),
    );
    m.insert(
        MediumPurple,
        RgbColor::new(147, 112, 219, BasicColor::Magenta),
    );
    m.insert(
        MediumSeaGreen,
        RgbColor::new(60, 179, 113, BasicColor::Green),
    );
    m.insert(MediumSlateBlue, RgbColor::new(123, 104, 238, BrightBlue));
    m.insert(MediumSpringGreen, RgbColor::new(0, 250, 154, BrightGreen));
    m.insert(MediumTurquoise, RgbColor::new(72, 209, 204, BrightCyan));
    m.insert(
        MediumVioletRed,
        RgbColor::new(199, 21, 133, BasicColor::Magenta),
    );
    m.insert(MidnightBlue, RgbColor::new(25, 25, 112, BasicColor::Blue));
    m.insert(MintCream, RgbColor::new(245, 255, 250, BrightWhite));
    m.insert(MistyRose, RgbColor::new(255, 228, 225, BasicColor::White));
    m.insert(Moccasin, RgbColor::new(255, 228, 181, BrightYellow));

    // N
    m.insert(NavajoWhite, RgbColor::new(255, 222, 173, BrightYellow));
    m.insert(Navy, RgbColor::new(0, 0, 128, BasicColor::Blue));
    m.insert(OldLace, RgbColor::new(253, 245, 230, BasicColor::White));
    m.insert(Olive, RgbColor::new(128, 128, 0, BasicColor::Yellow));
    m.insert(OliveDrab, RgbColor::new(107, 142, 35, BasicColor::Green));
    m.insert(Orange, RgbColor::new(255, 165, 0, BrightYellow));
    m.insert(OrangeRed, RgbColor::new(255, 69, 0, BrightRed));
    m.insert(Orchid, RgbColor::new(218, 112, 214, BasicColor::Magenta));

    // P
    m.insert(PaleGoldenrod, RgbColor::new(238, 232, 170, BrightYellow));
    m.insert(PaleGreen, RgbColor::new(152, 251, 152, BrightGreen));
    m.insert(PaleTurquoise, RgbColor::new(175, 238, 238, BrightCyan));
    m.insert(
        PaleVioletRed,
        RgbColor::new(219, 112, 147, BasicColor::Magenta),
    );
    m.insert(PapayaWhip, RgbColor::new(255, 239, 213, BasicColor::White));
    m.insert(PeachPuff, RgbColor::new(255, 218, 185, BrightYellow));
    m.insert(Peru, RgbColor::new(205, 133, 63, BasicColor::Yellow));
    m.insert(Pink, RgbColor::new(255, 192, 203, BrightMagenta));
    m.insert(Plum, RgbColor::new(221, 160, 221, BrightMagenta));
    m.insert(PowderBlue, RgbColor::new(176, 224, 230, BrightCyan));
    m.insert(Purple, RgbColor::new(128, 0, 128, BasicColor::Magenta));

    // R
    m.insert(
        RebeccaPurple,
        RgbColor::new(102, 51, 153, BasicColor::Magenta),
    );
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tailwind {
    // “specials” commonly present in Tailwind palettes
    Inherit,
    Current,
    Transparent,
    Black,
    White,

    // Red
    Red50, Red100, Red200, Red300, Red400, Red500, Red600, Red700, Red800, Red900, Red950,
    // Orange
    Orange50, Orange100, Orange200, Orange300, Orange400, Orange500, Orange600, Orange700, Orange800, Orange900, Orange950,
    // Amber
    Amber50, Amber100, Amber200, Amber300, Amber400, Amber500, Amber600, Amber700, Amber800, Amber900, Amber950,
    // Yellow
    Yellow50, Yellow100, Yellow200, Yellow300, Yellow400, Yellow500, Yellow600, Yellow700, Yellow800, Yellow900, Yellow950,
    // Lime
    Lime50, Lime100, Lime200, Lime300, Lime400, Lime500, Lime600, Lime700, Lime800, Lime900, Lime950,
    // Green
    Green50, Green100, Green200, Green300, Green400, Green500, Green600, Green700, Green800, Green900, Green950,
    // Emerald
    Emerald50, Emerald100, Emerald200, Emerald300, Emerald400, Emerald500, Emerald600, Emerald700, Emerald800, Emerald900, Emerald950,
    // Teal
    Teal50, Teal100, Teal200, Teal300, Teal400, Teal500, Teal600, Teal700, Teal800, Teal900, Teal950,
    // Cyan
    Cyan50, Cyan100, Cyan200, Cyan300, Cyan400, Cyan500, Cyan600, Cyan700, Cyan800, Cyan900, Cyan950,
    // Sky
    Sky50, Sky100, Sky200, Sky300, Sky400, Sky500, Sky600, Sky700, Sky800, Sky900, Sky950,
    // Blue
    Blue50, Blue100, Blue200, Blue300, Blue400, Blue500, Blue600, Blue700, Blue800, Blue900, Blue950,
    // Indigo
    Indigo50, Indigo100, Indigo200, Indigo300, Indigo400, Indigo500, Indigo600, Indigo700, Indigo800, Indigo900, Indigo950,
    // Violet
    Violet50, Violet100, Violet200, Violet300, Violet400, Violet500, Violet600, Violet700, Violet800, Violet900, Violet950,
    // Purple
    Purple50, Purple100, Purple200, Purple300, Purple400, Purple500, Purple600, Purple700, Purple800, Purple900, Purple950,
    // Fuchsia
    Fuchsia50, Fuchsia100, Fuchsia200, Fuchsia300, Fuchsia400, Fuchsia500, Fuchsia600, Fuchsia700, Fuchsia800, Fuchsia900, Fuchsia950,
    // Pink
    Pink50, Pink100, Pink200, Pink300, Pink400, Pink500, Pink600, Pink700, Pink800, Pink900, Pink950,
    // Rose
    Rose50, Rose100, Rose200, Rose300, Rose400, Rose500, Rose600, Rose700, Rose800, Rose900, Rose950,

    // Slate
    Slate50, Slate100, Slate200, Slate300, Slate400, Slate500, Slate600, Slate700, Slate800, Slate900, Slate950,
    // Gray
    Gray50, Gray100, Gray200, Gray300, Gray400, Gray500, Gray600, Gray700, Gray800, Gray900, Gray950,
    // Zinc
    Zinc50, Zinc100, Zinc200, Zinc300, Zinc400, Zinc500, Zinc600, Zinc700, Zinc800, Zinc900, Zinc950,
    // Neutral
    Neutral50, Neutral100, Neutral200, Neutral300, Neutral400, Neutral500, Neutral600, Neutral700, Neutral800, Neutral900, Neutral950,
    // Stone
    Stone50, Stone100, Stone200, Stone300, Stone400, Stone500, Stone600, Stone700, Stone800, Stone900, Stone950,
}

// Generated Tailwind color implementations from build.rs
// Provides: to_hdr_color(), css_var(), hex()
include!(concat!(env!("OUT_DIR"), "/tailwind_colors.rs"));

/// An enumeration for specifying color for the terminal
pub enum Color {
    /// Use a basic color which can be used in any terminal
    /// which supports color
    Basic(BasicColor),
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

/// Wrapper for basic colors that implements `RenderableWrapper`.
#[derive(Debug, Clone, Copy)]
pub struct BasicColorWrapper(pub BasicColor);

impl RenderableWrapper for BasicColorWrapper {
    fn render<T: Into<String>>(self, content: T) -> String {
        let content = content.into();
        format!("\x1b[{}m{}\x1b[0m", color_code(self.0), content)
    }

    fn fallback_render<T: Into<String>>(self, content: T, _term: &Terminal) -> String {
        let content = content.into();
        format!("\x1b[{}m{}\x1b[0m", color_code(self.0), content)
    }
}


/// Wrapper for RGB colors that implements `RenderableWrapper`.
#[derive(Debug, Clone, Copy)]
pub struct RgbColorWrapper(pub RgbColor);

impl RenderableWrapper for RgbColorWrapper {
    fn render<T: Into<String>>(self, content: T) -> String {
        let content = content.into();
        let rgb = self.0;
        format!(
            "\x1b[38;2;{};{};{}m{}\x1b[0m",
            rgb.red(),
            rgb.green(),
            rgb.blue(),
            content
        )
    }

    fn fallback_render<T: Into<String>>(self, content: T, term: &Terminal) -> String {
        let content = content.into();
        let rgb = self.0;
        // Check terminal color depth and use appropriate encoding
        match term.color_depth {
            crate::discovery::detection::ColorDepth::TrueColor => {
                format!(
                    "\x1b[38;2;{};{};{}m{}\x1b[0m",
                    rgb.red(),
                    rgb.green(),
                    rgb.blue(),
                    content
                )
            }
            crate::discovery::detection::ColorDepth::Enhanced => {
                // 256-color palette: ESC[38;5;<n>m
                // Convert RGB to nearest 256-color index (simplified approach)
                let r = rgb.red() as f32;
                let g = rgb.green() as f32;
                let b = rgb.blue() as f32;
                // Simple 6x6x6 color cube approximation
                let color_idx = ((r / 256.0 * 36.0).floor() as u8)
                    + ((g / 256.0 * 6.0).floor() as u8)
                    + ((b / 256.0 * 1.0).floor() as u8)
                    + 16;
                format!("\x1b[38;5;{}m{}\x1b[0m", color_idx, content)
            }
            _ => {
                // Fallback to basic color
                format!("\x1b[{}m{}\x1b[0m", color_code(rgb.fallback()), content)
            }
        }
    }
}


/// Wrapper for web colors that implements `RenderableWrapper`.
#[derive(Debug, Clone, Copy)]
pub struct WebColorWrapper(pub WebColor);

impl RenderableWrapper for WebColorWrapper {
    fn render<T: Into<String>>(self, content: T) -> String {
        let content = content.into();
        let rgb = WEB_COLOR_LOOKUP
            .get(&self.0)
            .copied()
            .unwrap_or(RgbColor::new(128, 128, 128, BasicColor::White));
        format!(
            "\x1b[38;2;{};{};{}m{}\x1b[0m",
            rgb.red(),
            rgb.green(),
            rgb.blue(),
            content
        )
    }

    fn fallback_render<T: Into<String>>(self, content: T, term: &Terminal) -> String {
        let content = content.into();
        let rgb = WEB_COLOR_LOOKUP
            .get(&self.0)
            .copied()
            .unwrap_or(RgbColor::new(128, 128, 128, BasicColor::White));
        match term.color_depth {
            crate::discovery::detection::ColorDepth::TrueColor => {
                format!(
                    "\x1b[38;2;{};{};{}m{}\x1b[0m",
                    rgb.red(),
                    rgb.green(),
                    rgb.blue(),
                    content
                )
            }
            crate::discovery::detection::ColorDepth::Enhanced => {
                let r = rgb.red() as f32;
                let g = rgb.green() as f32;
                let b = rgb.blue() as f32;
                // Simple 6x6x6 color cube approximation
                let color_idx = ((r / 256.0 * 36.0).floor() as u8)
                    + ((g / 256.0 * 6.0).floor() as u8)
                    + ((b / 256.0 * 1.0).floor() as u8)
                    + 16;
                format!("\x1b[38;5;{}m{}\x1b[0m", color_idx, content)
            }
            _ => {
                format!("\x1b[{}m{}\x1b[0m", color_code(rgb.fallback()), content)
            }
        }
    }
}

/// Wrapper for Tailwind colors that implements `RenderableWrapper`.
///
/// Uses `Tailwind::to_hdr_color()` to get the underlying HDR color values
/// for rendering. Special values (Inherit, Current, Transparent) return
/// the content unchanged since they have no meaningful terminal representation.
#[derive(Debug, Clone, Copy)]
pub struct TailwindColorWrapper(pub Tailwind);

impl RenderableWrapper for TailwindColorWrapper {
    fn render<T: Into<String>>(self, content: T) -> String {
        let content = content.into();

        // Get the HDR color; special values (Inherit/Current/Transparent) return None
        match self.0.to_hdr_color() {
            Some(hdr) => {
                // Render as 24-bit truecolor
                format!(
                    "\x1b[38;2;{};{};{}m{}\x1b[0m",
                    hdr.red(),
                    hdr.green(),
                    hdr.blue(),
                    content
                )
            }
            None => {
                // Special values: return content unchanged
                content
            }
        }
    }

    fn fallback_render<T: Into<String>>(self, content: T, term: &Terminal) -> String {
        let content = content.into();

        // Get the HDR color; special values (Inherit/Current/Transparent) return None
        match self.0.to_hdr_color() {
            Some(hdr) => {
                match term.color_depth {
                    crate::discovery::detection::ColorDepth::TrueColor => {
                        format!(
                            "\x1b[38;2;{};{};{}m{}\x1b[0m",
                            hdr.red(),
                            hdr.green(),
                            hdr.blue(),
                            content
                        )
                    }
                    crate::discovery::detection::ColorDepth::Enhanced => {
                        // 256-color palette: ESC[38;5;<n>m
                        // Convert RGB to nearest 256-color index using 6x6x6 color cube
                        let r = hdr.red() as f32;
                        let g = hdr.green() as f32;
                        let b = hdr.blue() as f32;
                        let color_idx = ((r / 256.0 * 36.0).floor() as u8)
                            + ((g / 256.0 * 6.0).floor() as u8)
                            + ((b / 256.0 * 1.0).floor() as u8)
                            + 16;
                        format!("\x1b[38;5;{}m{}\x1b[0m", color_idx, content)
                    }
                    _ => {
                        // Fallback to basic ANSI color
                        format!("\x1b[{}m{}\x1b[0m", color_code(hdr.fallback()), content)
                    }
                }
            }
            None => {
                // Special values: return content unchanged
                content
            }
        }
    }
}


/// Helper function to convert BasicColor to ANSI color code
fn color_code(color: BasicColor) -> u8 {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that special Tailwind values return None
    #[test]
    fn tailwind_special_values_return_none() {
        assert!(Tailwind::Inherit.to_hdr_color().is_none());
        assert!(Tailwind::Current.to_hdr_color().is_none());
        assert!(Tailwind::Transparent.to_hdr_color().is_none());
    }

    /// Test that special Tailwind values return correct CSS values
    #[test]
    fn tailwind_special_css_vars() {
        assert_eq!(Tailwind::Inherit.css_var(), "inherit");
        assert_eq!(Tailwind::Current.css_var(), "currentColor");
        assert_eq!(Tailwind::Transparent.css_var(), "transparent");
    }

    /// Test that special Tailwind values return None for hex
    #[test]
    fn tailwind_special_hex_values() {
        assert!(Tailwind::Inherit.hex().is_none());
        assert!(Tailwind::Current.hex().is_none());
        assert!(Tailwind::Transparent.hex().is_none());
    }

    /// Test black and white basic values
    #[test]
    fn tailwind_black_white() {
        let black = Tailwind::Black.to_hdr_color().unwrap();
        assert_eq!(black.red(), 0);
        assert_eq!(black.green(), 0);
        assert_eq!(black.blue(), 0);

        let white = Tailwind::White.to_hdr_color().unwrap();
        assert_eq!(white.red(), 255);
        assert_eq!(white.green(), 255);
        assert_eq!(white.blue(), 255);
    }

    /// Test that black/white hex values are correct
    #[test]
    fn tailwind_black_white_hex() {
        assert_eq!(Tailwind::Black.hex(), Some("#000000"));
        assert_eq!(Tailwind::White.hex(), Some("#ffffff"));
    }

    /// Test sample reference colors against Tailwind v4 official values.
    /// Official hex values from tailwindcss.com/docs/colors:
    /// - slate-50: #f8fafc
    /// - slate-500: #64748b
    /// - slate-950: #020617
    /// - red-500: #ef4444
    /// - blue-500: #3b82f6
    /// - indigo-500: #6366f1
    #[test]
    fn tailwind_reference_color_accuracy() {
        // Slate family
        let slate_50 = Tailwind::Slate50.to_hdr_color().unwrap();
        assert_eq!(slate_50.red(), 248);
        assert_eq!(slate_50.green(), 250);
        assert_eq!(slate_50.blue(), 252);
        assert_eq!(Tailwind::Slate50.hex(), Some("#f8fafc"));

        let slate_500 = Tailwind::Slate500.to_hdr_color().unwrap();
        // Tailwind v4 uses OKLCH, so exact hex may differ slightly
        // The official slate-500 from Tailwind v4 is oklch(0.554 0.046 257.417)
        // Our conversion should be very close to #62748e (the sRGB fallback)
        assert!((slate_500.red() as i16 - 100).abs() < 5, "slate-500 red should be ~100");
        assert!((slate_500.green() as i16 - 116).abs() < 5, "slate-500 green should be ~116");
        assert!((slate_500.blue() as i16 - 139).abs() < 5, "slate-500 blue should be ~139");

        let slate_950 = Tailwind::Slate950.to_hdr_color().unwrap();
        assert!((slate_950.red() as i16).abs() < 10, "slate-950 red should be ~2");
        assert!((slate_950.green() as i16 - 6).abs() < 10, "slate-950 green should be ~6");
        assert!((slate_950.blue() as i16 - 23).abs() < 10, "slate-950 blue should be ~23");

        // Red-500: oklch(0.637 0.237 25.331) -> approximately #ef4444
        let red_500 = Tailwind::Red500.to_hdr_color().unwrap();
        assert!(red_500.red() > 220, "red-500 should have high red channel");
        assert!(red_500.green() < 100, "red-500 should have low green channel");
        assert!(red_500.blue() < 100, "red-500 should have low blue channel");

        // Blue-500: oklch(0.623 0.214 259.815) -> approximately #3b82f6
        let blue_500 = Tailwind::Blue500.to_hdr_color().unwrap();
        assert!(blue_500.red() < 100, "blue-500 should have low red channel");
        assert!(blue_500.green() > 100 && blue_500.green() < 150, "blue-500 green should be ~130");
        assert!(blue_500.blue() > 230, "blue-500 should have high blue channel");

        // Indigo-500: oklch(0.585 0.233 277.117) -> approximately #6366f1
        let indigo_500 = Tailwind::Indigo500.to_hdr_color().unwrap();
        assert!(indigo_500.red() > 80 && indigo_500.red() < 120, "indigo-500 red should be ~99");
        assert!(indigo_500.green() > 80 && indigo_500.green() < 130, "indigo-500 green should be ~102");
        assert!(indigo_500.blue() > 200, "indigo-500 should have high blue channel");
    }

    /// Test that all concrete color variants return Some for to_hdr_color
    #[test]
    fn all_concrete_colors_return_some() {
        // Test a sample from each family
        let families = [
            Tailwind::Slate500, Tailwind::Gray500, Tailwind::Zinc500,
            Tailwind::Neutral500, Tailwind::Stone500, Tailwind::Red500,
            Tailwind::Orange500, Tailwind::Amber500, Tailwind::Yellow500,
            Tailwind::Lime500, Tailwind::Green500, Tailwind::Emerald500,
            Tailwind::Teal500, Tailwind::Cyan500, Tailwind::Sky500,
            Tailwind::Blue500, Tailwind::Indigo500, Tailwind::Violet500,
            Tailwind::Purple500, Tailwind::Fuchsia500, Tailwind::Pink500,
            Tailwind::Rose500,
        ];

        for color in families {
            assert!(
                color.to_hdr_color().is_some(),
                "{:?} should return Some for to_hdr_color",
                color
            );
            assert!(
                color.hex().is_some(),
                "{:?} should return Some for hex",
                color
            );
            assert!(
                !color.css_var().is_empty(),
                "{:?} should have non-empty css_var",
                color
            );
        }
    }

    /// Test OKLCH values are stored correctly
    #[test]
    fn oklch_values_preserved() {
        let slate_50 = Tailwind::Slate50.to_hdr_color().unwrap();
        let (l, c, h) = slate_50.oklch();
        assert!((l - 0.984).abs() < 0.001, "Lightness should be ~0.984");
        assert!((c - 0.003).abs() < 0.001, "Chroma should be ~0.003");
        assert!((h - 247.858).abs() < 0.1, "Hue should be ~247.858");

        let red_500 = Tailwind::Red500.to_hdr_color().unwrap();
        let (l, c, h) = red_500.oklch();
        assert!((l - 0.637).abs() < 0.001, "Red-500 L should be ~0.637");
        assert!((c - 0.237).abs() < 0.001, "Red-500 C should be ~0.237");
        assert!((h - 25.331).abs() < 0.1, "Red-500 H should be ~25.331");
    }

    /// Test CSS variable names follow Tailwind convention
    #[test]
    fn css_var_names_follow_convention() {
        assert_eq!(Tailwind::Black.css_var(), "--color-black");
        assert_eq!(Tailwind::White.css_var(), "--color-white");
        assert_eq!(Tailwind::Slate50.css_var(), "--color-slate-50");
        assert_eq!(Tailwind::Slate500.css_var(), "--color-slate-500");
        assert_eq!(Tailwind::Red500.css_var(), "--color-red-500");
        assert_eq!(Tailwind::Blue500.css_var(), "--color-blue-500");
    }

    /// Test hex values are properly formatted
    #[test]
    fn hex_format_is_valid() {
        let hex = Tailwind::Slate500.hex().unwrap();
        assert!(hex.starts_with('#'), "Hex should start with #");
        assert_eq!(hex.len(), 7, "Hex should be 7 characters (#rrggbb)");
        assert!(hex[1..].chars().all(|c| c.is_ascii_hexdigit()), "Hex should contain only hex digits");
    }

    /// Test that neutral grays are truly achromatic (no color cast)
    #[test]
    fn neutral_grays_are_achromatic() {
        for shade in [
            Tailwind::Neutral50, Tailwind::Neutral100, Tailwind::Neutral200,
            Tailwind::Neutral300, Tailwind::Neutral400, Tailwind::Neutral500,
            Tailwind::Neutral600, Tailwind::Neutral700, Tailwind::Neutral800,
            Tailwind::Neutral900, Tailwind::Neutral950,
        ] {
            let color = shade.to_hdr_color().unwrap();
            let r = color.red() as i16;
            let g = color.green() as i16;
            let b = color.blue() as i16;

            // For truly neutral colors, R, G, and B should be identical
            // Allow small rounding tolerance
            assert!(
                (r - g).abs() <= 1 && (g - b).abs() <= 1,
                "{:?} should be achromatic: RGB({}, {}, {})",
                shade, r, g, b
            );
        }
    }

    /// Test fallback colors are appropriate for the shade
    #[test]
    fn fallback_colors_appropriate() {
        // Light colors (50, 100) should fall back to bright white
        let slate_50 = Tailwind::Slate50.to_hdr_color().unwrap();
        assert_eq!(slate_50.fallback(), BasicColor::BrightWhite);

        // Mid-light colors (200, 300) should fall back to white
        let slate_200 = Tailwind::Slate200.to_hdr_color().unwrap();
        assert_eq!(slate_200.fallback(), BasicColor::White);

        // Mid colors (400-700) should fall back to bright black
        let slate_500 = Tailwind::Slate500.to_hdr_color().unwrap();
        assert_eq!(slate_500.fallback(), BasicColor::BrightBlack);

        // Dark colors (800-950) should fall back to black
        let slate_950 = Tailwind::Slate950.to_hdr_color().unwrap();
        assert_eq!(slate_950.fallback(), BasicColor::Black);

        // Colorful 500s should fall back to their basic color
        let red_500 = Tailwind::Red500.to_hdr_color().unwrap();
        assert_eq!(red_500.fallback(), BasicColor::Red);

        let blue_500 = Tailwind::Blue500.to_hdr_color().unwrap();
        assert_eq!(blue_500.fallback(), BasicColor::Blue);

        let green_500 = Tailwind::Green500.to_hdr_color().unwrap();
        assert_eq!(green_500.fallback(), BasicColor::Green);
    }

    // ==========================================================================
    // TailwindColorWrapper tests
    // ==========================================================================

    /// Test that TailwindColorWrapper::render produces correct ANSI escape sequence
    #[test]
    fn tailwind_wrapper_render_produces_correct_ansi() {
        let wrapper = TailwindColorWrapper(Tailwind::Indigo500);
        let output = wrapper.render("test");

        // Should start with ESC[38;2; for 24-bit color
        assert!(output.starts_with("\x1b[38;2;"));
        // Should end with ESC[0m (reset)
        assert!(output.ends_with("\x1b[0m"));
        // Should contain the content
        assert!(output.contains("test"));
    }

    /// Test that render output format matches expected pattern for known color
    #[test]
    fn tailwind_wrapper_render_format_for_known_color() {
        let wrapper = TailwindColorWrapper(Tailwind::Black);
        let output = wrapper.render("hello");

        // Black is RGB(0,0,0)
        assert_eq!(output, "\x1b[38;2;0;0;0mhello\x1b[0m");

        let wrapper = TailwindColorWrapper(Tailwind::White);
        let output = wrapper.render("world");

        // White is RGB(255,255,255)
        assert_eq!(output, "\x1b[38;2;255;255;255mworld\x1b[0m");
    }

    /// Test that special values return content unchanged
    #[test]
    fn tailwind_wrapper_special_values_return_unchanged() {
        let inherit_wrapper = TailwindColorWrapper(Tailwind::Inherit);
        assert_eq!(inherit_wrapper.render("content"), "content");

        let current_wrapper = TailwindColorWrapper(Tailwind::Current);
        assert_eq!(current_wrapper.render("content"), "content");

        let transparent_wrapper = TailwindColorWrapper(Tailwind::Transparent);
        assert_eq!(transparent_wrapper.render("content"), "content");
    }

    /// Test that special values don't panic in fallback_render
    #[test]
    fn tailwind_wrapper_special_values_fallback_no_panic() {
        let term = Terminal::new();

        let inherit_wrapper = TailwindColorWrapper(Tailwind::Inherit);
        let result = inherit_wrapper.fallback_render("test", &term);
        assert_eq!(result, "test");

        let current_wrapper = TailwindColorWrapper(Tailwind::Current);
        let result = current_wrapper.fallback_render("test", &term);
        assert_eq!(result, "test");

        let transparent_wrapper = TailwindColorWrapper(Tailwind::Transparent);
        let result = transparent_wrapper.fallback_render("test", &term);
        assert_eq!(result, "test");
    }

    /// Test fallback rendering respects TrueColor depth
    #[test]
    fn tailwind_wrapper_fallback_truecolor() {
        use crate::discovery::detection::ColorDepth;

        let mut term = Terminal::new();
        term.color_depth = ColorDepth::TrueColor;

        let wrapper = TailwindColorWrapper(Tailwind::Red500);
        let output = wrapper.fallback_render("error", &term);

        // Should use 24-bit truecolor format
        assert!(output.starts_with("\x1b[38;2;"));
        assert!(output.ends_with("\x1b[0m"));
        assert!(output.contains("error"));
    }

    /// Test fallback rendering respects Enhanced (256-color) depth
    #[test]
    fn tailwind_wrapper_fallback_enhanced() {
        use crate::discovery::detection::ColorDepth;

        let mut term = Terminal::new();
        term.color_depth = ColorDepth::Enhanced;

        let wrapper = TailwindColorWrapper(Tailwind::Blue500);
        let output = wrapper.fallback_render("info", &term);

        // Should use 256-color format: ESC[38;5;<n>m
        assert!(output.starts_with("\x1b[38;5;"));
        assert!(output.ends_with("\x1b[0m"));
        assert!(output.contains("info"));
    }

    /// Test fallback rendering uses basic ANSI for minimal color depth
    #[test]
    fn tailwind_wrapper_fallback_basic() {
        use crate::discovery::detection::ColorDepth;

        let mut term = Terminal::new();
        term.color_depth = ColorDepth::Basic;

        let wrapper = TailwindColorWrapper(Tailwind::Red500);
        let output = wrapper.fallback_render("warning", &term);

        // Should use basic ANSI format: ESC[<n>m where n is 30-37 or 90-97
        // Red500 fallback is BasicColor::Red which is code 31
        assert_eq!(output, "\x1b[31mwarning\x1b[0m");
    }

    /// Test fallback rendering uses basic ANSI for no color support
    #[test]
    fn tailwind_wrapper_fallback_none() {
        use crate::discovery::detection::ColorDepth;

        let mut term = Terminal::new();
        term.color_depth = ColorDepth::None;

        let wrapper = TailwindColorWrapper(Tailwind::Green500);
        let output = wrapper.fallback_render("success", &term);

        // Should still output basic ANSI (the terminal just won't render it)
        // Green500 fallback is BasicColor::Green which is code 32
        assert_eq!(output, "\x1b[32msuccess\x1b[0m");
    }

    /// Test wrapper can render various Tailwind colors without panic
    #[test]
    fn tailwind_wrapper_renders_all_families() {
        let colors = [
            Tailwind::Slate500,
            Tailwind::Gray500,
            Tailwind::Zinc500,
            Tailwind::Neutral500,
            Tailwind::Stone500,
            Tailwind::Red500,
            Tailwind::Orange500,
            Tailwind::Amber500,
            Tailwind::Yellow500,
            Tailwind::Lime500,
            Tailwind::Green500,
            Tailwind::Emerald500,
            Tailwind::Teal500,
            Tailwind::Cyan500,
            Tailwind::Sky500,
            Tailwind::Blue500,
            Tailwind::Indigo500,
            Tailwind::Violet500,
            Tailwind::Purple500,
            Tailwind::Fuchsia500,
            Tailwind::Pink500,
            Tailwind::Rose500,
        ];

        for color in colors {
            let wrapper = TailwindColorWrapper(color);
            let output = wrapper.render("test");

            assert!(
                output.starts_with("\x1b[38;2;"),
                "{:?} should produce truecolor escape",
                color
            );
            assert!(
                output.ends_with("\x1b[0m"),
                "{:?} should end with reset",
                color
            );
        }
    }

    /// Test wrapper with empty content
    #[test]
    fn tailwind_wrapper_empty_content() {
        let wrapper = TailwindColorWrapper(Tailwind::Indigo500);
        let output = wrapper.render("");

        // Should still have escape codes even with empty content
        assert!(output.starts_with("\x1b[38;2;"));
        assert!(output.ends_with("\x1b[0m"));
    }

    /// Test wrapper with content containing escape sequences
    #[test]
    fn tailwind_wrapper_content_with_escapes() {
        let wrapper = TailwindColorWrapper(Tailwind::Red500);
        let input = "already \x1b[1mbold\x1b[0m text";
        let output = wrapper.render(input);

        // Should wrap the content with color escapes
        assert!(output.starts_with("\x1b[38;2;"));
        assert!(output.ends_with("\x1b[0m"));
        assert!(output.contains("already \x1b[1mbold\x1b[0m text"));
    }
}
