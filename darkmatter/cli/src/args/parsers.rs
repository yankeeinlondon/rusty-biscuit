use darkmatter::markdown::highlighting::ThemePair;

/// Parses and validates list indentation width.
pub fn parse_indent_size(s: &str) -> Result<usize, String> {
    let value = s
        .parse::<usize>()
        .map_err(|_| format!("'{s}' is not a valid integer"))?;

    match value {
        2 | 4 | 8 => Ok(value),
        _ => Err("indent must be one of: 2, 4, 8".to_string()),
    }
}

/// Parses a compose collection limit: a positive integer that fits in 64 bits.
///
/// There is no unlimited spelling; zero, signs, and overflow are usage errors.
pub fn parse_positive_limit(s: &str) -> Result<std::num::NonZeroU64, String> {
    let value = s
        .parse::<u64>()
        .map_err(|_| format!("'{s}' is not a positive integer that fits in 64 bits"))?;
    std::num::NonZeroU64::new(value).ok_or_else(|| "the limit must be at least 1".to_string())
}

/// Parses a theme name string into ThemePair.
pub fn parse_theme_name(s: &str) -> Result<ThemePair, String> {
    ThemePair::try_from(s).map_err(|e| e.to_string())
}

/// Parses a boolean string (`true`/`false`/`1`/`0`/`yes`/`no`).
pub fn parse_bool_str(s: &str) -> Result<bool, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" | "y" => Ok(true),
        "false" | "0" | "no" | "off" | "n" => Ok(false),
        _ => Err(format!("expected true/false, got '{s}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_indent_size_accepts_valid_values() {
        assert_eq!(parse_indent_size("2"), Ok(2));
        assert_eq!(parse_indent_size("4"), Ok(4));
        assert_eq!(parse_indent_size("8"), Ok(8));
    }

    #[test]
    fn parse_indent_size_rejects_invalid() {
        assert!(parse_indent_size("3").is_err());
        assert!(parse_indent_size("abc").is_err());
    }

    #[test]
    fn parse_bool_str_accepts_truthy_values() {
        for val in ["true", "1", "yes", "on", "y"] {
            assert!(parse_bool_str(val).unwrap(), "value: {val}");
        }
    }

    #[test]
    fn parse_bool_str_accepts_falsy_values() {
        for val in ["false", "0", "no", "off", "n"] {
            assert!(!parse_bool_str(val).unwrap(), "value: {val}");
        }
    }

    #[test]
    fn parse_bool_str_rejects_invalid() {
        assert!(parse_bool_str("maybe").is_err());
        assert!(parse_bool_str("").is_err());
    }
}
