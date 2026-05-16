//! Target-agnostic color data types.
//!
//! This module is the single source of truth for color *data* shared across
//! render targets. Terminal-specific ANSI emission (the `TermColor` trait and
//! its impls) lives in `biscuit-terminal`.

pub mod basic;
pub mod color_enum;
pub mod hdr;
pub mod octet;
pub mod rgb;
pub mod tailwind;
pub mod web;

pub use basic::{BasicColor, FgBg, basic_color_to_rgb};
pub use color_enum::Color;
pub use hdr::HdrColor;
pub use octet::{Octet, OctetError};
pub use rgb::RgbColor;
pub use tailwind::Tailwind;
pub use web::{WEB_COLOR_LOOKUP, WebColor};

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
    #[test]
    fn tailwind_reference_color_accuracy() {
        // Slate family
        let slate_50 = Tailwind::Slate50.to_hdr_color().unwrap();
        assert_eq!(slate_50.red(), 248);
        assert_eq!(slate_50.green(), 250);
        assert_eq!(slate_50.blue(), 252);
        assert_eq!(Tailwind::Slate50.hex(), Some("#f8fafc"));

        let slate_500 = Tailwind::Slate500.to_hdr_color().unwrap();
        assert!(
            (slate_500.red() as i16 - 100).abs() < 5,
            "slate-500 red should be ~100"
        );
        assert!(
            (slate_500.green() as i16 - 116).abs() < 5,
            "slate-500 green should be ~116"
        );
        assert!(
            (slate_500.blue() as i16 - 139).abs() < 5,
            "slate-500 blue should be ~139"
        );

        let slate_950 = Tailwind::Slate950.to_hdr_color().unwrap();
        assert!(
            (slate_950.red() as i16).abs() < 10,
            "slate-950 red should be ~2"
        );
        assert!(
            (slate_950.green() as i16 - 6).abs() < 10,
            "slate-950 green should be ~6"
        );
        assert!(
            (slate_950.blue() as i16 - 23).abs() < 10,
            "slate-950 blue should be ~23"
        );

        // Red-500
        let red_500 = Tailwind::Red500.to_hdr_color().unwrap();
        assert!(red_500.red() > 220, "red-500 should have high red channel");
        assert!(
            red_500.green() < 100,
            "red-500 should have low green channel"
        );
        assert!(red_500.blue() < 100, "red-500 should have low blue channel");

        // Blue-500
        let blue_500 = Tailwind::Blue500.to_hdr_color().unwrap();
        assert!(blue_500.red() < 100, "blue-500 should have low red channel");
        assert!(
            blue_500.green() > 100 && blue_500.green() < 150,
            "blue-500 green should be ~130"
        );
        assert!(
            blue_500.blue() > 230,
            "blue-500 should have high blue channel"
        );

        // Indigo-500
        let indigo_500 = Tailwind::Indigo500.to_hdr_color().unwrap();
        assert!(
            indigo_500.red() > 80 && indigo_500.red() < 120,
            "indigo-500 red should be ~99"
        );
        assert!(
            indigo_500.green() > 80 && indigo_500.green() < 130,
            "indigo-500 green should be ~102"
        );
        assert!(
            indigo_500.blue() > 200,
            "indigo-500 should have high blue channel"
        );
    }

    /// Test that all concrete color variants return Some for to_hdr_color
    #[test]
    fn all_concrete_colors_return_some() {
        let families = [
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
        assert!(
            hex[1..].chars().all(|c| c.is_ascii_hexdigit()),
            "Hex should contain only hex digits"
        );
    }

    /// Test that neutral grays are truly achromatic (no color cast)
    #[test]
    fn neutral_grays_are_achromatic() {
        for shade in [
            Tailwind::Neutral50,
            Tailwind::Neutral100,
            Tailwind::Neutral200,
            Tailwind::Neutral300,
            Tailwind::Neutral400,
            Tailwind::Neutral500,
            Tailwind::Neutral600,
            Tailwind::Neutral700,
            Tailwind::Neutral800,
            Tailwind::Neutral900,
            Tailwind::Neutral950,
        ] {
            let color = shade.to_hdr_color().unwrap();
            let r = color.red() as i16;
            let g = color.green() as i16;
            let b = color.blue() as i16;

            assert!(
                (r - g).abs() <= 1 && (g - b).abs() <= 1,
                "{:?} should be achromatic: RGB({}, {}, {})",
                shade,
                r,
                g,
                b
            );
        }
    }

    /// Test fallback colors are appropriate for the shade
    #[test]
    fn fallback_colors_appropriate() {
        let slate_50 = Tailwind::Slate50.to_hdr_color().unwrap();
        assert_eq!(slate_50.fallback(), BasicColor::BrightWhite);

        let slate_200 = Tailwind::Slate200.to_hdr_color().unwrap();
        assert_eq!(slate_200.fallback(), BasicColor::White);

        let slate_500 = Tailwind::Slate500.to_hdr_color().unwrap();
        assert_eq!(slate_500.fallback(), BasicColor::BrightBlack);

        let slate_950 = Tailwind::Slate950.to_hdr_color().unwrap();
        assert_eq!(slate_950.fallback(), BasicColor::Black);

        let red_500 = Tailwind::Red500.to_hdr_color().unwrap();
        assert_eq!(red_500.fallback(), BasicColor::Red);

        let blue_500 = Tailwind::Blue500.to_hdr_color().unwrap();
        assert_eq!(blue_500.fallback(), BasicColor::Blue);

        let green_500 = Tailwind::Green500.to_hdr_color().unwrap();
        assert_eq!(green_500.fallback(), BasicColor::Green);
    }
}
