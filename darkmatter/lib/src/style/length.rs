//! Custom deserializers that lower frontmatter length strings into
//! `renderable::layout::Length` (horizontal) and `u16` (vertical row counts).

use renderable::layout::Length;
use serde::de::{self, Deserializer};
use serde::Deserialize;

/// Parse a single horizontal length value.
///
/// ## Accepted forms
///
/// - `"2ch"` / `"2 ch"` → `Length::Ch(2)`
/// - `"40"` (bare) → `Length::Ch(40)`
/// - `"50%"` / `"50.5%"` → `Length::Percent(50.0)` / `Length::Percent(50.5)`
///
/// ## Errors
///
/// Returns one of the reasons:
/// `"empty length"`, `"negative length"`, `"malformed percent"`,
/// `"malformed ch length"`, `"percent out of range; must be in 0.0..=100.0"`,
/// or `"unsupported unit; allowed: ch, %"`.
pub fn parse_horizontal(raw: &str) -> Result<Length, &'static str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("empty length");
    }
    if trimmed.starts_with('-') {
        return Err("negative length");
    }

    // Percent: trailing `%`.
    if let Some(num_part) = trimmed.strip_suffix('%') {
        let n: f32 = num_part
            .trim()
            .parse()
            .map_err(|_| "malformed percent")?;
        if !(0.0..=100.0).contains(&n) || !n.is_finite() {
            return Err("percent out of range; must be in 0.0..=100.0");
        }
        return Ok(Length::Percent(n));
    }

    // `Nch` (with or without space).
    let lower = trimmed.to_ascii_lowercase();
    if let Some(num_part) = lower.strip_suffix("ch") {
        let n: u32 = num_part.trim().parse().map_err(|_| "malformed ch length")?;
        return Ok(Length::Ch(n));
    }

    // Bare number → Ch.
    if let Ok(n) = trimmed.parse::<u32>() {
        return Ok(Length::Ch(n));
    }

    Err("unsupported unit; allowed: ch, %")
}

/// Serde deserializer for `Option<Length>` reading a string.
pub fn deserialize_optional_length<'de, D>(de: D) -> Result<Option<Length>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: Option<String> = Option::deserialize(de)?;
    match raw {
        None => Ok(None),
        Some(s) => parse_horizontal(&s)
            .map(Some)
            .map_err(de::Error::custom),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(raw: &str) -> Result<Length, &'static str> {
        parse_horizontal(raw)
    }

    #[test]
    fn bare_number_is_ch() {
        assert_eq!(parse("40"), Ok(Length::Ch(40)));
        assert_eq!(parse("0"), Ok(Length::Ch(0)));
    }

    #[test]
    fn nch_parses() {
        assert_eq!(parse("2ch"), Ok(Length::Ch(2)));
        assert_eq!(parse("2 ch"), Ok(Length::Ch(2)));
        assert_eq!(parse("0ch"), Ok(Length::Ch(0)));
    }

    #[test]
    fn percent_parses() {
        assert_eq!(parse("50%"), Ok(Length::Percent(50.0)));
        assert_eq!(parse("50.5%"), Ok(Length::Percent(50.5)));
        assert_eq!(parse("100%"), Ok(Length::Percent(100.0)));
        assert_eq!(parse("0%"), Ok(Length::Percent(0.0)));
    }

    #[test]
    fn negative_rejected() {
        assert_eq!(parse("-2"), Err("negative length"));
        assert_eq!(parse("-2ch"), Err("negative length"));
        assert_eq!(parse("-50%"), Err("negative length"));
    }

    #[test]
    fn empty_rejected() {
        assert_eq!(parse(""), Err("empty length"));
        assert_eq!(parse("   "), Err("empty length"));
    }

    #[test]
    fn unsupported_unit_rejected() {
        assert!(parse("2px").is_err());
        assert!(parse("2em").is_err());
        assert!(parse("2rem").is_err());
    }

    #[test]
    fn malformed_percent_rejected() {
        assert_eq!(parse("50%%"), Err("malformed percent"));
        assert_eq!(parse("abc%"), Err("malformed percent"));
    }

    #[test]
    fn percent_out_of_range_rejected() {
        assert_eq!(
            parse("101%"),
            Err("percent out of range; must be in 0.0..=100.0")
        );
    }

    #[test]
    fn deserialize_via_serde() {
        #[derive(Debug, serde::Deserialize)]
        struct Wrap {
            #[serde(deserialize_with = "deserialize_optional_length")]
            v: Option<Length>,
        }
        let w: Wrap = serde_json::from_str(r#"{"v": "2ch"}"#).unwrap();
        assert_eq!(w.v, Some(Length::Ch(2)));
        let w: Wrap = serde_json::from_str(r#"{"v": null}"#).unwrap();
        assert_eq!(w.v, None);
        let err = serde_json::from_str::<Wrap>(r#"{"v": "2px"}"#).unwrap_err();
        assert!(err.to_string().contains("unsupported unit"));
    }
}
