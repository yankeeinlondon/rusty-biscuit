//! [`DarkmatterPage`] - the page-level layout primitive that owns margins,
//! padding, page background, max-width, alignment, and per-component fill
//! settings for darkmatter rendering.

use std::collections::HashMap;
use std::path::PathBuf;

use biscuit_terminal::discovery::detection::ColorMode as TerminalColorMode;
use biscuit_terminal::terminal::Terminal;

use super::error::PageRenderError;
use super::types::{
    PageAlignment, PageBackground, PageComponent, PageFill, PageMargin, PagePadding,
};
use crate::markdown::highlighting::{ColorMode, ThemePair};
use crate::markdown::output::terminal::{
    ColorDepth, HyperlinkMode, ItalicMode, MermaidMode, TerminalImageMode, TerminalOptions,
};

/// A page-level layout primitive that owns layout state for darkmatter
/// terminal and browser rendering.
///
/// `DarkmatterPage` is constructed against a [`Terminal`] so it can capture
/// terminal width, color mode, and capability information by value at
/// construction; the page does not borrow the `Terminal`.
///
/// The builder is consuming (`self -> Self`) for ergonomic chaining. With no
/// builder calls, [`DarkmatterPage::render`] is byte-for-byte equivalent to
/// `for_terminal(&md, TerminalOptions::default())`.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::terminal::Terminal;
/// use darkmatter::layout::{DarkmatterPage, PageBackground};
///
/// let term = Terminal::new_optimistic(120);
/// let page = DarkmatterPage::new(&term)
///     .with_margin(2)
///     .with_padding(1)
///     .with_page_background(PageBackground::Subtle)
///     .with_max_width(100);
/// ```
#[derive(Debug, Clone)]
pub struct DarkmatterPage {
    terminal_width: u16,
    terminal_color_mode: TerminalColorMode,
    margin: PageMargin,
    padding: PagePadding,
    page_background: PageBackground,
    max_width: Option<u16>,
    line_numbers: bool,
    alignments: HashMap<PageComponent, PageAlignment>,
    fills: HashMap<PageComponent, PageFill>,
    options: TerminalOptions,
}

impl DarkmatterPage {
    /// Construct a new page that captures terminal width and color mode by
    /// value.
    ///
    /// All layout fields default to values that preserve the existing
    /// `for_terminal` behavior: zero margin, zero padding, transparent
    /// background, no max width, no line numbers, left component alignment,
    /// and full component fill.
    pub fn new(terminal: &Terminal) -> Self {
        Self {
            terminal_width: clamp_width(terminal.width()),
            terminal_color_mode: terminal.color_mode.clone(),
            margin: PageMargin::ZERO,
            padding: PagePadding::ZERO,
            page_background: PageBackground::Transparent,
            max_width: None,
            line_numbers: false,
            alignments: HashMap::new(),
            fills: HashMap::new(),
            options: TerminalOptions::default(),
        }
    }

    /// Captured terminal width in columns.
    pub fn terminal_width(&self) -> u16 {
        self.terminal_width
    }

    /// Captured terminal color mode (from biscuit-terminal capability
    /// detection).
    ///
    /// Distinct from the highlighting [`ColorMode`] used by
    /// [`TerminalOptions::color_mode`], which only ever resolves to `Light`
    /// or `Dark`. This accessor preserves the underlying terminal's reported
    /// state, including [`TerminalColorMode::Unknown`].
    pub fn terminal_color_mode(&self) -> &TerminalColorMode {
        &self.terminal_color_mode
    }

    /// Configured page margin.
    pub fn margin(&self) -> PageMargin {
        self.margin
    }

    /// Configured page padding.
    pub fn padding(&self) -> PagePadding {
        self.padding
    }

    /// Configured page background.
    pub fn page_background(&self) -> PageBackground {
        self.page_background
    }

    /// Configured max width, if any.
    pub fn max_width(&self) -> Option<u16> {
        self.max_width
    }

