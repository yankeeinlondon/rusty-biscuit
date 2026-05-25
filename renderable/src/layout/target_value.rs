//! Per-target layout values.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::layout::Length;
use crate::layout::length::LayoutError;
use crate::target::RenderTarget;

/// A layout value that is either universal or specified per render target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetValue<T> {
    /// One value for every target. Universal-units only (for `Length`).
    Universal(T),
    /// Per-target values. Non-empty; each entry may use that target's
    /// native units. A target not named here does not receive the property.
    PerTarget(BTreeMap<RenderTarget, T>),
}

impl<T> TargetValue<T> {
    /// A universal value.
    pub fn universal(value: T) -> TargetValue<T> {
        TargetValue::Universal(value)
    }

    /// Resolve the value for `target`.
    ///
    /// `Universal` always resolves. `PerTarget` looks up `target`; a
    /// `MarkdownPlus` lookup falls back to the `Markdown` entry. Returns
    /// `None` when a `PerTarget` map names neither.
    pub fn resolve(&self, target: RenderTarget) -> Option<&T> {
        match self {
            TargetValue::Universal(value) => Some(value),
            TargetValue::PerTarget(map) => map.get(&target).or_else(|| {
                if target == RenderTarget::MarkdownPlus {
                    map.get(&RenderTarget::Markdown)
                } else {
                    None
                }
            }),
        }
    }
}

impl TargetValue<Length> {
    /// Validate this length value.
    ///
    /// ## Errors
    /// - [`LayoutError::NonUniversalUnit`] — a `Length::Css` in the
    ///   `Universal` branch.
    /// - [`LayoutError::EmptyPerTarget`] — an empty `PerTarget` map.
    /// - [`LayoutError::InvalidPercent`] — a non-finite / out-of-range percent.
    pub fn validate(&self) -> Result<(), LayoutError> {
        match self {
            TargetValue::Universal(length) => {
                check_percent(length)?;
                if !length.is_universal() {
                    return Err(LayoutError::NonUniversalUnit(format!("{length:?}")));
                }
                Ok(())
            }
            TargetValue::PerTarget(map) => {
                if map.is_empty() {
                    return Err(LayoutError::EmptyPerTarget);
                }
                for length in map.values() {
                    check_percent(length)?;
                }
                Ok(())
            }
        }
    }
}

fn check_percent(length: &Length) -> Result<(), LayoutError> {
    if let Length::Percent(pct) = length
        && !(pct.is_finite() && (0.0..=100.0).contains(pct))
    {
        return Err(LayoutError::InvalidPercent(*pct));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stylesheet::CssSizing;

    #[test]
    fn universal_resolves_for_every_target() {
        let v = TargetValue::universal(Length::ch(2));
        for t in [
            RenderTarget::Terminal,
            RenderTarget::Browser,
            RenderTarget::Markdown,
            RenderTarget::MarkdownPlus,
        ] {
            assert_eq!(v.resolve(t), Some(&Length::ch(2)));
        }
    }

    #[test]
    fn per_target_resolves_named_targets_only() {
        let mut map = BTreeMap::new();
        map.insert(RenderTarget::Browser, Length::css(CssSizing::px(8.0)));
        map.insert(RenderTarget::Terminal, Length::ch(2));
        let v = TargetValue::PerTarget(map);
        assert_eq!(v.resolve(RenderTarget::Terminal), Some(&Length::ch(2)));
        assert_eq!(v.resolve(RenderTarget::Markdown), None);
    }

    #[test]
    fn markdown_plus_falls_back_to_markdown() {
        let mut map = BTreeMap::new();
        map.insert(RenderTarget::Markdown, Length::ch(1));
        let v = TargetValue::PerTarget(map);
        assert_eq!(v.resolve(RenderTarget::MarkdownPlus), Some(&Length::ch(1)));
    }

    #[test]
    fn validate_rejects_css_in_universal() {
        let v = TargetValue::universal(Length::css(CssSizing::rem(1.0)));
        assert!(matches!(
            v.validate(),
            Err(LayoutError::NonUniversalUnit(_))
        ));
    }

    #[test]
    fn validate_rejects_empty_per_target() {
        let v: TargetValue<Length> = TargetValue::PerTarget(BTreeMap::new());
        assert_eq!(v.validate(), Err(LayoutError::EmptyPerTarget));
    }

    #[test]
    fn validate_accepts_css_in_per_target() {
        let mut map = BTreeMap::new();
        map.insert(RenderTarget::Browser, Length::css(CssSizing::rem(1.0)));
        assert!(TargetValue::PerTarget(map).validate().is_ok());
    }
}
