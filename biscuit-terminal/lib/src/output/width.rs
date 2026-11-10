//! `--width` parsing and resolution.
//!
//! | `--width` | Resolves to |
//! |---|---|
//! | `auto` | `OUTPUT_COLUMNS` if set, else the terminal's columns, else [`Width::Unlimited`] |
//! | `N` | `N` columns (`OUTPUT_COLUMNS` and the terminal are not consulted) |
//! | `N.M` | `N` columns, with a warning that the width takes integers |
//! | `N%` | `N` percent of `OUTPUT_COLUMNS`, else of the terminal, floored; unlimited when neither is known |
//! | `source` | [`Width::Source`] |
//! | `unlimited` | [`Width::Unlimited`] |
//!
//! ```
//! use biscuit_terminal::output::{parse_width_request, resolve_width};
//! use renderable::Width;
//!
//! let request = parse_width_request("50%").unwrap();
//! let resolved = resolve_width(request, Some("120"), Some(80)).unwrap();
//! assert_eq!(resolved.width, Width::Columns(60));
//!
//! let resolved = resolve_width(parse_width_request("auto").unwrap(), None, None).unwrap();
//! assert_eq!(resolved.width, Width::Unlimited);
//! ```
//!
//! The terminal's columns are known only when the stream being written is a
//! terminal; a caller passes `None` otherwise. What goes to stderr never uses
//! the deliverable's `--width` or `OUTPUT_COLUMNS`: see
//! [`resolve_stderr_width`].

use renderable::Width;

/// A `--width` value as given, before resolution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WidthRequest {
    /// `auto`: `OUTPUT_COLUMNS`, else the terminal, else unlimited.
    Auto,
    /// `source`: keep authored line breaks.
    Source,
    /// `unlimited`: never wrap, never fill.
    Unlimited,
    /// A whole number of columns (never zero).
    Columns(u32),
    /// A number with a fractional part; it floors to a positive column count
    /// and resolves with a warning.
    Fractional(f64),
    /// A positive, finite percentage of the available columns.
    Percent(f64),
}

/// A resolved width and the warning resolution produced, if any.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedWidth {
    /// The width to render at.
    pub width: Width,
    /// Text for a warning to deliver with the output (a fractional width).
    pub warning: Option<String>,
}

/// A `--width` or `OUTPUT_COLUMNS` value that cannot be used. Each is a usage
/// error.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum WidthError {
    /// The value is not one of the accepted forms.
    #[error("invalid width `{0}`: expected a positive number, N%, `auto`, `source`, or `unlimited`")]
    Malformed(String),
    /// The value floors to zero columns.
    #[error("invalid width `{0}`: it is less than one column")]
    TooSmall(String),
    /// The value is larger than the largest supported column count.
    #[error("invalid width `{0}`: it is more than {max} columns", max = u32::MAX)]
    TooLarge(String),
    /// `OUTPUT_COLUMNS` was needed but is not a positive whole number.
    #[error("invalid OUTPUT_COLUMNS `{0}`: expected a positive whole number of columns")]
    InvalidOutputColumns(String),
}

/// Parses a `--width` value.
///
/// ## Errors
///
/// [`WidthError::Malformed`] for anything that is not a keyword, a positive
/// finite number, or a positive finite percentage; [`WidthError::TooSmall`]
/// for a number that floors to zero; [`WidthError::TooLarge`] for a number
/// that does not fit in a `u32`.
pub fn parse_width_request(raw: &str) -> Result<WidthRequest, WidthError> {
    let malformed = || WidthError::Malformed(raw.to_owned());
    let text = raw.trim();
    match text {
        "auto" => return Ok(WidthRequest::Auto),
        "source" => return Ok(WidthRequest::Source),
        "unlimited" => return Ok(WidthRequest::Unlimited),
        _ => {}
    }
    if let Some(number) = text.strip_suffix('%') {
        let percent = parse_number(number).ok_or_else(malformed)?;
        if percent <= 0.0 {
            return Err(WidthError::TooSmall(raw.to_owned()));
        }
        return Ok(WidthRequest::Percent(percent));
    }
    let number = parse_number(text).ok_or_else(malformed)?;
    let floored = number.floor();
    if floored < 1.0 {
        return Err(WidthError::TooSmall(raw.to_owned()));
    }
    if floored > f64::from(u32::MAX) {
        return Err(WidthError::TooLarge(raw.to_owned()));
    }
    if floored == number {
        // `floored` is a whole number in 1..=u32::MAX, so the cast is exact.
        Ok(WidthRequest::Columns(floored as u32))
    } else {
        Ok(WidthRequest::Fractional(number))
    }
}

