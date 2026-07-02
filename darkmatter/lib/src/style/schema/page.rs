//! `PageStyle` — page-level frontmatter bucket.

use renderable::layout::{Alignment, Length};
use serde::Deserialize;

use crate::layout::PageBackground;
use crate::style::alignment::deserialize_optional_alignment;
use crate::style::color::{StyleColor, deserialize_optional_color};
use crate::style::length::{deserialize_optional_length, deserialize_optional_row_count};

/// Page-level style settings from frontmatter.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct PageStyle {
    // Margins.
    #[serde(
        deserialize_with = "deserialize_optional_length",
        alias = "left_margin"
    )]
    pub left_margin: Option<Length>,
    #[serde(
        deserialize_with = "deserialize_optional_length",
        alias = "right_margin"
    )]
    pub right_margin: Option<Length>,
    #[serde(
        deserialize_with = "deserialize_optional_row_count",
        alias = "top_margin"
    )]
    pub top_margin: Option<u16>,
    #[serde(
        deserialize_with = "deserialize_optional_row_count",
        alias = "bottom_margin"
    )]
    pub bottom_margin: Option<u16>,

    // Padding.
    #[serde(
        deserialize_with = "deserialize_optional_length",
        alias = "left_padding"
    )]
    pub left_padding: Option<Length>,
    #[serde(
        deserialize_with = "deserialize_optional_length",
        alias = "right_padding"
    )]
    pub right_padding: Option<Length>,
    #[serde(
        deserialize_with = "deserialize_optional_row_count",
        alias = "top_padding"
    )]
    pub top_padding: Option<u16>,
    #[serde(
        deserialize_with = "deserialize_optional_row_count",
        alias = "bottom_padding"
    )]
    pub bottom_padding: Option<u16>,

    // Page knobs.
    #[serde(deserialize_with = "deserialize_optional_length", alias = "max_width")]
    pub max_width: Option<Length>,
    #[serde(deserialize_with = "deserialize_optional_alignment")]
    pub alignment: Option<Alignment>,
    #[serde(deserialize_with = "deserialize_optional_color")]
    pub color: Option<StyleColor>,
    #[serde(deserialize_with = "deserialize_optional_color", alias = "bg_color")]
    pub bg_color: Option<StyleColor>,
    pub background: Option<PageBackground>,

    // Bespoke (parsed; inactive in v1).
    pub stylesheet: Option<String>,
    pub meta: Option<serde_json::Value>,
    pub code: Option<CodeStyle>,
}

/// Opaque `style.page.code` bucket. Detailed shape lands in sub-spec #7.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct CodeStyle {
    pub theme: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_yields_default() {
        let p: PageStyle = serde_json::from_str("{}").unwrap();
        assert_eq!(p, PageStyle::default());
    }

    #[test]
    fn parses_margins_from_test_doc() {
        // Matches the layout from
        // `darkmatter/example-docs/rendering/style-prop.md`.
        let json = r#"{
            "left-margin": "2ch",
            "right-margin": "4ch",
            "top-margin": 1,
            "bottom-margin": 0
        }"#;
        let p: PageStyle = serde_json::from_str(json).unwrap();
        assert_eq!(p.left_margin, Some(Length::Ch(2)));
        assert_eq!(p.right_margin, Some(Length::Ch(4)));
        assert_eq!(p.top_margin, Some(1));
        assert_eq!(p.bottom_margin, Some(0));
    }

    #[test]
    fn rejects_unit_on_vertical_margin() {
        let err = serde_json::from_str::<PageStyle>(r#"{"top-margin": "2ch"}"#).unwrap_err();
        assert!(err.to_string().contains("non-negative integer"));
    }

    #[test]
    fn snake_case_max_width_alias_accepted() {
        let p: PageStyle = serde_json::from_str(r#"{"max_width": "80"}"#).unwrap();
        assert_eq!(p.max_width, Some(Length::Ch(80)));
    }

    #[test]
    fn background_enum_parses() {
        let p: PageStyle = serde_json::from_str(r#"{"background": "subtle"}"#).unwrap();
        assert_eq!(p.background, Some(PageBackground::Subtle));
    }

    /// Validation guard (spec item 8): an unsupported `background` variant is
    /// rejected at deserialize time, never silently dropped.
    #[test]
    fn rejects_unknown_background() {
        let err = serde_json::from_str::<PageStyle>(r#"{"background": "glossy"}"#).unwrap_err();
        assert!(
            err.to_string().contains("unknown variant") || err.to_string().contains("glossy"),
            "unexpected error: {err}"
        );
    }
}
