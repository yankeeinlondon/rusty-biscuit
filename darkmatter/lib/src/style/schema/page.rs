//! `PageStyle` — page-level frontmatter bucket.

use renderable::layout::{Alignment, Length};
use serde::de::{self, Deserializer};
use serde::Deserialize;

use crate::layout::PageBackground;
use crate::markdown::highlighting::{CodeBlockMode, ThemePair};
use crate::style::alignment::deserialize_optional_alignment;
use crate::style::color::{StyleColor, deserialize_optional_color};
use crate::style::length::{
    deserialize_optional_length, deserialize_optional_positive_length,
    deserialize_optional_row_count,
};

/// Page-level style settings from frontmatter.
///
/// Spacing keys use CSS order (`margin-left`, `padding-top`). The page
/// shorthands (`margin`, `mx`, `ml`, ...) and the top-level spellings of
/// these keys have no field here: style normalization expands them onto
/// these fields before deserialization.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct PageStyle {
    // Margins.
    #[serde(deserialize_with = "deserialize_optional_length")]
    pub margin_left: Option<Length>,
    #[serde(deserialize_with = "deserialize_optional_length")]
    pub margin_right: Option<Length>,
    #[serde(deserialize_with = "deserialize_optional_row_count")]
    pub margin_top: Option<u16>,
    #[serde(deserialize_with = "deserialize_optional_row_count")]
    pub margin_bottom: Option<u16>,

    // Padding.
    #[serde(deserialize_with = "deserialize_optional_length")]
    pub padding_left: Option<Length>,
    #[serde(deserialize_with = "deserialize_optional_length")]
    pub padding_right: Option<Length>,
    #[serde(deserialize_with = "deserialize_optional_row_count")]
    pub padding_top: Option<u16>,
    #[serde(deserialize_with = "deserialize_optional_row_count")]
    pub padding_bottom: Option<u16>,

    // Page knobs.
    /// Content width cap; always positive.
    #[serde(
        deserialize_with = "deserialize_optional_positive_length",
        alias = "max_width"
    )]
    pub max_width: Option<Length>,
    #[serde(deserialize_with = "deserialize_optional_alignment")]
    pub alignment: Option<Alignment>,
    #[serde(deserialize_with = "deserialize_optional_color")]
    pub color: Option<StyleColor>,
    #[serde(deserialize_with = "deserialize_optional_color", alias = "bg_color")]
    pub bg_color: Option<StyleColor>,
    pub background: Option<PageBackground>,
    /// Prose theme pair. When absent, the presentation defaults decide it
    /// after the merge (`style::presentation`).
    #[serde(deserialize_with = "deserialize_optional_theme")]
    pub theme: Option<ThemePair>,

    // Bespoke knobs.
    /// CSS file reference or URL; HTML output only.
    pub stylesheet: Option<String>,
    /// Browser metadata passed through to generated HTML.
    pub meta: Option<serde_json::Value>,
    pub code: Option<CodeStyle>,
}

/// `style.page.code`: page defaults for code blocks.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct CodeStyle {
    /// Code theme pair; a direct component theme still wins.
    #[serde(deserialize_with = "deserialize_optional_theme")]
    pub theme: Option<ThemePair>,
    /// How a code block's theme variant relates to the page color mode;
    /// `inverse` when absent.
    #[serde(deserialize_with = "deserialize_optional_code_block_mode")]
    pub mode: Option<CodeBlockMode>,
}

/// Reads a theme pair name through `ThemePair::try_from`, so every case and
/// alias spelling it accepts (`Base16-Ocean`, `onehalf`, `vs-dark`) works.
pub(crate) fn deserialize_optional_theme<'de, D>(de: D) -> Result<Option<ThemePair>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<serde_json::Value>::deserialize(de)? {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(name)) => ThemePair::try_from(name.as_str())
            .map(Some)
            .map_err(de::Error::custom),
        Some(_) => Err(de::Error::custom("theme must be a theme name string")),
    }
}

