use std::fmt;
use std::str::FromStr;

/// Parsed display width specification for images and diagrams.
///
/// Accepted formats:
/// - `"50%"` -- percentage of terminal width
/// - `"80ch"` or `"80"` -- fixed character width
/// - `"fill"` -- fill available terminal width
///
/// Implements `FromStr` for clap parse-time validation.
///
/// ## Examples
///
/// ```
/// use std::str::FromStr;
/// # use biscuit_terminal_cli::types::WidthSpec;
///
/// assert_eq!("50%".parse::<WidthSpec>().unwrap(), WidthSpec::Percent(50));
/// assert_eq!("80ch".parse::<WidthSpec>().unwrap(), WidthSpec::Chars(80));
/// assert_eq!("80".parse::<WidthSpec>().unwrap(), WidthSpec::Chars(80));
/// assert_eq!("fill".parse::<WidthSpec>().unwrap(), WidthSpec::Fill);
/// ```
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum WidthSpec {
    /// Percentage of terminal width (1-100)
    Percent(u8),
    /// Fixed width in characters
    Chars(u32),
    /// Fill available terminal width
    Fill,
}

impl FromStr for WidthSpec {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        if s.eq_ignore_ascii_case("fill") {
            return Ok(Self::Fill);
        }

        if let Some(pct) = s.strip_suffix('%') {
            let value: u8 = pct
                .parse()
                .map_err(|_| format!("invalid percentage: '{}'", pct))?;
            if value == 0 || value > 100 {
                return Err(format!("percentage must be 1-100, got {}", value));
            }
            return Ok(Self::Percent(value));
        }

        if let Some(chars) = s.strip_suffix("ch") {
            let value: u32 = chars
                .parse()
                .map_err(|_| format!("invalid character width: '{}'", chars))?;
            if value == 0 {
                return Err("character width must be > 0".to_string());
            }
            return Ok(Self::Chars(value));
        }

        // Plain number = characters
        let value: u32 = s.parse().map_err(|_| {
            format!(
                "invalid width spec '{}': expected percentage (e.g., 50%), \
                 characters (e.g., 80 or 80ch), or 'fill'",
                s
            )
        })?;
        if value == 0 {
            return Err("character width must be > 0".to_string());
        }
        Ok(Self::Chars(value))
    }
}

impl fmt::Display for WidthSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Percent(p) => write!(f, "{}%", p),
            Self::Chars(c) => write!(f, "{}", c),
            Self::Fill => write!(f, "fill"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_percentage() {
        assert_eq!("50%".parse::<WidthSpec>().unwrap(), WidthSpec::Percent(50));
        assert_eq!(
            "100%".parse::<WidthSpec>().unwrap(),
            WidthSpec::Percent(100)
        );
        assert_eq!("1%".parse::<WidthSpec>().unwrap(), WidthSpec::Percent(1));
    }

    #[test]
    fn parse_percentage_rejects_zero() {
        assert!("0%".parse::<WidthSpec>().is_err());
    }

    #[test]
    fn parse_percentage_rejects_over_100() {
        assert!("101%".parse::<WidthSpec>().is_err());
        assert!("200%".parse::<WidthSpec>().is_err());
    }

    #[test]
    fn parse_chars_with_suffix() {
        assert_eq!("80ch".parse::<WidthSpec>().unwrap(), WidthSpec::Chars(80));
        assert_eq!("1ch".parse::<WidthSpec>().unwrap(), WidthSpec::Chars(1));
    }

    #[test]
    fn parse_chars_plain_number() {
        assert_eq!("80".parse::<WidthSpec>().unwrap(), WidthSpec::Chars(80));
        assert_eq!("1".parse::<WidthSpec>().unwrap(), WidthSpec::Chars(1));
    }

    #[test]
    fn parse_chars_rejects_zero() {
        assert!("0".parse::<WidthSpec>().is_err());
        assert!("0ch".parse::<WidthSpec>().is_err());
    }

    #[test]
    fn parse_fill() {
        assert_eq!("fill".parse::<WidthSpec>().unwrap(), WidthSpec::Fill);
        assert_eq!("FILL".parse::<WidthSpec>().unwrap(), WidthSpec::Fill);
        assert_eq!("Fill".parse::<WidthSpec>().unwrap(), WidthSpec::Fill);
    }

    #[test]
    fn parse_trims_whitespace() {
        assert_eq!(
            " 50% ".parse::<WidthSpec>().unwrap(),
            WidthSpec::Percent(50)
        );
        assert_eq!(" fill ".parse::<WidthSpec>().unwrap(), WidthSpec::Fill);
    }

    #[test]
    fn parse_rejects_invalid() {
        assert!("abc".parse::<WidthSpec>().is_err());
        assert!("".parse::<WidthSpec>().is_err());
        assert!("%50".parse::<WidthSpec>().is_err());
    }

    #[test]
    fn display_roundtrip() {
        let specs = vec![
            WidthSpec::Percent(50),
            WidthSpec::Chars(80),
            WidthSpec::Fill,
        ];
        for spec in specs {
            let displayed = spec.to_string();
            let reparsed: WidthSpec = displayed.parse().unwrap();
            assert_eq!(spec, reparsed);
        }
    }
}

