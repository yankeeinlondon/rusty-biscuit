//! `CommonStyle` — the five mutations shared by every component bucket
//! (`width`, `max-width`, `alignment`, `color`, `bg-color`).

use renderable::layout::{Alignment, Length};
use serde::Deserialize;

use crate::style::alignment::deserialize_optional_alignment;
use crate::style::color::{deserialize_optional_color, StyleColor};
use crate::style::length::deserialize_optional_length;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct CommonStyle {
    #[serde(deserialize_with = "deserialize_optional_length")]
    pub width: Option<Length>,
    #[serde(
        deserialize_with = "deserialize_optional_length",
        alias = "max_width"
    )]
    pub max_width: Option<Length>,
    #[serde(deserialize_with = "deserialize_optional_alignment")]
    pub alignment: Option<Alignment>,
    #[serde(deserialize_with = "deserialize_optional_color")]
    pub color: Option<StyleColor>,
    #[serde(
        deserialize_with = "deserialize_optional_color",
        alias = "bg_color"
    )]
    pub bg_color: Option<StyleColor>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_object_yields_default() {
        let c: CommonStyle = serde_json::from_str("{}").unwrap();
        assert_eq!(c, CommonStyle::default());
    }

    #[test]
    fn parses_max_width_percent() {
        let c: CommonStyle =
            serde_json::from_str(r#"{"max-width": "50%"}"#).unwrap();
        assert_eq!(c.max_width, Some(Length::Percent(50.0)));
    }

    #[test]
    fn parses_alignment_centered() {
        let c: CommonStyle =
            serde_json::from_str(r#"{"alignment": "centered"}"#).unwrap();
        assert_eq!(c.alignment, Some(Alignment::Center));
    }

    #[test]
    fn snake_case_max_width_alias_accepted() {
        // serde `alias` accepts the snake_case form. The Deprecated warning
        // is emitted by the canonicalization walker (Task 17), not by serde.
        let c: CommonStyle =
            serde_json::from_str(r#"{"max_width": "50%"}"#).unwrap();
        assert_eq!(c.max_width, Some(Length::Percent(50.0)));
    }
}
