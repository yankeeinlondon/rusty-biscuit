//! Internal layout context computed at render time from [`DarkmatterPage`](super::DarkmatterPage)
//! state and the captured terminal dimensions.

// `LayoutContext` is derived from the deprecated page-layout types
// (`PageMargin`, `PagePadding`, `PageAlignment`, `PageFill`). It remains the
// internal render-time representation; deprecation is suppressed here while
// the bespoke row-decoration pipeline coexists with `renderable::layout`.
#![allow(deprecated)]

use super::PageRenderError;
use super::types::{
    PageAlignment, PageBackground, PageComponent, PageFill, PageMargin, PagePadding, WidthUnit,
};
use crate::markdown::highlighting::ColorMode;
use crate::style::StyleColor;
use crate::style::schema::CommonStyle;

/// Resolved layout state used during terminal rendering.
///
/// Created from a [`DarkmatterPage`](super::DarkmatterPage) at render time so
/// downstream code does not recompute widths and colors on every row.
#[derive(Debug, Clone)]
pub(crate) struct LayoutContext {
    #[allow(dead_code)]
    /// Original terminal width in columns.
    pub terminal_width: u16,
    /// Top margin in rows.
    pub margin_top: u16,
    /// Bottom margin in rows.
    pub margin_bottom: u16,
    /// Left margin in columns.
    pub margin_left: u16,
    /// Right margin in columns.
    pub margin_right: u16,
    /// Top padding in rows.
    pub padding_top: u16,
    /// Bottom padding in rows.
    pub padding_bottom: u16,
    /// Left padding in columns.
    pub padding_left: u16,
    /// Right padding in columns.
    pub padding_right: u16,
    #[allow(dead_code)]
    /// Width available for content after margin and padding are removed.
    pub content_width: u16,
    /// Final render width (capped by `max_width` if set).
    pub effective_width: u16,
    /// Whether any layout settings are non-default ( triggers row decoration).
    pub has_layout: bool,
    /// Resolved page background color, if any.
    pub background_color: Option<BackgroundColor>,
    /// Color mode passed to the markdown renderer (may be inverted for Pronounced).
    pub render_color_mode: ColorMode,
    #[allow(dead_code)]
    /// Per-component alignments.
    pub alignments: std::collections::HashMap<PageComponent, PageAlignment>,
    #[allow(dead_code)]
    /// Per-component fills.
    pub fills: std::collections::HashMap<PageComponent, PageFill>,
    /// Per-component list left margins.
    pub list_left_margins: std::collections::HashMap<PageComponent, WidthUnit>,
    /// Page foreground color.
    #[allow(dead_code)]
    pub page_color: Option<StyleColor>,
    /// Page background color.
    #[allow(dead_code)]
    pub page_bg_color: Option<StyleColor>,
    /// Per-component foreground colors.
    #[allow(dead_code)]
    pub component_colors: std::collections::HashMap<PageComponent, StyleColor>,
    /// Per-component background colors.
    #[allow(dead_code)]
    pub component_bg_colors: std::collections::HashMap<PageComponent, StyleColor>,
    /// Global hyperlink style from `style.hyperlinks.*`.
    pub hyperlink_style: Option<CommonStyle>,
    /// Local hyperlink override from `style.hyperlinks.local-style`.
    pub local_hyperlink_style: Option<CommonStyle>,
    /// Local image override from `style.images.local-style`.
    pub local_image_style: Option<CommonStyle>,
}

/// Concrete RGB background color resolved from [`PageBackground`] and the
/// terminal's color mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BackgroundColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl BackgroundColor {
    /// ANSI 48;2 escape sequence for this color.
    pub fn ansi_bg(&self) -> String {
        format!("\x1b[48;2;{};{};{}m", self.r, self.g, self.b)
    }

    #[allow(dead_code)]
    /// ANSI 38;2 escape sequence for this color (foreground).
    pub fn ansi_fg(&self) -> String {
        format!("\x1b[38;2;{};{};{}m", self.r, self.g, self.b)
    }
}

// Named constants for page background colors so they can be tuned independently.

