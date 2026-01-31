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

impl Tailwind {
    /// Returns the Tailwind v4 theme variable name (without `var(...)`).
    pub const fn css_var_name(self) -> &'static str {
        use Tailwind::*;
        match self {
            Inherit => "inherit",
            Current => "currentColor",
            Transparent => "transparent",
            Black => "--color-black",
            White => "--color-white",

            // The rest are generated by build.rs into `tailwind_color_hex.rs`,
            // but we also want a stable var-name mapping for all variants.
            _ => {
                // NOTE: We override this via generated code too if you want.
                // This placeholder keeps this function total; the generated impl below
                // is the one you will actually use for var names and hex.
                "UNSUPPORTED"
            }
        }
    }
}

// build.rs will generate these:
//
// impl TailwindColor {
//   pub const fn var(self) -> &'static str { ... "var(--color-slate-50)" ... }
//   pub const fn hex(self) -> &'static str { ... "#f8fafc" ... } // nearest sRGB hex
// }

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
