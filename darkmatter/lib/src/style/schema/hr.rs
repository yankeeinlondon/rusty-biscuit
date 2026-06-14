//! `HrStyle` — horizontal-rule bucket.
//!
//! The legacy per-block `style: waves` attribute is migrated to
//! `style.hr.kind` by sub-spec #6.

use renderable::layout::Length;
use serde::Deserialize;

use crate::style::color::{StyleColor, deserialize_optional_color};
use crate::style::length::deserialize_optional_length;

/// Visual style of a horizontal rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HrKind {
    Dashes,
    Dots,
    Waves,
    LineStar,
    LineCircle,
    InsetLine,
    CurtainRod,
}

/// Visual weight (thickness) of a horizontal rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HrWeight {
    Thin,
    Medium,
    Thick,
}

/// Horizontal alignment of a horizontal rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HrAlignment {
    Full,
    Left,
    #[serde(alias = "centered")]
    Center,
    Right,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct HrStyle {
    #[serde(deserialize_with = "deserialize_optional_length")]
    pub width: Option<Length>,
    #[serde(deserialize_with = "deserialize_optional_length", alias = "max_width")]
    pub max_width: Option<Length>,
    #[serde(deserialize_with = "deserialize_optional_color")]
    pub color: Option<StyleColor>,
    #[serde(deserialize_with = "deserialize_optional_color", alias = "bg_color")]
    pub bg_color: Option<StyleColor>,
    pub alignment: Option<HrAlignment>,
    pub kind: Option<HrKind>,
    pub weight: Option<HrWeight>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_kind() {
        let h: HrStyle = serde_json::from_str(r#"{"kind": "waves"}"#).unwrap();
        assert_eq!(h.kind, Some(HrKind::Waves));
    }

    #[test]
    fn parses_weight() {
        let h: HrStyle = serde_json::from_str(r#"{"weight": "thick"}"#).unwrap();
        assert_eq!(h.weight, Some(HrWeight::Thick));
    }

    #[test]
    fn parses_alignment() {
        let h: HrStyle = serde_json::from_str(r#"{"alignment": "full"}"#).unwrap();
        assert_eq!(h.alignment, Some(HrAlignment::Full));
    }

    #[test]
    fn parses_centered_alias() {
        let h: HrStyle = serde_json::from_str(r#"{"alignment": "centered"}"#).unwrap();
        assert_eq!(h.alignment, Some(HrAlignment::Center));
    }

    #[test]
    fn parses_all_fields() {
        let json = r#"{
            "width": "50%",
            "max-width": "80ch",
            "color": "red-500",
            "bg-color": "black",
            "alignment": "center",
            "kind": "line-star",
            "weight": "medium"
        }"#;
        let h: HrStyle = serde_json::from_str(json).unwrap();
        assert_eq!(h.width, Some(Length::Percent(50.0)));
        assert_eq!(h.max_width, Some(Length::Ch(80)));
        assert!(h.color.is_some());
        assert!(h.bg_color.is_some());
        assert_eq!(h.alignment, Some(HrAlignment::Center));
        assert_eq!(h.kind, Some(HrKind::LineStar));
        assert_eq!(h.weight, Some(HrWeight::Medium));
    }

    #[test]
    fn empty_yields_default() {
        let h: HrStyle = serde_json::from_str("{}").unwrap();
        assert_eq!(h, HrStyle::default());
    }

    #[test]
    fn hr_kind_variants_deserialize() {
        assert_eq!(
            serde_json::from_str::<HrKind>(r#""dashes""#).unwrap(),
            HrKind::Dashes
        );
        assert_eq!(
            serde_json::from_str::<HrKind>(r#""dots""#).unwrap(),
            HrKind::Dots
        );
        assert_eq!(
            serde_json::from_str::<HrKind>(r#""waves""#).unwrap(),
            HrKind::Waves
        );
        assert_eq!(
            serde_json::from_str::<HrKind>(r#""line-star""#).unwrap(),
            HrKind::LineStar
        );
        assert_eq!(
            serde_json::from_str::<HrKind>(r#""line-circle""#).unwrap(),
            HrKind::LineCircle
        );
        assert_eq!(
            serde_json::from_str::<HrKind>(r#""inset-line""#).unwrap(),
            HrKind::InsetLine
        );
        assert_eq!(
            serde_json::from_str::<HrKind>(r#""curtain-rod""#).unwrap(),
            HrKind::CurtainRod
        );
    }

    #[test]
    fn hr_weight_variants_deserialize() {
        assert_eq!(
            serde_json::from_str::<HrWeight>(r#""thin""#).unwrap(),
            HrWeight::Thin
        );
        assert_eq!(
            serde_json::from_str::<HrWeight>(r#""medium""#).unwrap(),
            HrWeight::Medium
        );
        assert_eq!(
            serde_json::from_str::<HrWeight>(r#""thick""#).unwrap(),
            HrWeight::Thick
        );
    }

    #[test]
    fn hr_alignment_variants_deserialize() {
        assert_eq!(
            serde_json::from_str::<HrAlignment>(r#""full""#).unwrap(),
            HrAlignment::Full
        );
        assert_eq!(
            serde_json::from_str::<HrAlignment>(r#""left""#).unwrap(),
            HrAlignment::Left
        );
        assert_eq!(
            serde_json::from_str::<HrAlignment>(r#""center""#).unwrap(),
            HrAlignment::Center
        );
        assert_eq!(
            serde_json::from_str::<HrAlignment>(r#""right""#).unwrap(),
            HrAlignment::Right
        );
    }

    #[test]
    fn hr_alignment_centered_alias_deserializes() {
        assert_eq!(
            serde_json::from_str::<HrAlignment>(r#""centered""#).unwrap(),
            HrAlignment::Center
        );
    }

    #[test]
    fn snake_case_max_width_alias_accepted() {
        let h: HrStyle = serde_json::from_str(r#"{"max_width": "50%"}"#).unwrap();
        assert_eq!(h.max_width, Some(Length::Percent(50.0)));
    }

    #[test]
    fn snake_case_bg_color_alias_accepted() {
        let h: HrStyle = serde_json::from_str(r#"{"bg_color": "red-500"}"#).unwrap();
        assert!(h.bg_color.is_some());
    }
}
