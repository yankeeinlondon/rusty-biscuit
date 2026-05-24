//! Page-layout configuration types used by
//! [`DarkmatterPage`](super::DarkmatterPage).
//!
//! These page-layout types are deprecated in favor of
//! [`renderable::layout::Layout`]. They remain `pub` because the darkmatter
//! CLI still constructs them through the [`DarkmatterPage`](super::DarkmatterPage)
//! builder. The `From`/`TryFrom` conversions below bridge them onto the new
//! [`renderable::layout`] primitives.

// This module is the home of the deprecated page-layout types; its own
// `impl`s, conversions, and tests legitimately reference them.
#![allow(deprecated)]

use super::error::PageRenderError;

/// Margin (transparent space outside the page's content rectangle).
///
/// Margins are specified in terminal cells: rows for vertical sides, columns
/// for horizontal sides.
#[deprecated(
    since = "0.1.0",
    note = "use renderable::layout::Layout (via renderable::layout::Margin)"
)]
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
#[deprecated(
    since = "0.1.0",
    note = "use renderable::layout::Layout (via renderable::layout::Margin)"
)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    /// Unordered lists.
    Ul,
    /// Ordered lists.
    Ol,
    /// List items.
    Li,
    /// Hyperlinks.
    Hyperlinks,
    /// Lists (ordered and unordered).
    #[deprecated(note = "use PageComponent::{Ul, Ol, Li}")]
    Lists,
}

impl PageComponent {
    /// All page-component variants in canonical order.
    pub const ALL: [PageComponent; 8] = [
        PageComponent::Images,
        PageComponent::BlockQuotes,
        PageComponent::Tables,
        PageComponent::CodeBlocks,
        PageComponent::Ul,
        PageComponent::Ol,
        PageComponent::Li,
        PageComponent::Hyperlinks,
    ];

    /// The three concrete list component variants.
    pub const LISTS: [PageComponent; 3] = [Self::Ul, Self::Ol, Self::Li];
}

/// Horizontal alignment for a [`PageComponent`].
#[deprecated(since = "0.1.0", note = "use renderable::layout::Alignment")]
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
#[deprecated(since = "0.1.0", note = "use renderable::layout::Layout")]
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

// ---------- Deprecation bridges onto `renderable::layout` ----------

impl From<PageMargin> for renderable::layout::Margin {
    /// Map a [`PageMargin`] (terminal cells) onto a
    /// [`renderable::layout::Margin`] of universal `Ch` lengths.
    fn from(m: PageMargin) -> renderable::layout::Margin {
        use renderable::layout::{Length, TargetValue};
        let cell = |n: u16| TargetValue::universal(Length::ch(u32::from(n)));
        renderable::layout::Margin {
            top: cell(m.top),
            right: cell(m.right),
            bottom: cell(m.bottom),
            left: cell(m.left),
        }
    }
}

impl From<PagePadding> for renderable::layout::Margin {
    /// Map a [`PagePadding`] (terminal cells) onto a
    /// [`renderable::layout::Margin`] of universal `Ch` lengths.
    ///
    /// The new [`Layout`](renderable::layout::Layout) primitive has no
    /// separate padding concept; page padding is expressed as additional
    /// margin cells. Background-fill semantics that previously distinguished
    /// margin from padding are a `Style` concern and are not represented here.
    fn from(p: PagePadding) -> renderable::layout::Margin {
        use renderable::layout::{Length, TargetValue};
        let cell = |n: u16| TargetValue::universal(Length::ch(u32::from(n)));
        renderable::layout::Margin {
            top: cell(p.top),
            right: cell(p.right),
            bottom: cell(p.bottom),
            left: cell(p.left),
        }
    }
}

impl From<PageAlignment> for renderable::layout::Alignment {
    /// Map a [`PageAlignment`] onto a [`renderable::layout::Alignment`].
    fn from(a: PageAlignment) -> renderable::layout::Alignment {
        match a {
            PageAlignment::Left => renderable::layout::Alignment::Left,
            PageAlignment::Center => renderable::layout::Alignment::Center,
            PageAlignment::Right => renderable::layout::Alignment::Right,
        }
    }
}

