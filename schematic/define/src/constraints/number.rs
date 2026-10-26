//! Lossless numeric limits.

use std::cmp::Ordering;
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The largest decimal exponent a limit may have, in either direction.
///
/// Matches the runtime checker's bound, so every limit a definition accepts
/// is one the generated client can evaluate.
pub const MAX_EXPONENT: i64 = 10_000;

/// A numeric limit held as JSON number text, so a bound beyond `i64`/`u64`
/// or a decimal such as `0.1` is never narrowed through a float.
///
/// Builders accept integers, floats (through their shortest round-trip
/// text), and decimal text; text is kept exactly as written. Serialized
/// definitions store the text as a JSON string.
///
/// ## Examples
///
/// ```
/// use schematic_define::constraints::ConstraintNumber;
///
/// assert_eq!(ConstraintNumber::from(100).as_str(), "100");
/// assert_eq!(ConstraintNumber::from(0.1).as_str(), "0.1");
/// assert_eq!(ConstraintNumber::from("18446744073709551616").as_str(), "18446744073709551616");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstraintNumber(String);

impl ConstraintNumber {
    /// The number's text as written.
    pub fn as_str(&self) -> &str { &self.0 }

    pub(crate) fn exact(&self) -> Result<Exact, &'static str> { Exact::parse(&self.0) }
}

impl fmt::Display for ConstraintNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.0) }
}

macro_rules! from_display {
    ($($t:ty),*) => {$(
        impl From<$t> for ConstraintNumber {
            fn from(value: $t) -> Self { Self(value.to_string()) }
        }
    )*};
}
// Rust's float `Display` writes the shortest text that round-trips, in plain
// (never exponent) notation, so `0.1_f32` is `0.1`. NaN and infinities write
// text that is not a JSON number and fail definition validation.
from_display!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64);

impl From<&str> for ConstraintNumber {
    fn from(value: &str) -> Self { Self(value.to_owned()) }
}
impl From<String> for ConstraintNumber {
    fn from(value: String) -> Self { Self(value) }
}

impl Serialize for ConstraintNumber {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> { serializer.serialize_str(&self.0) }
}

impl<'de> Deserialize<'de> for ConstraintNumber {
    /// Only a string holding JSON number text within [`MAX_EXPONENT`]; a
    /// JSON number is a wrong type, because reading it would narrow it.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Exact::parse(&text).map_err(|reason| serde::de::Error::custom(format!("numeric limit {reason}")))?;
        Ok(Self(text))
    }
}

/// An exact decimal: `digits × 10^exponent`, with no leading or trailing
/// zero digits; zero has no digits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Exact {
    negative: bool,
    digits: Vec<u8>,
    exponent: i64,
}

impl Exact {
    /// Parses JSON number text (RFC 8259 grammar).
    pub(crate) fn parse(text: &str) -> Result<Self, &'static str> {
        const SYNTAX: &str = "is not JSON number text";
        let bytes = text.as_bytes();
        let mut at = 0;
        let negative = bytes.first() == Some(&b'-');
        if negative { at += 1; }
        let digit_run = |from: usize| bytes[from..].iter().take_while(|byte| byte.is_ascii_digit()).count();

        let integer = &bytes[at..at + digit_run(at)];
        at += integer.len();
        if integer.is_empty() || (integer.len() > 1 && integer[0] == b'0') { return Err(SYNTAX); }
        let mut fraction: &[u8] = &[];
        if bytes.get(at) == Some(&b'.') {
            at += 1;
            fraction = &bytes[at..at + digit_run(at)];
            at += fraction.len();
            if fraction.is_empty() { return Err(SYNTAX); }
        }
        let mut written: i64 = 0;
        if matches!(bytes.get(at), Some(b'e' | b'E')) {
            at += 1;
            let sign_negative = match bytes.get(at) {
                Some(b'-') => { at += 1; true }
                Some(b'+') => { at += 1; false }
                _ => false,
            };
            let digits = &bytes[at..at + digit_run(at)];
            at += digits.len();
            if digits.is_empty() { return Err(SYNTAX); }
            for &digit in digits {
                written = written.saturating_mul(10).saturating_add(i64::from(digit - b'0'));
            }
            if sign_negative { written = -written; }
        }
        if at != bytes.len() { return Err(SYNTAX); }

        let all: Vec<u8> = integer.iter().chain(fraction).copied().collect();
        let leading = all.iter().take_while(|&&digit| digit == b'0').count();
        let significant = &all[leading..];
        let trailing = significant.iter().rev().take_while(|&&digit| digit == b'0').count();
        let significant = &significant[..significant.len() - trailing];
        if significant.is_empty() { return Ok(Self { negative: false, digits: vec![], exponent: 0 }); }
        let exponent = written.saturating_sub(fraction.len() as i64).saturating_add(trailing as i64);
        if exponent.abs() > MAX_EXPONENT { return Err("has an exponent beyond ±10000"); }
        Ok(Self { negative, digits: significant.to_vec(), exponent })
    }

    pub(crate) fn is_positive(&self) -> bool { !self.negative && !self.digits.is_empty() }
}

impl Ord for Exact {
    fn cmp(&self, other: &Self) -> Ordering {
        let sign = |value: &Self| if value.digits.is_empty() { 0 } else if value.negative { -1 } else { 1 };
        match sign(self).cmp(&sign(other)) {
            Ordering::Equal if sign(self) == 0 => Ordering::Equal,
            Ordering::Equal => {
                // Same sign: compare magnitudes by the position of the leading
                // digit, then digit by digit (no trailing zeros, so a proper
                // prefix is the smaller magnitude).
                let magnitude = (self.exponent + self.digits.len() as i64)
                    .cmp(&(other.exponent + other.digits.len() as i64))
                    .then_with(|| self.digits.cmp(&other.digits));
                if self.negative { magnitude.reverse() } else { magnitude }
            }
            unequal => unequal,
        }
    }
}
impl PartialOrd for Exact {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact(text: &str) -> Exact { Exact::parse(text).unwrap() }

    #[test]
    fn spellings_of_one_value_are_equal() {
        assert_eq!(exact("1"), exact("1.0"));
        assert_eq!(exact("1"), exact("1e0"));
        assert_eq!(exact("100"), exact("1E2"));
        assert_eq!(exact("0"), exact("-0.000"));
    }

    #[test]
    fn ordering_is_mathematical() {
        let ascending = ["-18446744073709551617", "-1.5", "-1", "-0.1", "0", "0.1", "0.35", "1", "1.01", "9.9", "10",
            "9223372036854775808", "18446744073709551616", "1e30"];
        for pair in ascending.windows(2) {
            assert!(exact(pair[0]) < exact(pair[1]), "{} < {}", pair[0], pair[1]);
        }
    }

    #[test]
    fn malformed_text_and_huge_exponents_are_refused() {
        for text in ["", "-", "01", "1.", ".5", "1e", "+1", "0x10", "NaN", "inf", "1 ", " 1", "1e+"] {
            assert!(Exact::parse(text).is_err(), "{text:?}");
        }
        assert!(Exact::parse("1e10000").is_ok());
        assert!(Exact::parse("1e10001").is_err());
        assert!(Exact::parse("1e-10001").is_err());
    }
}