    /// Whether line numbers are enabled for code blocks.
    pub fn line_numbers(&self) -> bool {
        self.line_numbers
    }

    /// Resolve alignment for the given component, defaulting to
    /// [`PageAlignment::Left`].
    pub fn alignment_for(&self, component: PageComponent) -> PageAlignment {
        self.alignments
            .get(&component)
            .copied()
            .unwrap_or(PageAlignment::Left)
    }

    /// Resolve fill for the given component, defaulting to [`PageFill::Full`].
    pub fn fill_for(&self, component: PageComponent) -> PageFill {
        self.fills
            .get(&component)
            .copied()
            .unwrap_or(PageFill::Full)
    }

    /// Read-only view of the underlying [`TerminalOptions`].
    pub fn terminal_options(&self) -> &TerminalOptions {
        &self.options
    }

    // ---------- Margin builders ----------

    /// Set all four sides of the margin to `n` cells.
    pub fn with_margin(mut self, n: u16) -> Self {
        self.margin = PageMargin::all(n);
        self
    }

    /// Set the horizontal margin (left + right) to `n` columns.
    pub fn with_margin_x(mut self, n: u16) -> Self {
        self.margin.left = n;
        self.margin.right = n;
        self
    }

    /// Set the vertical margin (top + bottom) to `n` rows.
    pub fn with_margin_y(mut self, n: u16) -> Self {
        self.margin.top = n;
        self.margin.bottom = n;
        self
    }

    /// Set the top margin to `n` rows.
    pub fn with_margin_top(mut self, n: u16) -> Self {
        self.margin.top = n;
        self
    }

    /// Set the bottom margin to `n` rows.
    pub fn with_margin_bottom(mut self, n: u16) -> Self {
        self.margin.bottom = n;
        self
    }

    /// Set the left margin to `n` columns.
    pub fn with_margin_left(mut self, n: u16) -> Self {
        self.margin.left = n;
        self
    }

    /// Set the right margin to `n` columns.
    pub fn with_margin_right(mut self, n: u16) -> Self {
        self.margin.right = n;
        self
    }

    // ---------- Padding builders ----------

    /// Set all four sides of the padding to `n` cells.
    pub fn with_padding(mut self, n: u16) -> Self {
        self.padding = PagePadding::all(n);
        self
    }

    /// Set the horizontal padding (left + right) to `n` columns.
    pub fn with_padding_x(mut self, n: u16) -> Self {
        self.padding.left = n;
        self.padding.right = n;
        self
    }

    /// Set the vertical padding (top + bottom) to `n` rows.
    pub fn with_padding_y(mut self, n: u16) -> Self {
        self.padding.top = n;
        self.padding.bottom = n;
        self
    }

    /// Set the top padding to `n` rows.
    pub fn with_padding_top(mut self, n: u16) -> Self {
        self.padding.top = n;
        self
    }

    /// Set the bottom padding to `n` rows.
    pub fn with_padding_bottom(mut self, n: u16) -> Self {
        self.padding.bottom = n;
        self
    }

    /// Set the left padding to `n` columns.
    pub fn with_padding_left(mut self, n: u16) -> Self {
        self.padding.left = n;
        self
    }

    /// Set the right padding to `n` columns.
    pub fn with_padding_right(mut self, n: u16) -> Self {
        self.padding.right = n;
        self
    }

    // ---------- Page knobs ----------

    /// Set the page background fill strategy.
    pub fn with_page_background(mut self, bg: PageBackground) -> Self {
        self.page_background = bg;
        self
    }

    /// Cap the content width at `max_width` columns.
    pub fn with_max_width(mut self, max_width: u16) -> Self {
        self.max_width = Some(max_width);
        self
    }

    /// Enable line numbers in code blocks.
    pub fn use_line_numbers(mut self) -> Self {
        self.line_numbers = true;
        self
    }

    /// Set whether code blocks include line numbers.
    pub fn with_line_numbers(mut self, on: bool) -> Self {
        self.line_numbers = on;
        self
    }

    // ---------- Alignment / fill ----------

