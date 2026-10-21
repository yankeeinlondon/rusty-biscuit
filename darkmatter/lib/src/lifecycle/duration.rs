//! The `{number}{unit}` duration grammar lifecycle delays share with host
//! timeouts.
//!
//! A number (digits and an optional decimal point), optional whitespace, then a
//! unit: seconds (`s`, `sec`, `second`, `seconds`), minutes (`m`, `min`,
//! `minute`, `minutes`), or hours (`h`, `hr`, `hour`, `hours`). The grammar
//! enforces no sign constraint; each caller decides whether zero is allowed.
//!
//! ## Examples
//!
//! ```
//! use darkmatter::lifecycle::duration::parse_duration_secs;
//!
//! assert_eq!(parse_duration_secs("5 min").unwrap(), 300.0);
//! assert_eq!(parse_duration_secs("0.5s").unwrap(), 0.5);
//! assert!(parse_duration_secs("500ms").is_err());
//! ```

use thiserror::Error;

/// The input does not follow the duration grammar.
///
/// `detail` is the complete reason, worded so a host can append it to its own
/// "invalid timeout" message unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{detail}")]
pub struct DurationSyntaxError {
    /// Why the input was rejected.
    pub detail: String,
}

impl DurationSyntaxError {
    fn new(detail: impl Into<String>) -> Self {
        Self { detail: detail.into() }
    }
}

const EXPECTED_UNITS: &str = "s, sec, second, seconds, m, min, minute, minutes, h, hr, hour, hours";

/// Parse `input` and return its length in seconds.
///
/// ## Errors
///
/// [`DurationSyntaxError`] for empty input, a missing or non-numeric number, a
/// missing unit, or an unknown unit.
pub fn parse_duration_secs(input: &str) -> Result<f64, DurationSyntaxError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(DurationSyntaxError::new("timeout string is empty"));
    }

    let num_end = trimmed
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(trimmed.len());
    let num_str = trimmed[..num_end].trim();
    let unit_str = trimmed[num_end..].trim();

    if num_str.is_empty() {
        return Err(DurationSyntaxError::new("no numeric value found"));
    }
    if unit_str.is_empty() {
        return Err(DurationSyntaxError::new(format!("missing unit; expected one of: {EXPECTED_UNITS}")));
    }

    let value: f64 = num_str
        .parse()
        .map_err(|_| DurationSyntaxError::new(format!("\"{num_str}\" is not a valid number")))?;

    let multiplier: f64 = match unit_str {
        "s" | "sec" | "second" | "seconds" => 1.0,
        "m" | "min" | "minute" | "minutes" => 60.0,
        "h" | "hr" | "hour" | "hours" => 3600.0,
        _ => {
            return Err(DurationSyntaxError::new(format!(
                "unknown unit \"{unit_str}\"; expected one of: {EXPECTED_UNITS}"
            )));
        }
    };

    Ok(value * multiplier)
}