/// Subtle background for dark terminals (slightly lighter than black).
pub(crate) const PAGE_BG_SUBTLE_DARK: BackgroundColor = BackgroundColor {
    r: 30,
    g: 30,
    b: 35,
};

/// Subtle background for light terminals (slightly darker than white).
pub(crate) const PAGE_BG_SUBTLE_LIGHT: BackgroundColor = BackgroundColor {
    r: 240,
    g: 240,
    b: 245,
};

/// Pronounced background for dark terminals (near-white surface).
pub(crate) const PAGE_BG_PRONOUNCED_DARK: BackgroundColor = BackgroundColor {
    r: 245,
    g: 245,
    b: 245,
};

/// Pronounced background for light terminals (near-black surface).
pub(crate) const PAGE_BG_PRONOUNCED_LIGHT: BackgroundColor = BackgroundColor {
    r: 20,
    g: 20,
    b: 25,
};

impl LayoutContext {
    /// Build a layout context from page state.
    ///
    /// ## Errors
    ///
    /// Returns [`PageRenderError::MarginsExceedTerminalWidth`] when margin +
    /// padding meets or exceeds the terminal width.
    #[allow(clippy::too_many_arguments)]
    pub fn from_page(
        terminal_width: u16,
        margin: PageMargin,
        padding: PagePadding,
        page_background: PageBackground,
        max_width: Option<u16>,
        terminal_color_mode: &biscuit_terminal::discovery::detection::ColorMode,
        options_color_mode: ColorMode,
        alignments: std::collections::HashMap<PageComponent, PageAlignment>,
        fills: std::collections::HashMap<PageComponent, PageFill>,
        list_left_margins: std::collections::HashMap<PageComponent, WidthUnit>,
        page_color: Option<StyleColor>,
        page_bg_color: Option<StyleColor>,
        component_colors: std::collections::HashMap<PageComponent, StyleColor>,
        component_bg_colors: std::collections::HashMap<PageComponent, StyleColor>,
        hyperlink_style: Option<CommonStyle>,
        local_hyperlink_style: Option<CommonStyle>,
        local_image_style: Option<CommonStyle>,
    ) -> Result<Self, PageRenderError> {
        let margin_x = margin.horizontal();
        let padding_x = padding.horizontal();
        let required = margin_x.saturating_add(padding_x);
        if required >= terminal_width {
            return Err(PageRenderError::MarginsExceedTerminalWidth {
                terminal_width,
                required,
            });
        }

        let content_width = terminal_width.saturating_sub(required);
        let effective_width = match max_width {
            Some(0) => return Err(PageRenderError::MaxWidthZero),
            Some(mw) => content_width.min(mw),
            None => content_width,
        };

        let has_layout = margin != PageMargin::ZERO
            || padding != PagePadding::ZERO
            || page_background != PageBackground::Transparent
            || max_width.is_some()
            || !alignments.is_empty()
            || !fills.is_empty()
            || !list_left_margins.is_empty()
            || page_color.is_some()
            || page_bg_color.is_some()
            || !component_colors.is_empty()
            || !component_bg_colors.is_empty();

        // Per spec, page background resolution uses the *terminal's* detected
        // color mode (so `Subtle` picks a dark-vs-light surface based on the
        // actual terminal). When detection is `Unknown`, fall back to the
        // caller-supplied `options_color_mode` for stability.
        let surface_mode = match terminal_color_mode {
            biscuit_terminal::discovery::detection::ColorMode::Dark => ColorMode::Dark,
            biscuit_terminal::discovery::detection::ColorMode::Light => ColorMode::Light,
            biscuit_terminal::discovery::detection::ColorMode::Unknown => options_color_mode,
        };

        // Resolve background color and render color mode.
        let (background_color, render_color_mode) = match page_background {
            PageBackground::Transparent => (None, options_color_mode),
            PageBackground::Subtle => {
                let bg = match surface_mode {
                    ColorMode::Dark => PAGE_BG_SUBTLE_DARK,
                    ColorMode::Light => PAGE_BG_SUBTLE_LIGHT,
                };
                (Some(bg), options_color_mode)
            }
            PageBackground::Pronounced => {
                let (bg, inverted) = match surface_mode {
                    ColorMode::Dark => (PAGE_BG_PRONOUNCED_DARK, ColorMode::Light),
                    ColorMode::Light => (PAGE_BG_PRONOUNCED_LIGHT, ColorMode::Dark),
                };
                (Some(bg), inverted)
            }
        };

        Ok(Self {
            terminal_width,
            margin_top: margin.top,
            margin_bottom: margin.bottom,
            margin_left: margin.left,
            margin_right: margin.right,
            padding_top: padding.top,
            padding_bottom: padding.bottom,
            padding_left: padding.left,
            padding_right: padding.right,
            content_width,
            effective_width,
            has_layout,
            background_color,
            render_color_mode,
            alignments,
            fills,
            list_left_margins,
            page_color,
            page_bg_color,
            component_colors,
            component_bg_colors,
            hyperlink_style,
            local_hyperlink_style,
            local_image_style,
        })
    }