/// A plain decimal number (digits with an optional fraction), finite and not
/// negative. Signs, exponents, `inf`, and `NaN` are rejected.
fn parse_number(text: &str) -> Option<f64> {
    let valid = !text.is_empty()
        && text.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
        && text.chars().filter(|&ch| ch == '.').count() <= 1
        && text.chars().any(|ch| ch.is_ascii_digit());
    if !valid {
        return None;
    }
    text.parse::<f64>().ok().filter(|value| value.is_finite())
}

/// Resolves a request against the captured `OUTPUT_COLUMNS` value and the
/// output stream's terminal columns (`None` when that stream is not a
/// terminal).
///
/// `output_columns` is read only when the request needs it (`auto` or a
/// percentage), so a bad `OUTPUT_COLUMNS` does not fail an explicit width.
///
/// ## Errors
///
/// [`WidthError::InvalidOutputColumns`] when a needed `OUTPUT_COLUMNS` is not
/// a positive whole number; [`WidthError::TooSmall`] or
/// [`WidthError::TooLarge`] when a percentage resolves outside
/// `1..=u32::MAX` columns.
pub fn resolve_width(
    request: WidthRequest,
    output_columns: Option<&str>,
    terminal_columns: Option<u32>,
) -> Result<ResolvedWidth, WidthError> {
    let plain = |width| ResolvedWidth {
        width,
        warning: None,
    };
    let available = || -> Result<Option<u32>, WidthError> {
        match output_columns {
            Some(raw) => parse_output_columns(raw).map(Some),
            None => Ok(terminal_columns.filter(|&columns| columns > 0)),
        }
    };
    match request {
        WidthRequest::Source => Ok(plain(Width::Source)),
        WidthRequest::Unlimited => Ok(plain(Width::Unlimited)),
        WidthRequest::Columns(columns) => Ok(plain(
            Width::columns(columns).ok_or_else(|| WidthError::TooSmall(columns.to_string()))?,
        )),
        WidthRequest::Fractional(number) => {
            let floored = number.floor();
            if floored < 1.0 {
                return Err(WidthError::TooSmall(number.to_string()));
            }
            if floored > f64::from(u32::MAX) {
                return Err(WidthError::TooLarge(number.to_string()));
            }
            // `floored` is a whole number in 1..=u32::MAX, so the cast is exact.
            let columns = floored as u32;
            Ok(ResolvedWidth {
                width: Width::Columns(columns),
                warning: Some(format!(
                    "--width takes a whole number of columns; using {columns} for {number}"
                )),
            })
        }
        WidthRequest::Auto => Ok(plain(available()?.map_or(Width::Unlimited, Width::Columns))),
        WidthRequest::Percent(percent) => {
            let Some(base) = available()? else {
                return Ok(plain(Width::Unlimited));
            };
            let columns = (f64::from(base) * percent / 100.0).floor();
            let shown = format!("{percent}%");
            if columns < 1.0 {
                return Err(WidthError::TooSmall(shown));
            }
            if columns > f64::from(u32::MAX) {
                return Err(WidthError::TooLarge(shown));
            }
            // `columns` is a whole number in 1..=u32::MAX, so the cast is exact.
            Ok(plain(Width::Columns(columns as u32)))
        }
    }
}

fn parse_output_columns(raw: &str) -> Result<u32, WidthError> {
    raw.trim()
        .parse::<u32>()
        .ok()
        .filter(|&columns| columns > 0)
        .ok_or_else(|| WidthError::InvalidOutputColumns(raw.to_owned()))
}

/// The width for text written to stderr: the stderr terminal's columns, or
/// unlimited when stderr is not a terminal.
#[must_use]
pub fn resolve_stderr_width(stderr_terminal_columns: Option<u32>) -> Width {
    stderr_terminal_columns
        .and_then(Width::columns)
        .unwrap_or(Width::Unlimited)
}