impl TryFrom<WidthUnit> for renderable::layout::Length {
    type Error = PageRenderError;

    /// Map a [`WidthUnit`] onto a universal [`renderable::layout::Length`].
    ///
    /// `Fixed(n)` becomes `Length::Ch(n)`; `Percent(p)` becomes
    /// `Length::Percent(p)` after range validation.
    ///
    /// ## Errors
    ///
    /// Returns [`PageRenderError::InvalidPercent`] when a percent value is
    /// outside `0.0..=100.0`.
    fn try_from(unit: WidthUnit) -> Result<renderable::layout::Length, PageRenderError> {
        use renderable::layout::Length;
        match unit {
            WidthUnit::Fixed(n) => Ok(Length::ch(u32::from(n))),
            WidthUnit::Percent(p) => {
                Length::percent(p).map_err(|_| PageRenderError::InvalidPercent(p))
            }
        }
    }
}

impl TryFrom<PageFill> for Option<renderable::layout::TargetValue<renderable::layout::Length>> {
    type Error = PageRenderError;

    /// Map a [`PageFill`] onto an optional max-width
    /// [`TargetValue<Length>`](renderable::layout::TargetValue).
    ///
    /// Only the width-capping variants convert here:
    ///
    /// - `Max(unit)` / `Explicit(unit)` → `Some(universal(length))`
    /// - `Full` → `None` (component may use the full content width)
    ///
    /// `Pad` and `Indent` are *margin* contributions, not width caps, so they
    /// also resolve to `None` for the max-width slot; callers that need their
    /// indent should route the contained [`WidthUnit`] through
    /// [`PageFill::margin_contribution`] instead. See that method for the
    /// margin-side conversion.
    ///
    /// ## Errors
    ///
    /// Returns [`PageRenderError::InvalidPercent`] when a percent value is
    /// outside `0.0..=100.0`.
    fn try_from(
        fill: PageFill,
    ) -> Result<Option<renderable::layout::TargetValue<renderable::layout::Length>>, PageRenderError>
    {
        use renderable::layout::TargetValue;
        match fill {
            PageFill::Full | PageFill::Pad(_) | PageFill::Indent(_) => Ok(None),
            PageFill::Max(unit) | PageFill::Explicit(unit) => {
                let length = renderable::layout::Length::try_from(unit)?;
                Ok(Some(TargetValue::universal(length)))
            }
        }
    }
}

