//! Custom deserializers that lower frontmatter length strings into
//! `renderable::layout::Length` (horizontal) and `u16` (vertical row counts).

use renderable::layout::Length;
use serde::Deserialize;
use serde::de::{self, Deserializer};

/// Typed reason for a horizontal-length parse failure. Used by the pre-
/// validator to choose between `StyleParseError::InvalidLength` and
/// `StyleParseError::InvalidPercent`.
#[derive(Debug, Clone, PartialEq)]
pub enum HorizontalLengthError {
    Empty,
    Negative,
    MalformedPercent,
    MalformedCh,
    /// Percent value was numerically parseable but outside `0.0..=100.0`.
    /// Carries the offending value for inclusion in the public error.
    PercentOutOfRange(f32),
    UnsupportedUnit,
}

impl HorizontalLengthError {
    /// Human-readable reason. Used both as a `&'static str` for serde
    /// `Error::custom` messages and as the `reason` field on
    /// `StyleParseError::InvalidLength`.
    pub fn as_static_reason(&self) -> &'static str {
        match self {
            Self::Empty => "empty length",
            Self::Negative => "negative length",
            Self::MalformedPercent => "malformed percent",
            Self::MalformedCh => "malformed ch length",
            Self::PercentOutOfRange(_) => "percent out of range; must be in 0.0..=100.0",
            Self::UnsupportedUnit => "unsupported unit; allowed: ch, %",
        }
    }
}

/// Parse a single horizontal length value, returning a typed error.
///
/// ## Accepted forms
///
/// - `"2ch"` / `"2 ch"` → `Length::Ch(2)`
/// - `"40"` (bare) → `Length::Ch(40)`
/// - `"50%"` / `"50.5%"` → `Length::Percent(50.0)` / `Length::Percent(50.5)`
pub fn parse_horizontal_typed(raw: &str) -> Result<Length, HorizontalLengthError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(HorizontalLengthError::Empty);
    }
    if trimmed.starts_with('-') {
        return Err(HorizontalLengthError::Negative);
    }

    // Percent: trailing `%`.
    if let Some(num_part) = trimmed.strip_suffix('%') {
        let n: f32 = num_part
            .trim()
            .parse()
            .map_err(|_| HorizontalLengthError::MalformedPercent)?;
        if !n.is_finite() {
            return Err(HorizontalLengthError::MalformedPercent);
        }
        if !(0.0..=100.0).contains(&n) {
            return Err(HorizontalLengthError::PercentOutOfRange(n));
        }
        return Ok(Length::Percent(n));
    }

    // `Nch` (with or without space).
    let lower = trimmed.to_ascii_lowercase();
    if let Some(num_part) = lower.strip_suffix("ch") {
        let n: u32 = num_part
            .trim()
            .parse()
            .map_err(|_| HorizontalLengthError::MalformedCh)?;
        return Ok(Length::Ch(n));
    }

    // Bare number → Ch.
    if let Ok(n) = trimmed.parse::<u32>() {
        return Ok(Length::Ch(n));
    }

    Err(HorizontalLengthError::UnsupportedUnit)
}

/// Thin wrapper over [`parse_horizontal_typed`] that returns a static reason
/// string. Kept for backward compatibility with the serde-layer deserializers.
///
/// ## Errors
///
/// Returns one of: `"empty length"`, `"negative length"`,
/// `"malformed percent"`, `"malformed ch length"`,
/// `"percent out of range; must be in 0.0..=100.0"`, or
/// `"unsupported unit; allowed: ch, %"`.
pub fn parse_horizontal(raw: &str) -> Result<Length, &'static str> {
    parse_horizontal_typed(raw).map_err(|e| e.as_static_reason())
}

/// Serde deserializer for `Option<Length>` reading a string or bare integer.
///
/// Accepts:
/// - A YAML/JSON string such as `"2ch"`, `"50%"`, or `"40"`.
/// - A bare JSON/YAML integer such as `40` — treated as `ch`.
/// - `null` / absent — yields `None`.
pub fn deserialize_optional_length<'de, D>(de: D) -> Result<Option<Length>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Option::deserialize(de)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => {
            parse_horizontal(&s).map(Some).map_err(de::Error::custom)
        }
        Some(serde_json::Value::Number(n)) => n
            .as_u64()
            .and_then(|u| u32::try_from(u).ok())
            .map(|u| Some(Length::Ch(u)))
            .ok_or_else(|| {
                de::Error::custom(
                    "length must be a non-negative integer in 0..=4294967295 \
                     or a string like \"2ch\", \"50%\"",
                )
            }),
        Some(other) => Err(de::Error::custom(format!(
            "length must be a string or non-negative integer (got {})",
            type_name_of(&other)
        ))),
    }
}

/// Serde deserializer for `Option<u16>` row counts.
///
/// Explicitly rejects strings so `top-margin: "2ch"` produces a clear error
/// rather than serde's default "invalid type" message.
pub fn deserialize_optional_row_count<'de, D>(de: D) -> Result<Option<u16>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Option::deserialize(de)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(n)) => n
            .as_u64()
            .and_then(|v| u16::try_from(v).ok())
            .map(Some)
            .ok_or_else(|| de::Error::custom("row count out of range for u16")),
        Some(other) => Err(de::Error::custom(format!(
            "row count must be a non-negative integer (got {})",
            type_name_of(&other)
        ))),
    }
}

fn type_name_of(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
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
        // Bare integer is accepted and treated as ch.
        let w: Wrap = serde_json::from_str(r#"{"v": 40}"#).unwrap();
        assert_eq!(w.v, Some(Length::Ch(40)));
    }

    #[test]
    fn row_count_accepts_integers() {
        #[derive(serde::Deserialize, Debug)]
        struct Wrap {
            #[serde(deserialize_with = "deserialize_optional_row_count")]
            v: Option<u16>,
        }
        let w: Wrap = serde_json::from_str(r#"{"v": 0}"#).unwrap();
        assert_eq!(w.v, Some(0));
        let w: Wrap = serde_json::from_str(r#"{"v": 1}"#).unwrap();
        assert_eq!(w.v, Some(1));
        let w: Wrap = serde_json::from_str(r#"{"v": 42}"#).unwrap();
        assert_eq!(w.v, Some(42));
    }

    #[test]
    fn row_count_rejects_strings() {
        #[derive(serde::Deserialize, Debug)]
        struct Wrap {
            #[serde(deserialize_with = "deserialize_optional_row_count")]
            #[allow(dead_code)]
            v: Option<u16>,
        }
        let err = serde_json::from_str::<Wrap>(r#"{"v": "2ch"}"#).unwrap_err();
        assert!(err.to_string().contains("must be a non-negative integer"));
        assert!(err.to_string().contains("string"));
    }

    #[test]
    fn row_count_null_is_none() {
        #[derive(serde::Deserialize, Debug)]
        struct Wrap {
            #[serde(default, deserialize_with = "deserialize_optional_row_count")]
            v: Option<u16>,
        }
        let w: Wrap = serde_json::from_str(r#"{"v": null}"#).unwrap();
        assert_eq!(w.v, None);
    }

    #[test]
    fn length_rejects_u32_overflow_integer() {
        #[derive(serde::Deserialize, Debug)]
        struct Wrap {
            #[serde(deserialize_with = "deserialize_optional_length")]
            #[allow(dead_code)]
            v: Option<Length>,
        }
        // 5_000_000_000 > u32::MAX (4_294_967_295); must error.
        let err =
            serde_json::from_str::<Wrap>(r#"{"v": 5000000000}"#).unwrap_err();
        assert!(err.to_string().contains("0..=4294967295"));
    }
}
