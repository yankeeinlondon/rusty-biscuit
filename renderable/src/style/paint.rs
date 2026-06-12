//! Alpha-bearing paint color for the render tree.
//!
//! [`PaintColor`] pairs a [`Color`] with an [`Opacity`] alpha channel so the
//! render tree can carry the full paint a target needs without a side channel.
//! Terminal targets read [`PaintColor::color`] and ignore the alpha; browser
//! targets lower the pair to an `rgb()` / `rgba()` (or keyword) CSS color.

use serde::{Deserialize, Serialize};

use crate::color::Color;

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
}
