//! Edge box (margins / padding) and alignment for [`Layout`](super::Layout).

use serde::{Deserialize, Serialize};

use crate::layout::length::LayoutError;
use crate::layout::{Length, TargetValue};

/// Horizontal alignment of a block within its parent's available width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum Alignment {
    /// Left-aligned (default).
    #[default]
    Left,
    /// Centered.
    Center,
    /// Right-aligned.
    Right,
}

/// A four-sided edge box (used for both `margin` and `padding`). Each side is
/// a [`TargetValue<Length>`].
///
/// All sides accept the same `Ch` / `Percent` / `Zero` units; the browser
/// renderer lowers vertical sides (`top` / `bottom`) to `lh` automatically.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edges {
    pub top: TargetValue<Length>,
    pub right: TargetValue<Length>,
    pub bottom: TargetValue<Length>,
    pub left: TargetValue<Length>,
}

impl Default for Edges {
    fn default() -> Edges {
        Edges {
            top: TargetValue::universal(Length::Zero),
            right: TargetValue::universal(Length::Zero),
            bottom: TargetValue::universal(Length::Zero),
            left: TargetValue::universal(Length::Zero),
        }
    }
}

impl Edges {
    /// An edge box with all four sides set to the same universal length.
    pub fn all(length: Length) -> Edges {
        Edges {
            top: TargetValue::universal(length.clone()),
            right: TargetValue::universal(length.clone()),
            bottom: TargetValue::universal(length.clone()),
            left: TargetValue::universal(length),
        }
    }

    /// An edge box with left + right set to `length`, top + bottom zero.
    pub fn x(length: Length) -> Edges {
        Edges {
            right: TargetValue::universal(length.clone()),
            left: TargetValue::universal(length),
            ..Edges::default()
        }
    }

    /// An edge box with top + bottom set to `length`, left + right zero.
    pub fn y(length: Length) -> Edges {
        Edges {
            top: TargetValue::universal(length.clone()),
            bottom: TargetValue::universal(length),
            ..Edges::default()
        }
    }

    /// Validate every side.
    ///
    /// ## Errors
    /// Propagates the first [`LayoutError`] from any side's
    /// [`TargetValue::validate`].
    pub fn validate(&self) -> Result<(), LayoutError> {
        self.top.validate()?;
        self.right.validate()?;
        self.bottom.validate()?;
        self.left.validate()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_all_zero() {
        let m = Edges::default();
        assert_eq!(m.left, TargetValue::universal(Length::Zero));
        assert_eq!(m.top, TargetValue::universal(Length::Zero));
    }

    #[test]
    fn x_sets_only_horizontal() {
        let m = Edges::x(Length::ch(4));
        assert_eq!(m.left, TargetValue::universal(Length::ch(4)));
        assert_eq!(m.right, TargetValue::universal(Length::ch(4)));
        assert_eq!(m.top, TargetValue::universal(Length::Zero));
    }

    #[test]
    fn validate_propagates_errors() {
        let m = Edges {
            left: TargetValue::universal(Length::Percent(150.0)),
            ..Edges::default()
        };
        assert!(m.validate().is_err());
    }

    #[test]
    fn alignment_default_is_left() {
        assert_eq!(Alignment::default(), Alignment::Left);
    }
}