impl PageFill {
    /// The margin contribution of this fill, if any.
    ///
    /// `Pad` and `Indent` reduce a component's available width by inset cells;
    /// expressed against the new [`Layout`](renderable::layout::Layout)
    /// primitive that inset is a margin. This returns the resolved
    /// [`Length`](renderable::layout::Length) for those variants and `None`
    /// for `Full` / `Max` / `Explicit` (which are width caps, not margins).
    ///
    /// One-sided vs. symmetric placement depends on the component's
    /// [`PageAlignment`]; callers decide which side(s) to apply the length to.
    ///
    /// ## Errors
    ///
    /// Returns [`PageRenderError::InvalidPercent`] when a percent value is
    /// outside `0.0..=100.0`.
    pub fn margin_contribution(
        &self,
    ) -> Result<Option<renderable::layout::Length>, PageRenderError> {
        match self {
            PageFill::Full | PageFill::Max(_) | PageFill::Explicit(_) => Ok(None),
            PageFill::Pad(unit) | PageFill::Indent(unit) => {
                Ok(Some(renderable::layout::Length::try_from(*unit)?))
            }
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
        assert_eq!(PageComponent::ALL.len(), 8);
    }

    #[test]
    fn page_component_lists_contains_exactly_three_concrete_variants() {
        assert_eq!(PageComponent::LISTS.len(), 3);
        assert!(PageComponent::LISTS.contains(&PageComponent::Ul));
        assert!(PageComponent::LISTS.contains(&PageComponent::Ol));
        assert!(PageComponent::LISTS.contains(&PageComponent::Li));
    }

    // ---------- Deprecation-bridge conversions ----------

    #[test]
    fn page_margin_converts_to_layout_margin() {
        use renderable::layout::{Length, Margin as RMargin, TargetValue};

        let page = PageMargin::all(2);
        let rendered: RMargin = page.into();
        assert_eq!(rendered.left, TargetValue::universal(Length::ch(2)));
        assert_eq!(rendered.top, TargetValue::universal(Length::ch(2)));
        assert_eq!(rendered.right, TargetValue::universal(Length::ch(2)));
        assert_eq!(rendered.bottom, TargetValue::universal(Length::ch(2)));
    }

    #[test]
    fn page_alignment_converts_to_layout_alignment() {
        use renderable::layout::Alignment;
        assert_eq!(Alignment::from(PageAlignment::Center), Alignment::Center);
        assert_eq!(Alignment::from(PageAlignment::Left), Alignment::Left);
        assert_eq!(Alignment::from(PageAlignment::Right), Alignment::Right);
    }

    #[test]
    fn page_padding_converts_to_layout_margin() {
        use renderable::layout::{Length, Margin as RMargin, TargetValue};

        let padding = PagePadding::all(3);
        let rendered: RMargin = padding.into();
        assert_eq!(rendered.left, TargetValue::universal(Length::ch(3)));
        assert_eq!(rendered.bottom, TargetValue::universal(Length::ch(3)));
    }

    #[test]
    fn width_unit_converts_to_length() {
        use renderable::layout::Length;
        assert_eq!(
            Length::try_from(WidthUnit::Fixed(5)).unwrap(),
            Length::ch(5)
        );
        assert_eq!(
            Length::try_from(WidthUnit::Percent(50.0)).unwrap(),
            Length::Percent(50.0)
        );
        assert_eq!(
            Length::try_from(WidthUnit::Percent(150.0)).unwrap_err(),
            PageRenderError::InvalidPercent(150.0)
        );
    }

    #[test]
    fn page_fill_converts_to_max_width_target_value() {
        use renderable::layout::{Length, TargetValue};

        type MaxWidth = Option<TargetValue<Length>>;

        // Full / Pad / Indent carry no width cap.
        assert_eq!(MaxWidth::try_from(PageFill::Full).unwrap(), None);
        assert_eq!(
            MaxWidth::try_from(PageFill::Pad(WidthUnit::Fixed(4))).unwrap(),
            None
        );
        assert_eq!(
            MaxWidth::try_from(PageFill::Indent(WidthUnit::Fixed(4))).unwrap(),
            None
        );

        // Max / Explicit produce a universal max-width.
        assert_eq!(
            MaxWidth::try_from(PageFill::Max(WidthUnit::Fixed(60))).unwrap(),
            Some(TargetValue::universal(Length::ch(60)))
        );
        assert_eq!(
            MaxWidth::try_from(PageFill::Explicit(WidthUnit::Percent(75.0))).unwrap(),
            Some(TargetValue::universal(Length::Percent(75.0)))
        );

        // Invalid percent propagates.
        assert_eq!(
            MaxWidth::try_from(PageFill::Max(WidthUnit::Percent(150.0))).unwrap_err(),
            PageRenderError::InvalidPercent(150.0)
        );
    }

    #[test]
    fn page_fill_margin_contribution() {
        use renderable::layout::Length;

        assert_eq!(PageFill::Full.margin_contribution().unwrap(), None);
        assert_eq!(
            PageFill::Max(WidthUnit::Fixed(4))
                .margin_contribution()
                .unwrap(),
            None
        );
        assert_eq!(
            PageFill::Pad(WidthUnit::Fixed(6))
                .margin_contribution()
                .unwrap(),
            Some(Length::ch(6))
        );
        assert_eq!(
            PageFill::Indent(WidthUnit::Percent(10.0))
                .margin_contribution()
                .unwrap(),
            Some(Length::Percent(10.0))
        );
    }

    #[test]
    fn deprecation_bridge_produces_valid_layout() {
        use renderable::layout::{Alignment, Layout, Length, Margin, TargetValue};

        // Page margin + padding are summed into the layout margin, exactly as
        // `DarkmatterPage::rebuild_layout` documents (the `Layout` primitive
        // has no separate padding concept).
        let page_margin = PageMargin::all(2);
        let page_padding = PagePadding::all(1);
        let page_alignment = PageAlignment::Center;

        let margin: Margin = page_margin.into();
        let padding: Margin = page_padding.into();
        let alignment: Alignment = page_alignment.into();

        let cells = |tv: &TargetValue<Length>| match tv {
            TargetValue::Universal(Length::Ch(n)) => *n,
            _ => 0,
        };
        let sum = |a: &TargetValue<Length>, b: &TargetValue<Length>| {
            TargetValue::universal(Length::ch(cells(a) + cells(b)))
        };
        let combined_margin = Margin {
            top: sum(&margin.top, &padding.top),
            right: sum(&margin.right, &padding.right),
            bottom: sum(&margin.bottom, &padding.bottom),
            left: sum(&margin.left, &padding.left),
        };
        // 2 (margin) + 1 (padding) = 3 cells per side.
        assert_eq!(cells(&combined_margin.left), 3);

        let layout = Layout {
            margin: combined_margin,
            alignment,
            max_width: None,
            word_wrap: renderable::wrap_policy::WordWrap::None,
        };

        assert!(
            layout.validate().is_ok(),
            "bridge-produced Layout must pass validation"
        );
    }

    #[test]
    fn deprecation_bridge_page_fill_max_width_produces_valid_layout() {
        use renderable::layout::{Alignment, Layout, Length, Margin, TargetValue};

        let fill = PageFill::Max(WidthUnit::Fixed(60));
        let max_width: Option<TargetValue<Length>> = fill.try_into().unwrap();
        assert_eq!(max_width, Some(TargetValue::universal(Length::ch(60))));

        let layout = Layout {
            margin: Margin::default(),
            alignment: Alignment::default(),
            max_width,
            word_wrap: renderable::wrap_policy::WordWrap::None,
        };
        assert!(layout.validate().is_ok());
    }

    #[test]
    fn deprecation_bridge_rejects_invalid_percent() {
        let fill = PageFill::Max(WidthUnit::Percent(150.0));
        let result: Result<Option<renderable::layout::TargetValue<renderable::layout::Length>>, _> =
            fill.try_into();
        assert!(result.is_err());
    }

    // ---------- Deferral compatibility boundary (Finding 3) ----------
    //
    // `layout/mod.rs` documents that constructing a `DarkmatterPage` via the
    // builder produces results identical to constructing the equivalent
    // `renderable::layout::Layout` and converting through the bridge. The
    // tests below prove that claim against rendered output, not just
    // `Layout::validate`.

    /// Builds the `Layout` that the documented bridge produces for a page with
    /// the given margin/padding/max-width — independently of `DarkmatterPage`.
    ///
    /// This mirrors `DarkmatterPage::rebuild_layout`'s documented contract:
    /// page margin and padding are both transparent space outside the content
    /// rectangle, so the bridge sums them into the single `Layout` margin, and
    /// the max-width cap is a universal `Ch` length.
    fn bridge_layout(
        margin: PageMargin,
        padding: PagePadding,
        max_width: Option<u16>,
    ) -> renderable::layout::Layout {
        use renderable::layout::{Layout, Length, Margin as RMargin, TargetValue};

        let m: RMargin = margin.into();
        let p: RMargin = padding.into();
        let cells = |tv: &TargetValue<Length>| match tv {
            TargetValue::Universal(Length::Ch(n)) => *n,
            _ => 0,
        };
        let sum = |a: &TargetValue<Length>, b: &TargetValue<Length>| {
            TargetValue::universal(Length::ch(cells(a) + cells(b)))
        };
        Layout {
            margin: RMargin {
                top: sum(&m.top, &p.top),
                right: sum(&m.right, &p.right),
                bottom: sum(&m.bottom, &p.bottom),
                left: sum(&m.left, &p.left),
            },
            alignment: renderable::layout::Alignment::default(),
            max_width: max_width.map(|w| TargetValue::universal(Length::ch(u32::from(w)))),
            word_wrap: renderable::wrap_policy::WordWrap::None,
        }
    }

    /// The `Layout` a `DarkmatterPage` builder exposes via `TerminalRenderable`
    /// is byte-identical to the `Layout` produced by the documented bridge —
    /// for margin, padding, and max-width cases.
    #[test]
    fn darkmatter_page_layout_matches_bridge_layout() {
        use crate::layout::DarkmatterPage;
        use biscuit_terminal::components::renderable::TerminalRenderable;
        use biscuit_terminal::terminal::Terminal;

        let term = Terminal::new_optimistic(80);

        // Margin-only.
        let page = DarkmatterPage::new(&term).with_margin(4);
        assert_eq!(
            page.layout(),
            &bridge_layout(PageMargin::all(4), PagePadding::all(0), None),
            "margin-only builder must match bridge Layout"
        );

        // Margin + padding (summed into the layout margin).
        let page = DarkmatterPage::new(&term).with_margin(2).with_padding(3);
        assert_eq!(
            page.layout(),
            &bridge_layout(PageMargin::all(2), PagePadding::all(3), None),
            "margin+padding builder must match bridge Layout"
        );

        // Max-width cap.
        let page = DarkmatterPage::new(&term).with_margin(1).with_max_width(60);
        assert_eq!(
            page.layout(),
            &bridge_layout(PageMargin::all(1), PagePadding::all(0), Some(60)),
            "max-width builder must match bridge Layout"
        );
    }

    /// `DarkmatterPage::render` produces visible output whose left-margin
    /// indent on content rows matches the left margin of the bridge `Layout`.
    ///
    /// `DarkmatterPage` applies margins through its row-decoration pass while
    /// `renderable::layout::Layout::apply_layout` applies them per line; they
    /// share no code, so this proves the documented equivalence on rendered
    /// output. Vertical margins and per-component alignment live on different
    /// code paths (`DarkmatterPage` emits blank margin rows itself and tracks
    /// alignment per component, not as a single `Layout::alignment`), so this
    /// test is deliberately scoped to the provable horizontal-margin and
    /// max-width facets.
    #[test]
    fn darkmatter_page_render_margin_matches_bridge_layout() {
        use crate::layout::DarkmatterPage;
        use crate::markdown::Markdown;
        use biscuit_terminal::terminal::Terminal;
        use biscuit_terminal::utils::layout::LayoutTerminalExt;

        let term = Terminal::new_optimistic(80);
        let md: Markdown = "Hello world".into();

        // Render the body with no layout, then apply the bridge Layout's
        // margins exactly as a `renderable::layout::Layout` consumer would.
        let plain = DarkmatterPage::new(&term)
            .render(&md)
            .expect("default render succeeds");
        let bridge = bridge_layout(PageMargin::all(5), PagePadding::all(0), None);
        let via_bridge = bridge.apply_layout(&plain, 80);

        // Render the same markdown through the DarkmatterPage builder.
        let via_page = DarkmatterPage::new(&term)
            .with_margin(5)
            .render(&md)
            .expect("margin render succeeds");

        // Compare the visible left-margin indent on every content row.
        let indent_of = |s: &str| -> Vec<usize> {
            let stripped = biscuit_terminal::prelude::strip_escape_codes(s);
            stripped
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.chars().take_while(|c| *c == ' ').count())
                .collect()
        };

        let page_indents = indent_of(&via_page);
        let bridge_indents = indent_of(&via_bridge);
        assert!(!page_indents.is_empty(), "expected rendered content rows");
        assert_eq!(
            page_indents, bridge_indents,
            "DarkmatterPage margin indent must match the bridge Layout's"
        );
        for indent in &page_indents {
            assert_eq!(*indent, 5, "every content row carries the 5-cell margin");
        }
    }
}
