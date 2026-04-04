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
#[derive(Debug, Clone, PartialEq)]
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
        assert_eq!(" 50% ".parse::<WidthSpec>().unwrap(), WidthSpec::Percent(50));
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
