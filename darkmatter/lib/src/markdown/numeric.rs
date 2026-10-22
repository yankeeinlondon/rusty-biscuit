//! Darkmatter's one numeric model: admission of written numbers into `f64`.
//!
//! Every Darkmatter number is a finite `f64`. A number enters that model only
//! through this module, which decides from the number's **written spelling**
//! (or from an exact integer) whether it can be held:
//!
//! - a mathematically whole value must be represented exactly, whatever its
//!   spelling: `9007199254740993`, `9007199254740993.0`, and
//!   `9.007199254740993e15` are all rejected as "cannot be represented
//!   exactly";
//! - a value with a non-zero fractional part takes the nearest `f64`, so `0.1`
//!   is admitted;
//! - a non-zero value that would become `0` is rejected (underflow);
//! - a value whose magnitude overflows is rejected as "not finite".
//!
//! An `f64` that reached Darkmatter already rounded can only be checked for
//! finiteness ([`admit_f64`]); digits lost before that point cannot be
//! recovered. The user-facing rules are in Darkmatter's numbers topic doc.

use std::fmt;

/// Why a written or supplied number cannot become a Darkmatter number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericAdmissionError {
    /// A whole value that no `f64` holds exactly.
    Inexact,
    /// A value whose magnitude overflows `f64`, or an `inf`/`NaN` `f64`.
    NotFinite,
    /// A non-zero value that would round to `0`.
    Underflow,
}

impl fmt::Display for NumericAdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Inexact => "cannot be represented exactly",
            Self::NotFinite => "not finite",
            Self::Underflow => "cannot be represented exactly: a non-zero value would become 0",
        })
    }
}

impl std::error::Error for NumericAdmissionError {}

/// Why text cannot be admitted as a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberTextError {
    /// The text is not a number spelling at all (`"pear"`, `"0x10"`, `"NaN"`).
    NotANumber,
    /// The text is a number spelling that cannot be admitted.
    Admission(NumericAdmissionError),
}

impl fmt::Display for NumberTextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotANumber => f.write_str("not a number"),
            Self::Admission(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for NumberTextError {}

impl From<NumericAdmissionError> for NumberTextError {
    fn from(error: NumericAdmissionError) -> Self {
        Self::Admission(error)
    }
}

/// Admits a number exactly as a loader or lexer wrote it.
///
/// Accepts an optional sign, decimal digits with an optional fraction (a
/// leading dot `.5` and a trailing dot `5.` are allowed, as YAML allows both),
/// and an optional exponent. No whitespace, underscores, hexadecimal, or
/// `inf`/`NaN` words.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::numeric::{admit_number_spelling, NumericAdmissionError, NumberTextError};
///
/// assert_eq!(admit_number_spelling("1e3"), Ok(1000.0));
/// assert_eq!(admit_number_spelling("0.1"), Ok(0.1));
/// assert_eq!(
///     admit_number_spelling("9007199254740993.0"),
///     Err(NumberTextError::Admission(NumericAdmissionError::Inexact))
/// );
/// ```
///
/// ## Errors
///
/// [`NumberTextError::NotANumber`] for any other spelling, and
/// [`NumberTextError::Admission`] when the written value cannot be held.
pub fn admit_number_spelling(spelling: &str) -> Result<f64, NumberTextError> {
    let parts = DecimalParts::parse(spelling).ok_or(NumberTextError::NotANumber)?;
    Ok(parts.admit(spelling)?)
}

/// Admits text as a number under the lenient conversion rules.
///
/// Surrounding whitespace, a leading sign, an exponent, a leading dot, and
/// single underscores between digits are accepted (`" 4 "`, `"+4"`, `"1e3"`,
/// `".5"`, `"1_000"`). A trailing dot, a leading, trailing, or doubled
/// underscore, and empty text are not numbers. This is the one canonical
/// numeric-text parser: conversion, `numberlike`, and `number()`/`round()`
/// all call it.
///
/// ## Errors
///
/// As [`admit_number_spelling`].
pub fn admit_number_text(text: &str) -> Result<f64, NumberTextError> {
    let trimmed = text.trim();
    if trimmed.ends_with('.') {
        return Err(NumberTextError::NotANumber);
    }
    let spelling = remove_digit_separators(trimmed).ok_or(NumberTextError::NotANumber)?;
    admit_number_spelling(&spelling)
}

/// Admits an `f64` that was supplied already rounded: only finiteness can be
/// checked.
///
/// ## Errors
///
/// [`NumericAdmissionError::NotFinite`] for `inf`, `-inf`, and `NaN`.
pub fn admit_f64(value: f64) -> Result<f64, NumericAdmissionError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(NumericAdmissionError::NotFinite)
    }
}