    /// Whether row decoration should be applied.
    pub fn needs_decoration(&self) -> bool {
        self.has_layout
    }

    /// Whether any per-component alignment or fill is configured.
    pub fn has_component_styles(&self) -> bool {
        !self.alignments.is_empty()
            || !self.fills.is_empty()
            || !self.list_left_margins.is_empty()
            || !self.component_colors.is_empty()
            || !self.component_bg_colors.is_empty()
    }

    /// Resolve effective foreground color for a component.
    ///
    /// Returns the component-specific color when set, otherwise falls back
    /// to the page-level color.
    #[allow(dead_code)]
    pub fn component_color(&self, component: PageComponent) -> Option<&StyleColor> {
        self.component_colors
            .get(&component)
            .or(self.page_color.as_ref())
    }

    /// Resolve effective background color for a component.
    ///
    /// Returns the component-specific color when set, otherwise falls back
    /// to the page-level color.
    #[allow(dead_code)]
    pub fn component_bg_color(&self, component: PageComponent) -> Option<&StyleColor> {
        self.component_bg_colors
            .get(&component)
            .or(self.page_bg_color.as_ref())
    }

    /// Resolve effective hyperlink foreground/background colors.
    ///
    /// For local links (`is_local == true`), merges `local_hyperlink_style`
    /// over `hyperlink_style` and uses the resulting color fields. For remote
    /// links, uses the global `hyperlinks` component color.
    pub fn hyperlink_color(&self, is_local: bool) -> (Option<StyleColor>, Option<StyleColor>) {
        let merged = if is_local {
            self.local_hyperlink_style.as_ref().map(|local| {
                if let Some(base) = self.hyperlink_style.as_ref() {
                    crate::style::bespoke::merge_common_style(base, local)
                } else {
                    local.clone()
                }
            })
        } else {
            self.hyperlink_style.clone()
        };

        let fg = merged
            .as_ref()
            .and_then(|s| s.color.clone())
            .or_else(|| self.component_colors.get(&PageComponent::Hyperlinks).cloned())
            .or_else(|| self.page_color.clone());
        let bg = merged
            .as_ref()
            .and_then(|s| s.bg_color.clone())
            .or_else(|| self.component_bg_colors.get(&PageComponent::Hyperlinks).cloned())
            .or_else(|| self.page_bg_color.clone());

        (fg, bg)
    }

    /// Resolve the effective hyperlink [`CommonStyle`] for a link.
    ///
    /// For local links (`is_local == true`), merges `local_hyperlink_style`
    /// over `hyperlink_style`. For remote links, returns `hyperlink_style`.
    pub fn effective_hyperlink_style(&self, is_local: bool) -> Option<CommonStyle> {
        if is_local {
            self.local_hyperlink_style.as_ref().map(|local| {
                if let Some(base) = self.hyperlink_style.as_ref() {
                    crate::style::bespoke::merge_common_style(base, local)
                } else {
                    local.clone()
                }
            }).or_else(|| self.hyperlink_style.clone())
        } else {
            self.hyperlink_style.clone()
        }
    }

