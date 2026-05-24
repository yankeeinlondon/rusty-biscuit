use super::{BasicColor, Octet};

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
