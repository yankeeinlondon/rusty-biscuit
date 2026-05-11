//! Page-layout configuration types used by
//! [`DarkmatterPage`](super::DarkmatterPage).

use super::error::PageRenderError;

/// Margin (transparent space outside the page's content rectangle).
///
/// Margins are specified in terminal cells: rows for vertical sides, columns
/// for horizontal sides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PageMargin {
    /// Top margin in rows.
    pub top: u16,
    /// Right margin in columns.
    pub right: u16,
    /// Bottom margin in rows.
    pub bottom: u16,
    /// Left margin in columns.
    pub left: u16,
}

impl PageMargin {
    /// All-zero margin.
    pub const ZERO: Self = Self {
        top: 0,
        right: 0,
        bottom: 0,
        left: 0,
    };

    /// Build a margin with all four sides set to the same value.
    pub const fn all(n: u16) -> Self {
        Self {
            top: n,
            right: n,
            bottom: n,
            left: n,
        }
    }

    /// Total horizontal margin (left + right) in columns.
    pub const fn horizontal(&self) -> u16 {
        self.left.saturating_add(self.right)
    }

    /// Total vertical margin (top + bottom) in rows.
    pub const fn vertical(&self) -> u16 {
        self.top.saturating_add(self.bottom)
    }
}

/// Padding (filled space between the page's margin and content).
///
/// When [`PageBackground`] is non-transparent, padded cells are filled with
/// the page background color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PagePadding {
    /// Top padding in rows.
    pub top: u16,
    /// Right padding in columns.
    pub right: u16,
    /// Bottom padding in rows.
    pub bottom: u16,
    /// Left padding in columns.
    pub left: u16,
}

impl PagePadding {
    /// All-zero padding.
    pub const ZERO: Self = Self {
        top: 0,
        right: 0,
        bottom: 0,
        left: 0,
    };

    /// Build padding with all four sides set to the same value.
    pub const fn all(n: u16) -> Self {
        Self {
            top: n,
            right: n,
            bottom: n,
            left: n,
        }
    }

    /// Total horizontal padding (left + right) in columns.
    pub const fn horizontal(&self) -> u16 {
        self.left.saturating_add(self.right)
    }

    /// Total vertical padding (top + bottom) in rows.
    pub const fn vertical(&self) -> u16 {
        self.top.saturating_add(self.bottom)
    }
}

/// Page background fill strategy.
///
/// `Subtle` and `Pronounced` resolve to concrete colors at render time using
/// the captured terminal color mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageBackground {
    /// Default. Margin and padding are visually identical (both transparent).
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
    /// Lists (ordered and unordered).
    Lists,
}

impl PageComponent {
    /// All page-component variants in canonical order.
    pub const ALL: [PageComponent; 5] = [
        PageComponent::Images,
        PageComponent::BlockQuotes,
        PageComponent::Tables,
        PageComponent::CodeBlocks,
        PageComponent::Lists,
    ];
}

/// Horizontal alignment for a [`PageComponent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageAlignment {
    /// Left-aligned (default; preserves current behavior).
    #[default]
    Left,
    /// Centered.
    Center,
    /// Right-aligned.
    Right,
}

/// A width specifier used by [`PageFill`] variants.
///
/// `Percent` values must lie within `0.0..=100.0`; values outside the range
/// surface as [`PageRenderError::InvalidPercent`] when validated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WidthUnit {
    /// Fixed cell count (columns). Capped to the effective render width when
    /// it exceeds the available space.
    Fixed(u16),
    /// Percentage of the page's content width (`0.0..=100.0`).
    Percent(f32),
}