/// Admits a `serde_json` number that carries no spelling.
///
/// An integer (`i64`/`u64`) is exact, so it is admitted only when an `f64`
/// holds the same value; a float gets [`admit_f64`]'s finiteness check.
///
/// ## Errors
///
/// [`NumericAdmissionError::Inexact`] for an integer no `f64` holds exactly.
pub fn admit_json_number(number: &serde_json::Number) -> Result<f64, NumericAdmissionError> {
    if let Some(integer) = number.as_i64() {
        return admit_integer(i128::from(integer));
    }
    if let Some(integer) = number.as_u64() {
        return admit_integer(i128::from(integer));
    }
    admit_f64(number.as_f64().unwrap_or(f64::NAN))
}

/// Admits an exact integer.
///
/// ## Errors
///
/// [`NumericAdmissionError::Inexact`] when no `f64` holds the value.
pub fn admit_integer(integer: i128) -> Result<f64, NumericAdmissionError> {
    // `as f64` rounds to nearest; the round trip through `i128` is exact for
    // every finite whole `f64` below 2^127.
    let rounded = integer as f64;
    if rounded.is_finite() && rounded.abs() < 2f64.powi(127) && rounded as i128 == integer {
        Ok(rounded)
    } else {
        Err(NumericAdmissionError::Inexact)
    }
}

/// Reads a boolean word: `true`/`false`, `yes`/`no`, `on`/`off`, `1`/`0`, in
/// any letter case, after trimming surrounding whitespace.
pub fn parse_boolean_word(text: &str) -> Option<bool> {
    let word = text.trim();
    const TRUE: [&str; 4] = ["true", "yes", "on", "1"];
    const FALSE: [&str; 4] = ["false", "no", "off", "0"];
    if TRUE.iter().any(|candidate| candidate.eq_ignore_ascii_case(word)) {
        Some(true)
    } else if FALSE.iter().any(|candidate| candidate.eq_ignore_ascii_case(word)) {
        Some(false)
    } else {
        None
    }
}

/// The text form of a number: no decimal suffix on a whole value (`4.0` is
/// `"4"`), every meaningful fractional digit kept, no exponent, and never an
/// integer cast. Negative zero is `"0"`.
pub fn format_number(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    format!("{value}")
}

/// The `serde_json` number for an admitted `f64`: whole values within the
/// `i64`/`u64` range are stored as integers (losslessly, since the value is
/// whole), everything else as a float. `None` for a non-finite value.
pub fn json_number(value: f64) -> Option<serde_json::Number> {
    if !value.is_finite() {
        return None;
    }
    if value.fract() == 0.0 {
        if value >= -(2f64.powi(63)) && value < 2f64.powi(63) {
            return Some(serde_json::Number::from(value as i64));
        }
        if value >= 0.0 && value < 2f64.powi(64) {
            return Some(serde_json::Number::from(value as u64));
        }
    }
    serde_json::Number::from_f64(value)
}

/// Removes underscore digit separators, refusing any underscore that is not
/// between two ASCII digits.
fn remove_digit_separators(text: &str) -> Option<String> {
    if !text.contains('_') {
        return Some(text.to_string());
    }
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    for (index, &byte) in bytes.iter().enumerate() {
        if byte == b'_' {
            let before = index.checked_sub(1).map(|i| bytes[i]);
            let after = bytes.get(index + 1).copied();
            if !before.is_some_and(|b| b.is_ascii_digit()) || !after.is_some_and(|b| b.is_ascii_digit()) {
                return None;
            }
        } else {
            out.push(byte as char);
        }
    }
    Some(out)
}

/// A validated decimal spelling split into its significant digits and power
/// of ten.
struct DecimalParts {
    /// Mantissa digits (integer then fraction) without leading zeros; empty
    /// for zero.
    digits: String,
    /// Power of ten applied to `digits`; saturates for absurd exponents.
    exponent: i64,
}

