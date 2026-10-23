//! Exact decimal numbers for numeric constraints.

use std::cmp::Ordering;
use std::fmt;

use num_bigint::BigUint;

/// The largest decimal exponent magnitude a [`Decimal`] accepts.
///
/// The bound applies to the exponent after trailing zeros move into it, so
/// `1e10000` and `1e-10000` parse and `1e10001` does not. It keeps the
/// scaling done by comparisons and [`Decimal::is_multiple_of`] bounded; a
/// definition carrying a larger exponent is refused when it is validated.
pub const MAX_EXPONENT: i64 = 10_000;

/// An exact decimal number: a sign, an arbitrary-precision mantissa, and a
/// power-of-ten exponent.
///
/// Equality and ordering are mathematical, so `1`, `1.0`, and `1e0` are
/// equal, and `0.1` is exact rather than the nearest binary fraction. The
/// representation is canonical (no trailing zeros in the mantissa, and zero
/// is always positive with exponent 0), which makes the derived `Eq` and
/// `Hash` mathematical as well.
///
/// ## Examples
///
/// ```
/// use schematic_runtime::validate::Decimal;
///
/// let tenth = Decimal::parse("0.1").unwrap();
/// assert_eq!(Decimal::parse("1").unwrap(), Decimal::parse("1.0e0").unwrap());
/// assert_eq!(Decimal::parse("0.3").unwrap().is_multiple_of(&tenth), Some(true));
/// assert_eq!(Decimal::parse("0.35").unwrap().is_multiple_of(&tenth), Some(false));
/// assert!(Decimal::parse("18446744073709551616").unwrap() > Decimal::parse("18446744073709551615").unwrap());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Decimal {
    negative: bool,
    mantissa: BigUint,
    /// Decimal digits in `mantissa`; 0 for zero.
    digits: u64,
    exponent: i64,
}

/// Why text is not a [`Decimal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecimalError {
    /// The text is not a JSON number.
    Syntax,
    /// The exponent is beyond [`MAX_EXPONENT`].
    ExponentOutOfRange,
}

impl fmt::Display for DecimalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax => f.write_str("not a JSON number"),
            Self::ExponentOutOfRange => write!(f, "the exponent is beyond ±{MAX_EXPONENT}"),
        }
    }
}

impl std::error::Error for DecimalError {}

impl Decimal {
    /// Parses JSON number text, such as `-12`, `0.1`, or `1.5e+30`.
    ///
    /// The grammar is JSON's: no leading `+`, no leading zeros, digits on
    /// both sides of a decimal point, and no surrounding whitespace.
    ///
    /// ## Errors
    ///
    /// [`DecimalError::Syntax`] when `text` is not a JSON number, and
    /// [`DecimalError::ExponentOutOfRange`] when its canonical exponent is
    /// beyond [`MAX_EXPONENT`].
    pub fn parse(text: &str) -> Result<Self, DecimalError> {
        let bytes = text.as_bytes();
        let mut at = 0;
        let negative = bytes.first() == Some(&b'-');
        if negative {
            at += 1;
        }

        let integer_start = at;
        at += digit_run(&bytes[at..]);
        let integer = &bytes[integer_start..at];
        if integer.is_empty() || (integer.len() > 1 && integer[0] == b'0') {
            return Err(DecimalError::Syntax);
        }

        let mut fraction: &[u8] = &[];
        if bytes.get(at) == Some(&b'.') {
            at += 1;
            let start = at;
            at += digit_run(&bytes[at..]);
            fraction = &bytes[start..at];
            if fraction.is_empty() {
                return Err(DecimalError::Syntax);
            }
        }

        let mut written_exponent: i64 = 0;
        if matches!(bytes.get(at), Some(b'e' | b'E')) {
            at += 1;
            let exponent_negative = match bytes.get(at) {
                Some(b'-') => {
                    at += 1;
                    true
                }
                Some(b'+') => {
                    at += 1;
                    false
                }
                _ => false,
            };
            let start = at;
            at += digit_run(&bytes[at..]);
            let digits = &bytes[start..at];
            if digits.is_empty() {
                return Err(DecimalError::Syntax);
            }
            for &digit in digits {
                // Saturate: anything past i64 is far beyond the bound anyway.
                written_exponent = written_exponent.saturating_mul(10).saturating_add(i64::from(digit - b'0'));
            }
            if exponent_negative {
                written_exponent = -written_exponent;
            }
        }
        if at != bytes.len() {
            return Err(DecimalError::Syntax);
        }

        let mut all_digits = Vec::with_capacity(integer.len() + fraction.len());
        all_digits.extend_from_slice(integer);
        all_digits.extend_from_slice(fraction);
        let leading = all_digits.iter().take_while(|&&digit| digit == b'0').count();
        let significant = &all_digits[leading..];
        let trailing = significant.iter().rev().take_while(|&&digit| digit == b'0').count();
        let significant = &significant[..significant.len() - trailing];
        if significant.is_empty() {
            return Ok(Self::zero());
        }

        let exponent = written_exponent
            .saturating_sub(fraction.len() as i64)
            .saturating_add(trailing as i64);
        if exponent.abs() > MAX_EXPONENT {
            return Err(DecimalError::ExponentOutOfRange);
        }
        let mantissa = BigUint::parse_bytes(significant, 10).ok_or(DecimalError::Syntax)?;
        Ok(Self {
            negative,
            mantissa,
            digits: significant.len() as u64,
            exponent,
        })
    }