    /// Resolve effective image foreground/background colors for fallback text.
    ///
    /// For local images (`is_local == true`), merges `local_image_style`
    /// over the global `style.images.*` colors. For remote images, uses the
    /// global `images` component color.
    pub fn image_color(&self, is_local: bool) -> (Option<StyleColor>, Option<StyleColor>) {
        let merged = if is_local {
            self.local_image_style.as_ref().map(|local| {
                let base = CommonStyle {
                    color: self.component_colors.get(&PageComponent::Images).cloned(),
                    bg_color: self.component_bg_colors.get(&PageComponent::Images).cloned(),
                    ..CommonStyle::default()
                };
                crate::style::bespoke::merge_common_style(&base, local)
            })
        } else {
            None
        };

        let fg = merged
            .as_ref()
            .and_then(|s| s.color.clone())
            .or_else(|| self.component_colors.get(&PageComponent::Images).cloned())
            .or_else(|| self.page_color.clone());
        let bg = merged
            .as_ref()
            .and_then(|s| s.bg_color.clone())
            .or_else(|| self.component_bg_colors.get(&PageComponent::Images).cloned())
            .or_else(|| self.page_bg_color.clone());

        (fg, bg)
    }

    /// Get the list left margin for a component, if any.
    #[allow(dead_code)]
    pub fn list_left_margin(&self, component: PageComponent) -> Option<WidthUnit> {
        self.list_left_margins.get(&component).copied()
    }