/// Reads a code-block mode. Only the lowercase names the schema declares
/// are accepted, so schema validation and the parser agree.
fn deserialize_optional_code_block_mode<'de, D>(de: D) -> Result<Option<CodeBlockMode>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<serde_json::Value>::deserialize(de)? {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(name)) => code_block_mode_from_name(&name)
            .map(Some)
            .ok_or_else(|| de::Error::custom(CODE_BLOCK_MODE_EXPECTED)),
        Some(_) => Err(de::Error::custom(CODE_BLOCK_MODE_EXPECTED)),
    }
}

pub(crate) const CODE_BLOCK_MODE_EXPECTED: &str =
    "code-block mode must be one of inverse, dark, light, same";

/// The mode named exactly `name` (`inverse`, `dark`, `light`, `same`).
pub(crate) fn code_block_mode_from_name(name: &str) -> Option<CodeBlockMode> {
    match name {
        "inverse" => Some(CodeBlockMode::Inverse),
        "dark" => Some(CodeBlockMode::Dark),
        "light" => Some(CodeBlockMode::Light),
        "same" => Some(CodeBlockMode::Same),
        _ => None,
    }
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
            "margin-left": "2ch",
            "margin-right": "4ch",
            "margin-top": 1,
            "margin-bottom": 0
        }"#;
        let p: PageStyle = serde_json::from_str(json).unwrap();
        assert_eq!(p.margin_left, Some(Length::Ch(2)));
        assert_eq!(p.margin_right, Some(Length::Ch(4)));
        assert_eq!(p.margin_top, Some(1));
        assert_eq!(p.margin_bottom, Some(0));
    }

    #[test]
    fn parses_padding_in_css_order() {
        let json = r#"{"padding-left": 2, "padding-right": "10%", "padding-top": 3, "padding-bottom": 4}"#;
        let p: PageStyle = serde_json::from_str(json).unwrap();
        assert_eq!(p.padding_left, Some(Length::Ch(2)));
        assert_eq!(p.padding_right, Some(Length::Percent(10.0)));
        assert_eq!((p.padding_top, p.padding_bottom), (Some(3), Some(4)));
    }

    #[test]
    fn removed_spacing_names_do_not_reach_the_typed_fields() {
        let json = r#"{"left-margin": "2ch", "left_margin": "2ch", "top-padding": 1}"#;
        let p: PageStyle = serde_json::from_str(json).unwrap();
        assert_eq!(p, PageStyle::default());
    }

    #[test]
    fn rejects_unit_on_vertical_margin() {
        let err = serde_json::from_str::<PageStyle>(r#"{"margin-top": "2ch"}"#).unwrap_err();
        assert!(err.to_string().contains("non-negative integer"));
    }

    #[test]
    fn rejects_zero_max_width_in_every_spelling() {
        for zero in ["0", r#""0""#, r#""0ch""#, r#""0%""#] {
            let err = serde_json::from_str::<PageStyle>(&format!(r#"{{"max-width": {zero}}}"#)).unwrap_err();
            assert!(err.to_string().contains("greater than zero"), "{zero}: {err}");
        }
    }

    #[test]
    fn theme_accepts_try_from_spellings() {
        let p: PageStyle = serde_json::from_str(r#"{"theme": "Base16-Ocean", "code": {"theme": "vs-dark"}}"#).unwrap();
        assert_eq!(p.theme, Some(ThemePair::Base16Ocean));
        assert_eq!(p.code.unwrap().theme, Some(ThemePair::VisualStudioDark));
    }

    #[test]
    fn theme_rejects_unknown_and_empty_names() {
        for name in [r#""neon""#, r#""""#, "5"] {
            assert!(serde_json::from_str::<PageStyle>(&format!(r#"{{"theme": {name}}}"#)).is_err(), "{name}");
        }
    }

    #[test]
    fn code_mode_parses_each_name_and_rejects_others() {
        for (name, mode) in [
            ("inverse", CodeBlockMode::Inverse),
            ("dark", CodeBlockMode::Dark),
            ("light", CodeBlockMode::Light),
            ("same", CodeBlockMode::Same),
        ] {
            let code: CodeStyle = serde_json::from_str(&format!(r#"{{"mode": "{name}"}}"#)).unwrap();
            assert_eq!(code.mode, Some(mode));
        }
        for bad in [r#""Dark""#, r#""auto""#, "true"] {
            assert!(serde_json::from_str::<CodeStyle>(&format!(r#"{{"mode": {bad}}}"#)).is_err(), "{bad}");
        }
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
