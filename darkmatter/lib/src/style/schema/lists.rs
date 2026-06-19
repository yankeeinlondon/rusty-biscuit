//! List buckets: `ul`, `ol`, `li`.

use renderable::layout::Length;
use serde::Deserialize;

use super::common::CommonStyle;
use crate::style::length::deserialize_optional_length;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct UlStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
    /// Left margin applied to ul content. Lowers to `ComponentPolicy.layout`
    /// `margin.left` for `PageComponent::Ul` (an independent indent channel);
    /// only the `ul` bucket is accepted.
    #[serde(
        deserialize_with = "deserialize_optional_length",
        alias = "left_margin"
    )]
    pub left_margin: Option<Length>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct OlStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct LiStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderable::layout::Alignment;

    #[test]
    fn ul_parses_left_margin_alignment_and_max_width() {
        // Matches the layout from
        // `darkmatter/example-docs/rendering/style-prop.md`:
        // ul:
        //   alignment: left
        //   left-margin: 4ch
        //   max-width: 40
        let json = r#"{
            "alignment": "left",
            "left-margin": "4ch",
            "max-width": "40"
        }"#;
        let ul: UlStyle = serde_json::from_str(json).unwrap();
        assert_eq!(ul.common.alignment, Some(Alignment::Left));
        assert_eq!(ul.left_margin, Some(Length::Ch(4)));
        assert_eq!(ul.common.max_width, Some(Length::Ch(40)));
    }

    #[test]
    fn ol_parses_alignment() {
        let ol: OlStyle = serde_json::from_str(r#"{"alignment": "right"}"#).unwrap();
        assert_eq!(ol.common.alignment, Some(Alignment::Right));
    }

    #[test]
    fn li_is_common_only() {
        let li: LiStyle = serde_json::from_str("{}").unwrap();
        assert_eq!(li, LiStyle::default());
    }

    #[test]
    fn snake_case_left_margin_alias_accepted() {
        let ul: UlStyle = serde_json::from_str(r#"{"left_margin": "4ch"}"#).unwrap();
        assert_eq!(ul.left_margin, Some(Length::Ch(4)));
    }
}