    /// The exact value of a JSON number.
    ///
    /// A number that `serde_json` holds as a float is read through the
    /// shortest text that round-trips it, which is the text `serde_json`
    /// writes when the value is sent: a request field of `0.3` is `0.3`
    /// here, not the nearest binary fraction.
    ///
    /// ## Errors
    ///
    /// [`DecimalError::ExponentOutOfRange`] for a number beyond
    /// [`MAX_EXPONENT`], which only a `serde_json` build that keeps
    /// arbitrary-precision number text can hold.
    pub fn from_json(number: &serde_json::Number) -> Result<Self, DecimalError> {
        Self::parse(&number.to_string())
    }

    /// Whether this number is an integral multiple of `divisor`, or `None`
    /// when `divisor` is not positive.
    ///
    /// Zero is a multiple of every positive divisor.
    pub fn is_multiple_of(&self, divisor: &Self) -> Option<bool> {
        if divisor.negative || divisor.is_zero() {
            return None;
        }
        if self.is_zero() {
            return Some(true);
        }
        let common = self.exponent.min(divisor.exponent);
        let dividend = scaled(&self.mantissa, self.exponent - common);
        let divisor = scaled(&divisor.mantissa, divisor.exponent - common);
        Some(dividend % divisor == BigUint::ZERO)
    }

    /// Whether this number is zero.
    pub fn is_zero(&self) -> bool {
        self.digits == 0
    }

    /// Whether this number is below zero.
    pub fn is_negative(&self) -> bool {
        self.negative
    }

    fn zero() -> Self {
        Self {
            negative: false,
            mantissa: BigUint::ZERO,
            digits: 0,
            exponent: 0,
        }
    }

    /// The power of ten of the leading digit; meaningless for zero.
    fn magnitude(&self) -> i64 {
        self.exponent + self.digits as i64 - 1
    }

    fn cmp_magnitude(&self, other: &Self) -> Ordering {
        match (self.is_zero(), other.is_zero()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            (false, false) => {}
        }
        self.magnitude().cmp(&other.magnitude()).then_with(|| {
            // Equal leading powers: align the exponents, which differ by at
            // most the longer mantissa's digit count.
            let common = self.exponent.min(other.exponent);
            scaled(&self.mantissa, self.exponent - common).cmp(&scaled(&other.mantissa, other.exponent - common))
        })
    }
}

impl Ord for Decimal {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => self.cmp_magnitude(other),
            (true, true) => other.cmp_magnitude(self),
        }
    }
}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Plain notation when the number has at most 21 integer digits or six
/// leading fractional zeros, scientific notation otherwise; the same
/// thresholds as JavaScript's number printing.
impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_zero() {
            return f.write_str("0");
        }
        if self.negative {
            f.write_str("-")?;
        }
        let digits = self.mantissa.to_string();
        let point = self.digits as i64 + self.exponent;
        if self.exponent >= 0 && point <= 21 {
            write!(f, "{digits}{}", "0".repeat(self.exponent as usize))
        } else if self.exponent < 0 && point > 0 {
            let (whole, fraction) = digits.split_at(point as usize);
            write!(f, "{whole}.{fraction}")
        } else if self.exponent < 0 && point > -6 {
            write!(f, "0.{}{digits}", "0".repeat(-point as usize))
        } else {
            let (lead, rest) = digits.split_at(1);
            let separator = if rest.is_empty() { "" } else { "." };
            write!(f, "{lead}{separator}{rest}e{}", self.magnitude())
        }
    }
}

fn digit_run(bytes: &[u8]) -> usize {
    bytes.iter().take_while(|byte| byte.is_ascii_digit()).count()
}

/// `mantissa × 10^shift`; `shift` is nonnegative and bounded by the
/// exponent bound.
fn scaled(mantissa: &BigUint, shift: i64) -> BigUint {
    mantissa * BigUint::from(10u8).pow(shift as u32)
}