/// Stderr's measured columns, when stderr is a terminal.
///
/// [`Terminal::detected_columns`](crate::terminal::Terminal::detected_columns)
/// measures stdout; a deliverable and what accompanies it can have different
/// widths.
#[must_use]
pub fn stderr_terminal_columns() -> Option<u32> {
    use std::io::IsTerminal as _;
    if !std::io::stderr().is_terminal() {
        return None;
    }
    terminal_size::terminal_size_of(std::io::stderr())
        .map(|(terminal_size::Width(columns), _)| u32::from(columns))
        .filter(|&columns| columns > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(raw: &str, output_columns: Option<&str>, terminal: Option<u32>) -> Result<ResolvedWidth, WidthError> {
        resolve_width(parse_width_request(raw)?, output_columns, terminal)
    }

    fn width(raw: &str, output_columns: Option<&str>, terminal: Option<u32>) -> Width {
        resolve(raw, output_columns, terminal).unwrap().width
    }

    #[test]
    fn keywords_parse() {
        assert_eq!(parse_width_request("auto"), Ok(WidthRequest::Auto));
        assert_eq!(parse_width_request("source"), Ok(WidthRequest::Source));
        assert_eq!(parse_width_request(" unlimited "), Ok(WidthRequest::Unlimited));
    }

    #[test]
    fn auto_prefers_output_columns_then_terminal_then_unlimited() {
        assert_eq!(width("auto", Some("100"), Some(80)), Width::Columns(100));
        assert_eq!(width("auto", None, Some(80)), Width::Columns(80));
        assert_eq!(width("auto", None, None), Width::Unlimited);
    }

    #[test]
    fn an_explicit_width_ignores_output_columns_and_terminal() {
        assert_eq!(width("72", Some("100"), Some(80)), Width::Columns(72));
        assert_eq!(width("72", Some("garbage"), None), Width::Columns(72));
        assert_eq!(width("source", Some("garbage"), Some(80)), Width::Source);
        assert_eq!(width("unlimited", Some("100"), Some(80)), Width::Unlimited);
    }

    #[test]
    fn a_fractional_width_floors_with_a_warning() {
        let resolved = resolve("72.9", None, None).unwrap();
        assert_eq!(resolved.width, Width::Columns(72));
        let warning = resolved.warning.unwrap();
        assert!(warning.contains("72") && warning.contains("72.9"), "{warning}");
        assert_eq!(resolve("72", None, None).unwrap().warning, None);
        assert_eq!(resolve("72.0", None, None).unwrap().warning, None);
    }

    #[test]
    fn percentages_floor_against_output_columns_or_terminal() {
        assert_eq!(width("50%", Some("101"), Some(80)), Width::Columns(50));
        assert_eq!(width("50%", None, Some(81)), Width::Columns(40));
        assert_eq!(width("150%", None, Some(80)), Width::Columns(120));
        assert_eq!(width("12.5%", None, Some(80)), Width::Columns(10));
        assert_eq!(width("80%", None, None), Width::Unlimited);
    }

    #[test]
    fn invalid_widths_are_usage_errors() {
        for raw in ["", "abc", "-5", "+5", "1e3", "inf", "NaN", "5..1", "%", "abc%", "-5%", "5 %x"] {
            assert!(
                matches!(parse_width_request(raw), Err(WidthError::Malformed(_))),
                "{raw:?}"
            );
        }
        for raw in ["0", "0.5", "0%"] {
            assert!(matches!(parse_width_request(raw), Err(WidthError::TooSmall(_))), "{raw}");
        }
        assert!(matches!(parse_width_request("4294967296"), Err(WidthError::TooLarge(_))));
        assert_eq!(parse_width_request("4294967295"), Ok(WidthRequest::Columns(u32::MAX)));
    }

    #[test]
    fn a_percentage_outside_the_column_range_is_rejected() {
        assert!(matches!(resolve("1%", None, Some(50)), Err(WidthError::TooSmall(_))));
        assert!(matches!(
            resolve("1000000000%", Some("4294967295"), None),
            Err(WidthError::TooLarge(_))
        ));
    }

    #[test]
    fn a_bad_output_columns_fails_only_when_needed() {
        for bad in ["0", "-3", "wide", "7.5", ""] {
            assert!(
                matches!(resolve("auto", Some(bad), Some(80)), Err(WidthError::InvalidOutputColumns(_))),
                "{bad:?}"
            );
            assert!(
                matches!(resolve("50%", Some(bad), Some(80)), Err(WidthError::InvalidOutputColumns(_))),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn stderr_width_is_its_own_terminal_or_unlimited() {
        assert_eq!(resolve_stderr_width(Some(120)), Width::Columns(120));
        assert_eq!(resolve_stderr_width(Some(0)), Width::Unlimited);
        assert_eq!(resolve_stderr_width(None), Width::Unlimited);
    }
}
