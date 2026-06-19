//! Alpha-bearing paint color for the render tree.
//!
//! [`PaintColor`] pairs a [`Color`] with an [`Opacity`] alpha channel so the
//! render tree can carry the full paint a target needs without a side channel.
//! Terminal targets read [`PaintColor::color`] and ignore the alpha; browser
//! targets lower the pair to an `rgb()` / `rgba()` (or keyword) CSS color.

use serde::{Deserialize, Serialize};

use crate::color::{BasicColor, Color, RgbColor, Tailwind};

/// Error parsing a CSS color string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseColorError {
    /// The input that could not be parsed.
    pub input: String,
    /// A concise reason for the failure.
    pub reason: String,
}

impl std::fmt::Display for ParseColorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid color '{}': {}",
            self.input, self.reason
        )
    }
}

impl std::error::Error for ParseColorError {}

impl ParseColorError {
    fn new(input: &str, reason: impl Into<String>) -> Self {
        Self {
            input: input.to_string(),
            reason: reason.into(),
        }
    }
}

/// An 8-bit alpha channel, stored as a raw `0..=255` byte.
///
/// [`OPAQUE`](Self::OPAQUE) is the default so an alpha-less color paints
/// solid, matching the pre-alpha render-tree behavior. Construction from a
/// `0..=100` percentage rounds to the nearest byte via
/// [`from_percent`](Self::from_percent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Opacity(pub u8);

impl Default for Opacity {
    fn default() -> Self {
        Self::OPAQUE
    }
}

impl Opacity {
    /// Fully transparent (`alpha == 0`).
    pub const TRANSPARENT: Opacity = Opacity(0);
    /// Fully opaque (`alpha == 255`).
    pub const OPAQUE: Opacity = Opacity(255);

    /// Exact 8-bit construction from a raw alpha byte.
    pub const fn new(alpha: u8) -> Opacity {
        Opacity(alpha)
    }

    /// The raw `0..=255` alpha byte.
    pub const fn alpha(self) -> u8 {
        self.0
    }

    /// `true` when this alpha is fully opaque.
    pub const fn is_opaque(&self) -> bool {
        self.0 == Self::OPAQUE.0
    }

    /// Checked construction from a `0..=100` percentage.
    ///
    /// Maps the percentage to a byte with rounding via `(pct * 255 + 50) /
    /// 100`, so `50` becomes `128` and `100` becomes `255`.
    ///
    /// ## Returns
    ///
    /// `None` when `pct` exceeds `100`.
    pub fn from_percent(pct: u8) -> Option<Opacity> {
        if pct > 100 {
            return None;
        }
        // `pct <= 100`, so `pct * 255 + 50 <= 25550` — well within `u16`.
        let scaled = (u16::from(pct) * 255 + 50) / 100;
        Some(Opacity(scaled as u8))
    }

    /// The alpha normalized to the CSS `0.0..=1.0` range.
    pub fn as_css_alpha(self) -> f32 {
        f32::from(self.0) / 255.0
    }
}

/// A paint color: a [`Color`] plus an [`Opacity`] alpha channel.
///
/// The alpha defaults to [`Opacity::OPAQUE`] and is elided from the serialized
/// form when opaque, so a render tree that never sets alpha serializes exactly
/// as it did before alpha existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaintColor {
    /// The underlying color.
    pub color: Color,
    /// The alpha channel. Opaque is elided on serialization and assumed on
    /// deserialization when the field is absent.
    #[serde(default, skip_serializing_if = "Opacity::is_opaque")]
    pub opacity: Opacity,
}

impl PaintColor {
    /// An opaque paint of `color`.
    pub const fn new(color: Color) -> PaintColor {
        PaintColor {
            color,
            opacity: Opacity::OPAQUE,
        }
    }

    /// Returns this paint with `opacity` applied.
    #[must_use]
    pub fn with_opacity(mut self, opacity: Opacity) -> PaintColor {
        self.opacity = opacity;
        self
    }

    /// Parses a CSS color string into a [`PaintColor`].
    ///
    /// Accepts:
    /// - `#RGB` / `#RRGGBB` hex strings (e.g. `#1e1e23`).
    /// - Comma-separated `R,G,B` triples in the 0-255 range (e.g. `30,30,35`).
    /// - Tailwind palette names (e.g. `red-500`, `slate-50`).
    /// - CSS special keywords: `transparent`, `currentColor`, `inherit`.
    pub fn from_css_str(s: &str) -> Result<PaintColor, ParseColorError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(ParseColorError::new(s, "color must not be empty"));
        }

        let lower = trimmed.to_ascii_lowercase();
        match lower.as_str() {
            "transparent" => {
                return Ok(PaintColor::new(Color::Tailwind(Tailwind::Transparent)));
            }
            "currentcolor" => {
                return Ok(PaintColor::new(Color::Tailwind(Tailwind::Current)));
            }
            "inherit" => {
                return Ok(PaintColor::new(Color::Tailwind(Tailwind::Inherit)));
            }
            _ => {}
        }

        if let Some(hex) = trimmed.strip_prefix('#') {
            return parse_hex_color(hex).map_err(|reason| ParseColorError::new(s, reason));
        }

        if trimmed.contains(',') {
            return parse_rgb_triple(trimmed).map_err(|reason| ParseColorError::new(s, reason));
        }

        if let Some(tw) = Tailwind::from_kebab_name(&lower) {
            return Ok(PaintColor::new(Color::Tailwind(tw)));
        }

        Err(ParseColorError::new(
            s,
            "expected #RGB / #RRGGBB hex, R,G,B triple, Tailwind name (e.g. red-500), \
             or one of transparent / currentColor / inherit",
        ))
    }
}

