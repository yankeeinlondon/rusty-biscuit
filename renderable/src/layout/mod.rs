//! Target-agnostic layout configuration.

mod edges;
mod length;
mod target_value;
mod width;

pub use edges::{Alignment, Edges};
pub use length::{LayoutError, Length};
pub use target_value::TargetValue;
pub use width::Width;

pub use crate::wrap_policy::WordWrap;

use serde::{Deserialize, Serialize};

/// A block-level component's CSS box: margins, padding, content-box width,
/// max-width, alignment within the parent, and content wrapping.
///
/// Inline components carry no `Layout`. Appearance (background, border) is a
/// `Style` concern and is not represented here; `padding` is reserved here but
/// painted by `Style.background`.
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
    pub margin: Edges,
    /// Reserved inner space, painted by `Style.background`. `#[serde(default)]`
    /// so trees serialized before `padding` existed deserialize to zero.
    #[serde(default)]
    pub padding: Edges,
    /// Content-box width mode. `#[serde(default)]` → `Width::Auto`.
    #[serde(default)]
    pub width: Width,
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
            margin: Edges::default(),
            padding: Edges::default(),
            width: Width::Auto,
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
    /// Propagates the first [`LayoutError`] from `margin`, `padding`, `width`,
    /// or `max_width`.
    pub fn validate(&self) -> Result<(), LayoutError> {
        self.margin.validate()?;
        self.padding.validate()?;
        self.width.validate()?;
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
        assert_eq!(layout.margin, Edges::default());
        assert_eq!(layout.alignment, Alignment::Left);
        assert!(layout.max_width.is_none());
    }

    #[test]
    fn default_layout_pins_every_field() {
        // The defaulting contract: each field reproduces today's un-styled
        // node. `word_wrap` is hand-written to `WordWrap::None`, NOT
        // `WordWrap::default()`, so it is pinned explicitly here.
        let layout = Layout::default();
        assert_eq!(layout.margin, Edges::default());
        assert_eq!(layout.padding, Edges::default());
        assert_eq!(layout.width, Width::Auto);
        assert_eq!(layout.max_width, None);
        assert_eq!(layout.alignment, Alignment::Left);
        assert_eq!(layout.word_wrap, WordWrap::None);
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
            margin: Edges::x(Length::ch(2)),
            alignment: Alignment::Center,
            ..Layout::default()
        };
        let json = serde_json::to_string(&layout).unwrap();
        let back: Layout = serde_json::from_str(&json).unwrap();
        assert_eq!(layout, back);
    }

    #[test]
    fn default_layout_has_zero_padding_and_auto_width() {
        let layout = Layout::default();
        assert_eq!(layout.padding, Edges::default());
        assert_eq!(layout.width, Width::Auto);
    }

    #[test]
    fn old_payload_without_padding_or_width_deserializes_with_defaults() {
        // A render tree serialized before `padding`/`width` existed.
        let json = r#"{
            "margin": {
                "top": { "universal": "zero" },
                "right": { "universal": "zero" },
                "bottom": { "universal": "zero" },
                "left": { "universal": { "ch": 2 } }
            },
            "alignment": "left",
            "max_width": null,
            "word_wrap": "none"
        }"#;
        let layout: Layout = serde_json::from_str(json).unwrap();
        assert_eq!(layout.padding, Edges::default());
        assert_eq!(layout.width, Width::Auto);
        assert_eq!(layout.margin.left, TargetValue::universal(Length::ch(2)));
    }

    #[test]
    fn validate_rejects_bad_padding_percent() {
        let layout = Layout {
            padding: Edges {
                left: TargetValue::universal(Length::Percent(150.0)),
                ..Edges::default()
            },
            ..Layout::default()
        };
        assert!(layout.validate().is_err());
    }

    #[test]
    fn validate_rejects_bad_fixed_width_percent() {
        let layout = Layout {
            width: Width::Fixed(TargetValue::universal(Length::Percent(150.0))),
            ..Layout::default()
        };
        assert!(layout.validate().is_err());
    }

    #[test]
    fn layout_with_padding_and_width_serde_roundtrips() {
        let layout = Layout {
            padding: Edges::x(Length::ch(4)),
            width: Width::Fixed(TargetValue::universal(Length::ch(60))),
            ..Layout::default()
        };
        let json = serde_json::to_string(&layout).unwrap();
        let back: Layout = serde_json::from_str(&json).unwrap();
        assert_eq!(layout, back);
    }
}
