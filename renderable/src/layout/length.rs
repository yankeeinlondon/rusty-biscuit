//! The `Length` layout value.

use serde::{Deserialize, Serialize};

use crate::stylesheet::CssSizing;

/// A layout length.
///
/// `Zero`, `Ch`, and `Percent` are the **universal units** — valid on every
/// render target. `Css` carries a target-native value and is valid only
/// inside the per-target branch of a [`TargetValue`](super::TargetValue).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Length {
    /// Zero — unit-independent.
    Zero,
    /// Whole cells. Columns on horizontal sides, rows on vertical sides.
    Ch(u32),
    /// Percentage of the available width, `0.0..=100.0`.
    Percent(f32),
    /// A target-native CSS length. Only valid in a per-target branch.
    Css(CssSizing),
}

/// An error constructing or validating a layout value.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum LayoutError {
    /// A percentage outside `0.0..=100.0`, or non-finite.
    #[error("invalid percentage `{0}`: must be a finite value in 0.0..=100.0")]
    InvalidPercent(f32),
    /// A `Length::Css` value used in a `TargetValue::Universal` branch.
    #[error(
        "non-universal unit in a universal value: `{0}`; use a per-target \
         map (e.g. {{ browser: ..., terminal: ... }}) for target-native units"
    )]
    NonUniversalUnit(String),
    /// An empty `TargetValue::PerTarget` map.
    #[error("per-target value map is empty")]
    EmptyPerTarget,
}

impl Length {
    /// Zero length.
    pub fn zero() -> Length {
        Length::Zero
    }

    /// `n` whole cells.
    pub fn ch(n: u32) -> Length {
        Length::Ch(n)
    }

    /// A validated percentage in `0.0..=100.0`.
    ///
    /// ## Errors
    /// [`LayoutError::InvalidPercent`] when `pct` is non-finite or out of range.
    pub fn percent(pct: f32) -> Result<Length, LayoutError> {
        if pct.is_finite() && (0.0..=100.0).contains(&pct) {
            Ok(Length::Percent(pct))
        } else {
            Err(LayoutError::InvalidPercent(pct))
        }
    }

    /// A target-native CSS length.
    pub fn css(sizing: CssSizing) -> Length {
        Length::Css(sizing)
    }

    /// Whether this length uses a universal unit (valid on every target).
    pub fn is_universal(&self) -> bool {
        !matches!(self, Length::Css(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_rejects_out_of_range() {
        assert_eq!(Length::percent(150.0), Err(LayoutError::InvalidPercent(150.0)));
        assert_eq!(Length::percent(-1.0), Err(LayoutError::InvalidPercent(-1.0)));
        assert!(matches!(
            Length::percent(f32::NAN),
            Err(LayoutError::InvalidPercent(_))
        ));
    }

    #[test]
    fn percent_accepts_in_range() {
        assert_eq!(Length::percent(0.0), Ok(Length::Percent(0.0)));
        assert_eq!(Length::percent(100.0), Ok(Length::Percent(100.0)));
    }

    #[test]
    fn is_universal_is_false_only_for_css() {
        assert!(Length::zero().is_universal());
        assert!(Length::ch(4).is_universal());
        assert!(Length::Percent(50.0).is_universal());
        assert!(!Length::css(CssSizing::px(8.0)).is_universal());
    }

    #[test]
    fn length_serde_roundtrip() {
        for value in [Length::Zero, Length::Ch(4), Length::Percent(50.0)] {
            let json = serde_json::to_string(&value).unwrap();
            let back: Length = serde_json::from_str(&json).unwrap();
            assert_eq!(value, back);
        }
    }
}