impl WidthUnit {
    /// Validate this value, returning an error if a `Percent` is outside the
    /// `0.0..=100.0` range.
    ///
    /// ## Errors
    ///
    /// Returns [`PageRenderError::InvalidPercent`] for percent values outside
    /// `0.0..=100.0` (NaN values are also rejected).
    pub fn validate(&self) -> Result<(), PageRenderError> {
        match self {
            WidthUnit::Fixed(_) => Ok(()),
            WidthUnit::Percent(p) => {
                if !p.is_finite() || *p < 0.0 || *p > 100.0 {
                    Err(PageRenderError::InvalidPercent(*p))
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Resolve this width unit against a base column count.
    ///
    /// `Fixed(n)` resolves to `n.min(base)`. `Percent(p)` resolves to
    /// `((p / 100) * base).round()` capped to `base`. Returns `Err` if the
    /// percent value is invalid.
    ///
    /// ## Errors
    ///
    /// See [`WidthUnit::validate`].
    pub fn resolve(&self, base: u16) -> Result<u16, PageRenderError> {
        self.validate()?;
        let resolved = match self {
            WidthUnit::Fixed(n) => (*n).min(base),
            WidthUnit::Percent(p) => {
                let resolved = (f32::from(base) * (p / 100.0)).round();
                let clamped = resolved.clamp(0.0, f32::from(base));
                clamped as u16
            }
        };
        Ok(resolved)
    }
}

/// Fill strategy for a [`PageComponent`].
///
/// `Pad` and `Indent` reduce the component's available width. `Max` and
/// `Explicit` advise the component on its target render width.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PageFill {
    /// Default. Component may use the full content width.
    #[default]
    Full,
    /// Symmetric padding on both sides (filled with page background).
    Pad(WidthUnit),
    /// One-sided padding driven by the component's alignment.
    ///
    /// - Left alignment ⇒ padding on the left only
    /// - Right alignment ⇒ padding on the right only
    /// - Center alignment ⇒ behaves like [`PageFill::Pad`]
    Indent(WidthUnit),
    /// Cap on the component's render width; the component renders at
    /// `min(natural_width, max)`.
    Max(WidthUnit),
    /// Explicit render width. Resolved against content width and capped.
    Explicit(WidthUnit),
}

impl PageFill {
    /// Validate the contained [`WidthUnit`] (if any).
    ///
    /// ## Errors
    ///
    /// Returns [`PageRenderError::InvalidPercent`] when a percent value is
    /// outside `0.0..=100.0`.
    pub fn validate(&self) -> Result<(), PageRenderError> {
        match self {
            PageFill::Full => Ok(()),
            PageFill::Pad(unit)
            | PageFill::Indent(unit)
            | PageFill::Max(unit)
            | PageFill::Explicit(unit) => unit.validate(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_margin_zero_and_helpers() {
        let m = PageMargin::ZERO;
        assert_eq!(m.horizontal(), 0);
        assert_eq!(m.vertical(), 0);
        let m = PageMargin::all(3);
        assert_eq!(m.horizontal(), 6);
        assert_eq!(m.vertical(), 6);
    }

    #[test]
    fn page_padding_zero_and_helpers() {
        let p = PagePadding::ZERO;
        assert_eq!(p.horizontal(), 0);
        assert_eq!(p.vertical(), 0);
        let p = PagePadding::all(2);
        assert_eq!(p.horizontal(), 4);
        assert_eq!(p.vertical(), 4);
    }

    #[test]
    fn defaults_match_spec() {
        assert_eq!(PageMargin::default(), PageMargin::ZERO);
        assert_eq!(PagePadding::default(), PagePadding::ZERO);
        assert_eq!(PageBackground::default(), PageBackground::Transparent);
        assert_eq!(PageAlignment::default(), PageAlignment::Left);
        assert_eq!(PageFill::default(), PageFill::Full);
    }

    #[test]
    fn width_unit_validate_fixed_ok() {
        assert!(WidthUnit::Fixed(0).validate().is_ok());
        assert!(WidthUnit::Fixed(80).validate().is_ok());
    }

    #[test]
    fn width_unit_validate_percent_range() {
        assert!(WidthUnit::Percent(0.0).validate().is_ok());
        assert!(WidthUnit::Percent(100.0).validate().is_ok());
        assert_eq!(
            WidthUnit::Percent(-0.1).validate().unwrap_err(),
            PageRenderError::InvalidPercent(-0.1)
        );
        assert_eq!(
            WidthUnit::Percent(100.1).validate().unwrap_err(),
            PageRenderError::InvalidPercent(100.1)
        );
        assert!(matches!(
            WidthUnit::Percent(f32::NAN).validate().unwrap_err(),
            PageRenderError::InvalidPercent(_)
        ));
    }

    #[test]
    fn width_unit_resolve_fixed_caps_to_base() {
        assert_eq!(WidthUnit::Fixed(50).resolve(80).unwrap(), 50);
        assert_eq!(WidthUnit::Fixed(200).resolve(80).unwrap(), 80);
    }

    #[test]
    fn width_unit_resolve_percent() {
        assert_eq!(WidthUnit::Percent(50.0).resolve(80).unwrap(), 40);
        assert_eq!(WidthUnit::Percent(0.0).resolve(80).unwrap(), 0);
        assert_eq!(WidthUnit::Percent(100.0).resolve(80).unwrap(), 80);
    }

    #[test]
    fn page_fill_validate_propagates_unit_errors() {
        assert!(PageFill::Full.validate().is_ok());
        assert!(PageFill::Pad(WidthUnit::Fixed(3)).validate().is_ok());
        let err = PageFill::Max(WidthUnit::Percent(150.0))
            .validate()
            .unwrap_err();
        assert_eq!(err, PageRenderError::InvalidPercent(150.0));
    }

    #[test]
    fn page_component_all_covers_every_variant() {
        assert_eq!(PageComponent::ALL.len(), 5);
    }
}
