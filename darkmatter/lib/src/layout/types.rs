//! Page-layout configuration types used by
//! [`DarkmatterPage`](super::DarkmatterPage).
//!
//! These types are the minimal surface retained after the deprecated `PageMargin`,
//! `PagePadding`, `PageAlignment`, `PageFill`, `WidthUnit`, and
//! `PageComponent::Lists` vocabulary was deleted. `PageComponent` (minus `Lists`),
//! `PageBackground`, and `StyleColor` remain because they are not layout math
//! types — they are the page-assembler taxonomy and background-mode knob.

/// Page background fill strategy.
///
/// `Subtle` and `Pronounced` resolve to concrete colors at render time using
/// the captured terminal color mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageBackground {
    /// Default. Edges and padding are visually identical (both transparent).
    #[default]
    Transparent,
    /// Slightly off-background fill — darker than terminal bg in light mode,
    /// lighter than terminal bg in dark mode.
    Subtle,
    /// High-contrast inverse fill that flips the renderer's effective color
    /// mode so themes remain readable on the inverted surface.
    Pronounced,
}

/// Page-level component categories that can be aligned and filled
/// independently from the main document stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageComponent {
    /// Block images.
    Images,
    /// Block quotes.
    BlockQuotes,
    /// Tables.
    Tables,
    /// Code blocks.
    CodeBlocks,
    /// Unordered lists.
    Ul,
    /// Ordered lists.
    Ol,
    /// List items.
    Li,
    /// Hyperlinks.
    Hyperlinks,
    /// Horizontal rules.
    Hr,
}

impl PageComponent {
    /// All page-component variants in canonical order.
    pub const ALL: [PageComponent; 9] = [
        PageComponent::Images,
        PageComponent::BlockQuotes,
        PageComponent::Tables,
        PageComponent::CodeBlocks,
        PageComponent::Ul,
        PageComponent::Ol,
        PageComponent::Li,
        PageComponent::Hyperlinks,
        PageComponent::Hr,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_spec() {
        assert_eq!(PageBackground::default(), PageBackground::Transparent);
    }

    #[test]
    fn page_component_all_covers_every_variant() {
        assert_eq!(PageComponent::ALL.len(), 9);
    }
}
