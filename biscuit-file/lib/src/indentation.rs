//! Indentation preferences shared by writers.
//!
//! These types carry no format knowledge, so they build without any format
//! feature. A caller resolves a preference for a destination (Sniff does this
//! from a repository's EditorConfig and formatter settings) and hands it to a
//! writer such as [`to_yaml_string`](crate::yaml::to_yaml_string). A caller
//! without a destination uses [`Indentation::default`]: four spaces.
//!
//! See `biscuit-file/docs/topics/yaml-writing.md`.

use std::num::NonZeroU8;

use serde::{Deserialize, Serialize};

/// The character an indentation level is written with.
///
/// Serializes as `space` or `tab`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IndentStyle {
    #[default]
    #[serde(rename = "space")]
    Spaces,
    #[serde(rename = "tab")]
    Tabs,
}

/// An indentation preference: a style and the columns one level occupies.
///
/// A format that forbids tabs (YAML) writes `width` spaces per level when
/// `style` is [`IndentStyle::Tabs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Indentation {
    pub style: IndentStyle,
    /// Columns per indentation level.
    pub width: NonZeroU8,
}

impl Indentation {
    /// The fallback width when nothing states a preference.
    pub const DEFAULT_WIDTH: NonZeroU8 = NonZeroU8::new(4).unwrap();

    #[must_use]
    pub const fn new(style: IndentStyle, width: NonZeroU8) -> Self {
        Self { style, width }
    }

    /// `width` spaces per level.
    #[must_use]
    pub const fn spaces(width: NonZeroU8) -> Self {
        Self::new(IndentStyle::Spaces, width)
    }
}

impl Default for Indentation {
    fn default() -> Self {
        Self::spaces(Self::DEFAULT_WIDTH)
    }
}

// Also run by `just test-minimal` under `--no-default-features`, which proves
// these types build without any format feature.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_four_spaces_and_styles_serialize_as_space_and_tab() {
        let default = crate::Indentation::default();
        assert_eq!(default.style, crate::IndentStyle::Spaces);
        assert_eq!(default.width.get(), 4);
        assert_eq!(serde_json::to_string(&IndentStyle::Spaces).unwrap(), "\"space\"");
        assert_eq!(serde_json::to_string(&IndentStyle::Tabs).unwrap(), "\"tab\"");
        let tab: IndentStyle = serde_json::from_str("\"tab\"").unwrap();
        assert_eq!(tab, IndentStyle::Tabs);
        assert!(serde_json::from_str::<IndentStyle>("\"tabs\"").is_err());
    }
}