    /// Override alignment for a single [`PageComponent`].
    pub fn use_alignment(mut self, component: PageComponent, alignment: PageAlignment) -> Self {
        self.alignments.insert(component, alignment);
        self
    }

    /// Apply the same alignment to every [`PageComponent`].
    pub fn use_alignment_for_all(mut self, alignment: PageAlignment) -> Self {
        for component in PageComponent::ALL {
            self.alignments.insert(component, alignment);
        }
        self
    }

    /// Override fill for a single [`PageComponent`].
    pub fn with_fill(mut self, component: PageComponent, fill: PageFill) -> Self {
        self.fills.insert(component, fill);
        self
    }

    /// Apply the same fill to every [`PageComponent`].
    pub fn with_fill_for_all(mut self, fill: PageFill) -> Self {
        for component in PageComponent::ALL {
            self.fills.insert(component, fill);
        }
        self
    }

    // ---------- TerminalOptions passthrough ----------

    /// Replace the entire underlying [`TerminalOptions`] in one call.
    ///
    /// First-class builders called *after* this method override individual
    /// fields on the replaced options.
    pub fn with_terminal_options(mut self, options: TerminalOptions) -> Self {
        self.options = options;
        self
    }

    /// Pass through to [`TerminalOptions::image_mode`].
    pub fn with_image_mode(mut self, mode: TerminalImageMode) -> Self {
        self.options.image_mode = mode;
        self
    }

    /// Pass through to [`TerminalOptions::mermaid_mode`].
    pub fn with_mermaid_mode(mut self, mode: MermaidMode) -> Self {
        self.options.mermaid_mode = mode;
        self
    }

    /// Pass through to [`TerminalOptions::hyperlink_mode`].
    pub fn with_hyperlink_mode(mut self, mode: HyperlinkMode) -> Self {
        self.options.hyperlink_mode = mode;
        self
    }

    /// Pass through to [`TerminalOptions::italic_mode`].
    pub fn with_italic_mode(mut self, mode: ItalicMode) -> Self {
        self.options.italic_mode = mode;
        self
    }

    /// Pass through to [`TerminalOptions::dim_mode`].
    pub fn with_dim_mode(
        mut self,
        mode: crate::markdown::output::terminal::DimMode,
    ) -> Self {
        self.options.dim_mode = mode;
        self
    }

    /// Pass through to [`TerminalOptions::color_depth`].
    pub fn with_color_depth(mut self, depth: ColorDepth) -> Self {
        self.options.color_depth = Some(depth);
        self
    }

    /// Pass through to [`TerminalOptions::color_mode`].
    ///
    /// Accepts the darkmatter highlighting [`ColorMode`] (`Light`/`Dark`)
    /// because that is what [`TerminalOptions`] consumes for theme
    /// resolution.
    ///
    /// Note: [`Self::with_page_background`] with [`PageBackground::Pronounced`]
    /// inverts this value during render.
    pub fn with_color_mode(mut self, mode: ColorMode) -> Self {
        self.options.color_mode = mode;
        self
    }

    /// Pass through to [`TerminalOptions::code_theme`].
    pub fn with_code_theme(mut self, theme: impl Into<String>) -> Self {
        self.options.code_theme = ThemePair::from_str_or_default(&theme.into());
        self
    }

    /// Pass through to [`TerminalOptions::prose_theme`].
    pub fn with_prose_theme(mut self, theme: impl Into<String>) -> Self {
        self.options.prose_theme = ThemePair::from_str_or_default(&theme.into());
        self
    }

