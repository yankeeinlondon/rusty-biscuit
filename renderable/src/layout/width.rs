//! The `Width` content-box sizing mode for [`Layout`](super::Layout).

use serde::{Deserialize, Serialize};

use crate::layout::length::LayoutError;
use crate::layout::{Length, TargetValue};

/// How a block sizes its content box horizontally (CSS `width`).
///
/// Composes with [`Layout::max_width`](super::Layout::max_width): the cap is a
/// separate, orthogonal field, so `FitContent` + a `max_width` cap, or `Auto`
/// + a cap, are both expressible.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Width {
    /// Fill the parent's available width (CSS `width: auto` on a block).
    #[default]
    Auto,
    /// Size to the content's widest line (CSS `width: fit-content`).
    FitContent,
    /// An explicit width (cells / percent / per-target CSS length).
    Fixed(TargetValue<Length>),
}

impl Width {
    /// The content-hugging width mode.
    pub fn fit_content() -> Width {
        Width::FitContent
    }

    /// Validate the contained length, if any.
    ///
    /// ## Errors
    /// Propagates the first [`LayoutError`] from a `Fixed` value's
    /// [`TargetValue::validate`].
    pub fn validate(&self) -> Result<(), LayoutError> {
        match self {
            Width::Auto | Width::FitContent => Ok(()),
            Width::Fixed(value) => value.validate(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Length, TargetValue};

    #[test]
    fn default_is_auto() {
        assert_eq!(Width::default(), Width::Auto);
    }

    #[test]
    fn serde_tags_are_snake_case() {
        assert_eq!(serde_json::to_string(&Width::Auto).unwrap(), "\"auto\"");
        assert_eq!(
            serde_json::to_string(&Width::FitContent).unwrap(),
            "\"fit_content\""
        );
        let fixed = Width::Fixed(TargetValue::universal(Length::ch(60)));
        let json = serde_json::to_string(&fixed).unwrap();
        assert_eq!(json, r#"{"fixed":{"universal":{"ch":60}}}"#);
        assert_eq!(serde_json::from_str::<Width>(&json).unwrap(), fixed);
    }

    #[test]
    fn fit_content_constructor() {
        assert_eq!(Width::fit_content(), Width::FitContent);
    }
}