impl DecimalParts {
    fn parse(spelling: &str) -> Option<Self> {
        let bytes = spelling.as_bytes();
        let mut index = 0;
        if matches!(bytes.first(), Some(b'+' | b'-')) {
            index += 1;
        }
        let integer_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        let integer = &spelling[integer_start..index];
        let mut fraction = "";
        if bytes.get(index) == Some(&b'.') {
            index += 1;
            let fraction_start = index;
            while bytes.get(index).is_some_and(u8::is_ascii_digit) {
                index += 1;
            }
            fraction = &spelling[fraction_start..index];
        }
        if integer.is_empty() && fraction.is_empty() {
            return None;
        }
        let mut written_exponent: i64 = 0;
        if matches!(bytes.get(index), Some(b'e' | b'E')) {
            index += 1;
            let negative = match bytes.get(index) {
                Some(b'-') => {
                    index += 1;
                    true
                }
                Some(b'+') => {
                    index += 1;
                    false
                }
                _ => false,
            };
            let exponent_start = index;
            while bytes.get(index).is_some_and(u8::is_ascii_digit) {
                written_exponent = written_exponent
                    .saturating_mul(10)
                    .saturating_add(i64::from(bytes[index] - b'0'));
                index += 1;
            }
            if index == exponent_start {
                return None;
            }
            if negative {
                written_exponent = -written_exponent;
            }
        }
        if index != bytes.len() {
            return None;
        }
        let mut digits: String = integer.chars().chain(fraction.chars()).collect();
        let mut exponent = written_exponent.saturating_sub(fraction.len() as i64);
        let significant_start = digits.find(|c| c != '0').unwrap_or(digits.len());
        digits.drain(..significant_start);
        let trailing_zeros = digits.len() - digits.trim_end_matches('0').len();
        digits.truncate(digits.len() - trailing_zeros);
        exponent = exponent.saturating_add(trailing_zeros as i64);
        Some(Self { digits, exponent })
    }