/// A validated hexadecimal color value.
///
/// Accepted formats: `#rgb`, `#rrggbb`, `#rrggbbaa`
///
/// Implements `FromStr` for clap parse-time validation.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct HexColor(String);

impl HexColor {
    /// Returns the hex color string including the `#` prefix.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for HexColor {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if !s.starts_with('#') {
            return Err(format!("hex color must start with '#', got '{}'", s));
        }

        let hex_part = &s[1..];
        if !matches!(hex_part.len(), 3 | 6 | 8) {
            return Err(format!(
                "invalid hex color '{}': expected #rgb, #rrggbb, or #rrggbbaa",
                s
            ));
        }

        if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("invalid hex characters in '{}'", s));
        }

        Ok(Self(s.to_string()))
    }
}

impl fmt::Display for HexColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod hex_color_tests {
    use super::*;

    #[test]
    fn parse_rgb_shorthand() {
        let c: HexColor = "#abc".parse().unwrap();
        assert_eq!(c.as_str(), "#abc");
    }

    #[test]
    fn parse_rrggbb() {
        let c: HexColor = "#e8f5e9".parse().unwrap();
        assert_eq!(c.as_str(), "#e8f5e9");
    }

    #[test]
    fn parse_rrggbbaa() {
        let c: HexColor = "#e8f5e9ff".parse().unwrap();
        assert_eq!(c.as_str(), "#e8f5e9ff");
    }

    #[test]
    fn parse_trims_whitespace() {
        let c: HexColor = " #abc ".parse().unwrap();
        assert_eq!(c.as_str(), "#abc");
    }

    #[test]
    fn parse_rejects_missing_hash() {
        assert!("e8f5e9".parse::<HexColor>().is_err());
    }

    #[test]
    fn parse_rejects_wrong_length() {
        assert!("#ab".parse::<HexColor>().is_err());
        assert!("#abcd".parse::<HexColor>().is_err());
        assert!("#abcde".parse::<HexColor>().is_err());
        assert!("#abcdefg".parse::<HexColor>().is_err());
    }

    #[test]
    fn parse_rejects_non_hex_chars() {
        assert!("#xyz123".parse::<HexColor>().is_err());
    }

    #[test]
    fn display_roundtrip() {
        let c: HexColor = "#e8f5e9".parse().unwrap();
        assert_eq!(c.to_string(), "#e8f5e9");
    }
}

/// A positive, finite f32 value (> 0.0, not NaN, not infinity).
///
/// Used for aspect ratios and other values that must be positive real numbers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositiveF32(f32);

impl PositiveF32 {
    /// Returns the inner f32 value.
    pub fn value(self) -> f32 {
        self.0
    }
}

impl FromStr for PositiveF32 {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: f32 = s.parse().map_err(|_| format!("invalid number: '{}'", s))?;
        if !value.is_finite() {
            return Err(format!("value must be finite, got {}", value));
        }
        if value <= 0.0 {
            return Err(format!("value must be positive, got {}", value));
        }
        Ok(Self(value))
    }
}

impl fmt::Display for PositiveF32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod positive_f32_tests {
    use super::*;

    #[test]
    fn parse_valid_positive() {
        let v: PositiveF32 = "1.5".parse().unwrap();
        assert_eq!(v.value(), 1.5_f32);
    }

    #[test]
    fn parse_rejects_zero() {
        assert!("0.0".parse::<PositiveF32>().is_err());
        assert!("0".parse::<PositiveF32>().is_err());
    }

    #[test]
    fn parse_rejects_negative() {
        assert!("-1.0".parse::<PositiveF32>().is_err());
        assert!("-0.001".parse::<PositiveF32>().is_err());
    }

    #[test]
    fn parse_rejects_infinity() {
        assert!("inf".parse::<PositiveF32>().is_err());
        assert!("-inf".parse::<PositiveF32>().is_err());
    }

    #[test]
    fn parse_rejects_nan() {
        assert!("NaN".parse::<PositiveF32>().is_err());
    }

    #[test]
    fn parse_rejects_non_numeric() {
        assert!("abc".parse::<PositiveF32>().is_err());
        assert!("".parse::<PositiveF32>().is_err());
    }

    #[test]
    fn display_roundtrip() {
        let v: PositiveF32 = "2.5".parse().unwrap();
        assert_eq!(v.to_string(), "2.5");
    }
}

/// Unescape common shell escape sequences.
///
/// The shell passes literal `\n`, `\t`, `\r` as two-character strings.
/// This converts them back to actual control characters.
pub fn unescape_shell_escapes(s: &str) -> String {
    s.replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\r", "\r")
}
