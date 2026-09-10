//! Timeout string parser for harness frontmatter.
//!
//! Accepts `{number}{unit}` with optional whitespace between number and unit.
//! Units: seconds (`s`, `sec`, `second`, `seconds`), minutes (`m`, `min`,
//! `minute`, `minutes`), hours (`h`, `hr`, `hour`, `hours`).

use std::path::Path;
use std::time::Duration;

use crate::harness::error::HarnessError;

/// Parse a human-friendly timeout string into a [`Duration`].
///
/// ## Examples
///
/// ```
/// use claudine::harness::parse_timeout;
/// use std::path::Path;
/// use std::time::Duration;
///
/// let d = parse_timeout("30s", Path::new("test.md")).unwrap();
/// assert_eq!(d, Duration::from_secs(30));
///
/// let d = parse_timeout("5 min", Path::new("test.md")).unwrap();
/// assert_eq!(d, Duration::from_secs(300));
/// ```
pub fn parse_timeout(input: &str, source_path: &Path) -> Result<Duration, HarnessError> {
    let total_secs = parse_duration_secs(input, source_path)?;
    if total_secs <= 0.0 {
        return Err(HarnessError::InvalidTimeout {
            source_path: source_path.to_path_buf(),
            raw: input.to_string(),
            detail: "timeout must be greater than zero".to_string(),
        });
    }
    Ok(Duration::from_secs_f64(total_secs))
}

/// Parse a human-friendly timeout string into a [`Duration`], permitting a
/// zero-valued duration.
///
/// Uses the same grammar as [`parse_timeout`] (`{number}{unit}`, decimals
/// allowed) but accepts any zero input (`0s`, `0.0s`, `0m`), returning
/// [`Duration::ZERO`]. Non-zero values parse identically to [`parse_timeout`].
/// Used for surfaces where `0s` is a meaningful disable sentinel (e.g. the
/// OpenCode stalled-generation `stall_timeout`).
///
/// ## Examples
///
/// ```
/// use claudine::harness::parse_timeout_allow_zero;
/// use std::path::Path;
/// use std::time::Duration;
///
/// let d = parse_timeout_allow_zero("0s", Path::new("test.md")).unwrap();
/// assert_eq!(d, Duration::ZERO);
///
/// let d = parse_timeout_allow_zero("500ms", Path::new("test.md"));
/// assert!(d.is_err()); // `ms` is not a recognized unit
///
/// let d = parse_timeout_allow_zero("0.5s", Path::new("test.md")).unwrap();
/// assert_eq!(d, Duration::from_millis(500));
/// ```
///
/// ## Errors
///
/// Returns [`HarnessError::InvalidTimeout`] for the same structural problems
/// [`parse_timeout`] rejects (empty input, no numeric value, missing unit,
/// unknown unit, non-number). It does **not** reject zero.
pub fn parse_timeout_allow_zero(input: &str, source_path: &Path) -> Result<Duration, HarnessError> {
    let total_secs = parse_duration_secs(input, source_path)?;
    Ok(Duration::from_secs_f64(total_secs.max(0.0)))
}

/// Shared grammar for [`parse_timeout`] and [`parse_timeout_allow_zero`].
///
/// Splits `input` into a numeric prefix (digits and a decimal point) and a
/// unit suffix, parses the number as `f64`, and multiplies by the unit's
/// per-second factor. Returns the total seconds. Rejects empty input, an
/// empty/non-numeric numeric component, a missing unit, and an unknown unit,
/// exactly as the original `parse_timeout`. Does **not** enforce any sign
/// constraint — the public wrappers decide whether zero is permitted.
fn parse_duration_secs(input: &str, source_path: &Path) -> Result<f64, HarnessError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(HarnessError::InvalidTimeout {
            source_path: source_path.to_path_buf(),
            raw: input.to_string(),
            detail: "timeout string is empty".to_string(),
        });
    }

    // Split into numeric prefix and unit suffix.
    let num_end = trimmed
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(trimmed.len());

    let num_str = trimmed[..num_end].trim();
    let unit_str = trimmed[num_end..].trim();

    if num_str.is_empty() {
        return Err(HarnessError::InvalidTimeout {
            source_path: source_path.to_path_buf(),
            raw: input.to_string(),
            detail: "no numeric value found".to_string(),
        });
    }

    if unit_str.is_empty() {
        return Err(HarnessError::InvalidTimeout {
            source_path: source_path.to_path_buf(),
            raw: input.to_string(),
            detail: "missing unit; expected one of: s, sec, second, seconds, m, min, minute, minutes, h, hr, hour, hours".to_string(),
        });
    }

    let value: f64 = num_str.parse().map_err(|_| HarnessError::InvalidTimeout {
        source_path: source_path.to_path_buf(),
        raw: input.to_string(),
        detail: format!("\"{num_str}\" is not a valid number"),
    })?;

    let multiplier: f64 = match unit_str {
        "s" | "sec" | "second" | "seconds" => 1.0,
        "m" | "min" | "minute" | "minutes" => 60.0,
        "h" | "hr" | "hour" | "hours" => 3600.0,
        _ => {
            return Err(HarnessError::InvalidTimeout {
                source_path: source_path.to_path_buf(),
                raw: input.to_string(),
                detail: format!(
                    "unknown unit \"{unit_str}\"; expected one of: s, sec, second, seconds, m, min, minute, minutes, h, hr, hour, hours"
                ),
            });
        }
    };

    Ok(value * multiplier)
}

