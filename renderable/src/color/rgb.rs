use serde::{Deserialize, Serialize};

use super::{BasicColor, Octet};

/// An RGB color with a fallback for terminals with limited color support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RgbColor {
    red: Octet,
    green: Octet,
    blue: Octet,
    fallback: BasicColor,
}

impl RgbColor {
    /// Creates a new RGB color with the specified channel values and fallback.
    #[inline]
    pub const fn new(red: u8, green: u8, blue: u8, fallback: BasicColor) -> Self {
        Self {
            red: Octet::new(red),
            green: Octet::new(green),
            blue: Octet::new(blue),
            fallback,
        }
    }

    /// Returns the red channel value.
    #[inline]
    pub const fn red(&self) -> u8 {
        self.red.value()
    }

    /// Returns the green channel value.
    #[inline]
    pub const fn green(&self) -> u8 {
        self.green.value()
    }

    /// Returns the blue channel value.
    #[inline]
    pub const fn blue(&self) -> u8 {
        self.blue.value()
    }

    /// Returns the fallback color for terminals with limited color support.
    #[inline]
    pub const fn fallback(&self) -> BasicColor {
        self.fallback
    }
}
