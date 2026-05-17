//! [`DarkmatterPage`] - the page-level layout primitive that owns margins,
//! padding, page background, max-width, alignment, and per-component fill
//! settings for darkmatter rendering.

// `DarkmatterPage` is built from the deprecated page-layout types
// (`PageMargin`, `PagePadding`, `PageAlignment`, `PageFill`); the builder is
// the CLI's only construction path, so this module legitimately references
// them. Page margins are mapped onto `renderable::layout::Layout` via the
// `From`/`TryFrom` bridges in `super::types`.
#![allow(deprecated)]

use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::discovery::detection::ColorMode as TerminalColorMode;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Layout;

use super::context::LayoutContext;
use super::error::PageRenderError;
use super::types::{
    PageAlignment, PageBackground, PageComponent, PageFill, PageMargin, PagePadding, WidthUnit,
};
use crate::markdown::Markdown;
use crate::markdown::highlighting::{ColorMode, ThemePair};
use crate::markdown::output::html::HtmlOptions;
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
    /// Stored markdown for [`TerminalRenderable`] support.
    markdown: Option<Markdown>,
    /// Layout for [`TerminalRenderable`] trait compliance.
    layout: Layout,
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
            markdown: None,
            layout: Layout::default(),
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

    /// Set the markdown document to render for [`TerminalRenderable`] support.
    pub fn with_markdown(mut self, md: Markdown) -> Self {
        self.markdown = Some(md);
        self
    }

    // ---------- Layout synchronization ----------

    /// Rebuild the [`renderable::layout::Layout`] mirror from current page
    /// state.
    ///
    /// Page margin **and** padding are both transparent/filled space outside
    /// the content rectangle; the new [`Layout`] primitive has no separate
    /// padding concept, so the two are summed into the layout margin. The
    /// max-width cap is mapped to a universal `Ch` length. This keeps the
    /// [`TerminalRenderable::layout`] accessor consistent with the builder
    /// state without disturbing the bespoke row-decoration pipeline.
    fn rebuild_layout(&mut self) {
        use renderable::layout::{Length, Margin as RMargin, TargetValue};

        let margin: RMargin = self.margin.into();
        let padding: RMargin = self.padding.into();
        let sum = |a: &TargetValue<Length>, b: &TargetValue<Length>| {
            let cells = |tv: &TargetValue<Length>| match tv {
                TargetValue::Universal(Length::Ch(n)) => *n,
                _ => 0,
            };
            TargetValue::universal(Length::ch(cells(a) + cells(b)))
        };
        self.layout.margin = RMargin {
            top: sum(&margin.top, &padding.top),
            right: sum(&margin.right, &padding.right),
            bottom: sum(&margin.bottom, &padding.bottom),
            left: sum(&margin.left, &padding.left),
        };
        self.layout.max_width = self
            .max_width
            .map(|mw| TargetValue::universal(Length::ch(u32::from(mw))));
    }

    // ---------- Margin builders ----------

    /// Set all four sides of the margin to `n` cells.
    pub fn with_margin(mut self, n: u16) -> Self {
        self.margin = PageMargin::all(n);
        self.rebuild_layout();
        self
    }

    /// Set the horizontal margin (left + right) to `n` columns.
    pub fn with_margin_x(mut self, n: u16) -> Self {
        self.margin.left = n;
        self.margin.right = n;
        self.rebuild_layout();
        self
    }

    /// Set the vertical margin (top + bottom) to `n` rows.
    pub fn with_margin_y(mut self, n: u16) -> Self {
        self.margin.top = n;
        self.margin.bottom = n;
        self.rebuild_layout();
        self
    }

    /// Set the top margin to `n` rows.
    pub fn with_margin_top(mut self, n: u16) -> Self {
        self.margin.top = n;
        self.rebuild_layout();
        self
    }

    /// Set the bottom margin to `n` rows.
    pub fn with_margin_bottom(mut self, n: u16) -> Self {
        self.margin.bottom = n;
        self.rebuild_layout();
        self
    }

    /// Set the left margin to `n` columns.
    pub fn with_margin_left(mut self, n: u16) -> Self {
        self.margin.left = n;
        self.rebuild_layout();
        self
    }

    /// Set the right margin to `n` columns.
    pub fn with_margin_right(mut self, n: u16) -> Self {
        self.margin.right = n;
        self.rebuild_layout();
        self
    }

    // ---------- Padding builders ----------

    /// Set all four sides of the padding to `n` cells.
    pub fn with_padding(mut self, n: u16) -> Self {
        self.padding = PagePadding::all(n);
        self.rebuild_layout();
        self
    }

    /// Set the horizontal padding (left + right) to `n` columns.
    pub fn with_padding_x(mut self, n: u16) -> Self {
        self.padding.left = n;
        self.padding.right = n;
        self.rebuild_layout();
        self
    }

    /// Set the vertical padding (top + bottom) to `n` rows.
    pub fn with_padding_y(mut self, n: u16) -> Self {
        self.padding.top = n;
        self.padding.bottom = n;
        self.rebuild_layout();
        self
    }

    /// Set the top padding to `n` rows.
    pub fn with_padding_top(mut self, n: u16) -> Self {
        self.padding.top = n;
        self.rebuild_layout();
        self
    }

    /// Set the bottom padding to `n` rows.
    pub fn with_padding_bottom(mut self, n: u16) -> Self {
        self.padding.bottom = n;
        self.rebuild_layout();
        self
    }

    /// Set the left padding to `n` columns.
    pub fn with_padding_left(mut self, n: u16) -> Self {
        self.padding.left = n;
        self.rebuild_layout();
        self
    }

    /// Set the right padding to `n` columns.
    pub fn with_padding_right(mut self, n: u16) -> Self {
        self.padding.right = n;
        self.rebuild_layout();
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
        self.rebuild_layout();
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
    pub fn with_dim_mode(mut self, mode: crate::markdown::output::terminal::DimMode) -> Self {
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

    // ---------- Rendering ----------

    /// Render the given markdown document through the page layout.
    ///
    /// Derives [`TerminalOptions`] from the page state, delegates to the
    /// existing terminal renderer, then applies row decoration (margins,
    /// padding, background fill) when any layout setting is non-default.
    ///
    /// When all layout fields are at their defaults, this is byte-for-byte
    /// equivalent to `for_terminal(&md, TerminalOptions::default())`.
    ///
    /// ## Errors
    ///
    /// Returns [`PageRenderError::MarginsExceedTerminalWidth`] when margin +
    /// padding meets or exceeds the terminal width.
    ///
    /// Returns [`PageRenderError::MaxWidthZero`] when `max_width` is `Some(0)`.
    ///
    /// Returns [`PageRenderError::Render`] when the underlying markdown
    /// renderer fails.
    pub fn render(&self, md: &Markdown) -> Result<String, PageRenderError> {
        self.validate_fills()?;
        let ctx = LayoutContext::from_page(
            self.terminal_width,
            self.margin,
            self.padding,
            self.page_background,
            self.max_width,
            &self.terminal_color_mode,
            self.options.color_mode,
            self.alignments.clone(),
            self.fills.clone(),
        )?;

        // Build derived TerminalOptions.
        let mut options = self.options.clone();
        // Only cap max_width when layout is actually configured; otherwise
        // delegate with the same auto-detection behaviour as
        // `for_terminal(..., TerminalOptions::default())`.
        if ctx.needs_decoration() || self.max_width.is_some() {
            options.max_width = Some(ctx.effective_width);
        }
        options.include_line_numbers = self.line_numbers;
        options.color_mode = ctx.render_color_mode;

        // Delegate to the existing terminal renderer. When no layout builder
        // has been called we must NOT thread a layout context — doing so leaks
        // the page's captured terminal width into component width resolution
        // and breaks byte-for-byte equivalence with
        // `for_terminal(&md, TerminalOptions::default())`, which performs its
        // own width auto-detection.
        let body = if self.is_default_layout() {
            md.as_terminal_with_layout(options, None)
        } else {
            md.as_terminal_with_layout(options, Some(&ctx))
        }
        .map_err(|e| PageRenderError::Render(e.to_string()))?;

        if !ctx.needs_decoration() {
            return Ok(body);
        }

        Ok(apply_row_decoration(&body, &ctx))
    }

    // ---------- Browser Rendering ----------

    /// Render the given markdown document to browser-compatible HTML through
    /// the page layout.
    ///
    /// Derives [`HtmlOptions`] from the page state, delegates to the existing
    /// HTML renderer, then wraps the result in a page-level `<div>` with CSS
    /// styles for margin, padding, max-width, background-color, and
    /// per-component alignment / fill.
    ///
    /// When all layout fields are at their defaults, the output is the same as
    /// `md.as_html(HtmlOptions::default())` with no wrapper.
    ///
    /// ## Errors
    ///
    /// Returns [`PageRenderError::MarginsExceedTerminalWidth`] when margin +
    /// padding meets or exceeds the terminal width.
    ///
    /// Returns [`PageRenderError::MaxWidthZero`] when `max_width` is `Some(0)`.
    ///
    /// Returns [`PageRenderError::Render`] when the underlying markdown HTML
    /// renderer fails.
    pub fn render_to_browser(&self, md: &Markdown) -> Result<String, PageRenderError> {
        self.validate_fills()?;
        let ctx = LayoutContext::from_page(
            self.terminal_width,
            self.margin,
            self.padding,
            self.page_background,
            self.max_width,
            &self.terminal_color_mode,
            self.options.color_mode,
            self.alignments.clone(),
            self.fills.clone(),
        )?;

        // Build HtmlOptions from TerminalOptions.
        let html_options = HtmlOptions {
            code_theme: self.options.code_theme,
            prose_theme: self.options.prose_theme,
            color_mode: ctx.render_color_mode,
            include_line_numbers: self.line_numbers,
            include_styles: true,
            mermaid_mode: self.options.mermaid_mode,
            hr_css_variables: std::collections::HashMap::new(),
        };

        let body = md
            .as_html(html_options)
            .map_err(|e| PageRenderError::Render(e.to_string()))?;

        if !ctx.needs_decoration() && !ctx.has_component_styles() {
            return Ok(body);
        }

        Ok(wrap_browser_html(&body, &ctx))
    }

    // ---------- Validation ----------

    /// Whether all layout fields are at their defaults.
    ///
    /// When `true`, downstream rendering can short-circuit row decoration and
    /// emit byte-for-byte the same output as
    /// `for_terminal(&md, TerminalOptions::default())`.
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

/// Wrap rendered markdown body with margin/padding rows and background fill.
fn apply_row_decoration(body: &str, ctx: &LayoutContext) -> String {
    let mut output = String::new();

    let bg = ctx.background_color;
    let reset = "\x1b[0m";

    // Build repeated cell strings.
    let margin_left = " ".repeat(ctx.margin_left as usize);
    let margin_right = " ".repeat(ctx.margin_right as usize);
    let padding_left = " ".repeat(ctx.padding_left as usize);
    let padding_right = " ".repeat(ctx.padding_right as usize);

    let bg_open = bg.as_ref().map(|c| c.ansi_bg());
    let bg_reset = bg.as_ref().map(|_| reset);

    // Top margin: transparent empty rows.
    for _ in 0..ctx.margin_top {
        output.push_str(&margin_left);
        output.push_str(&margin_right);
        output.push('\n');
    }

    // Top padding: bg-filled rows.
    for _ in 0..ctx.padding_top {
        output.push_str(&margin_left);
        if let Some(ref open) = bg_open {
            output.push_str(open);
        }
        output.push_str(&padding_left);
        // Fill to effective_width.
        if bg_open.is_some() {
            let fill = " ".repeat(ctx.effective_width as usize);
            output.push_str(&fill);
        }
        output.push_str(&padding_right);
        if let Some(r) = bg_reset {
            output.push_str(r);
        }
        output.push_str(&margin_right);
        output.push('\n');
    }

    // Content rows.
    for line in body.lines() {
        output.push_str(&margin_left);
        if let Some(open) = &bg_open {
            output.push_str(open);
        }
        output.push_str(&padding_left);
        output.push_str(line);
        // Pad to effective_width with background fill if needed.
        // Only pad when there is an actual background color; otherwise
        // transparent padding is unnecessary and would make lines longer
        // than their natural width.
        if bg_open.is_some() {
            let visible_len =
                biscuit_terminal::utils::block_constraint::visible_width(line) as usize;
            if visible_len < ctx.effective_width as usize {
                let fill = " ".repeat(ctx.effective_width as usize - visible_len);
                if let Some(open) = &bg_open {
                    output.push_str(open);
                }
                output.push_str(&fill);
            }
        }
        output.push_str(&padding_right);
        if let Some(r) = bg_reset {
            output.push_str(r);
        }
        output.push_str(&margin_right);
        output.push('\n');
    }

    // Bottom padding: bg-filled rows.
    for _ in 0..ctx.padding_bottom {
        output.push_str(&margin_left);
        if let Some(open) = &bg_open {
            output.push_str(open);
        }
        output.push_str(&padding_left);
        if bg_open.is_some() {
            let fill = " ".repeat(ctx.effective_width as usize);
            output.push_str(&fill);
        }
        output.push_str(&padding_right);
        if let Some(r) = bg_reset {
            output.push_str(r);
        }
        output.push_str(&margin_right);
        output.push('\n');
    }

    // Bottom margin: transparent empty rows.
    for _ in 0..ctx.margin_bottom {
        output.push_str(&margin_left);
        output.push_str(&margin_right);
        output.push('\n');
    }

    output
}

/// Saturating cast from u32 terminal width to u16, clamped to `u16::MAX`.
fn clamp_width(width: u32) -> u16 {
    width.min(u16::MAX as u32) as u16
}

impl TerminalRenderable for DarkmatterPage {
    fn render(&self, _term: &Terminal) -> String {
        match self.markdown.as_ref() {
            Some(md) => self
                .render(md)
                .unwrap_or_else(|e| format!("[render error: {}]\n", e)),
            None => "[DarkmatterPage: no markdown set]\n".to_string(),
        }
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn is_block_level(&self) -> bool {
        true
    }
}

// `DarkmatterPage` deliberately does not implement `BrowserRenderable`
// (decisions.md item 12A): it is a page assembler that consumes many
// fragments, not a single composable component. Browser output goes
// through the inherent [`DarkmatterPage::render_to_browser`] method,
// which takes the `Markdown` explicitly.

/// Wrap HTML markdown body in a page-level container with layout CSS.
fn wrap_browser_html(body: &str, ctx: &LayoutContext) -> String {
    let mut output = String::new();

    // Build page-level wrapper styles.
    let mut wrapper_styles = String::new();
    wrapper_styles.push_str(&format!(
        "margin: {mt}ch {mr}ch {mb}ch {ml}ch; ",
        mt = ctx.margin_top,
        mr = ctx.margin_right,
        mb = ctx.margin_bottom,
        ml = ctx.margin_left
    ));
    wrapper_styles.push_str(&format!(
        "padding: {pt}ch {pr}ch {pb}ch {pl}ch; ",
        pt = ctx.padding_top,
        pr = ctx.padding_right,
        pb = ctx.padding_bottom,
        pl = ctx.padding_left
    ));

    if let Some(bg) = ctx.background_color {
        wrapper_styles.push_str(&format!(
            "background-color: rgb({r},{g},{b}); ",
            r = bg.r,
            g = bg.g,
            b = bg.b
        ));
    }

    // Use a very large max-width (terminal_width) when no max-width is set,
    // otherwise use the effective width.
    let max_width_ch = if ctx.effective_width < ctx.terminal_width {
        ctx.effective_width
    } else {
        ctx.terminal_width
    };
    wrapper_styles.push_str(&format!("max-width: {mw}ch;", mw = max_width_ch));

    // Start the wrapper div.
    output.push_str("<div class=\"darkmatter-page\" style=\"");
    output.push_str(&wrapper_styles);
    output.push_str("\">\n");

    // Add per-component alignment and fill CSS in a <style> block.
    let component_css = build_component_css(ctx);
    if !component_css.is_empty() {
        output.push_str("<style>\n");
        output.push_str(&component_css);
        output.push_str("</style>\n");
    }

    output.push_str(body);
    output.push_str("</div>\n");

    output
}

/// Generate CSS rules for per-component alignment and fill.
fn build_component_css(ctx: &LayoutContext) -> String {
    let mut css = String::new();

    for component in PageComponent::ALL {
        let alignment = ctx.component_alignment(component);
        let fill = ctx.component_fill(component);

        // Only emit CSS when non-default.
        if alignment == PageAlignment::Left && fill == PageFill::Full {
            continue;
        }

        let selectors = component_selectors(component);
        if selectors.is_empty() {
            continue;
        }

        css.push_str(&format!(".darkmatter-page {} {{\n", selectors));

        // Alignment.
        match alignment {
            PageAlignment::Left => {
                // Default, no rule needed.
            }
            PageAlignment::Center => {
                css.push_str("  margin-left: auto;\n");
                css.push_str("  margin-right: auto;\n");
            }
            PageAlignment::Right => {
                css.push_str("  margin-left: auto;\n");
                css.push_str("  margin-right: 0;\n");
            }
        }

        // Fill.
        match fill {
            PageFill::Full => {
                // Default, no rule needed.
            }
            PageFill::Pad(unit) => {
                if let Ok(pad) = resolve_width_unit_for_browser(unit, ctx) {
                    css.push_str(&format!("  padding-left: {pad}ch;\n"));
                    css.push_str(&format!("  padding-right: {pad}ch;\n"));
                }
            }
            PageFill::Indent(unit) => {
                if let Ok(indent) = resolve_width_unit_for_browser(unit, ctx) {
                    match alignment {
                        PageAlignment::Left => {
                            css.push_str(&format!("  padding-left: {indent}ch;\n"));
                        }
                        PageAlignment::Right => {
                            css.push_str(&format!("  padding-right: {indent}ch;\n"));
                        }
                        PageAlignment::Center => {
                            css.push_str(&format!("  padding-left: {indent}ch;\n"));
                            css.push_str(&format!("  padding-right: {indent}ch;\n"));
                        }
                    }
                }
            }
            PageFill::Max(unit) => {
                if let Ok(max) = resolve_width_unit_for_browser(unit, ctx) {
                    css.push_str(&format!("  max-width: {max}ch;\n"));
                }
            }
            PageFill::Explicit(unit) => {
                if let Ok(width) = resolve_width_unit_for_browser(unit, ctx) {
                    css.push_str(&format!("  width: {width}ch;\n"));
                }
            }
        }

        css.push_str("}\n");
    }

    css
}

/// CSS selectors for a page component.
fn component_selectors(component: PageComponent) -> &'static str {
    match component {
        PageComponent::Images => "img",
        PageComponent::BlockQuotes => "blockquote",
        PageComponent::Tables => "table",
        PageComponent::CodeBlocks => ".code-block, pre",
        PageComponent::Lists => "ul, ol",
    }
}

/// Resolve a WidthUnit for browser CSS, returning the value in `ch` units.
fn resolve_width_unit_for_browser(
    unit: WidthUnit,
    ctx: &LayoutContext,
) -> Result<u16, PageRenderError> {
    match unit {
        WidthUnit::Fixed(n) => Ok(n.min(ctx.effective_width)),
        WidthUnit::Percent(p) => {
            if !p.is_finite() || !(0.0..=100.0).contains(&p) {
                return Err(PageRenderError::InvalidPercent(p));
            }
            let resolved = (f32::from(ctx.content_width) * (p / 100.0)).round();
            let clamped = resolved.clamp(0.0, f32::from(ctx.effective_width));
            Ok(clamped as u16)
        }
    }
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
        let page = DarkmatterPage::new(&term)
            .with_margin_x(5)
            .with_padding_x(1);
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
        assert_eq!(page.terminal_options().image_mode, TerminalImageMode::Force);
    }

    #[test]
    fn captures_terminal_color_mode() {
        let page = page();
        // Optimistic terminal default color_mode value is exposed.
        let _mode = page.terminal_color_mode().clone();
    }

    // ---------- Phase 2: render tests ----------

    #[test]
    fn zero_config_render_matches_for_terminal() {
        let term = Terminal::new();
        let page = DarkmatterPage::new(&term);
        let md: Markdown = "# Hello World\n\nSome prose here.\n".into();

        let page_out = page.render(&md).unwrap();
        let direct_out =
            crate::markdown::output::terminal::for_terminal(&md, TerminalOptions::default())
                .unwrap();

        assert_eq!(
            page_out, direct_out,
            "zero-config DarkmatterPage::render must match for_terminal byte-for-byte"
        );
    }

    #[test]
    fn zero_config_render_ignores_captured_terminal_width() {
        // Construct a DarkmatterPage from a Terminal whose captured width
        // differs from TerminalOptions::default() auto-detection. The page
        // must NOT leak that captured width into component width resolution;
        // output must remain byte-for-byte identical to `for_terminal()` with
        // default options. Without the `is_default_layout()` short-circuit in
        // `render`, image/list/blockquote/table/code component paths would
        // resolve widths against the captured Terminal width and diverge.
        for width in [40u32, 100, 200] {
            let term = Terminal::new_optimistic(width);
            let page = DarkmatterPage::new(&term);
            let md: Markdown = "# Heading\n\n- List item\n\n> Quoted prose\n\n```rust\nfn main() {}\n```\n\n| A | B |\n| - | - |\n| 1 | 2 |\n".into();

            let page_out = page.render(&md).unwrap();
            let direct_out =
                crate::markdown::output::terminal::for_terminal(&md, TerminalOptions::default())
                    .unwrap();

            assert_eq!(
                page_out, direct_out,
                "zero-config render with captured_width={width} must equal for_terminal default",
            );
        }
    }

    #[test]
    fn zero_config_render_with_list_matches_for_terminal() {
        let term = Terminal::new();
        let page = DarkmatterPage::new(&term);
        let md: Markdown = "# List\n\n- Item one\n- Item two\n".into();

        let page_out = page.render(&md).unwrap();
        let direct_out =
            crate::markdown::output::terminal::for_terminal(&md, TerminalOptions::default())
                .unwrap();

        assert_eq!(
            page_out, direct_out,
            "zero-config render with list must match for_terminal"
        );
    }

    #[test]
    fn zero_config_render_with_code_block_matches_for_terminal() {
        let term = Terminal::new();
        let page = DarkmatterPage::new(&term);
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();

        let page_out = page.render(&md).unwrap();
        let direct_out =
            crate::markdown::output::terminal::for_terminal(&md, TerminalOptions::default())
                .unwrap();

        assert_eq!(
            page_out, direct_out,
            "zero-config render with code block must match for_terminal"
        );
    }

    #[test]
    fn render_with_margin_adds_margin_rows() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_margin_top(2)
            .with_margin_bottom(1);
        let md: Markdown = "# Hello\n".into();

        let out = page.render(&md).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        // First two lines should be empty (top margin).
        assert!(
            lines[0].trim().is_empty(),
            "first line should be top margin"
        );
        assert!(
            lines[1].trim().is_empty(),
            "second line should be top margin"
        );
        // Last line should be empty (bottom margin).
        assert!(
            lines.last().unwrap().trim().is_empty(),
            "last line should be bottom margin"
        );
    }

    #[test]
    fn render_with_padding_adds_bg_rows() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_padding_top(1)
            .with_padding_bottom(1)
            .with_page_background(PageBackground::Subtle);
        let md: Markdown = "# Hello\n".into();

        let out = page.render(&md).unwrap();
        // Should contain ANSI background codes for subtle color.
        assert!(
            out.contains("\x1b[48;2;"),
            "padding rows should have background color"
        );
    }

    #[test]
    fn renderable_trait_with_markdown() {
        let term = Terminal::new_optimistic(80);
        let md: Markdown = "# TerminalRenderable\n".into();
        let page = DarkmatterPage::new(&term).with_markdown(md);

        let out = TerminalRenderable::render(&page, &term);
        let plain = crate::testing::strip_ansi_codes(&out);
        assert!(
            plain.contains("TerminalRenderable"),
            "TerminalRenderable::render should output markdown content"
        );
    }

    #[test]
    fn renderable_trait_without_markdown_shows_placeholder() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);

        let out = TerminalRenderable::render(&page, &term);
        assert!(
            out.contains("no markdown set"),
            "TerminalRenderable without markdown should show placeholder"
        );
    }

    #[test]
    fn renderable_trait_block_level() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        assert!(TerminalRenderable::is_block_level(&page));
    }

    #[test]
    fn renderable_trait_as_any() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        assert!(
            TerminalRenderable::as_any(&page)
                .downcast_ref::<DarkmatterPage>()
                .is_some()
        );
    }

    #[test]
    fn render_with_max_width_caps_content() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term).with_max_width(60);
        let md: Markdown =
            "# Hello\n\nThis is a paragraph that should wrap at the max width.\n".into();

        let out = page.render(&md).unwrap();
        // Verify it renders without error.
        assert!(
            !out.is_empty(),
            "render with max_width should produce output"
        );
    }

    #[test]
    fn render_error_for_max_width_zero() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term).with_max_width(0);
        let md: Markdown = "# Hello\n".into();

        let err = page.render(&md).unwrap_err();
        assert_eq!(err, PageRenderError::MaxWidthZero);
    }

    #[test]
    fn render_error_for_margins_exceed_width() {
        let term = Terminal::new_optimistic(10);
        let page = DarkmatterPage::new(&term)
            .with_margin_x(5)
            .with_padding_x(1);
        let md: Markdown = "# Hello\n".into();

        let err = page.render(&md).unwrap_err();
        assert!(matches!(
            err,
            PageRenderError::MarginsExceedTerminalWidth { .. }
        ));
    }

    // ---------- Phase 3: component layout tests ----------

    #[test]
    fn render_code_block_center_aligned_with_max_fill() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .use_alignment(PageComponent::CodeBlocks, PageAlignment::Center)
            .with_fill(
                PageComponent::CodeBlocks,
                PageFill::Max(WidthUnit::Fixed(40)),
            );
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        // With Max(40) the code block header renders at 40 cols, then the whole
        // 40-col block is centered in 80 => 20 spaces of alignment padding.
        // The header itself is right-aligned within 40: 34 spaces + " rust ".
        // Total leading spaces = 20 + 34 = 54.
        let first_line = plain.lines().next().unwrap();
        let leading_spaces = first_line.len() - first_line.trim_start().len();
        assert!(
            leading_spaces >= 50,
            "code block header should be centered with significant left padding, got {} leading spaces: {:?}",
            leading_spaces,
            first_line
        );
        assert!(first_line.contains("rust"));
    }

    #[test]
    fn render_table_right_aligned_with_max_fill() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .use_alignment(PageComponent::Tables, PageAlignment::Right)
            .with_fill(PageComponent::Tables, PageFill::Max(WidthUnit::Fixed(30)));
        let md: Markdown = "| A | B |\n|---|---|\n| 1 | 2 |\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        // Table rendered at 30 cols, right-aligned in 80 => left pad = 80-30 = 50.
        let table_lines: Vec<&str> = plain
            .lines()
            .filter(|l| l.contains('│') || l.contains('┌') || l.contains('├') || l.contains('└'))
            .collect();
        assert!(!table_lines.is_empty(), "table should render");
        let first = table_lines[0];
        assert!(
            first.starts_with("                                                  "),
            "table should be right-aligned, got: {:?}",
            first
        );
    }

    #[test]
    fn render_code_block_with_max_fill() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term).with_fill(
            PageComponent::CodeBlocks,
            PageFill::Max(WidthUnit::Fixed(40)),
        );
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        // Header row should be capped at 40 cols.
        let first_line = plain.lines().next().unwrap();
        assert!(
            first_line.len() <= 40,
            "code block header should be capped to 40 cols, got len={}",
            first_line.len()
        );
    }

    #[test]
    fn render_code_block_with_pad_fill() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term).with_fill(
            PageComponent::CodeBlocks,
            PageFill::Pad(WidthUnit::Fixed(4)),
        );
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);

        // Pad(4) is symmetric: the component renders at effective_width - 8
        // = 72 cols, and the apply_component_layout helper shifts the block
        // right by 4 cols of left padding (even with the default Left
        // alignment). So lines should be 4 + 72 = 76 visible cols wide.
        //
        // The second line of the rendered block is the top padding row
        // (background fill spanning the full component width), which is the
        // simplest line to measure since it carries no header text.
        let padding_row = plain.lines().nth(1).unwrap();
        assert_eq!(
            padding_row.len(),
            76,
            "padding row should be 4 left pad + 72 content cols, got len={}",
            padding_row.len()
        );
        assert!(
            padding_row.starts_with("    "),
            "padding row should start with 4 leading spaces (Pad left padding)"
        );
    }

    #[test]
    fn render_blockquote_with_indent_fill() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_fill(
                PageComponent::BlockQuotes,
                PageFill::Indent(WidthUnit::Fixed(10)),
            )
            .use_alignment(PageComponent::BlockQuotes, PageAlignment::Left);
        // Long content so the wrap point is observable. Without the active
        // width override, this line would render in a single 80-col span.
        let md: Markdown = "> This is a very long quoted paragraph that should be forced to wrap once the component-specific width override is applied, leaving the remaining text on subsequent lines below.\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        // Strip the blockquote prefix `▐   ` (4 visible cols) from each line.
        // With Indent(10) at 80 cols, prose wraps at 70 cols. The blockquote
        // prefix consumes 4 cols, so the final line content widths should not
        // exceed 70 visible columns.
        let lines: Vec<String> = plain
            .lines()
            .filter(|l| l.contains('▐'))
            .map(|l| l.trim_end().to_string())
            .collect();
        assert!(
            lines.len() >= 2,
            "blockquote should wrap onto multiple lines under Indent(10); got {} line(s):\n{}",
            lines.len(),
            plain
        );
        let max_len = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        assert!(
            max_len <= 70,
            "blockquote lines should be capped to 70 cols by Indent(10), got max={}:\n{}",
            max_len,
            plain
        );
    }

    #[test]
    fn render_list_with_max_fill() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_fill(PageComponent::Lists, PageFill::Max(WidthUnit::Fixed(50)));
        // Long list item so wrap is observable. Without the active width
        // override, this would render at the page width (80) on a single line.
        let md: Markdown = "- This is an unusually long bullet item that ought to be forced to wrap once Max(50) constrains the list rendering width to fifty columns.\n- Short follow-up.\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        let lines: Vec<&str> = plain.lines().collect();
        let max_len = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        assert!(
            max_len <= 50,
            "list lines should be capped to 50 cols, got max={}:\n{}",
            max_len,
            plain
        );
        // Confirm wrap actually occurred: the long item must span >=2 visible
        // lines so the test would fail without the active width override.
        let content_lines = plain.lines().filter(|l| !l.trim().is_empty()).count();
        assert!(
            content_lines >= 3,
            "expected the long item to wrap (>=3 non-empty lines incl. second item), got {}:\n{}",
            content_lines,
            plain
        );
    }

    #[test]
    fn render_image_center_aligned() {
        let term = Terminal::new_optimistic(80);
        let page =
            DarkmatterPage::new(&term).use_alignment(PageComponent::Images, PageAlignment::Center);
        let md: Markdown = "![alt text|20](nonexistent.png)\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        let image_line = plain.lines().find(|l| l.contains("IMAGE")).unwrap_or("");
        // "▉ IMAGE[alt text]\n" has visible width 17 (excluding \n), centered in 80 => (80-17)/2 = 31 spaces.
        assert!(
            image_line.starts_with("                               "),
            "image placeholder should be centered, got: {:?}",
            image_line
        );
    }

    #[test]
    fn zero_config_with_non_default_alignment_still_matches() {
        // When only alignment is set (no margin/padding/bg/max-width), the page
        // should still render successfully and alignment should be applied.
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .use_alignment(PageComponent::CodeBlocks, PageAlignment::Center);
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();

        let out = page.render(&md).unwrap();
        assert!(!out.is_empty());
    }

    // ---------- Phase 4: browser rendering tests ----------

    #[test]
    fn zero_config_browser_render_no_wrapper() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term);
        let md: Markdown = "# Hello World\n\nSome prose here.\n".into();

        let page_html = page.render_to_browser(&md).unwrap();

        // Zero-config page should not add a wrapper div.
        assert!(
            !page_html.contains("<div class=\"darkmatter-page\""),
            "zero-config page should not add wrapper"
        );
        // But should still contain the rendered markdown.
        assert!(
            page_html.contains("<h1>Hello World</h1>"),
            "zero-config page should still render markdown"
        );
    }

    #[test]
    fn browser_render_with_margin_padding_bg_wraps() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_margin(2)
            .with_padding(1)
            .with_page_background(PageBackground::Subtle);
        let md: Markdown = "# Hello\n".into();

        let html = page.render_to_browser(&md).unwrap();
        // Should contain the wrapper div.
        assert!(html.contains("<div class=\"darkmatter-page\""));
        // Should have margin style.
        assert!(html.contains("margin: 2ch 2ch 2ch 2ch"));
        // Should have padding style.
        assert!(html.contains("padding: 1ch 1ch 1ch 1ch"));
        // Should have background color for subtle dark (default is dark mode).
        assert!(html.contains("background-color: rgb(30,30,35)"));
        // Should close the wrapper.
        assert!(html.contains("</div>"));
    }

    #[test]
    fn browser_render_with_max_width() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term).with_max_width(100);
        let md: Markdown = "# Hello\n".into();

        let html = page.render_to_browser(&md).unwrap();
        assert!(html.contains("<div class=\"darkmatter-page\""));
        assert!(html.contains("max-width: 100ch"));
    }

    #[test]
    fn browser_render_with_pronounced_bg() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term).with_page_background(PageBackground::Pronounced);
        let md: Markdown = "# Hello\n".into();

        let html = page.render_to_browser(&md).unwrap();
        // Default color mode is Dark, pronounced on dark => near-white.
        assert!(html.contains("background-color: rgb(245,245,245)"));
    }

    #[test]
    fn browser_render_with_component_alignment_css() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .use_alignment(PageComponent::Tables, PageAlignment::Center)
            .use_alignment(PageComponent::BlockQuotes, PageAlignment::Right);
        let md: Markdown = "# Hello\n".into();

        let html = page.render_to_browser(&md).unwrap();
        // Should contain CSS for centered tables.
        assert!(html.contains(".darkmatter-page table {"));
        assert!(html.contains("margin-left: auto;"));
        assert!(html.contains("margin-right: auto;"));
        // Should contain CSS for right-aligned blockquotes.
        assert!(html.contains(".darkmatter-page blockquote {"));
        assert!(html.contains("margin-right: 0;"));
    }

    #[test]
    fn browser_render_with_component_fill_css() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_fill(
                PageComponent::CodeBlocks,
                PageFill::Max(WidthUnit::Fixed(60)),
            )
            .with_fill(PageComponent::Images, PageFill::Pad(WidthUnit::Fixed(4)));
        let md: Markdown = "# Hello\n".into();

        let html = page.render_to_browser(&md).unwrap();
        // Max fill => max-width CSS.
        assert!(html.contains("max-width: 60ch"));
        // Pad fill => padding CSS.
        assert!(html.contains("padding-left: 4ch"));
        assert!(html.contains("padding-right: 4ch"));
    }

    #[test]
    fn render_to_browser_emits_markdown_html() {
        let term = Terminal::new_optimistic(80);
        let md: Markdown = "# Browser\n".into();
        let page = DarkmatterPage::new(&term);

        // Browser output goes through the inherent method, not a
        // `BrowserRenderable` impl (decisions.md item 12A).
        let html = page.render_to_browser(&md).unwrap();
        assert!(
            html.contains("<h1>Browser</h1>"),
            "render_to_browser should output markdown HTML content"
        );
        // Zero-config page should not add a wrapper div.
        assert!(
            !html.contains("<div class=\"darkmatter-page\""),
            "a zero-config page should not add a wrapper"
        );
    }

    #[test]
    fn browser_render_error_for_max_width_zero() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term).with_max_width(0);
        let md: Markdown = "# Hello\n".into();

        let err = page.render_to_browser(&md).unwrap_err();
        assert_eq!(err, PageRenderError::MaxWidthZero);
    }

    #[test]
    fn browser_render_error_for_margins_exceed_width() {
        let term = Terminal::new_optimistic(10);
        let page = DarkmatterPage::new(&term)
            .with_margin_x(5)
            .with_padding_x(1);
        let md: Markdown = "# Hello\n".into();

        let err = page.render_to_browser(&md).unwrap_err();
        assert!(matches!(
            err,
            PageRenderError::MarginsExceedTerminalWidth { .. }
        ));
    }

    // ---------- Phase 6: error reachability tests ----------

    #[test]
    fn render_error_for_invalid_percent_from_public_api() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term).with_fill(
            PageComponent::CodeBlocks,
            PageFill::Pad(WidthUnit::Percent(150.0)),
        );
        let md: Markdown = "# Hello\n".into();

        let err = page.render(&md).unwrap_err();
        assert_eq!(err, PageRenderError::InvalidPercent(150.0));
    }

    #[test]
    fn browser_render_error_for_invalid_percent_from_public_api() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term).with_fill(
            PageComponent::Tables,
            PageFill::Max(WidthUnit::Percent(-1.0)),
        );
        let md: Markdown = "# Hello\n".into();

        let err = page.render_to_browser(&md).unwrap_err();
        assert!(matches!(err, PageRenderError::InvalidPercent(_)));
    }

    #[test]
    fn render_error_produces_render_variant_from_browser_api() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        // Invalid highlight range triggers MarkdownError::InvalidLineRange
        // which is mapped to PageRenderError::Render in render_to_browser.
        let md: Markdown = "```rust highlight=1-2-3\nfn main() {}\n```\n".into();

        let err = page.render_to_browser(&md).unwrap_err();
        assert!(
            matches!(&err, PageRenderError::Render(msg) if msg.contains("Invalid line range")),
            "expected Render variant with InvalidLineRange message, got: {:?}",
            err
        );
    }

    // ---------- Phase 6: pronounced background test ----------

    #[test]
    fn pronounced_background_on_dark_terminal_inverts_color_mode() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term).with_page_background(PageBackground::Pronounced);
        let md: Markdown = "# Hello\n".into();

        let out = page.render(&md).unwrap();
        // Pronounced on dark terminal => near-white background (245,245,245)
        assert!(
            out.contains("\x1b[48;2;245;245;245m"),
            "pronounced background on dark terminal should emit near-white bg"
        );
    }

    // ---------- Phase 6: regression tests for zero-config equivalence ----------

    #[test]
    fn zero_config_render_with_blockquote_matches_for_terminal() {
        let term = Terminal::new();
        let page = DarkmatterPage::new(&term);
        let md: Markdown = "> A quoted paragraph\n".into();

        let page_out = page.render(&md).unwrap();
        let direct_out =
            crate::markdown::output::terminal::for_terminal(&md, TerminalOptions::default())
                .unwrap();

        assert_eq!(
            page_out, direct_out,
            "zero-config render with blockquote must match for_terminal"
        );
    }

    #[test]
    fn zero_config_render_with_table_matches_for_terminal() {
        let term = Terminal::new();
        let page = DarkmatterPage::new(&term);
        let md: Markdown = "| A | B |\n|---|---|\n| 1 | 2 |\n".into();

        let page_out = page.render(&md).unwrap();
        let direct_out =
            crate::markdown::output::terminal::for_terminal(&md, TerminalOptions::default())
                .unwrap();

        assert_eq!(
            page_out, direct_out,
            "zero-config render with table must match for_terminal"
        );
    }

    #[test]
    fn zero_config_render_with_horizontal_rule_matches_for_terminal() {
        let term = Terminal::new();
        let page = DarkmatterPage::new(&term);
        let md: Markdown = "# Hello\n\n---\n\nWorld\n".into();

        let page_out = page.render(&md).unwrap();
        let direct_out =
            crate::markdown::output::terminal::for_terminal(&md, TerminalOptions::default())
                .unwrap();

        assert_eq!(
            page_out, direct_out,
            "zero-config render with horizontal rule must match for_terminal"
        );
    }

    // ---------- Phase 6: end-to-end snapshot test ----------

    #[test]
    fn end_to_end_example_from_spec() {
        // Terminal is dark mode, 120 cols wide.
        // md doc.md --margin 2 --padding 1 --page-bg subtle --max-width 100 --line-numbers true --align-code-blocks center
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_margin(2)
            .with_padding(1)
            .with_page_background(PageBackground::Subtle)
            .with_max_width(100)
            .use_line_numbers()
            .use_alignment(PageComponent::CodeBlocks, PageAlignment::Center);
        let md: Markdown = "# Title\n\nSome prose here.\n\n```rust\nfn main() {}\n```\n".into();

        let out = page.render(&md).unwrap();
        let lines: Vec<&str> = out.lines().collect();

        // 2 transparent rows top (margin)
        assert!(
            lines[0].trim().is_empty(),
            "first line should be top margin"
        );
        assert!(
            lines[1].trim().is_empty(),
            "second line should be top margin"
        );

        // 1 subtle-bg row (top padding)
        assert!(
            lines[2].contains("\x1b[48;2;30;30;35m"),
            "third line should be top padding with subtle bg"
        );

        // Content should start after margin + padding
        let content_start = 3;
        assert!(
            lines[content_start].contains("Title"),
            "content should start after margin and padding"
        );

        // Find the last content line and verify padding/margin after it
        let _last_content_idx = lines.len() - 4; // 1 bottom padding + 2 bottom margin + trailing newline handling

        // Bottom padding row
        let bottom_padding_idx = lines.len() - 3;
        assert!(
            lines[bottom_padding_idx].contains("\x1b[48;2;30;30;35m"),
            "line before bottom margin should be bottom padding with subtle bg"
        );

        // Bottom margin rows
        assert!(
            lines[lines.len() - 2].trim().is_empty(),
            "second-to-last line should be bottom margin"
        );
        assert!(
            lines[lines.len() - 1].trim().is_empty(),
            "last line should be bottom margin"
        );

        // Verify effective width is capped at 100
        // Each content row should have: 2 margin + 1 padding + content + 1 padding + 2 margin + surplus
        let content_line = lines[content_start];
        let plain = crate::testing::strip_ansi_codes(content_line);
        assert!(
            plain.len() <= 120,
            "content line should not exceed terminal width"
        );
    }
}