    fn admit(&self, spelling: &str) -> Result<f64, NumericAdmissionError> {
        // The grammar above is a subset of what `f64::from_str` accepts, and
        // that parse is correctly rounded.
        let nearest: f64 = spelling.parse().map_err(|_| NumericAdmissionError::NotFinite)?;
        if self.digits.is_empty() {
            return Ok(nearest);
        }
        if !nearest.is_finite() {
            return Err(NumericAdmissionError::NotFinite);
        }
        if nearest == 0.0 {
            return Err(NumericAdmissionError::Underflow);
        }
        if self.exponent < 0 {
            // Trailing zeros were stripped, so a negative power of ten leaves
            // a non-zero fractional part: a decimal takes the nearest `f64`.
            return Ok(nearest);
        }
        // A finite `f64` is below 10^309, so a whole value that reached here
        // has at most 309 digits.
        let whole = format!("{}{}", self.digits, "0".repeat(self.exponent as usize));
        if format!("{:.0}", nearest.abs()) == whole {
            Ok(nearest)
        } else {
            Err(NumericAdmissionError::Inexact)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inexact() -> Result<f64, NumberTextError> {
        Err(NumberTextError::Admission(NumericAdmissionError::Inexact))
    }

    #[test]
    fn whole_numbers_are_exact_in_every_spelling() {
        assert_eq!(admit_number_text("9007199254740992"), Ok(9007199254740992.0));
        assert_eq!(admit_number_text("-9007199254740992"), Ok(-9007199254740992.0));
        for spelling in [
            "9007199254740993",
            "-9007199254740993",
            "9007199254740993.0",
            "9.007199254740993e15",
            "18446744073709551615",
            "99999999999999999999",
            "1e300",
        ] {
            assert_eq!(admit_number_text(spelling), inexact(), "{spelling}");
        }
        assert_eq!(admit_number_text("1e3"), Ok(1000.0));
        assert_eq!(admit_number_text("4.0"), Ok(4.0));
        assert_eq!(admit_number_text("1e22"), Ok(1e22));
        assert_eq!(admit_number_text("1e23"), inexact());
    }

    #[test]
    fn decimals_round_and_extremes_are_rejected() {
        assert_eq!(admit_number_text("0.1"), Ok(0.1));
        assert_eq!(admit_number_text("1.2"), Ok(1.2));
        assert_eq!(admit_number_text("-3.5"), Ok(-3.5));
        assert_eq!(
            admit_number_text("1e400"),
            Err(NumberTextError::Admission(NumericAdmissionError::NotFinite))
        );
        assert_eq!(
            admit_number_text("1e-400"),
            Err(NumberTextError::Admission(NumericAdmissionError::Underflow))
        );
        assert_eq!(admit_number_text("0e-400"), Ok(0.0));
        assert_eq!(admit_number_text("0.000"), Ok(0.0));
    }

    #[test]
    fn lenient_text_spellings() {
        for (text, expected) in [(" 4 ", 4.0), ("+4", 4.0), (".5", 0.5), ("1_000", 1000.0), ("04", 4.0), ("2.5E-2", 0.025)] {
            assert_eq!(admit_number_text(text), Ok(expected), "{text}");
        }
        for text in ["", "  ", "5.", "_1", "1_", "1__0", "1,000", "NaN", "inf", "Infinity", "0x10", "pear", "1e", "+", "-.", "4 4"] {
            assert_eq!(admit_number_text(text), Err(NumberTextError::NotANumber), "{text:?}");
        }
    }

    #[test]
    fn loader_spellings_allow_a_trailing_dot_but_not_separators() {
        assert_eq!(admit_number_spelling("5."), Ok(5.0));
        assert_eq!(admit_number_spelling(".0"), Ok(0.0));
        assert_eq!(admit_number_spelling("1_000"), Err(NumberTextError::NotANumber));
        assert_eq!(admit_number_spelling(" 4"), Err(NumberTextError::NotANumber));
    }

    #[test]
    fn json_numbers_and_supplied_floats() {
        assert_eq!(admit_json_number(&serde_json::Number::from(9007199254740992_u64)), Ok(9007199254740992.0));
        assert_eq!(
            admit_json_number(&serde_json::Number::from(9007199254740993_u64)),
            Err(NumericAdmissionError::Inexact)
        );
        assert_eq!(admit_json_number(&serde_json::Number::from(u64::MAX)), Err(NumericAdmissionError::Inexact));
        assert_eq!(admit_json_number(&serde_json::Number::from(i64::MIN)), Ok(-(2f64.powi(63))));
        assert_eq!(admit_json_number(&serde_json::Number::from(i64::MAX)), Err(NumericAdmissionError::Inexact));
        assert_eq!(admit_f64(0.1), Ok(0.1));
        assert_eq!(admit_f64(f64::INFINITY), Err(NumericAdmissionError::NotFinite));
        assert_eq!(admit_f64(f64::NAN), Err(NumericAdmissionError::NotFinite));
    }

    #[test]
    fn boolean_words_ignore_case_and_surrounding_whitespace() {
        for word in ["true", "Yes", " ON ", "1"] {
            assert_eq!(parse_boolean_word(word), Some(true), "{word}");
        }
        for word in ["FALSE", "no", "Off", "0"] {
            assert_eq!(parse_boolean_word(word), Some(false), "{word}");
        }
        for word in ["", "maybe", "y", "o n", "2"] {
            assert_eq!(parse_boolean_word(word), None, "{word}");
        }
    }

    #[test]
    fn presentation_never_narrows() {
        assert_eq!(format_number(4.0), "4");
        assert_eq!(format_number(2.5), "2.5");
        assert_eq!(format_number(-0.0), "0");
        assert_eq!(format_number(9007199254740992.0), "9007199254740992");
        assert_eq!(format_number(1e20), "100000000000000000000");
        assert_eq!(format_number(0.1), "0.1");
    }

    #[test]
    fn json_numbers_store_whole_values_as_integers() {
        assert_eq!(json_number(4.0), Some(serde_json::Number::from(4)));
        assert_eq!(json_number(1e19), Some(serde_json::Number::from(10_000_000_000_000_000_000_u64)));
        assert_eq!(json_number(2.5), serde_json::Number::from_f64(2.5));
        assert_eq!(json_number(1e300), serde_json::Number::from_f64(1e300));
        assert_eq!(json_number(f64::NAN), None);
    }
}
