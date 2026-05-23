//! Target-agnostic layout configuration.

mod length;
mod margin;
mod target_value;

pub use length::{Length, LayoutError};
pub use margin::{Alignment, Margin};
pub use target_value::TargetValue;

pub use crate::wrap_policy::WordWrap;

use serde::{Deserialize, Serialize};

/// A block-level component's relationship to its parent: margins, alignment
/// within the parent, max-width, and content wrapping.
///
/// Inline components carry no `Layout`. Appearance (background, fill) is a
/// `Style` concern and is not represented here.
///
/// ## Serialized shape
///
/// `Layout` rides on `NodeAttrs` and serializes with the render tree. A node
/// with `margin-left: 2ch` and `margin-right` differing per target:
///
/// ```json
/// {
///   "margin": {
///     "top": { "universal": "zero" },
///     "right": { "per_target": { "browser": { "css": "5em" },
///                                "terminal": { "ch": 5 } } },
///     "bottom": { "universal": "zero" },
///     "left": { "universal": { "ch": 2 } }
///   },
///   "alignment": "left",
///   "max_width": null,
///   "word_wrap": "none"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    /// Outer margins.
    pub margin: Margin,
    /// Alignment within the parent's available width.
    pub alignment: Alignment,
    /// Optional cap on content width.
    pub max_width: Option<TargetValue<Length>>,
    /// Content-wrapping policy.
    pub word_wrap: WordWrap,
}

impl Default for Layout {
    /// Zero margins, left-aligned, no max-width, and no content wrapping.
    ///
    /// `word_wrap` defaults to [`WordWrap::None`] rather than
    /// [`WordWrap::default()`] so that block components do not wrap unless a
    /// caller opts in.
    fn default() -> Self {
        Self {
            margin: Margin::default(),
            alignment: Alignment::default(),
            max_width: None,
            word_wrap: WordWrap::None,
        }
    }
}

impl Layout {
    /// Validate every length-valued field.
    ///
    /// ## Errors
    /// Propagates the first [`LayoutError`] from `margin` or `max_width`.
    pub fn validate(&self) -> Result<(), LayoutError> {
        self.margin.validate()?;
        if let Some(max_width) = &self.max_width {
            max_width.validate()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layout_is_zero_margins_left_aligned() {
        let layout = Layout::default();
        assert_eq!(layout.margin, Margin::default());
        assert_eq!(layout.alignment, Alignment::Left);
        assert!(layout.max_width.is_none());
    }

    #[test]
    fn validate_rejects_bad_max_width() {
        let layout = Layout {
            max_width: Some(TargetValue::universal(Length::Percent(200.0))),
            ..Layout::default()
        };
        assert!(layout.validate().is_err());
    }

    #[test]
    fn layout_serde_roundtrip() {
        let layout = Layout {
            margin: Margin::x(Length::ch(2)),
            alignment: Alignment::Center,
            ..Layout::default()
        };
        let json = serde_json::to_string(&layout).unwrap();
        let back: Layout = serde_json::from_str(&json).unwrap();
        assert_eq!(layout, back);
    }
}