    /// Get the alignment for a component, defaulting to [`PageAlignment::Left`].
    ///
    /// For the concrete list variants (`Ul`, `Ol`, `Li`), falls back to the
    /// deprecated [`PageComponent::Lists`] entry when no explicit entry exists.
    pub fn component_alignment(&self, component: PageComponent) -> PageAlignment {
        self.alignments
            .get(&component)
            .copied()
            .or_else(|| {
                if PageComponent::LISTS.contains(&component) {
                    self.alignments.get(&PageComponent::Lists).copied()
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }

    /// Get the fill for a component, defaulting to [`PageFill::Full`].
    ///
    /// For the concrete list variants (`Ul`, `Ol`, `Li`), falls back to the
    /// deprecated [`PageComponent::Lists`] entry when no explicit entry exists.
    pub fn component_fill(&self, component: PageComponent) -> PageFill {
        self.fills
            .get(&component)
            .copied()
            .or_else(|| {
                if PageComponent::LISTS.contains(&component) {
                    self.fills.get(&PageComponent::Lists).copied()
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }

    /// Resolve the target render width for a component based on its fill setting.
    ///
    /// `PageFill::Percent` values are resolved against `content_width` (after margin
    /// and padding but before the max-width cap), then capped by `effective_width`.
    /// `PageFill::Fixed` values are capped to `effective_width`.
    ///
    /// ## Errors
    ///
    /// Returns [`PageRenderError::InvalidPercent`] for percent values outside
    /// `0.0..=100.0`.
    pub fn resolve_component_width(
        &self,
        component: PageComponent,
    ) -> Result<u16, PageRenderError> {
        let fill = self.component_fill(component);
        match fill {
            PageFill::Full => Ok(self.effective_width),
            PageFill::Pad(unit) => {
                let pad = resolve_width_unit(unit, self.content_width, self.effective_width)?;
                Ok(self.effective_width.saturating_sub(pad.saturating_mul(2)))
            }
            PageFill::Indent(unit) => {
                let indent = resolve_width_unit(unit, self.content_width, self.effective_width)?;
                Ok(self.effective_width.saturating_sub(indent))
            }
            PageFill::Max(unit) => {
                let max = resolve_width_unit(unit, self.content_width, self.effective_width)?;
                Ok(self.effective_width.min(max))
            }
            PageFill::Explicit(unit) => {
                let width = resolve_width_unit(unit, self.content_width, self.effective_width)?;
                Ok(self.effective_width.min(width))
            }
        }
    }

    /// Compute left padding needed to align a component of the given visible width.
    ///
    /// Returns `0` for [`PageAlignment::Left`], half the surplus for
    /// [`PageAlignment::Center`], and the full surplus for [`PageAlignment::Right`].
    pub fn alignment_padding(&self, component: PageComponent, rendered_width: u16) -> u16 {
        let alignment = self.component_alignment(component);
        let available = self.effective_width;
        let surplus = available.saturating_sub(rendered_width);
        match alignment {
            PageAlignment::Left => 0,
            PageAlignment::Center => surplus / 2,
            PageAlignment::Right => surplus,
        }
    }

    /// Compute left and right padding for a component based on its fill and alignment.
    ///
    /// Returns `(left_pad, right_pad)` in columns. For [`PageFill::Pad`], both sides
    /// get the resolved pad amount. For [`PageFill::Indent`], padding is one-sided
    /// based on alignment (left for left-align, right for right-align, both for center).
    pub fn component_side_padding(
        &self,
        component: PageComponent,
    ) -> Result<(u16, u16), PageRenderError> {
        let fill = self.component_fill(component);
        let alignment = self.component_alignment(component);
        match fill {
            PageFill::Full | PageFill::Max(_) | PageFill::Explicit(_) => Ok((0, 0)),
            PageFill::Pad(unit) => {
                let pad = resolve_width_unit(unit, self.content_width, self.effective_width)?;
                Ok((pad, pad))
            }
            PageFill::Indent(unit) => {
                let indent = resolve_width_unit(unit, self.content_width, self.effective_width)?;
                match alignment {
                    PageAlignment::Left => Ok((indent, 0)),
                    PageAlignment::Right => Ok((0, indent)),
                    PageAlignment::Center => Ok((indent, indent)),
                }
            }
        }
    }
}

/// Resolve a [`WidthUnit`] against the appropriate base width.
///
/// `Fixed(n)` resolves to `min(n, effective_width)`. `Percent(p)` resolves to
/// `(p / 100) * content_width`, capped to `effective_width`.
fn resolve_width_unit(
    unit: WidthUnit,
    content_width: u16,
    effective_width: u16,
) -> Result<u16, PageRenderError> {
    match unit {
        WidthUnit::Fixed(n) => Ok((n).min(effective_width)),
        WidthUnit::Percent(p) => {
            if !p.is_finite() || !(0.0..=100.0).contains(&p) {
                return Err(PageRenderError::InvalidPercent(p));
            }
            let resolved = (f32::from(content_width) * (p / 100.0)).round();
            let clamped = resolved.clamp(0.0, f32::from(effective_width));
            Ok(clamped as u16)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn ctx(content_width: u16, effective_width: u16) -> LayoutContext {
        LayoutContext {
            terminal_width: effective_width,
            margin_top: 0,
            margin_bottom: 0,
            margin_left: 0,
            margin_right: 0,
            padding_top: 0,
            padding_bottom: 0,
            padding_left: 0,
            padding_right: 0,
            content_width,
            effective_width,
            has_layout: true,
            background_color: None,
            render_color_mode: ColorMode::Dark,
            alignments: HashMap::new(),
            fills: HashMap::new(),
            list_left_margins: HashMap::new(),
            page_color: None,
            page_bg_color: None,
            component_colors: HashMap::new(),
            component_bg_colors: HashMap::new(),
            hyperlink_style: None,
            local_hyperlink_style: None,
            local_image_style: None,
        }
    }

    #[test]
    fn resolve_component_width_full() {
        let c = ctx(100, 80);
        assert_eq!(
            c.resolve_component_width(PageComponent::Tables).unwrap(),
            80
        );
    }

    #[test]
    fn resolve_component_width_pad_fixed() {
        let mut c = ctx(100, 80);
        c.fills.insert(
            PageComponent::CodeBlocks,
            PageFill::Pad(WidthUnit::Fixed(5)),
        );
        assert_eq!(
            c.resolve_component_width(PageComponent::CodeBlocks)
                .unwrap(),
            70
        ); // 80 - 2*5
    }

    #[test]
    fn resolve_component_width_pad_percent() {
        let mut c = ctx(100, 80);
        c.fills.insert(
            PageComponent::CodeBlocks,
            PageFill::Pad(WidthUnit::Percent(10.0)),
        );
        // 10% of content_width (100) = 10, so pad = 10, width = 80 - 20 = 60
        assert_eq!(
            c.resolve_component_width(PageComponent::CodeBlocks)
                .unwrap(),
            60
        );
    }

    #[test]
    fn resolve_component_width_indent_left() {
        let mut c = ctx(100, 80);
        c.fills.insert(
            PageComponent::BlockQuotes,
            PageFill::Indent(WidthUnit::Fixed(8)),
        );
        c.alignments
            .insert(PageComponent::BlockQuotes, PageAlignment::Left);
        assert_eq!(
            c.resolve_component_width(PageComponent::BlockQuotes)
                .unwrap(),
            72
        ); // 80 - 8
    }

    #[test]
    fn resolve_component_width_max() {
        let mut c = ctx(100, 80);
        c.fills
            .insert(PageComponent::Tables, PageFill::Max(WidthUnit::Fixed(60)));
        assert_eq!(
            c.resolve_component_width(PageComponent::Tables).unwrap(),
            60
        );
    }

    #[test]
    fn resolve_component_width_max_capped_by_effective() {
        let mut c = ctx(100, 80);
        c.fills
            .insert(PageComponent::Tables, PageFill::Max(WidthUnit::Fixed(100)));
        assert_eq!(
            c.resolve_component_width(PageComponent::Tables).unwrap(),
            80
        );
    }

    #[test]
    fn resolve_component_width_explicit() {
        let mut c = ctx(100, 80);
        c.fills.insert(
            PageComponent::Images,
            PageFill::Explicit(WidthUnit::Fixed(50)),
        );
        assert_eq!(
            c.resolve_component_width(PageComponent::Images).unwrap(),
            50
        );
    }

    #[test]
    fn resolve_component_width_explicit_percent() {
        let mut c = ctx(100, 80);
        c.fills.insert(
            PageComponent::Images,
            PageFill::Explicit(WidthUnit::Percent(50.0)),
        );
        // 50% of content_width (100) = 50, capped to effective_width (80) = 50
        assert_eq!(
            c.resolve_component_width(PageComponent::Images).unwrap(),
            50
        );
    }

    #[test]
    fn alignment_padding_left_is_zero() {
        let c = ctx(100, 80);
        assert_eq!(c.alignment_padding(PageComponent::Tables, 60), 0);
    }

    #[test]
    fn alignment_padding_center() {
        let mut c = ctx(100, 80);
        c.alignments
            .insert(PageComponent::Tables, PageAlignment::Center);
        assert_eq!(c.alignment_padding(PageComponent::Tables, 60), 10); // (80 - 60) / 2
    }

    #[test]
    fn alignment_padding_right() {
        let mut c = ctx(100, 80);
        c.alignments
            .insert(PageComponent::Tables, PageAlignment::Right);
        assert_eq!(c.alignment_padding(PageComponent::Tables, 60), 20); // 80 - 60
    }

    #[test]
    fn alignment_padding_does_not_overflow() {
        let mut c = ctx(100, 80);
        c.alignments
            .insert(PageComponent::Tables, PageAlignment::Right);
        assert_eq!(c.alignment_padding(PageComponent::Tables, 100), 0); // saturates
    }

    #[test]
    fn resolve_width_unit_fixed_caps() {
        assert_eq!(
            resolve_width_unit(WidthUnit::Fixed(50), 100, 80).unwrap(),
            50
        );
        assert_eq!(
            resolve_width_unit(WidthUnit::Fixed(100), 100, 80).unwrap(),
            80
        );
    }

    #[test]
    fn resolve_width_unit_percent() {
        assert_eq!(
            resolve_width_unit(WidthUnit::Percent(50.0), 100, 80).unwrap(),
            50
        );
        assert_eq!(
            resolve_width_unit(WidthUnit::Percent(100.0), 100, 80).unwrap(),
            80
        );
    }

    #[test]
    fn resolve_width_unit_percent_capped() {
        // 90% of 100 = 90, but capped to effective_width 80
        assert_eq!(
            resolve_width_unit(WidthUnit::Percent(90.0), 100, 80).unwrap(),
            80
        );
    }

    #[test]
    fn resolve_width_unit_invalid_percent() {
        assert!(matches!(
            resolve_width_unit(WidthUnit::Percent(150.0), 100, 80).unwrap_err(),
            PageRenderError::InvalidPercent(150.0)
        ));
    }

    #[test]
    fn component_side_padding_pad() {
        let mut c = ctx(100, 80);
        c.fills.insert(
            PageComponent::CodeBlocks,
            PageFill::Pad(WidthUnit::Fixed(4)),
        );
        assert_eq!(
            c.component_side_padding(PageComponent::CodeBlocks).unwrap(),
            (4, 4)
        );
    }

    #[test]
    fn component_side_padding_indent_left() {
        let mut c = ctx(100, 80);
        c.fills.insert(
            PageComponent::BlockQuotes,
            PageFill::Indent(WidthUnit::Fixed(6)),
        );
        c.alignments
            .insert(PageComponent::BlockQuotes, PageAlignment::Left);
        assert_eq!(
            c.component_side_padding(PageComponent::BlockQuotes)
                .unwrap(),
            (6, 0)
        );
    }

    #[test]
    fn component_side_padding_indent_right() {
        let mut c = ctx(100, 80);
        c.fills.insert(
            PageComponent::BlockQuotes,
            PageFill::Indent(WidthUnit::Fixed(6)),
        );
        c.alignments
            .insert(PageComponent::BlockQuotes, PageAlignment::Right);
        assert_eq!(
            c.component_side_padding(PageComponent::BlockQuotes)
                .unwrap(),
            (0, 6)
        );
    }

    #[test]
    fn component_side_padding_indent_center() {
        let mut c = ctx(100, 80);
        c.fills.insert(
            PageComponent::BlockQuotes,
            PageFill::Indent(WidthUnit::Fixed(6)),
        );
        c.alignments
            .insert(PageComponent::BlockQuotes, PageAlignment::Center);
        assert_eq!(
            c.component_side_padding(PageComponent::BlockQuotes)
                .unwrap(),
            (6, 6)
        );
    }

    // ---------- Phase 1: color context tests ----------

    use crate::style::StyleColor;
    use renderable::color::{Color, Tailwind};

    fn red_style_color() -> StyleColor {
        StyleColor {
            color: Color::Tailwind(Tailwind::Red500),
            opacity: None,
        }
    }

    fn blue_style_color() -> StyleColor {
        StyleColor {
            color: Color::Tailwind(Tailwind::Blue500),
            opacity: None,
        }
    }

    #[test]
    fn needs_decoration_true_when_page_color_set() {
        let mut c = ctx(100, 80);
        c.page_color = Some(red_style_color());
        assert!(c.needs_decoration());
    }

    #[test]
    fn needs_decoration_true_when_page_bg_color_set() {
        let mut c = ctx(100, 80);
        c.page_bg_color = Some(red_style_color());
        assert!(c.needs_decoration());
    }

    #[test]
    fn needs_decoration_true_when_component_color_set() {
        let mut c = ctx(100, 80);
        c.component_colors
            .insert(PageComponent::Tables, red_style_color());
        assert!(c.needs_decoration());
    }

    #[test]
    fn has_component_styles_true_when_component_colors_set() {
        let mut c = ctx(100, 80);
        assert!(!c.has_component_styles());
        c.component_colors
            .insert(PageComponent::Tables, red_style_color());
        assert!(c.has_component_styles());
    }

    #[test]
    fn has_component_styles_true_when_component_bg_colors_set() {
        let mut c = ctx(100, 80);
        c.component_bg_colors
            .insert(PageComponent::Tables, red_style_color());
        assert!(c.has_component_styles());
    }

    #[test]
    fn component_color_inherits_from_page() {
        let mut c = ctx(100, 80);
        c.page_color = Some(red_style_color());
        c.component_colors
            .insert(PageComponent::Tables, blue_style_color());

        assert_eq!(
            c.component_color(PageComponent::Tables),
            Some(&blue_style_color())
        );
        assert_eq!(
            c.component_color(PageComponent::Images),
            Some(&red_style_color())
        );
    }

    #[test]
    fn component_bg_color_inherits_from_page() {
        let mut c = ctx(100, 80);
        c.page_bg_color = Some(red_style_color());
        c.component_bg_colors
            .insert(PageComponent::Tables, blue_style_color());

        assert_eq!(
            c.component_bg_color(PageComponent::Tables),
            Some(&blue_style_color())
        );
        assert_eq!(
            c.component_bg_color(PageComponent::Images),
            Some(&red_style_color())
        );
    }
}