fn parse_hex_color(hex: &str) -> Result<PaintColor, String> {
    let bytes = hex.as_bytes();

    let nibble = |c: u8| -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        }
    };
    let pair = |chunk: &[u8]| -> Result<u8, String> {
        if chunk.len() != 2 {
            return Err(format!(
                "hex component must be 2 chars, got '{}'",
                std::str::from_utf8(chunk).unwrap_or("")
            ));
        }
        let hi = nibble(chunk[0])
            .ok_or_else(|| format!("invalid hex char '{}'", chunk[0] as char))?;
        let lo = nibble(chunk[1])
            .ok_or_else(|| format!("invalid hex char '{}'", chunk[1] as char))?;
        Ok((hi << 4) | lo)
    };

    let (r, g, b) = match bytes.len() {
        3 => {
            let r = nibble(bytes[0])
                .ok_or_else(|| format!("invalid hex char '{}'", bytes[0] as char))?;
            let g = nibble(bytes[1])
                .ok_or_else(|| format!("invalid hex char '{}'", bytes[1] as char))?;
            let b = nibble(bytes[2])
                .ok_or_else(|| format!("invalid hex char '{}'", bytes[2] as char))?;
            ((r << 4) | r, (g << 4) | g, (b << 4) | b)
        }
        6 => (pair(&bytes[0..2])?, pair(&bytes[2..4])?, pair(&bytes[4..6])?),
        _ => {
            return Err(format!(
                "hex color must be #RGB or #RRGGBB, got '#{hex}'"
            ));
        }
    };
    Ok(PaintColor::new(Color::Rgb(RgbColor::new(
        r, g, b, BasicColor::Black,
    ))))
}

fn parse_rgb_triple(s: &str) -> Result<PaintColor, String> {
    let parts: Vec<&str> = s.split(',').map(str::trim).collect();
    if parts.len() != 3 {
        return Err(format!(
            "R,G,B triple must have exactly three comma-separated values, got '{s}'"
        ));
    }
    let parse_component = |p: &str, name: &str| -> Result<u8, String> {
        let n: u16 = p
            .parse()
            .map_err(|_| format!("{name} component '{p}' is not a valid 0-255 integer"))?;
        if n > 255 {
            return Err(format!("{name} component '{p}' is out of range (0-255)"));
        }
        Ok(n as u8)
    };
    let r = parse_component(parts[0], "red")?;
    let g = parse_component(parts[1], "green")?;
    let b = parse_component(parts[2], "blue")?;
    Ok(PaintColor::new(Color::Rgb(RgbColor::new(
        r, g, b, BasicColor::Black,
    ))))
}

