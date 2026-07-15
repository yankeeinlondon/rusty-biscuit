//! [`DarkmatterPage`] - the page-level layout primitive that owns margins,
//! padding, page background, max-width, alignment, and per-component fill
//! settings for darkmatter rendering.

use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use renderable::browser::feature::{
    FeatureAssets, FeatureContext, FeatureResolver, PageFeature, resolve_features,
    serialize_features_body, serialize_features_head,
};
use renderable::target::RenderTarget;

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::discovery::detection::ColorMode as TerminalColorMode;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Layout;

use super::context::LayoutContext;
use super::error::PageRenderError;
use super::types::{PageBackground, PageComponent};
use crate::markdown::Markdown;
use crate::markdown::highlighting::{CodeBlockMode, ColorMode, ThemePair};
use crate::markdown::inline::HorizontalRuleAttrs;
use crate::markdown::output::html::HtmlOptions;
use crate::markdown::output::terminal::{
    ColorDepth, HyperlinkMode, ItalicMode, MermaidMode, TerminalImageMode, TerminalOptions,
};
use crate::style::schema::hr::{HrAlignment, HrKind, HrWeight};
use crate::markdown::block::{
    hr_alignment_to_string, hr_kind_to_string, hr_weight_to_string,
};

/// The renderable policy a `style:`-configured [`PageComponent`] contributes.
///
/// This is the **single source of truth** for a component's `style:` layout and
/// colors — there is no parallel per-component color map. `color` / `bg_color`
/// are stored as alpha-bearing [`PaintColor`] so opacity survives through the
/// render tree to the browser target; the terminal target drops opacity, as
/// documented in `docs/rendering/style.md`. [`StyleColor`] is lowered to
/// [`PaintColor`] at the parser/apply boundary (`style/apply.rs`).
#[derive(Debug, Clone, Default)]
pub struct ComponentPolicy {
    pub layout: renderable::layout::Layout,
    pub color: Option<renderable::style::PaintColor>,
    pub bg_color: Option<renderable::style::PaintColor>,
    pub emphasis: Option<renderable::style::TextEmphasis>,
    pub border: Option<renderable::style::Border>,
    pub word_wrap: Option<renderable::wrap_policy::WordWrap>,
}

