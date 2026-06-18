//! Deserializer for `renderable::layout::Alignment` accepting the documented
//! `centered` alias for `center`.

use renderable::layout::Alignment;
use serde::Deserialize;
use serde::de::{self, Deserializer};

/// Parse an alignment string.
///
/// Accepts `"left"`, `"center"`, `"centered"`, `"right"`.
pub fn parse(raw: &str) -> Result<Alignment, &'static str> {
    match raw.trim() {
        "left" => Ok(Alignment::Left),
        "center" | "centered" => Ok(Alignment::Center),
        "right" => Ok(Alignment::Right),
        _ => Err("alignment must be one of: left, center, centered, right"),
    }
}

pub fn deserialize_optional_alignment<'de, D>(de: D) -> Result<Option<Alignment>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: Option<String> = Option::deserialize(de)?;
    match raw {
        None => Ok(None),
        Some(s) => parse(&s).map(Some).map_err(de::Error::custom),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn left_center_right() {
        assert_eq!(parse("left"), Ok(Alignment::Left));
        assert_eq!(parse("center"), Ok(Alignment::Center));
        assert_eq!(parse("right"), Ok(Alignment::Right));
    }

    #[test]
    fn centered_alias_matches_center() {
        assert_eq!(parse("centered"), Ok(Alignment::Center));
    }

    #[test]
    fn unknown_value_rejected() {
        assert!(parse("middle").is_err());
        assert!(parse("justify").is_err());
        assert!(parse("").is_err());
    }

    #[test]
    fn deserialize_via_serde() {
        #[derive(serde::Deserialize, Debug)]
        struct Wrap {
            #[serde(deserialize_with = "deserialize_optional_alignment")]
            v: Option<Alignment>,
        }
        let w: Wrap = serde_json::from_str(r#"{"v": "centered"}"#).unwrap();
        assert_eq!(w.v, Some(Alignment::Center));
        let w: Wrap = serde_json::from_str(r#"{"v": null}"#).unwrap();
        assert_eq!(w.v, None);
    }
}
