use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A single byte value (0-255) representing one RGB color channel.
///
/// This is a newtype wrapper around `u8` that provides type safety for color
/// components, ensuring values are always in the valid 0-255 range.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::utils::color::Octet;
///
/// // From a u8 value
/// let red = Octet::new(255);
///
/// // Using From/Into
/// let green: Octet = 128u8.into();
///
/// // Get the inner value
/// assert_eq!(green.value(), 128);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Octet(u8);

/// Error returned when an invalid value is provided for `Octet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum OctetError {
    #[error("value {0} is out of range for Octet (must be 0-255)")]
    OutOfRange(i32),
}

impl Octet {
    /// Creates a new `Octet` from a `u8` value.
    #[inline]
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    /// Creates a new `Octet` from an integer value, validating it's in range 0-255.
    ///
    /// ## Errors
    ///
    /// Returns `OctetError::OutOfRange` if the value is not in the range 0-255.
    #[inline]
    pub fn try_from_int<T: Into<i32>>(value: T) -> Result<Self, OctetError> {
        let v = value.into();
        if (0..=255).contains(&v) {
            Ok(Self(v as u8))
        } else {
            Err(OctetError::OutOfRange(v))
        }
    }

    /// Returns the inner `u8` value.
    #[inline]
    pub const fn value(self) -> u8 {
        self.0
    }
}

impl From<u8> for Octet {
    #[inline]
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl TryFrom<u16> for Octet {
    type Error = OctetError;

    #[inline]
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if value <= 255 {
            Ok(Self(value as u8))
        } else {
            Err(OctetError::OutOfRange(value as i32))
        }
    }
}

impl TryFrom<u32> for Octet {
    type Error = OctetError;

    #[inline]
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value <= 255 {
            Ok(Self(value as u8))
        } else {
            Err(OctetError::OutOfRange(value as i32))
        }
    }
}

impl TryFrom<u64> for Octet {
    type Error = OctetError;

    #[inline]
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if value <= 255 {
            Ok(Self(value as u8))
        } else {
            Err(OctetError::OutOfRange(value as i32))
        }
    }
}

impl TryFrom<i8> for Octet {
    type Error = OctetError;

    #[inline]
    fn try_from(value: i8) -> Result<Self, Self::Error> {
        if value >= 0 {
            Ok(Self(value as u8))
        } else {
            Err(OctetError::OutOfRange(value as i32))
        }
    }
}

impl TryFrom<i16> for Octet {
    type Error = OctetError;

    #[inline]
    fn try_from(value: i16) -> Result<Self, Self::Error> {
        if (0..=255).contains(&value) {
            Ok(Self(value as u8))
        } else {
            Err(OctetError::OutOfRange(value as i32))
        }
    }
}

impl TryFrom<i32> for Octet {
    type Error = OctetError;

    #[inline]
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if (0..=255).contains(&value) {
            Ok(Self(value as u8))
        } else {
            Err(OctetError::OutOfRange(value))
        }
    }
}

impl TryFrom<i64> for Octet {
    type Error = OctetError;

    #[inline]
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if (0..=255).contains(&value) {
            Ok(Self(value as u8))
        } else {
            Err(OctetError::OutOfRange(value as i32))
        }
    }
}

impl From<Octet> for u8 {
    #[inline]
    fn from(octet: Octet) -> Self {
        octet.0
    }
}
