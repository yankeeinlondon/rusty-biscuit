//! sRGB colors and the WCAG 2 contrast arithmetic the palettes are built on.

use std::fmt;

/// An opaque sRGB color, 8 bits per channel.
///
/// ## Examples
///
/// ```rust
/// use biscuit_visualized::mermaid::Rgb;
///
/// let ink = Rgb::from_hex("#1a1b26").unwrap();
/// assert_eq!(ink, Rgb::new(0x1a, 0x1b, 0x26));
/// assert_eq!(ink.to_string(), "#1a1b26");
/// assert!(Rgb::WHITE.contrast(Rgb::BLACK) > 20.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const BLACK: Rgb = Rgb::new(0, 0, 0);
    pub const WHITE: Rgb = Rgb::new(255, 255, 255);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parses `#rgb` or `#rrggbb` (the `#` is optional); `None` for anything
    /// else.
    pub fn from_hex(hex: &str) -> Option<Self> {
        let digits = hex.trim().trim_start_matches('#');
        let channel = |text: &str| u8::from_str_radix(text, 16).ok();
        match digits.len() {
            3 if digits.is_ascii() => {
                let expand = |i: usize| channel(&digits[i..=i]).map(|value| value * 17);
                Some(Self::new(expand(0)?, expand(1)?, expand(2)?))
            }
            6 if digits.is_ascii() => Some(Self::new(
                channel(&digits[0..2])?,
                channel(&digits[2..4])?,
                channel(&digits[4..6])?,
            )),
            _ => None,
        }
    }

    /// WCAG 2 relative luminance, 0 (black) to 1 (white).
    pub fn luminance(self) -> f64 {
        let linear = |channel: u8| {
            let value = f64::from(channel) / 255.0;
            if value <= 0.040_45 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(self.r) + 0.7152 * linear(self.g) + 0.0722 * linear(self.b)
    }

    /// WCAG 2 contrast ratio against `other`, 1 to 21; symmetric.
    pub fn contrast(self, other: Rgb) -> f64 {
        let (a, b) = (self.luminance(), other.luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// Whether this color reads as a dark background (closer to black than to
    /// white by contrast).
    pub fn is_dark(self) -> bool {
        self.contrast(Rgb::WHITE) > self.contrast(Rgb::BLACK)
    }

    /// `self` laid over `under` at `amount` (0 = `under`, 1 = `self`), mixed
    /// per channel in sRGB.
    pub fn over(self, under: Rgb, amount: f64) -> Rgb {
        let amount = amount.clamp(0.0, 1.0);
        let mix = |top: u8, bottom: u8| {
            (f64::from(top) * amount + f64::from(bottom) * (1.0 - amount)).round() as u8
        };
        Rgb::new(mix(self.r, under.r), mix(self.g, under.g), mix(self.b, under.b))
    }

    /// Hue (degrees), saturation, and lightness (both 0–1).
    pub(crate) fn to_hsl(self) -> (f64, f64, f64) {
        let [r, g, b] = [self.r, self.g, self.b].map(|channel| f64::from(channel) / 255.0);
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let lightness = (max + min) / 2.0;
        let delta = max - min;
        if delta == 0.0 {
            return (0.0, 0.0, lightness);
        }
        let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs());
        let hue = if max == r {
            60.0 * (((g - b) / delta).rem_euclid(6.0))
        } else if max == g {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };
        (hue, saturation, lightness)
    }

    pub(crate) fn from_hsl(hue: f64, saturation: f64, lightness: f64) -> Rgb {
        let saturation = saturation.clamp(0.0, 1.0);
        let lightness = lightness.clamp(0.0, 1.0);
        let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
        let sector = hue.rem_euclid(360.0) / 60.0;
        let x = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
        let (r, g, b) = match sector as u32 {
            0 => (chroma, x, 0.0),
            1 => (x, chroma, 0.0),
            2 => (0.0, chroma, x),
            3 => (0.0, x, chroma),
            4 => (x, 0.0, chroma),
            _ => (chroma, 0.0, x),
        };
        let offset = lightness - chroma / 2.0;
        let channel = |value: f64| ((value + offset) * 255.0).round().clamp(0.0, 255.0) as u8;
        Rgb::new(channel(r), channel(g), channel(b))
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// WCAG AA for text (D45: every text element, whatever its size).
pub(crate) const TEXT_CONTRAST: f64 = 4.5;

/// WCAG AA for graphical objects (strokes and markers against the background).
pub(crate) const GRAPHIC_CONTRAST: f64 = 3.0;

/// The candidate with the highest contrast against `fill`.
pub(crate) fn most_readable(fill: Rgb, candidates: &[Rgb]) -> Rgb {
    candidates
        .iter()
        .copied()
        .max_by(|a, b| a.contrast(fill).total_cmp(&b.contrast(fill)))
        .unwrap_or(Rgb::BLACK)
}

/// `color` with its lightness moved away from `background`, in small steps,
/// until it reaches `ratio` against it (or black/white, whichever is
/// reached first). Hue and saturation are kept.
pub(crate) fn with_contrast(color: Rgb, background: Rgb, ratio: f64) -> Rgb {
    if color.contrast(background) >= ratio {
        return color;
    }
    let (hue, saturation, mut lightness) = color.to_hsl();
    let step = if background.is_dark() { 0.01 } else { -0.01 };
    let mut adjusted = color;
    for _ in 0..100 {
        lightness = (lightness + step).clamp(0.0, 1.0);
        adjusted = Rgb::from_hsl(hue, saturation, lightness);
        if adjusted.contrast(background) >= ratio || lightness == 0.0 || lightness == 1.0 {
            break;
        }
    }
    adjusted
}

/// `accent` over `background` at the largest strength up to `amount` whose
/// result keeps `text` at [`TEXT_CONTRAST`]; `background` itself when none
/// does (the caller chose `text` to pass on `background`).
pub(crate) fn readable_tint(accent: Rgb, background: Rgb, text: Rgb, amount: f64) -> Rgb {
    let mut strength = amount;
    while strength > 0.0 {
        let tint = accent.over(background, strength);
        if text.contrast(tint) >= TEXT_CONTRAST {
            return tint;
        }
        strength -= 0.02;
    }
    background
}