/// Render a [`Duration`] in the same grammar that [`parse_timeout`] accepts,
/// picking the largest whole unit that divides evenly (else falling back to
/// seconds). Used in error messages so the displayed value can be copied back
/// into config without rewriting.
pub(crate) fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs > 0 && secs.is_multiple_of(3600) {
        format!("{}h", secs / 3600)
    } else if secs > 0 && secs.is_multiple_of(60) {
        format!("{}m", secs / 60)
    } else {
        format!("{secs}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn p() -> &'static Path {
        Path::new("test.md")
    }

    #[test]
    fn parse_seconds_short() {
        assert_eq!(parse_timeout("30s", p()).unwrap(), Duration::from_secs(30));
    }

    #[test]
    fn parse_seconds_long() {
        assert_eq!(
            parse_timeout("30 seconds", p()).unwrap(),
            Duration::from_secs(30)
        );
    }

    #[test]
    fn parse_minutes_short() {
        assert_eq!(parse_timeout("5m", p()).unwrap(), Duration::from_secs(300));
    }

    #[test]
    fn parse_minutes_long() {
        assert_eq!(
            parse_timeout("5 min", p()).unwrap(),
            Duration::from_secs(300)
        );
    }

    #[test]
    fn parse_hours_short() {
        assert_eq!(parse_timeout("2h", p()).unwrap(), Duration::from_secs(7200));
    }

    #[test]
    fn parse_hours_long() {
        assert_eq!(
            parse_timeout("2 hours", p()).unwrap(),
            Duration::from_secs(7200)
        );
    }

    #[test]
    fn reject_zero() {
        let err = parse_timeout("0s", p()).unwrap_err();
        assert!(err.to_string().contains("greater than zero"), "got: {err}");
    }

    #[test]
    fn allow_zero_seconds() {
        assert_eq!(
            parse_timeout_allow_zero("0s", p()).unwrap(),
            Duration::ZERO
        );
    }

    #[test]
    fn allow_zero_decimal_seconds() {
        assert_eq!(
            parse_timeout_allow_zero("0.0s", p()).unwrap(),
            Duration::ZERO
        );
    }

    #[test]
    fn allow_zero_minutes() {
        assert_eq!(
            parse_timeout_allow_zero("0m", p()).unwrap(),
            Duration::ZERO
        );
    }

    #[test]
    fn allow_zero_parses_fractional_seconds() {
        assert_eq!(
            parse_timeout_allow_zero("0.5s", p()).unwrap(),
            Duration::from_millis(500)
        );
    }

    #[test]
    fn allow_zero_parses_minutes() {
        assert_eq!(
            parse_timeout_allow_zero("10m", p()).unwrap(),
            Duration::from_secs(600)
        );
    }

    #[test]
    fn allow_zero_rejects_invalid() {
        assert!(parse_timeout_allow_zero("nope", p()).is_err());
        assert!(parse_timeout_allow_zero("5x", p()).is_err());
        assert!(parse_timeout_allow_zero("5", p()).is_err());
    }

    #[test]
    fn reject_non_numeric() {
        let err = parse_timeout("abc", p()).unwrap_err();
        assert!(err.to_string().contains("no numeric value"), "got: {err}");
    }

    #[test]
    fn reject_unknown_unit() {
        let err = parse_timeout("5x", p()).unwrap_err();
        assert!(err.to_string().contains("unknown unit"), "got: {err}");
    }

    #[test]
    fn reject_missing_unit() {
        let err = parse_timeout("5", p()).unwrap_err();
        assert!(err.to_string().contains("missing unit"), "got: {err}");
    }

    #[test]
    fn parse_sec_unit() {
        assert_eq!(
            parse_timeout("10sec", p()).unwrap(),
            Duration::from_secs(10)
        );
    }

    #[test]
    fn parse_hr_unit() {
        assert_eq!(
            parse_timeout("1hr", p()).unwrap(),
            Duration::from_secs(3600)
        );
    }

    #[test]
    fn parse_with_whitespace() {
        assert_eq!(
            parse_timeout("  15  seconds  ", p()).unwrap(),
            Duration::from_secs(15)
        );
    }

    #[test]
    fn format_duration_hours_when_evenly_divisible() {
        assert_eq!(format_duration(Duration::from_secs(7200)), "2h");
        assert_eq!(format_duration(Duration::from_secs(3600)), "1h");
    }

    #[test]
    fn format_duration_minutes_when_evenly_divisible() {
        assert_eq!(format_duration(Duration::from_secs(300)), "5m");
        assert_eq!(format_duration(Duration::from_secs(60)), "1m");
    }

    #[test]
    fn format_duration_seconds_otherwise() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(90)), "90s");
        assert_eq!(format_duration(Duration::from_secs(0)), "0s");
    }
}