/// A page-level layout primitive that owns layout state for darkmatter
/// terminal and browser rendering.
///
/// `DarkmatterPage` is constructed against a [`Terminal`] so it can capture
/// terminal width, color mode, and capability information by value at
/// construction; the page does not borrow the `Terminal`.
///
/// The builder is consuming (`self -> Self`) for ergonomic chaining. With no
/// builder calls, [`DarkmatterPage::render`] matches
/// [`Markdown::as_terminal`](crate::markdown::Markdown::as_terminal) with
/// default options whenever the captured terminal's color mode agrees with the
/// detected default (or the terminal reports `Unknown`). When a real terminal
/// reports a different mode, the page honors *that* mode — the terminal is the
/// source of truth (Decision #4), which `Markdown::as_terminal` cannot observe
/// because it has no `Terminal`. Both the zero-config and decorated layout paths
/// route through the render-tree terminal document renderer; the decorated path
/// additionally applies row decoration (margins, padding, background fill).
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
    /// Color depth captured from the [`Terminal`] at construction, projected
    /// onto darkmatter's [`ColorDepth`] palette via the same thresholds as
    /// [`ColorDepth::auto_detect`]. Threaded into [`TerminalOptions`] on the
    /// decorated render path (see [`Self::render`]) when the caller has not
    /// set one explicitly, so a page built from `Terminal::new_optimistic`
    /// renders with that terminal's reported depth regardless of the ambient
    /// environment. The zero-config path deliberately leaves this unset to
    /// preserve byte-for-byte parity with `Markdown::as_terminal(default)`.
    terminal_color_depth: ColorDepth,
    page_margin: renderable::layout::Edges,
    page_padding: renderable::layout::Edges,
    page_background: PageBackground,
    page_max_width: Option<renderable::layout::TargetValue<renderable::layout::Length>>,
    line_numbers: bool,
    component_policies: HashMap<PageComponent, ComponentPolicy>,
    page_color: Option<renderable::style::PaintColor>,
    page_bg_color: Option<renderable::style::PaintColor>,
    hr_kind: Option<HrKind>,
    hr_weight: Option<HrWeight>,
    hr_alignment: Option<HrAlignment>,
    hr_width: Option<String>,
    options: TerminalOptions,
    /// Stored markdown for [`TerminalRenderable`] support.
    markdown: Option<Markdown>,
    /// Layout for [`TerminalRenderable`] trait compliance.
    layout: Layout,
    /// Page-level stylesheet for HTML output.
    stylesheet: Option<crate::style::bespoke::PageStylesheet>,
    /// Page-level HTML meta tags.
    page_meta: Option<crate::style::bespoke::PageMeta>,
    /// Page-level code block theme override.
    page_code_theme: Option<ThemePair>,
    /// Global hyperlink style from `style.hyperlinks.*`.
    hyperlink_style: Option<crate::style::schema::CommonStyle>,
    /// Local hyperlink override from `style.hyperlinks.local-style`.
    local_hyperlink_style: Option<crate::style::schema::CommonStyle>,
    /// Local image override from `style.images.local-style`.
    local_image_style: Option<crate::style::schema::CommonStyle>,
    /// How code-block theme variants are chosen relative to the page color mode.
    code_block_mode: CodeBlockMode,
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
            terminal_color_mode: terminal.color_mode,
            terminal_color_depth: ColorDepth::from(terminal.color_depth),
            page_margin: renderable::layout::Edges::default(),
            page_padding: renderable::layout::Edges::default(),
            page_background: PageBackground::Transparent,
            page_max_width: None,
            line_numbers: false,
            component_policies: HashMap::new(),
            page_color: None,
            page_bg_color: None,
            hr_kind: None,
            hr_weight: None,
            hr_alignment: None,
            hr_width: None,
            options: TerminalOptions::default(),
            markdown: None,
            layout: Layout::default(),
            stylesheet: None,
            page_meta: None,
            page_code_theme: None,
            hyperlink_style: None,
            local_hyperlink_style: None,
            local_image_style: None,
            code_block_mode: CodeBlockMode::default(),
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

    /// Captured terminal color depth, projected onto darkmatter's
    /// [`ColorDepth`] palette.
    ///
    /// The decorated render path threads this into [`TerminalOptions`] when
    /// the caller has not invoked [`Self::with_color_depth`], so renders
    /// produced through page layout honor the [`Terminal`] the page was
    /// built with rather than re-detecting from the ambient environment.
    pub fn terminal_color_depth(&self) -> ColorDepth {
        self.terminal_color_depth
    }

    /// Configured page margin (renderable [`Edges`](renderable::layout::Edges)).
    pub fn page_margin(&self) -> &renderable::layout::Edges {
        &self.page_margin
    }

    /// Configured page padding (renderable [`Edges`](renderable::layout::Edges)).
    pub fn page_padding(&self) -> &renderable::layout::Edges {
        &self.page_padding
    }

    /// Configured page max width (renderable
    /// [`TargetValue<Length>`](renderable::layout::TargetValue)), if any.
    pub fn page_max_width(&self) -> Option<&renderable::layout::TargetValue<renderable::layout::Length>> {
        self.page_max_width.as_ref()
    }

    /// Configured page background.
    pub fn page_background(&self) -> PageBackground {
        self.page_background
    }

    /// Configured max width resolved to terminal cells, if any.
    ///
    /// A percentage `max-width` resolves against the post-margin/post-padding
    /// content width; the authored [`Length`](renderable::layout::Length) is
    /// retained on the frame so the browser wrapper can emit it as `%`.
    pub fn max_width(&self) -> Option<u16> {
        let content = self.frame_content_width();
        self.page_max_width
            .as_ref()
            .map(|tv| length_to_cells(tv, content))
    }

    /// Terminal content width after horizontal page margins and padding are
    /// removed. The percent base for `max-width`.
    fn frame_content_width(&self) -> u16 {
        let margin_x = length_to_cells(&self.page_margin.left, self.terminal_width)
            .saturating_add(length_to_cells(&self.page_margin.right, self.terminal_width));
        let padding_x = length_to_cells(&self.page_padding.left, self.terminal_width)
            .saturating_add(length_to_cells(&self.page_padding.right, self.terminal_width));
        self.terminal_width
            .saturating_sub(margin_x.saturating_add(padding_x))
    }

    /// Whether line numbers are enabled for code blocks.
    pub fn line_numbers(&self) -> bool {
        self.line_numbers
    }

    /// The renderable [`ComponentPolicy`] for `component`, if any.
    pub fn component_policy(&self, component: PageComponent) -> Option<&ComponentPolicy> {
        self.component_policies.get(&component)
    }

    /// All renderable component policies.
    #[allow(dead_code)]
    pub(crate) fn component_policies(&self) -> &HashMap<PageComponent, ComponentPolicy> {
        &self.component_policies
    }

    /// Configured page foreground color, if any.
    pub fn page_color(&self) -> Option<&renderable::style::PaintColor> {
        self.page_color.as_ref()
    }

    /// Configured page background color, if any.
    pub fn page_bg_color(&self) -> Option<&renderable::style::PaintColor> {
        self.page_bg_color.as_ref()
    }

    /// Resolve effective foreground color for the given component.
    ///
    /// Returns the component-specific color when set, otherwise falls back
    /// to the page-level color.
    pub fn color_for(&self, component: PageComponent) -> Option<&renderable::style::PaintColor> {
        self.component_policies
            .get(&component)
            .and_then(|p| p.color.as_ref())
            .or(self.page_color.as_ref())
    }

    /// Resolve effective background color for the given component.
    ///
    /// Returns the component-specific color when set, otherwise falls back
    /// to the page-level color.
    pub fn bg_color_for(&self, component: PageComponent) -> Option<&renderable::style::PaintColor> {
        self.component_policies
            .get(&component)
            .and_then(|p| p.bg_color.as_ref())
            .or(self.page_bg_color.as_ref())
    }

    /// Configured HR kind, if any.
    pub fn hr_kind(&self) -> Option<HrKind> {
        self.hr_kind
    }

    /// Configured HR weight, if any.
    pub fn hr_weight(&self) -> Option<HrWeight> {
        self.hr_weight
    }

    /// Configured HR alignment, if any.
    pub fn hr_alignment(&self) -> Option<HrAlignment> {
        self.hr_alignment
    }

    /// Configured HR width string, if any.
    pub fn hr_width(&self) -> Option<&str> {
        self.hr_width.as_deref()
    }

    /// Configured page stylesheet, if any.
    pub fn stylesheet(&self) -> Option<&crate::style::bespoke::PageStylesheet> {
        self.stylesheet.as_ref()
    }

    /// Configured page meta tags, if any.
    pub fn page_meta(&self) -> Option<&crate::style::bespoke::PageMeta> {
        self.page_meta.as_ref()
    }

    /// Configured page code theme, if any.
    pub fn page_code_theme(&self) -> Option<&ThemePair> {
        self.page_code_theme.as_ref()
    }

    /// Configured global hyperlink style, if any.
    pub fn hyperlink_style(&self) -> Option<&crate::style::schema::CommonStyle> {
        self.hyperlink_style.as_ref()
    }

    /// Configured local hyperlink style override, if any.
    pub fn local_hyperlink_style(&self) -> Option<&crate::style::schema::CommonStyle> {
        self.local_hyperlink_style.as_ref()
    }

    /// Configured local image style override, if any.
    pub fn local_image_style(&self) -> Option<&crate::style::schema::CommonStyle> {
        self.local_image_style.as_ref()
    }

    /// Build [`HorizontalRuleAttrs`] from the page's resolved HR fields.
    ///
    /// Returns `None` when no HR-specific settings have been configured.
    pub fn hr_defaults(&self) -> Option<HorizontalRuleAttrs> {
        let mut attrs = HorizontalRuleAttrs::default();
        let mut has_any = false;

        if let Some(kind) = self.hr_kind() {
            attrs.kind = Some(hr_kind_to_string(kind).to_string());
            has_any = true;
        }
        if let Some(weight) = self.hr_weight() {
            attrs.weight = Some(hr_weight_to_string(weight).to_string());
            has_any = true;
        }
        if let Some(alignment) = self.hr_alignment() {
            attrs.alignment = Some(hr_alignment_to_string(alignment).to_string());
            has_any = true;
        }
        if let Some(width) = self.hr_width() {
            attrs.width = Some(width.to_string());
            has_any = true;
        }
        if let Some(color) = self.color_for(PageComponent::Hr)
            && let Some(css) = crate::style::color::paint_to_css_string(color)
        {
            attrs.color = Some(css);
            has_any = true;
        }

        if has_any { Some(attrs) } else { None }
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
        use renderable::layout::{Length, Edges as RMargin, TargetValue};

        // Percent sides resolve against the captured terminal width so the
        // cell mirror stays meaningful; vertical sides are always `Ch` rows.
        let base = self.terminal_width;
        let sum = |a: &TargetValue<Length>, b: &TargetValue<Length>| {
            TargetValue::universal(Length::ch(u32::from(
                length_to_cells(a, base).saturating_add(length_to_cells(b, base)),
            )))
        };
        self.layout.margin = RMargin {
            top: sum(&self.page_margin.top, &self.page_padding.top),
            right: sum(&self.page_margin.right, &self.page_padding.right),
            bottom: sum(&self.page_margin.bottom, &self.page_padding.bottom),
            left: sum(&self.page_margin.left, &self.page_padding.left),
        };
        self.layout.max_width = self.page_max_width.clone();
    }

    // ---------- Edges builders ----------

    /// Set all four sides of the margin to `n` cells.
    pub fn with_margin(mut self, n: u16) -> Self {
        self.page_margin = renderable::layout::Edges::all(renderable::layout::Length::ch(u32::from(n)));
        self.rebuild_layout();
        self
    }

    /// Set the horizontal margin (left + right) to `n` columns.
    pub fn with_margin_x(mut self, n: u16) -> Self {
        self.page_margin.left = renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n)));
        self.page_margin.right = renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n)));
        self.rebuild_layout();
        self
    }

    /// Set the vertical margin (top + bottom) to `n` rows.
    pub fn with_margin_y(mut self, n: u16) -> Self {
        self.page_margin.top = renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n)));
        self.page_margin.bottom = renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n)));
        self.rebuild_layout();
        self
    }

    /// Set the top margin to `n` rows.
    pub fn with_margin_top(mut self, n: u16) -> Self {
        self.page_margin.top = renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n)));
        self.rebuild_layout();
        self
    }

    /// Set the bottom margin to `n` rows.
    pub fn with_margin_bottom(mut self, n: u16) -> Self {
        self.page_margin.bottom = renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n)));
        self.rebuild_layout();
        self
    }

    /// Set the left margin to `n` columns.
    pub fn with_margin_left(mut self, n: u16) -> Self {
        self.page_margin.left = renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n)));
        self.rebuild_layout();
        self
    }

    /// Set the right margin to `n` columns.
    pub fn with_margin_right(mut self, n: u16) -> Self {
        self.page_margin.right = renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n)));
        self.rebuild_layout();
        self
    }

    // ---------- Padding builders ----------

    /// Set all four sides of the padding to `n` cells.
    pub fn with_padding(mut self, n: u16) -> Self {
        self.page_padding = renderable::layout::Edges::all(renderable::layout::Length::ch(u32::from(n)));
        self.rebuild_layout();
        self
    }

    /// Set the horizontal padding (left + right) to `n` columns.
    pub fn with_padding_x(mut self, n: u16) -> Self {
        self.page_padding.left = renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n)));
        self.page_padding.right = renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n)));
        self.rebuild_layout();
        self
    }

    /// Set the vertical padding (top + bottom) to `n` rows.
    pub fn with_padding_y(mut self, n: u16) -> Self {
        self.page_padding.top = renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n)));
        self.page_padding.bottom = renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n)));
        self.rebuild_layout();
        self
    }

    /// Set the top padding to `n` rows.
    pub fn with_padding_top(mut self, n: u16) -> Self {
        self.page_padding.top = renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n)));
        self.rebuild_layout();
        self
    }

    /// Set the bottom padding to `n` rows.
    pub fn with_padding_bottom(mut self, n: u16) -> Self {
        self.page_padding.bottom = renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n)));
        self.rebuild_layout();
        self
    }

    /// Set the left padding to `n` columns.
    pub fn with_padding_left(mut self, n: u16) -> Self {
        self.page_padding.left = renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n)));
        self.rebuild_layout();
        self
    }

    /// Set the right padding to `n` columns.
    pub fn with_padding_right(mut self, n: u16) -> Self {
        self.page_padding.right = renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n)));
        self.rebuild_layout();
        self
    }

    // ---------- Page knobs ----------

    /// Set the page background fill strategy.
    pub fn with_page_background(mut self, bg: PageBackground) -> Self {
        self.page_background = bg;
        self
    }

    /// Set the page foreground color.
    pub fn with_page_color(mut self, color: renderable::style::PaintColor) -> Self {
        self.page_color = Some(color);
        self
    }

    /// Set the page background color.
    pub fn with_page_bg_color(mut self, color: renderable::style::PaintColor) -> Self {
        self.page_bg_color = Some(color);
        self
    }

    /// Set the foreground color for a single [`PageComponent`].
    ///
    /// Upserts onto the component's [`ComponentPolicy`] — the single source of
    /// truth — preserving any layout already configured for it.
    pub fn with_component_color(
        mut self,
        component: PageComponent,
        color: renderable::style::PaintColor,
    ) -> Self {
        self.component_policies.entry(component).or_default().color = Some(color);
        self
    }

    /// Set the background color for a single [`PageComponent`].
    ///
    /// Upserts onto the component's [`ComponentPolicy`], preserving its layout.
    pub fn with_component_bg_color(
        mut self,
        component: PageComponent,
        color: renderable::style::PaintColor,
    ) -> Self {
        self.component_policies.entry(component).or_default().bg_color = Some(color);
        self
    }

    /// Set the renderable [`ComponentPolicy`] for a single [`PageComponent`].
    pub fn with_component_policy(mut self, component: PageComponent, policy: ComponentPolicy) -> Self {
        self.component_policies.insert(component, policy);
        self
    }

    /// Cap the content width at `max_width` columns.
    pub fn with_max_width(mut self, max_width: u16) -> Self {
        self.page_max_width = Some(renderable::layout::TargetValue::universal(
            renderable::layout::Length::ch(u32::from(max_width)),
        ));
        self.rebuild_layout();
        self
    }

    // ---------- Length-typed frame setters (apply layer) ----------
    //
    // The `style:` apply layer stores the authored `Length` directly so the
    // page frame can resolve percentages per target (terminal cells vs. browser
    // `%`). The `u16` builders above remain the cell-only public API.

    /// Set the left margin to an authored [`Length`](renderable::layout::Length).
    pub(crate) fn with_margin_left_length(mut self, len: renderable::layout::Length) -> Self {
        self.page_margin.left = renderable::layout::TargetValue::universal(len);
        self.rebuild_layout();
        self
    }

    /// Set the right margin to an authored [`Length`](renderable::layout::Length).
    pub(crate) fn with_margin_right_length(mut self, len: renderable::layout::Length) -> Self {
        self.page_margin.right = renderable::layout::TargetValue::universal(len);
        self.rebuild_layout();
        self
    }

    /// Set the left padding to an authored [`Length`](renderable::layout::Length).
    pub(crate) fn with_padding_left_length(mut self, len: renderable::layout::Length) -> Self {
        self.page_padding.left = renderable::layout::TargetValue::universal(len);
        self.rebuild_layout();
        self
    }

    /// Set the right padding to an authored [`Length`](renderable::layout::Length).
    pub(crate) fn with_padding_right_length(mut self, len: renderable::layout::Length) -> Self {
        self.page_padding.right = renderable::layout::TargetValue::universal(len);
        self.rebuild_layout();
        self
    }

    /// Cap the content width at an authored [`Length`](renderable::layout::Length).
    pub(crate) fn with_max_width_length(mut self, len: renderable::layout::Length) -> Self {
        self.page_max_width = Some(renderable::layout::TargetValue::universal(len));
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

    /// Set the HR kind.
    pub fn with_hr_kind(mut self, kind: HrKind) -> Self {
        self.hr_kind = Some(kind);
        self
    }

    /// Set the HR weight.
    pub fn with_hr_weight(mut self, weight: HrWeight) -> Self {
        self.hr_weight = Some(weight);
        self
    }

    /// Set the HR alignment.
    pub fn with_hr_alignment(mut self, alignment: HrAlignment) -> Self {
        self.hr_alignment = Some(alignment);
        self
    }

    /// Set the HR width string.
    pub fn with_hr_width(mut self, width: impl Into<String>) -> Self {
        self.hr_width = Some(width.into());
        self
    }

    // ---------- Bespoke style builders (sub-spec #7) ----------

    /// Set the page stylesheet for HTML output.
    pub fn with_stylesheet(
        mut self,
        stylesheet: crate::style::bespoke::PageStylesheet,
    ) -> Self {
        self.stylesheet = Some(stylesheet);
        self
    }

    /// Set the page meta tags for HTML output.
    pub fn with_page_meta(mut self, meta: crate::style::bespoke::PageMeta) -> Self {
        self.page_meta = Some(meta);
        self
    }

    /// Set the page-level code block theme.
    pub fn with_page_code_theme(mut self, theme: ThemePair) -> Self {
        self.page_code_theme = Some(theme);
        self
    }

    /// Set the global hyperlink style.
    pub fn with_hyperlink_style(
        mut self,
        style: crate::style::schema::CommonStyle,
    ) -> Self {
        self.hyperlink_style = Some(style);
        self
    }

    /// Set the local hyperlink style override.
    pub fn with_local_hyperlink_style(
        mut self,
        style: crate::style::schema::CommonStyle,
    ) -> Self {
        self.local_hyperlink_style = Some(style);
        self
    }

    /// Set the local image style override.
    pub fn with_local_image_style(
        mut self,
        style: crate::style::schema::CommonStyle,
    ) -> Self {
        self.local_image_style = Some(style);
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

    /// Sets how a code block's theme variant is chosen relative to the page
    /// color mode (see [`CodeBlockMode`]).
    #[must_use]
    pub fn with_code_block_mode(mut self, mode: CodeBlockMode) -> Self {
        self.code_block_mode = mode;
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
    /// Derives [`TerminalOptions`] from the page state, folds the document
    /// through the render-tree terminal renderer, then applies row decoration
    /// (margins, padding, background fill) when any layout setting is
    /// non-default.
    ///
    /// Fenced code blocks in `md` fold through [`CodeBlock`]'s
    /// [`TreeRenderable`](crate::markdown::code_block::CodeBlock) projection
    /// the same way the public `CodeBlock::render` path does — the
    /// render-tree terminal renderer wires darkmatter's
    /// [`TerminalCodeRenderer`](crate::markdown::render_tree::TerminalCodeRenderer)
    /// hook, which reproduces the
    /// [`render_terminal_code_block`](crate::markdown::output::code_block::render_terminal_code_block)
    /// output `CodeBlock` itself produces. A fenced ` ```rust ` block in `md`
    /// therefore renders byte-for-byte equal to `CodeBlock::rust(...).render()`
    /// for the same code, language, and metadata.
    ///
    /// When all layout fields are at their defaults, this matches
    /// `Markdown::as_terminal(default)` as long as the captured terminal color
    /// mode agrees with the detected default (or is `Unknown`); a real terminal
    /// reporting a different mode wins, since the terminal is the source of truth
    /// (Decision #4). The decorated path threads a `LayoutContext` into the same
    /// render-tree terminal renderer and then decorates the rendered rows.
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
        crate::style::bespoke::validate_terminal_inline_lengths(self)?;
        let ctx = LayoutContext::from_page(
            self.terminal_width,
            self.page_margin.clone(),
            self.page_padding.clone(),
            self.page_background,
            self.page_max_width.clone(),
            &self.terminal_color_mode,
            self.options.color_mode,
            self.page_color,
            self.page_bg_color,
        )?;

        // Build derived TerminalOptions. `max_width` is deliberately left unset:
        // it would select the optimistic pre-render terminal (its capabilities
        // *and* width). The page keeps width and capability selection independent
        // — width is pinned below via the page-frame geometry decision, color
        // depth rides `options.color_depth` (review-4 finding 1).
        let mut options = self.options.clone();
        options.include_line_numbers = self.line_numbers;
        options.color_mode = ctx.render_color_mode;
        options.hr_defaults = self.hr_defaults();
        options.code_block_mode = self.code_block_mode;

        // Apply page-level code theme override if set via frontmatter.
        if let Some(theme) = self.page_code_theme {
            options.code_theme = theme;
        }

        // Build construction-time context for the context-aware fold. HR
        // defaults come from explicit page options, including values projected
        // from `style.hr.*` frontmatter by the style applicator.
        let hr_defaults_owned = crate::markdown::render_tree::entrypoints::resolve_hr_defaults(
            md,
            &options.hr_defaults,
        );
        let build_ctx = crate::markdown::render_tree::build_context::TreeBuildContext {
            component_policies: &self.component_policies,
            page_color: self.page_color,
            page_bg_color: self.page_bg_color,
            hyperlink_style: self.hyperlink_style.as_ref(),
            local_hyperlink_style: self.local_hyperlink_style.as_ref(),
            local_image_style: self.local_image_style.as_ref(),
            hr_defaults: hr_defaults_owned.as_ref(),
        };

        let result = if self.is_default_layout() {
            // Zero-config path: byte-for-byte parity with
            // `Markdown::as_terminal(default)`. No build context is threaded —
            // doing so would leak the page's captured width into component width
            // resolution and break that equivalence.
            crate::markdown::render_tree::entrypoints::render_tree_terminal(md, &options)
        } else {
            // Build the typed Document ONCE (acceptance criterion 3: one tree
            // build followed by one target fold). The same owned tree feeds the
            // construction-color probe and the terminal fold — no second
            // construction fold (review-4 finding 2).
            let (doc, fold_diagnostics) =
                crate::markdown::render_tree::entrypoints::to_render_document_with_context(
                    md, &build_ctx,
                )?;

            let paints = self.paints_construction_color(&doc);
            // Honor the captured terminal's color depth only when this page
            // actually paints construction-time color. Page-level color always
            // paints; a *matched* component / link / image color paints only when
            // its node is present in the document. An *unmatched* policy bakes
            // nothing, so it must leave the depth unset and render unrelated
            // content at ambient detection — keeping capability selection
            // independent of policy presence (review-3/4). An explicit
            // `with_color_depth` wins.
            if options.color_depth.is_none() && paints {
                options.color_depth = Some(self.terminal_color_depth);
            }

            let has_geometry = self.has_frame_geometry();
            // Width follows page-frame geometry only: the captured
            // `effective_width` when the frame defines its own geometry,
            // otherwise the ambient width (so an unmatched policy cannot widen
            // the content box — review-2 finding 2).
            let content_width = if has_geometry {
                ctx.effective_width
            } else {
                ambient_terminal_width()
            };
            // The optimistic (TrueColor + OSC8) capability profile is selected by
            // page-frame geometry alone — a *deliberate* frame configuration —
            // never by a matched component policy. A matched layout / text-layout
            // policy (e.g. a centered table) bakes its attr onto its own node and
            // is resolved there by the target fold; it must not promote the whole
            // document to optimistic capabilities and so hand unrelated content
            // TrueColor or OSC8 the ambient terminal never advertised
            // (review-5 finding 1). Painted construction color still pins the
            // captured color *depth* above via `paints`, independent of this
            // profile.
            let optimistic_capabilities = has_geometry;
            crate::markdown::render_tree::entrypoints::render_page_terminal_document(
                &doc,
                fold_diagnostics,
                &options,
                content_width,
                optimistic_capabilities,
            )
        }
        .map_err(|e| PageRenderError::Render(e.to_string()))?;

        // Row decoration (vertical-rhythm normalization + margin/padding/
        // background rows) is a page-frame concern. Component policies produce
        // no row decoration — they are baked onto nodes — so the decision keys
        // off page geometry alone; an unmatched policy must not trigger the
        // decorated path (review-1 finding 2).
        if !ctx.needs_decoration() {
            return Ok(result.output);
        }

        // Normalize the body's vertical rhythm before margins are applied, so
        // leading/trailing blank rows come *only* from the configured margins
        // (no constant document-tail offset) and no interior run of >=2 blank
        // lines survives. Runs only on the decorated path; the zero-config path
        // returned above keeps byte-for-byte equivalence with
        // `Markdown::as_terminal(default)`.
        let body = normalize_body_rhythm(&result.output);

        Ok(apply_row_decoration(&body, &ctx))
    }

    // ---------- Browser Rendering ----------

    /// Render the given markdown document to a **body-only** browser HTML
    /// fragment through the page layout.
    ///
    /// The return shape is content-independent: this method **never** emits
    /// document scaffolding (`<!DOCTYPE>`/`<html>`/`<head>`/`<body>`), whatever
    /// the Markdown contains. It is the method an embedder calls when splicing a
    /// render into a host document. For a complete standalone document, call
    /// [`render_to_browser_document`](Self::render_to_browser_document).
    ///
    /// Derives [`HtmlOptions`] from the page state, delegates to the existing
    /// HTML renderer, then either returns the bare body or wraps it in a
    /// page-level `<div>` with CSS styles for margin, padding, max-width,
    /// background-color, and per-component alignment / fill.
    ///
    /// Fenced code blocks in `md` fold through [`CodeBlock`]'s
    /// [`BrowserRenderable`](crate::markdown::code_block::CodeBlock) projection
    /// the same way the public `CodeBlock::render_html_fragment` path does —
    /// the render-tree browser renderer wires darkmatter's
    /// [`TerminalCodeRenderer`](crate::markdown::render_tree::TerminalCodeRenderer)
    /// hook, which reproduces the
    /// [`render_html_code_block`](crate::markdown::output::code_block::render_html_code_block)
    /// output `CodeBlock` itself produces. A fenced ` ```rust ` block in `md`
    /// therefore renders byte-for-byte equal to
    /// `CodeBlock::rust(...).render_html_fragment()` for the same code,
    /// language, and metadata.
    ///
    /// For **feature-free**, undecorated content — no component requests a
    /// browser feature and all layout fields are at their defaults — the output
    /// is the bare `<body>` contents alone, with no wrapper and no document
    /// scaffold. The design-token `:root` block and `.code-block` panel
    /// stylesheet a standalone document carries in `<head>` are produced only by
    /// [`render_to_browser_document`](Self::render_to_browser_document).
    ///
    /// Feature-bearing content diverges, because this page path is
    /// feature-aware:
    ///
    /// - Mermaid defaults to the interactive experience here, whereas
    ///   `HtmlOptions::default()` (and the low-level renderable path) keeps
    ///   Mermaid in `Code` mode. A document with a mermaid fence therefore does
    ///   not match the low-level output.
    /// - Prompted links and Mermaid force the body wrapper into existence so
    ///   their inline `<style>`/`<script>` assets can be embedded (body-only
    ///   feature rendering), rather than emitting a bare no-wrapper body.
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
    ///
    /// Returns [`PageRenderError::FeatureResolution`] when a requested browser
    /// feature cannot be resolved or placed on this body-only path — either an
    /// unresolved browser feature
    /// ([`UnresolvedFeature`](renderable::browser::feature::FeatureResolveError::UnresolvedFeature)),
    /// or a feature whose assets require a document-head `<link>`
    /// ([`HeadRequired`](renderable::browser::feature::FeatureResolveError::HeadRequired)),
    /// which a body-only fragment cannot carry.
    pub fn render_to_browser(&self, md: &Markdown) -> Result<String, PageRenderError> {
        let parts = self.build_browser_parts(md)?;

        // A body-only render never emits document scaffolding. Undecorated,
        // feature-free content is the bare body; anything decorated or
        // feature-bearing rides the forced wrapper fragment (still a single
        // valid element with no nested `<!DOCTYPE>`/`<html>`/`<head>`/`<body>`).
        if parts.is_bare(self) {
            return Ok(parts.body);
        }

        // Body-only placement policy: serialize the resolved features for a
        // wrapper, which raises `HeadRequired` if any feature carries a
        // head-only `<link>` (a body fragment has no `<head>` to host it).
        let feature_assets = serialize_features_body(&parts.resolved_features)?;
        Ok(wrap_browser_html(
            &parts.body,
            &parts.page_assets,
            &parts.ctx,
            self,
            &parts.features,
            &feature_assets,
        ))
    }

    /// Render the given markdown document to a complete, standalone
    /// `<!DOCTYPE html>` browser document through the page layout.
    ///
    /// The return shape is content-independent: this method **always** emits a
    /// full document with a `<head>` and `<body>`, whatever the Markdown
    /// contains. It is the method a caller uses to produce a self-contained,
    /// browser-openable `.html` file. For an embeddable body-only fragment (no
    /// document scaffold), call [`render_to_browser`](Self::render_to_browser).
    ///
    /// Undecorated, feature-free content yields the render-tree's standalone
    /// document — carrying the design-token `:root` block and `.code-block`
    /// panel stylesheet in `<head>`. Decorated or feature-bearing content
    /// assembles a **real `<head>`** around a wrapper-only `<body>`: the head
    /// carries the render-tree's own head (charset, viewport, title, `:root`
    /// design tokens, `.code-block` panel stylesheet), then the page-authored
    /// `<meta>` tags and stylesheet (inline `<style>` or remote `<link>`), then
    /// the resolved feature assets — which, unlike the body-only path, may
    /// include a head-only `<link>` here. The `<body>` holds only the
    /// `<div class="darkmatter-page">` frame (margins, padding, background,
    /// max-width) wrapping the rendered content; the page/feature assets live in
    /// `<head>`, not inside the frame. Page-authored links/styles precede
    /// feature assets, per the spec's asset-ordering contract.
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
    ///
    /// Returns [`PageRenderError::FeatureResolution`] with
    /// [`UnresolvedFeature`](renderable::browser::feature::FeatureResolveError::UnresolvedFeature)
    /// when a requested browser feature cannot be resolved. A standalone
    /// document has a real `<head>`, so a feature's head-only `<link>` is placed
    /// there rather than rejected — unlike [`render_to_browser`](Self::render_to_browser),
    /// this path never raises
    /// [`HeadRequired`](renderable::browser::feature::FeatureResolveError::HeadRequired).
    pub fn render_to_browser_document(&self, md: &Markdown) -> Result<String, PageRenderError> {
        let parts = self.build_browser_parts(md)?;

        // Undecorated, feature-free content already has a complete standalone
        // document from the render-tree renderer (design tokens + panel
        // stylesheet in `<head>`).
        if parts.is_bare(self) {
            return Ok(parts.document);
        }

        // Decorated or feature-bearing: assemble a real `<head>` around a
        // wrapper-only `<body>`. The head starts with the render-tree's own head
        // (which already carries the `:root` tokens and `.code-block` panel
        // stylesheet, so `page_assets` is deliberately *not* re-emitted), then
        // the page-authored meta/stylesheet, then the feature assets serialized
        // for a head (`<link>`s legal here) — keeping page-authored assets ahead
        // of feature assets per the spec ordering. The body is the frame only.
        let mut head = parts.head.clone();
        append_page_meta_head(&mut head, self);
        append_page_stylesheet_head(&mut head, self);
        head.push_str(&serialize_features_head(&parts.resolved_features));

        let frame = wrap_browser_frame(&parts.body, &parts.ctx, self, &parts.features);
        Ok(format!(
            "<!DOCTYPE html><html><head>{head}</head><body>{frame}</body></html>"
        ))
    }

    /// Build the shared browser-render pieces both public browser methods
    /// consume: the standalone document, the body-only fragment, the page-level
    /// assets, the requested features, and their resolved inline assets, plus
    /// the resolved [`LayoutContext`].
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
    ///
    /// Returns [`PageRenderError::FeatureResolution`] when a requested browser
    /// feature cannot be resolved or placed on the body-only page fragment.
    fn build_browser_parts(&self, md: &Markdown) -> Result<BrowserRenderParts, PageRenderError> {
        let ctx = LayoutContext::from_page(
            self.terminal_width,
            self.page_margin.clone(),
            self.page_padding.clone(),
            self.page_background,
            self.page_max_width.clone(),
            &self.terminal_color_mode,
            self.options.color_mode,
            self.page_color,
            self.page_bg_color,
        )?;

        // Build HtmlOptions from TerminalOptions.
        let html_options = HtmlOptions {
            code_theme: self.page_code_theme.unwrap_or(self.options.code_theme),
            prose_theme: self.options.prose_theme,
            color_mode: ctx.render_color_mode,
            code_block_mode: self.code_block_mode,
            include_line_numbers: self.line_numbers,
            include_styles: true,
            mermaid_mode: self.options.mermaid_mode,
            hr_css_variables: std::collections::HashMap::new(),
            hr_defaults: self.hr_defaults(),
            hyperlink_style: self.hyperlink_style.clone(),
            local_hyperlink_style: self.local_hyperlink_style.clone(),
            local_image_style: self.local_image_style.clone(),
        };

        // Build construction-time context for the context-aware fold.
        let hr_defaults_owned = crate::markdown::render_tree::entrypoints::resolve_hr_defaults(
            md,
            &html_options.hr_defaults,
        );
        let build_ctx = crate::markdown::render_tree::build_context::TreeBuildContext {
            component_policies: &self.component_policies,
            page_color: self.page_color,
            page_bg_color: self.page_bg_color,
            hyperlink_style: self.hyperlink_style.as_ref(),
            local_hyperlink_style: self.local_hyperlink_style.as_ref(),
            local_image_style: self.local_image_style.as_ref(),
            hr_defaults: hr_defaults_owned.as_ref(),
        };

        // Darkmatter's full-page browser path is feature-aware: interactive
        // Mermaid is the default and the page owns feature placement. The
        // resolver derives Mermaid colors from the page's resolved code theme;
        // the resolved color mode rides the `FeatureContext`. Every other
        // feature (e.g. Popover) delegates to the shared `DefaultFeatureResolver`.
        let resolver: Rc<dyn FeatureResolver> = Rc::new(
            crate::mermaid::DarkmatterFeatureResolver::new(html_options.code_theme),
        );
        let feature_context = FeatureContext {
            color_mode: render_color_mode_to_renderable(ctx.render_color_mode),
            semantic_colors: Vec::new(),
        };
        // `image_mode == Never` caps browser Mermaid to code; every other mode
        // leaves the interactive/static path enabled (the streaming writer caps
        // `Vector` → static SVG and `Off` → code itself).
        let graphics_mode = match self.options.image_mode {
            TerminalImageMode::Never => renderable::tree::GraphicsMode::Off,
            _ => renderable::tree::GraphicsMode::Rich,
        };

        let rendered = crate::markdown::render_tree::entrypoints::render_tree_html_page_body(
            md,
            &html_options,
            &build_ctx,
            Rc::clone(&resolver),
            feature_context.clone(),
            graphics_mode,
        )
        .map_err(|e| PageRenderError::Render(e.to_string()))?;

        // Resolve the collected features into typed `(feature, assets)` pairs
        // once. Resolution is placement-agnostic — it only fails when a feature
        // is unresolved (`UnresolvedFeature`), which is wrong on either path —
        // and never rejects `<link>` dependencies here. Each public method then
        // applies its own placement policy: `render_to_browser` serializes for a
        // body wrapper (which rejects head-only `<link>`s), while
        // `render_to_browser_document` serializes into the real `<head>` (where
        // `<link>`s are legal).
        let resolved_features = resolve_features(
            &rendered.output.features,
            resolver.as_ref(),
            RenderTarget::Browser,
            &feature_context,
        )?;

        Ok(BrowserRenderParts {
            document: rendered.output.document,
            head: rendered.output.head,
            body: rendered.output.body,
            page_assets: rendered.output.assets,
            features: rendered.output.features,
            resolved_features,
            ctx,
        })
    }

    /// Render the given markdown document to MarkdownPlus through the page
    /// layout.
    ///
    /// MarkdownPlus emits standard Markdown for most constructs but renders
    /// disclosure blocks as HTML `<details>` / `<summary>` elements so they
    /// remain interactive in Markdown-friendly viewers.
    ///
    /// ## Errors
    ///
    /// Returns [`PageRenderError::Render`] when the underlying render-tree
    /// MarkdownPlus renderer fails.
    pub fn render_to_markdown_plus(&self, md: &Markdown) -> Result<String, PageRenderError> {
        let html_options = HtmlOptions {
            code_theme: self.page_code_theme.unwrap_or(self.options.code_theme),
            prose_theme: self.options.prose_theme,
            color_mode: self.options.color_mode,
            code_block_mode: self.code_block_mode,
            include_line_numbers: self.line_numbers,
            include_styles: false,
            mermaid_mode: self.options.mermaid_mode,
            hr_css_variables: std::collections::HashMap::new(),
            hr_defaults: self.hr_defaults(),
            hyperlink_style: self.hyperlink_style.clone(),
            local_hyperlink_style: self.local_hyperlink_style.clone(),
            local_image_style: self.local_image_style.clone(),
        };

        let hr_defaults_owned = crate::markdown::render_tree::entrypoints::resolve_hr_defaults(
            md,
            &html_options.hr_defaults,
        );
        let build_ctx = crate::markdown::render_tree::build_context::TreeBuildContext {
            component_policies: &self.component_policies,
            page_color: self.page_color,
            page_bg_color: self.page_bg_color,
            hyperlink_style: self.hyperlink_style.as_ref(),
            local_hyperlink_style: self.local_hyperlink_style.as_ref(),
            local_image_style: self.local_image_style.as_ref(),
            hr_defaults: hr_defaults_owned.as_ref(),
        };

        crate::markdown::render_tree::entrypoints::render_tree_markdown_plus_with_context(
            md, &build_ctx,
        )
        .map(|r| r.output)
        .map_err(|e| PageRenderError::Render(e.to_string()))
    }

    // ---------- ComponentPolicy merging ----------

    /// Reads the **already-built** `doc` (the single construction fold the caller
    /// passes in) for whether construction baked a foreground/background color: a
    /// page color, or a *matched* component / hyperlink / image color. Drives the
    /// captured color-depth override.
    ///
    /// Style colors are written onto a node *only* by the build context's policy
    /// application — the empty-context fold bakes none — so their presence is a
    /// faithful signal that the page's color configuration matched content.
    ///
    /// The capability *profile* (TrueColor + OSC8) is deliberately **not** keyed
    /// off this signal: a matched policy must stay local to its node and never
    /// promote unrelated content to optimistic capabilities (review-5 finding 1).
    /// Profile selection keys off page-frame geometry alone (see
    /// [`Self::has_frame_geometry`]).
    ///
    /// The read-only walk is skipped unless a color was not already painted at
    /// page level and a component / link / image policy source is configured, so
    /// geometry-only, page-color-only, and zero-config pages never pay for it.
    fn paints_construction_color(&self, doc: &renderable::tree::Document) -> bool {
        let mut paints = self.page_color.is_some() || self.page_bg_color.is_some();
        if !paints && self.has_node_policy_source() {
            scan_painted_color(&doc.root, &mut paints);
        }
        paints
    }

    /// Whether any component, hyperlink, or image policy is configured — the
    /// sources that can bake a per-node attribute. Gates the painted-color walk
    /// in [`Self::paints_construction_color`]; with no such source there is
    /// nothing to find, so the walk is skipped.
    fn has_node_policy_source(&self) -> bool {
        !self.component_policies.is_empty()
            || self.hyperlink_style.is_some()
            || self.local_hyperlink_style.is_some()
            || self.local_image_style.is_some()
    }

    // ---------- Validation ----------

    /// Whether the page frame defines its own content-box geometry: non-default
    /// margins, padding, full-page background, max-width, or line numbers.
    ///
    /// The terminal width cap keys off this rather than [`Self::is_default_layout`]
    /// so the page-frame width decision depends only on frame geometry, never on
    /// node-baked construction inputs (component policies, page/component colors,
    /// hyperlink/image styles, HR defaults). An unmatched component policy
    /// therefore cannot change frame width behavior (review-2 finding 2): it
    /// still threads the build context, but the frame renders at the same width
    /// it would with no policy. The zero-config (no-geometry) path leaves
    /// `max_width` unset, preserving byte-for-byte parity with
    /// `Markdown::as_terminal(default)`.
    ///
    /// The optimistic terminal capability profile (TrueColor + OSC8) keys off
    /// this too (review-5 finding 1): only deliberate frame geometry — never a
    /// matched component policy — promotes the whole document to optimistic
    /// capabilities, so a matched layout (e.g. a centered table) cannot hand
    /// unrelated content color or hyperlinks the ambient terminal never offered.
    fn has_frame_geometry(&self) -> bool {
        !edges_is_zero(&self.page_margin)
            || !edges_is_zero(&self.page_padding)
            || self.page_background != PageBackground::Transparent
            || self.page_max_width.is_some()
            || self.line_numbers
    }

    /// Whether all layout fields are at their defaults.
    ///
    /// When `true`, downstream rendering can short-circuit row decoration and
    /// emit byte-for-byte the same output as `Markdown::as_terminal(default)`
    /// (the zero-config render-tree terminal path).
    #[allow(dead_code)]
    pub(crate) fn is_default_layout(&self) -> bool {
        edges_is_zero(&self.page_margin)
            && edges_is_zero(&self.page_padding)
            && self.page_background == PageBackground::Transparent
            && self.page_max_width.is_none()
            && !self.line_numbers
            && self.component_policies.is_empty()
            && self.page_color.is_none()
            && self.page_bg_color.is_none()
            && self.hyperlink_style.is_none()
            && self.local_hyperlink_style.is_none()
            && self.local_image_style.is_none()
            && self.hr_kind.is_none()
            && self.hr_weight.is_none()
            && self.hr_alignment.is_none()
            && self.hr_width.is_none()
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
        let margin_x = length_to_cells(&self.page_margin.left, self.terminal_width)
            .saturating_add(length_to_cells(&self.page_margin.right, self.terminal_width));
        let padding_x = length_to_cells(&self.page_padding.left, self.terminal_width)
            .saturating_add(length_to_cells(&self.page_padding.right, self.terminal_width));
        let required = margin_x.saturating_add(padding_x);
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
        match self.max_width() {
            Some(0) => Err(PageRenderError::MaxWidthZero),
            _ => Ok(()),
        }
    }

    /// Run all validation helpers in order: horizontal space, max width.
    ///
    /// ## Errors
    ///
    /// Returns the first failing variant from
    /// [`Self::validate_horizontal_space`] or [`Self::validate_max_width`].
    pub fn validate(&self) -> Result<(), PageRenderError> {
        self.validate_horizontal_space()?;
        self.validate_max_width()?;
        Ok(())
    }
}

/// Collapse runs of ≥2 blank lines to one and strip trailing blank lines from a
/// rendered body, enforcing Markdown's vertical-rhythm invariant before page
/// margins are applied.
///
/// A line counts as blank only when it carries no visible glyphs **and** no
/// background fill — a code-block or page padding row (`\x1b[48…`) is content,
/// not blank, and is preserved.
fn normalize_body_rhythm(body: &str) -> String {
    use biscuit_terminal::prelude::strip_escape_codes;

    let is_blank =
        |line: &str| strip_escape_codes(line).trim().is_empty() && !line.contains("\x1b[48");

    let mut out: Vec<&str> = Vec::new();
    let mut prev_blank = false;
    for line in body.lines() {
        let blank = is_blank(line);
        if blank && prev_blank {
            continue; // collapse consecutive blank lines to a single one
        }
        out.push(line);
        prev_blank = blank;
    }
    while out.last().is_some_and(|l| is_blank(l)) {
        out.pop();
    }

    let mut normalized = out.join("\n");
    if !normalized.is_empty() {
        normalized.push('\n');
    }
    normalized
}

/// Wrap rendered markdown body with margin/padding rows and background fill.
fn apply_row_decoration(body: &str, ctx: &LayoutContext) -> String {
    let mut output = String::new();

    let bg = ctx.background_color;
    let reset = "\x1b[0m";

    // Build repeated cell strings. Percent sides resolve against the captured
    // terminal width; vertical sides are `Ch` rows.
    let base = ctx.terminal_width;
    let margin_left = " ".repeat(length_to_cells(&ctx.page_margin.left, base) as usize);
    let margin_right = " ".repeat(length_to_cells(&ctx.page_margin.right, base) as usize);
    let padding_left = " ".repeat(length_to_cells(&ctx.page_padding.left, base) as usize);
    let padding_right = " ".repeat(length_to_cells(&ctx.page_padding.right, base) as usize);

    let bg_open = bg.as_ref().map(|c| c.ansi_bg());
    let bg_reset = bg.as_ref().map(|_| reset);

    // Top margin: transparent empty rows.
    for _ in 0..length_to_cells(&ctx.page_margin.top, base) {
        output.push_str(&margin_left);
        output.push_str(&margin_right);
        output.push('\n');
    }

    // Top padding: bg-filled rows.
    for _ in 0..length_to_cells(&ctx.page_padding.top, base) {
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
    for _ in 0..length_to_cells(&ctx.page_padding.bottom, base) {
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
    for _ in 0..length_to_cells(&ctx.page_margin.bottom, base) {
        output.push_str(&margin_left);
        output.push_str(&margin_right);
        output.push('\n');
    }

    output
}

/// Resolve a page-frame [`TargetValue<Length>`] to whole terminal cells against
/// `base`.
///
/// The page frame retains the authored [`Length`](renderable::layout::Length)
/// so the browser can emit percentages natively (see [`length_to_css_frame`]);
/// the terminal resolves them here. [`Length::Percent`] resolves against `base`
/// (the terminal width for margins/padding, the post-margin/padding content
/// width for `max-width`) with the same rounding as the apply layer.
/// [`Length::Css`] is rejected before reaching the frame, so it maps to `0`.
pub(crate) fn length_to_cells(
    tv: &renderable::layout::TargetValue<renderable::layout::Length>,
    base: u16,
) -> u16 {
    use renderable::layout::{Length, TargetValue};
    match tv {
        TargetValue::Universal(Length::Zero) => 0,
        TargetValue::Universal(Length::Ch(n)) => u16::try_from(*n).unwrap_or(u16::MAX),
        TargetValue::Universal(Length::Percent(p)) => {
            (f32::from(base) * (p / 100.0))
                .round()
                .clamp(0.0, f32::from(u16::MAX)) as u16
        }
        _ => 0,
    }
}

/// Whether a page-frame [`TargetValue<Length>`] contributes no space —
/// [`Length::Zero`], a zero-cell `Ch`, or a `0%` percent. A positive percent is
/// **not** zero even though it has no fixed cell count.
pub(crate) fn length_is_zero(
    tv: &renderable::layout::TargetValue<renderable::layout::Length>,
) -> bool {
    use renderable::layout::{Length, TargetValue};
    match tv {
        TargetValue::Universal(Length::Zero) => true,
        TargetValue::Universal(Length::Ch(n)) => *n == 0,
        TargetValue::Universal(Length::Percent(p)) => *p == 0.0,
        _ => true,
    }
}

/// Lower a page-frame [`TargetValue<Length>`] to a CSS length string for the
/// browser wrapper — `0`, `{n}ch`, or `{p}%`.
pub(crate) fn length_to_css_frame(
    tv: &renderable::layout::TargetValue<renderable::layout::Length>,
) -> String {
    use renderable::layout::{Length, TargetValue};
    match tv {
        // `0ch` (not bare `0`) preserves byte-parity with the historical
        // cell-only wrapper, whose zero sides already serialized as `0ch`.
        TargetValue::Universal(Length::Zero) => "0ch".to_string(),
        TargetValue::Universal(Length::Ch(n)) => format!("{n}ch"),
        TargetValue::Universal(Length::Percent(p)) => format!("{p}%"),
        _ => "0ch".to_string(),
    }
}

/// Sets `paints` when `node` or any descendant carries a construction-baked
/// foreground/background color (see
/// [`DarkmatterPage::paints_construction_color`]).
///
/// At fold time a node's [`Style`](renderable::style::Style) color is written
/// only by the build context's policy application — syntax highlighting and the
/// renderer's own default shaping happen later, during the target fold — so its
/// presence here is a faithful signal that the page's color configuration
/// matched content during construction. Layout and text-layout attrs are
/// intentionally ignored: they resolve locally on their own node and do not
/// select the capability profile (review-5 finding 1).
fn scan_painted_color(node: &renderable::tree::RenderNode, paints: &mut bool) {
    if *paints {
        return;
    }
    if let Some(style) = node.attrs.style()
        && (style.color.is_some() || style.background.is_some())
    {
        *paints = true;
        return;
    }
    for child in node.children() {
        scan_painted_color(child, paints);
    }
}

/// Whether every side of `edges` contributes no space (see [`length_is_zero`]).
fn edges_is_zero(edges: &renderable::layout::Edges) -> bool {
    length_is_zero(&edges.top)
        && length_is_zero(&edges.right)
        && length_is_zero(&edges.bottom)
        && length_is_zero(&edges.left)
}

/// Saturating cast from u32 terminal width to u16, clamped to `u16::MAX`.
fn clamp_width(width: u32) -> u16 {
    width.min(u16::MAX as u32) as u16
}

/// The ambient terminal width the zero-config path renders at.
///
/// A construction-only page (baked attributes but no frame geometry) keeps its
/// content box at this ambient width, so an unmatched component policy cannot
/// widen it (review-2 finding 2). This matches the width
/// `Markdown::as_terminal(default)` resolves, since both fall back to
/// `Terminal::default()`'s detection.
fn ambient_terminal_width() -> u16 {
    clamp_width(Terminal::default().width())
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

/// Maps the layout context's resolved highlighting [`ColorMode`] onto the
/// renderable [`ColorMode`](renderable::color::ColorMode) the
/// [`FeatureContext`] carries.
fn render_color_mode_to_renderable(mode: ColorMode) -> renderable::color::ColorMode {
    match mode {
        ColorMode::Light => renderable::color::ColorMode::Light,
        ColorMode::Dark => renderable::color::ColorMode::Dark,
        ColorMode::Unknown => renderable::color::ColorMode::Unknown,
    }
}

/// Resolves the requested `features` into inline body assets for an embeddable
/// wrapper — the resolve-then-serialize-for-body composition that
/// [`DarkmatterPage::render_to_browser`] performs (via
/// [`resolve_features`] into [`BrowserRenderParts::resolved_features`] and then
/// [`serialize_features_body`]). Kept as a test-only helper so the body-only
/// placement policy — including the [`HeadRequired`](renderable::browser::feature::FeatureResolveError::HeadRequired)
/// rejection of a head-only `<link>` — can be exercised directly against a
/// synthetic resolver without driving a full page render.
///
/// Returns the serialized `<style>`/`<script>` markup (empty when no feature is
/// requested). Because the output is a body-only fragment with no controllable
/// `<head>`, a feature that resolves to a `<link>` dependency is rejected.
///
/// ## Errors
///
/// Returns [`PageRenderError::FeatureResolution`] when a feature is unresolved
/// for the Browser target or resolves to a `<head>`-only `<link>` dependency
/// ([`HeadRequired`](renderable::browser::feature::FeatureResolveError::HeadRequired)).
#[cfg(test)]
pub(crate) fn resolve_feature_body_assets(
    features: &[PageFeature],
    resolver: &dyn FeatureResolver,
    ctx: &FeatureContext,
) -> Result<String, PageRenderError> {
    if features.is_empty() {
        return Ok(String::new());
    }
    let resolved = resolve_features(features, resolver, RenderTarget::Browser, ctx)?;
    Ok(serialize_features_body(&resolved)?)
}

/// The shared browser-render pieces both [`DarkmatterPage::render_to_browser`]
/// and [`DarkmatterPage::render_to_browser_document`] consume. Building these
/// once keeps the two public methods thin so their return shapes stay
/// content-independent.
struct BrowserRenderParts {
    /// Complete standalone `<!DOCTYPE html>` document from the render-tree
    /// browser renderer (carries the `:root` design tokens and `.code-block`
    /// panel stylesheet in `<head>`). Used verbatim only by the bare
    /// (undecorated, feature-free) standalone path.
    document: String,
    /// The render-tree document's inner `<head>` content (charset / viewport /
    /// title / design-token `:root` block / `.code-block` panel stylesheet),
    /// with no feature assets. The decorated standalone path reuses this as the
    /// real `<head>` so it never emits an empty one; because it already carries
    /// the `:root` tokens and panel stylesheet, that path must **not** also
    /// re-emit `page_assets` into the head.
    head: String,
    /// Body-only fragment with no document scaffolding.
    body: String,
    /// Page-level `<style>`/`<script>` the body-only wrapper embeds inline
    /// (design-token `:root` block, `.code-block` panel stylesheet). The
    /// standalone path takes these from `head` instead, so it does not use this.
    page_assets: String,
    /// Requested browser features in first-seen order.
    features: Vec<PageFeature>,
    /// The requested features resolved to typed `(feature, assets)` pairs once,
    /// so each public method can apply its own placement policy:
    /// `render_to_browser` serializes them for a body wrapper
    /// ([`serialize_features_body`], which rejects head-only `<link>`s), while
    /// `render_to_browser_document` serializes them into the real `<head>`
    /// ([`serialize_features_head`], where `<link>`s are legal). Resolution
    /// itself fails only on an unresolved feature, so both paths agree on that.
    resolved_features: Vec<(PageFeature, FeatureAssets)>,
    /// Resolved layout context (decoration + color mode).
    ctx: LayoutContext,
}

impl BrowserRenderParts {
    /// True when the render needs neither a page wrapper nor feature assets, so
    /// the body stands alone: no decoration, no bespoke stylesheet, no page
    /// `<meta>`, and no requested feature. Both public browser methods branch on
    /// this to keep their return shape content-independent — `render_to_browser`
    /// returns the bare body, `render_to_browser_document` the standalone
    /// document.
    ///
    /// Component policies lower to inline CSS on the component elements, never
    /// the wrapper, so their presence must not force a wrapper an unmatched
    /// policy would not need (review-1 finding 2). A requested feature does force
    /// the wrapper into existence so its inline assets have somewhere to live.
    fn is_bare(&self, page: &DarkmatterPage) -> bool {
        !self.ctx.needs_decoration()
            && page.stylesheet().is_none()
            && page.page_meta().is_none()
            && self.features.is_empty()
    }
}

/// Wrap an HTML markdown body **fragment** in a page-level container with
/// layout CSS.
///
/// `body` must be a body-only fragment (no `<!DOCTYPE>`/`<html>`/`<head>`/
/// `<body>`); it is embedded directly so the wrapper is a single valid element.
/// `page_assets` carries the page-level `<style>`/`<script>` the standalone
/// document would place in `<head>` (design-token `:root` block, `.code-block`
/// panel stylesheet, component styles); it is emitted inside the wrapper before
/// the body so the fragment is self-contained.
///
/// When `features` is non-empty the wrapper is stamped with a stable
/// `data-darkmatter-features` attribute (space-separated feature names, for
/// debugging only) and `feature_assets` — the resolved inline `<style>` /
/// `<script>` markup — is emitted **before** the body inside the wrapper, so an
/// embeddable fragment carries its own feature assets.
fn wrap_browser_html(
    body: &str,
    page_assets: &str,
    ctx: &LayoutContext,
    page: &DarkmatterPage,
    features: &[PageFeature],
    feature_assets: &str,
) -> String {
    let mut output = String::new();

    // A body-only fragment has no `<head>`, so the page-authored `<meta>` tags
    // and stylesheet ride inside the wrapper alongside the body (the standalone
    // document path places these in the real `<head>` instead).
    append_page_meta_head(&mut output, page);
    append_page_stylesheet_head(&mut output, page);

    output.push_str(&darkmatter_frame_open(ctx, page, features));

    // Page-level `<style>`/`<script>` (the `:root` design tokens and the
    // `.code-block` panel stylesheet the standalone `<head>` would carry) are
    // embedded inside the wrapper so the fragment styles its own content without
    // a nested `<head>`.
    output.push_str(page_assets);

    // Inline feature assets are placed before the body so feature code can rely
    // on its own declarations being present when the body renders.
    output.push_str(feature_assets);

    output.push_str(body);
    output.push_str("</div>\n");

    output
}

/// Wrap an HTML markdown body **fragment** in the page-level frame `<div>` only.
///
/// Emits exactly `<div class="darkmatter-page" [data-darkmatter-features] …>{body}
/// </div>` — the frame's layout CSS (margins, padding, background, max-width)
/// around `body`, with **no** page `<meta>`, page stylesheet, `page_assets`, or
/// feature assets inside it. Those belong in the document `<head>` on the
/// standalone-document path ([`DarkmatterPage::render_to_browser_document`]),
/// which is the sole caller. The body-only path ([`wrap_browser_html`]) keeps
/// emitting those assets inside the wrapper because a fragment has no `<head>`.
fn wrap_browser_frame(
    body: &str,
    ctx: &LayoutContext,
    page: &DarkmatterPage,
    features: &[PageFeature],
) -> String {
    let mut output = darkmatter_frame_open(ctx, page, features);
    output.push_str(body);
    output.push_str("</div>\n");
    output
}

/// Appends the page's [`PageMeta`](crate::style::bespoke::PageMeta) tags to
/// `out`, one `<meta …/>` per line.
///
/// Shared by the body-only wrapper ([`wrap_browser_html`], which emits them
/// inside the wrapper) and the standalone document
/// ([`DarkmatterPage::render_to_browser_document`], which emits them in
/// `<head>`) so the two paths agree on the exact escaping and format.
fn append_page_meta_head(out: &mut String, page: &DarkmatterPage) {
    let Some(meta) = page.page_meta() else {
        return;
    };
    for tag in &meta.tags {
        match tag {
            crate::style::bespoke::MetaTag::Charset(charset) => {
                out.push_str(&format!(
                    r#"<meta charset="{}" />"#,
                    html_escape::encode_text(charset)
                ));
                out.push('\n');
            }
            crate::style::bespoke::MetaTag::Name { name, content } => {
                out.push_str(&format!(
                    r#"<meta name="{}" content="{}" />"#,
                    html_escape::encode_text(name),
                    html_escape::encode_text(content)
                ));
                out.push('\n');
            }
            crate::style::bespoke::MetaTag::Property { property, content } => {
                out.push_str(&format!(
                    r#"<meta property="{}" content="{}" />"#,
                    html_escape::encode_text(property),
                    html_escape::encode_text(content)
                ));
                out.push('\n');
            }
        }
    }
}

/// Appends the page-authored [`PageStylesheet`](crate::style::bespoke::PageStylesheet)
/// to `out` — an inline `<style>` block or a remote `<link rel="stylesheet">`.
///
/// Shared by the body-only wrapper and the standalone document so both paths
/// emit byte-identical markup (the body-only path inside the wrapper, the
/// standalone path in `<head>`).
fn append_page_stylesheet_head(out: &mut String, page: &DarkmatterPage) {
    let Some(sheet) = page.stylesheet() else {
        return;
    };
    match sheet {
        crate::style::bespoke::PageStylesheet::Inline { source, css } => {
            out.push_str(&format!(
                r#"<style data-darkmatter-source="{}">"#,
                html_escape::encode_text(&source.display().to_string())
            ));
            out.push('\n');
            out.push_str(css);
            if !css.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("</style>\n");
        }
        crate::style::bespoke::PageStylesheet::Remote { href } => {
            out.push_str(&format!(
                r#"<link rel="stylesheet" href="{}" />"#,
                html_escape::encode_text(href)
            ));
            out.push('\n');
        }
    }
}

/// Builds the opening `<div class="darkmatter-page" …>\n` tag carrying the
/// frame's layout CSS and the debug-only `data-darkmatter-features` stamp.
///
/// Shared by [`wrap_browser_html`] (body-only wrapper) and
/// [`wrap_browser_frame`] (standalone document) so the frame geometry is emitted
/// identically on both paths.
fn darkmatter_frame_open(
    ctx: &LayoutContext,
    page: &DarkmatterPage,
    features: &[PageFeature],
) -> String {
    // Build page-level wrapper styles. The page frame retains the authored
    // `Length`, so the browser emits the original unit (`%` percentages resolve
    // against the viewport, not the terminal-cell count the terminal resolves).
    //
    // Horizontal placement: when the author leaves both side margins at their
    // default (zero), a `max-width`-capped frame centers in the viewport via
    // `auto` side margins — the browser convention the spec retains for capped
    // page content. Explicitly authored side margins are emitted verbatim and
    // suppress auto-centering, mirroring the terminal frame's left/right margin
    // placement so the two targets agree.
    let center_frame =
        length_is_zero(&ctx.page_margin.left) && length_is_zero(&ctx.page_margin.right);
    let (ml_css, mr_css) = if center_frame {
        ("auto".to_string(), "auto".to_string())
    } else {
        (
            length_to_css_frame(&ctx.page_margin.left),
            length_to_css_frame(&ctx.page_margin.right),
        )
    };
    let mut wrapper_styles = String::new();
    wrapper_styles.push_str(&format!(
        "margin: {mt} {mr} {mb} {ml}; ",
        mt = length_to_css_frame(&ctx.page_margin.top),
        mr = mr_css,
        mb = length_to_css_frame(&ctx.page_margin.bottom),
        ml = ml_css,
    ));
    wrapper_styles.push_str(&format!(
        "padding: {pt} {pr} {pb} {pl}; ",
        pt = length_to_css_frame(&ctx.page_padding.top),
        pr = length_to_css_frame(&ctx.page_padding.right),
        pb = length_to_css_frame(&ctx.page_padding.bottom),
        pl = length_to_css_frame(&ctx.page_padding.left)
    ));

    if let Some(bg) = ctx.background_color {
        wrapper_styles.push_str(&format!(
            "background-color: rgb({r},{g},{b}); ",
            r = bg.r,
            g = bg.g,
            b = bg.b
        ));
    }

    // Page-level foreground is NOT emitted here: it rides the render tree's root
    // node (`apply_page_colors`), which the browser fold renders as a wrapping
    // `<div>` so the color inherits to descendants through CSS. Emitting it on
    // the frame too would duplicate the declaration.

    // Page-level background color from style frontmatter takes precedence
    // over the computed PageBackground color.
    if let Some(bg_color) = ctx.page_bg_color.as_ref().and_then(crate::style::color::paint_to_css_string) {
        wrapper_styles.push_str(&format!("background-color: {bg_color}; "));
    }

    // When a `max-width` is configured, emit its authored `Length` (a `%`
    // resolves against the viewport in the browser). With none set, fall back to
    // the resolved content width so the wrapper still caps to the frame.
    let max_width_css = match page.page_max_width() {
        Some(tv) => length_to_css_frame(tv),
        None => {
            let ch = if ctx.effective_width < ctx.terminal_width {
                ctx.effective_width
            } else {
                ctx.terminal_width
            };
            format!("{ch}ch")
        }
    };
    wrapper_styles.push_str(&format!("max-width: {max_width_css};"));

    // Start the wrapper div. A feature-bearing render stamps a stable
    // `data-darkmatter-features` attribute (debug-only; not a runtime lookup
    // surface) listing the requested feature names in first-seen order.
    let mut output = String::new();
    output.push_str("<div class=\"darkmatter-page\"");
    if !features.is_empty() {
        let names = features
            .iter()
            .map(|f| f.name())
            .collect::<Vec<_>>()
            .join(" ");
        output.push_str(&format!(" data-darkmatter-features=\"{names}\""));
    }
    output.push_str(" style=\"");
    output.push_str(&wrapper_styles);
    output.push_str("\">\n");
    output
}

#[cfg(test)]
mod tests;