    /// Pass through to [`TerminalOptions::base_path`].
    pub fn with_base_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.options.base_path = Some(path.into());
        self
    }

    // ---------- Validation ----------

    /// Whether all layout fields are at their defaults.
    ///
    /// When `true`, downstream rendering can short-circuit row decoration and
    /// emit byte-for-byte the same output as
    /// `for_terminal(&md, TerminalOptions::default())`. Reserved for the
    /// terminal-render integration that lands in a later phase.
    #[allow(dead_code)]
    pub(crate) fn is_default_layout(&self) -> bool {
        self.margin == PageMargin::ZERO
            && self.padding == PagePadding::ZERO
            && self.page_background == PageBackground::Transparent
            && self.max_width.is_none()
            && !self.line_numbers
            && self.alignments.is_empty()
            && self.fills.is_empty()
    }

    /// Validate horizontal space requirements against the captured terminal
    /// width.
    ///
    /// ## Errors
    ///
    /// Returns [`PageRenderError::MarginsExceedTerminalWidth`] when the
    /// combined horizontal margin + padding meets or exceeds the terminal
    /// width.
    pub fn validate_horizontal_space(&self) -> Result<(), PageRenderError> {
        let required = self
            .margin
            .horizontal()
            .saturating_add(self.padding.horizontal());
        if required >= self.terminal_width {
            Err(PageRenderError::MarginsExceedTerminalWidth {
                terminal_width: self.terminal_width,
                required,
            })
        } else {
            Ok(())
        }
    }

    /// Validate the `max_width` field, if set.
    ///
    /// ## Errors
    ///
    /// Returns [`PageRenderError::MaxWidthZero`] when `max_width = Some(0)`.
    pub fn validate_max_width(&self) -> Result<(), PageRenderError> {
        match self.max_width {
            Some(0) => Err(PageRenderError::MaxWidthZero),
            _ => Ok(()),
        }
    }

    /// Validate every configured fill (and its contained percent value).
    ///
    /// ## Errors
    ///
    /// Returns [`PageRenderError::InvalidPercent`] when any fill carries a
    /// percent value outside `0.0..=100.0`.
    pub fn validate_fills(&self) -> Result<(), PageRenderError> {
        for fill in self.fills.values() {
            fill.validate()?;
        }
        Ok(())
    }

    /// Run all validation helpers in order: horizontal space, max width,
    /// then fills.
    ///
    /// ## Errors
    ///
    /// Returns the first failing variant from
    /// [`Self::validate_horizontal_space`], [`Self::validate_max_width`], or
    /// [`Self::validate_fills`].
    pub fn validate(&self) -> Result<(), PageRenderError> {
        self.validate_horizontal_space()?;
        self.validate_max_width()?;
        self.validate_fills()?;
        Ok(())
    }
}