/// An opaque [`PaintColor`] wrapping `color`.
impl From<Color> for PaintColor {
    fn from(color: Color) -> PaintColor {
        PaintColor::new(color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{BasicColor, RgbColor, Tailwind};

    fn red() -> Color {
        Color::Rgb(RgbColor::new(255, 0, 0, BasicColor::Red))
    }

    #[test]
    fn opacity_default_is_opaque() {
        assert_eq!(Opacity::default(), Opacity::OPAQUE);
        assert_eq!(Opacity::default().alpha(), 255);
    }

    #[test]
    fn opacity_constants() {
        assert_eq!(Opacity::TRANSPARENT.alpha(), 0);
        assert_eq!(Opacity::OPAQUE.alpha(), 255);
        assert!(Opacity::OPAQUE.is_opaque());
        assert!(!Opacity::TRANSPARENT.is_opaque());
    }

    #[test]
    fn opacity_exact_construction() {
        assert_eq!(Opacity::new(128).alpha(), 128);
    }

    #[test]
    fn opacity_from_percent_rounds_to_nearest_byte() {
        assert_eq!(Opacity::from_percent(0), Some(Opacity(0)));
        assert_eq!(Opacity::from_percent(50), Some(Opacity(128)));
        assert_eq!(Opacity::from_percent(100), Some(Opacity(255)));
        // 75% → (75*255+50)/100 = 191.
        assert_eq!(Opacity::from_percent(75), Some(Opacity(191)));
    }

    #[test]
    fn opacity_from_percent_rejects_over_100() {
        assert_eq!(Opacity::from_percent(101), None);
        assert_eq!(Opacity::from_percent(255), None);
    }

    #[test]
    fn opacity_as_css_alpha_normalizes() {
        assert_eq!(Opacity::OPAQUE.as_css_alpha(), 1.0);
        assert_eq!(Opacity::TRANSPARENT.as_css_alpha(), 0.0);
        assert!((Opacity(128).as_css_alpha() - 0.501_96).abs() < 1e-4);
    }

    #[test]
    fn paint_color_from_color_is_opaque() {
        let paint = PaintColor::from(red());
        assert_eq!(paint.color, red());
        assert_eq!(paint.opacity, Opacity::OPAQUE);
    }

    #[test]
    fn paint_color_with_opacity() {
        let paint = PaintColor::new(red()).with_opacity(Opacity(128));
        assert_eq!(paint.opacity, Opacity(128));
    }

    #[test]
    fn paint_color_opaque_opacity_is_elided() {
        let paint = PaintColor::new(Color::Tailwind(Tailwind::Blue500));
        let value = serde_json::to_value(paint).unwrap();
        let object = value.as_object().unwrap();
        assert!(object.contains_key("color"));
        assert!(
            !object.contains_key("opacity"),
            "opaque opacity must be elided"
        );
    }

    #[test]
    fn paint_color_non_opaque_opacity_serializes() {
        let paint = PaintColor::new(red()).with_opacity(Opacity(128));
        let value = serde_json::to_value(paint).unwrap();
        assert_eq!(value["opacity"], serde_json::json!(128));
    }

    #[test]
    fn paint_color_missing_opacity_deserializes_as_opaque() {
        let json = r#"{ "color": { "Tailwind": "Blue500" } }"#;
        let paint: PaintColor = serde_json::from_str(json).unwrap();
        assert_eq!(paint.color, Color::Tailwind(Tailwind::Blue500));
        assert_eq!(paint.opacity, Opacity::OPAQUE);
    }

    #[test]
    fn paint_color_serde_roundtrip_with_alpha() {
        let paint = PaintColor::new(red()).with_opacity(Opacity::from_percent(50).unwrap());
        let json = serde_json::to_string(&paint).unwrap();
        let back: PaintColor = serde_json::from_str(&json).unwrap();
        assert_eq!(paint, back);
    }

    #[test]
    fn paint_color_from_css_hex_six() {
        let paint = PaintColor::from_css_str("#1e1e23").unwrap();
        assert_eq!(
            paint.color,
            Color::Rgb(RgbColor::new(0x1e, 0x1e, 0x23, BasicColor::Black))
        );
    }

    #[test]
    fn paint_color_from_css_hex_three() {
        let paint = PaintColor::from_css_str("#abc").unwrap();
        assert_eq!(
            paint.color,
            Color::Rgb(RgbColor::new(0xaa, 0xbb, 0xcc, BasicColor::Black))
        );
    }

    #[test]
    fn paint_color_from_css_rgb_triple() {
        let paint = PaintColor::from_css_str("30,30,35").unwrap();
        assert_eq!(
            paint.color,
            Color::Rgb(RgbColor::new(30, 30, 35, BasicColor::Black))
        );
    }

    #[test]
    fn paint_color_from_css_tailwind() {
        let paint = PaintColor::from_css_str("red-500").unwrap();
        assert_eq!(paint.color, Color::Tailwind(Tailwind::Red500));
    }

    #[test]
    fn paint_color_from_css_uppercase_tailwind() {
        let paint = PaintColor::from_css_str("RED-500").unwrap();
        assert_eq!(paint.color, Color::Tailwind(Tailwind::Red500));
    }

    #[test]
    fn paint_color_from_css_special_keywords() {
        assert_eq!(
            PaintColor::from_css_str("transparent").unwrap().color,
            Color::Tailwind(Tailwind::Transparent)
        );
        assert_eq!(
            PaintColor::from_css_str("currentColor").unwrap().color,
            Color::Tailwind(Tailwind::Current)
        );
        assert_eq!(
            PaintColor::from_css_str("inherit").unwrap().color,
            Color::Tailwind(Tailwind::Inherit)
        );
    }

    #[test]
    fn paint_color_from_css_rejects_invalid() {
        assert!(PaintColor::from_css_str("").is_err());
        assert!(PaintColor::from_css_str("not-a-color").is_err());
        assert!(PaintColor::from_css_str("#zzz").is_err());
        assert!(PaintColor::from_css_str("256,0,0").is_err());
        assert!(PaintColor::from_css_str("1,2").is_err());
        assert!(PaintColor::from_css_str("red").is_err());
        assert!(PaintColor::from_css_str("red-9999").is_err());
    }
}