/// Saturating cast from u32 terminal width to u16, clamped to `u16::MAX`.
fn clamp_width(width: u32) -> u16 {
    width.min(u16::MAX as u32) as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::WidthUnit;

    fn page() -> DarkmatterPage {
        let term = Terminal::new_optimistic(120);
        DarkmatterPage::new(&term)
    }

    #[test]
    fn defaults_match_spec() {
        let page = page();
        assert_eq!(page.margin(), PageMargin::ZERO);
        assert_eq!(page.padding(), PagePadding::ZERO);
        assert_eq!(page.page_background(), PageBackground::Transparent);
        assert_eq!(page.max_width(), None);
        assert!(!page.line_numbers());
        assert_eq!(
            page.alignment_for(PageComponent::Images),
            PageAlignment::Left
        );
        assert_eq!(page.fill_for(PageComponent::CodeBlocks), PageFill::Full);
        assert!(page.is_default_layout());
    }

    #[test]
    fn captures_terminal_width() {
        let page = page();
        assert_eq!(page.terminal_width(), 120);
    }

    #[test]
    fn margin_shorthand_then_specific_overrides() {
        let page = page().with_margin(2).with_margin_top(0);
        let m = page.margin();
        assert_eq!(m.top, 0);
        assert_eq!(m.right, 2);
        assert_eq!(m.bottom, 2);
        assert_eq!(m.left, 2);
    }

    #[test]
    fn margin_axis_helpers() {
        let page = page().with_margin_x(3).with_margin_y(1);
        let m = page.margin();
        assert_eq!(m.left, 3);
        assert_eq!(m.right, 3);
        assert_eq!(m.top, 1);
        assert_eq!(m.bottom, 1);
    }

    #[test]
    fn padding_shorthand_then_specific_overrides() {
        let page = page().with_padding(2).with_padding_left(0);
        let p = page.padding();
        assert_eq!(p.top, 2);
        assert_eq!(p.right, 2);
        assert_eq!(p.bottom, 2);
        assert_eq!(p.left, 0);
    }

    #[test]
    fn use_line_numbers_sets_flag() {
        let page = page().use_line_numbers();
        assert!(page.line_numbers());

        let page = page.with_line_numbers(false);
        assert!(!page.line_numbers());
    }

    #[test]
    fn alignment_overrides_per_component() {
        let page = page()
            .use_alignment_for_all(PageAlignment::Center)
            .use_alignment(PageComponent::Images, PageAlignment::Left);
        assert_eq!(
            page.alignment_for(PageComponent::Images),
            PageAlignment::Left
        );
        assert_eq!(
            page.alignment_for(PageComponent::Tables),
            PageAlignment::Center
        );
    }

    #[test]
    fn fill_overrides_per_component() {
        let page = page()
            .with_fill_for_all(PageFill::Pad(WidthUnit::Fixed(2)))
            .with_fill(PageComponent::CodeBlocks, PageFill::Full);
        assert_eq!(page.fill_for(PageComponent::CodeBlocks), PageFill::Full);
        assert_eq!(
            page.fill_for(PageComponent::Tables),
            PageFill::Pad(WidthUnit::Fixed(2))
        );
    }

    #[test]
    fn validate_horizontal_space_rejects_overflow() {
        let term = Terminal::new_optimistic(10);
        let page = DarkmatterPage::new(&term).with_margin_x(5).with_padding_x(1);
        let err = page.validate_horizontal_space().unwrap_err();
        assert_eq!(
            err,
            PageRenderError::MarginsExceedTerminalWidth {
                terminal_width: 10,
                required: 12,
            }
        );
    }

    #[test]
    fn validate_horizontal_space_allows_under_width() {
        let page = page().with_margin_x(4).with_padding_x(2);
        page.validate_horizontal_space().unwrap();
    }

    #[test]
    fn validate_max_width_rejects_zero() {
        let page = page().with_max_width(0);
        assert_eq!(
            page.validate_max_width().unwrap_err(),
            PageRenderError::MaxWidthZero
        );
    }

    #[test]
    fn validate_max_width_accepts_unset() {
        page().validate_max_width().unwrap();
    }

    #[test]
    fn validate_max_width_accepts_positive() {
        page().with_max_width(80).validate_max_width().unwrap();
    }

    #[test]
    fn validate_fills_rejects_invalid_percent() {
        let page = page().with_fill(
            PageComponent::CodeBlocks,
            PageFill::Pad(WidthUnit::Percent(150.0)),
        );
        assert_eq!(
            page.validate_fills().unwrap_err(),
            PageRenderError::InvalidPercent(150.0)
        );
    }

    #[test]
    fn validate_runs_in_order() {
        let term = Terminal::new_optimistic(10);
        let page = DarkmatterPage::new(&term)
            .with_margin_x(5)
            .with_padding_x(1)
            .with_max_width(0);
        // horizontal-space check fires first
        assert!(matches!(
            page.validate().unwrap_err(),
            PageRenderError::MarginsExceedTerminalWidth { .. }
        ));
    }

    #[test]
    fn terminal_options_passthrough_overrides_after_replace() {
        let custom = TerminalOptions {
            image_mode: TerminalImageMode::Never,
            ..TerminalOptions::default()
        };
        let page = page()
            .with_terminal_options(custom)
            .with_image_mode(TerminalImageMode::Force);
        assert_eq!(
            page.terminal_options().image_mode,
            TerminalImageMode::Force
        );
    }

    #[test]
    fn captures_terminal_color_mode() {
        let page = page();
        // Optimistic terminal default color_mode value is exposed.
        let _mode = page.terminal_color_mode().clone();
    }
}
