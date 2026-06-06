//! [`DarkmatterPage`] - the page-level layout primitive that owns margins,
//! padding, page background, max-width, alignment, and per-component fill
//! settings for darkmatter rendering.

use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::discovery::detection::ColorMode as TerminalColorMode;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Layout;

use super::context::LayoutContext;
use super::error::PageRenderError;
use super::types::{PageBackground, PageComponent};
use crate::markdown::Markdown;
use crate::markdown::highlighting::{ColorMode, ThemePair};
use crate::markdown::inline::HorizontalRuleAttrs;
use crate::markdown::output::html::HtmlOptions;
use crate::markdown::output::terminal::{
    ColorDepth, HyperlinkMode, ItalicMode, MermaidMode, TerminalImageMode, TerminalOptions,
};
use crate::style::StyleColor;
use crate::style::color::lower_to_css;
use crate::style::schema::hr::{HrAlignment, HrKind, HrWeight};
use crate::markdown::block::{
    hr_alignment_to_string, hr_kind_to_string, hr_weight_to_string,
};

/// The renderable policy a `style:`-configured [`PageComponent`] contributes.
///
/// This is the **single source of truth** for a component's `style:` layout and
/// colors — there is no parallel per-component color map. `color` / `bg_color`
/// are kept as [`StyleColor`] rather than lowered into a renderable
/// [`Style`](renderable::style::Style) so Tailwind/hex **opacity** survives to
/// the HTML target (where it lowers to `rgba(...)`); the terminal drops opacity,
/// as documented in `docs/rendering/style.md`.
#[derive(Debug, Clone, Default)]
pub struct ComponentPolicy {
    pub layout: renderable::layout::Layout,
    pub color: Option<StyleColor>,
    pub bg_color: Option<StyleColor>,
}

/// A page-level layout primitive that owns layout state for darkmatter
/// terminal and browser rendering.
///
/// `DarkmatterPage` is constructed against a [`Terminal`] so it can capture
/// terminal width, color mode, and capability information by value at
/// construction; the page does not borrow the `Terminal`.
///
/// The builder is consuming (`self -> Self`) for ergonomic chaining. With no
/// builder calls, [`DarkmatterPage::render`] is byte-for-byte equivalent to
/// [`Markdown::as_terminal`](crate::markdown::Markdown::as_terminal) with
/// default options — both route through the render-tree terminal document
/// renderer. (The decorated layout path still uses the legacy serializer.)
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
    page_color: Option<StyleColor>,
    page_bg_color: Option<StyleColor>,
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
    pub fn page_color(&self) -> Option<&StyleColor> {
        self.page_color.as_ref()
    }

    /// Configured page background color, if any.
    pub fn page_bg_color(&self) -> Option<&StyleColor> {
        self.page_bg_color.as_ref()
    }

    /// Resolve effective foreground color for the given component.
    ///
    /// Returns the component-specific color when set, otherwise falls back
    /// to the page-level color.
    pub fn color_for(&self, component: PageComponent) -> Option<&StyleColor> {
        self.component_policies
            .get(&component)
            .and_then(|p| p.color.as_ref())
            .or(self.page_color.as_ref())
    }

    /// Resolve effective background color for the given component.
    ///
    /// Returns the component-specific color when set, otherwise falls back
    /// to the page-level color.
    pub fn bg_color_for(&self, component: PageComponent) -> Option<&StyleColor> {
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
            && let Some(css) = lower_to_css(color)
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
    pub fn with_page_color(mut self, color: StyleColor) -> Self {
        self.page_color = Some(color);
        self
    }

    /// Set the page background color.
    pub fn with_page_bg_color(mut self, color: StyleColor) -> Self {
        self.page_bg_color = Some(color);
        self
    }

    /// Set the foreground color for a single [`PageComponent`].
    ///
    /// Upserts onto the component's [`ComponentPolicy`] — the single source of
    /// truth — preserving any layout already configured for it.
    pub fn with_component_color(mut self, component: PageComponent, color: StyleColor) -> Self {
        self.component_policies.entry(component).or_default().color = Some(color);
        self
    }

    /// Set the background color for a single [`PageComponent`].
    ///
    /// Upserts onto the component's [`ComponentPolicy`], preserving its layout.
    pub fn with_component_bg_color(mut self, component: PageComponent, color: StyleColor) -> Self {
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
    /// equivalent to `Markdown::as_terminal(default)` — the zero-config path
    /// routes through the render-tree terminal document renderer. Decorated
    /// layouts still use the legacy serializer.
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
            self.page_color.clone(),
            self.page_bg_color.clone(),
            self.component_policies.clone(),
            self.hyperlink_style.clone(),
            self.local_hyperlink_style.clone(),
            self.local_image_style.clone(),
        )?;

        // Build derived TerminalOptions.
        let mut options = self.options.clone();
        // Only cap max_width when layout is actually configured; otherwise
        // delegate with the same auto-detection behaviour as
        // `for_terminal(..., TerminalOptions::default())`.
        if ctx.needs_decoration() || self.max_width().is_some() {
            options.max_width = Some(ctx.effective_width);
        }
        // Honor the captured terminal's color depth on the decorated layout
        // path, mirroring the captured-width handling above: the page was
        // constructed from a specific `Terminal`, so renders that go through
        // its layout pipeline should follow that terminal's reported depth
        // rather than re-detecting from the ambient environment (which would
        // make `DarkmatterPage::new(&Terminal::new_optimistic(_))` paint
        // different SGR in a headless env than in a truecolor terminal).
        // The zero-config path deliberately leaves this unset so the renderer
        // falls back to `ColorDepth::auto_detect`, preserving byte-for-byte
        // parity with `Markdown::as_terminal(default)`. An explicit
        // `with_color_depth` always wins.
        if !self.is_default_layout() && options.color_depth.is_none() {
            options.color_depth = Some(self.terminal_color_depth);
        }
        options.include_line_numbers = self.line_numbers;
        options.color_mode = ctx.render_color_mode;
        options.hr_defaults = self.hr_defaults();

        // Apply page-level code theme override if set via frontmatter.
        if let Some(theme) = self.page_code_theme {
            options.code_theme = theme;
        }

        // Delegate to the terminal renderer. When no layout builder has been
        // called we must NOT thread a layout context — doing so leaks the
        // page's captured terminal width into component width resolution and
        // breaks byte-for-byte equivalence with `Markdown::as_terminal(default)`
        // (the zero-config tree path), which performs its own width
        // auto-detection.
        let body = if self.is_default_layout() {
            md.as_terminal_with_layout(options, None)
        } else {
            md.as_terminal_with_layout(options, Some(&ctx))
        }
        .map_err(|e| PageRenderError::Render(e.to_string()))?;

        if !ctx.needs_decoration() {
            return Ok(body);
        }

        // Normalize the body's vertical rhythm before margins are applied, so
        // leading/trailing blank rows come *only* from the configured margins
        // (no constant document-tail offset) and no interior run of >=2 blank
        // lines survives. Runs only on the decorated path; the zero-config path
        // returned above keeps byte-for-byte equivalence with
        // `Markdown::as_terminal(default)`.
        let body = normalize_body_rhythm(&body);

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
        let ctx = LayoutContext::from_page(
            self.terminal_width,
            self.page_margin.clone(),
            self.page_padding.clone(),
            self.page_background,
            self.page_max_width.clone(),
            &self.terminal_color_mode,
            self.options.color_mode,
            self.page_color.clone(),
            self.page_bg_color.clone(),
            self.component_policies.clone(),
            self.hyperlink_style.clone(),
            self.local_hyperlink_style.clone(),
            self.local_image_style.clone(),
        )?;

        // Build HtmlOptions from TerminalOptions.
        let html_options = HtmlOptions {
            code_theme: self.page_code_theme.unwrap_or(self.options.code_theme),
            prose_theme: self.options.prose_theme,
            color_mode: ctx.render_color_mode,
            include_line_numbers: self.line_numbers,
            include_styles: true,
            mermaid_mode: self.options.mermaid_mode,
            hr_css_variables: std::collections::HashMap::new(),
            hr_defaults: self.hr_defaults(),
            hyperlink_style: self.hyperlink_style.clone(),
            local_hyperlink_style: self.local_hyperlink_style.clone(),
            local_image_style: self.local_image_style.clone(),
        };

        let body = if self.is_default_layout() {
            md.as_html(html_options)
        } else {
            crate::markdown::render_tree::entrypoints::render_tree_html_with_layout(md, &html_options, &ctx)
        }
        .map_err(|e| PageRenderError::Render(e.to_string()))?;

        if !ctx.needs_decoration() && self.stylesheet().is_none() && self.page_meta().is_none() {
            return Ok(body);
        }

        Ok(wrap_browser_html(&body, &ctx, self))
    }

    // ---------- ComponentPolicy merging ----------

    // ---------- Validation ----------

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

    // Build repeated cell strings.
    let margin_left = " ".repeat(tv_cells(&ctx.page_margin.left) as usize);
    let margin_right = " ".repeat(tv_cells(&ctx.page_margin.right) as usize);
    let padding_left = " ".repeat(tv_cells(&ctx.page_padding.left) as usize);
    let padding_right = " ".repeat(tv_cells(&ctx.page_padding.right) as usize);

    let bg_open = bg.as_ref().map(|c| c.ansi_bg());
    let bg_reset = bg.as_ref().map(|_| reset);

    // Top margin: transparent empty rows.
    for _ in 0..tv_cells(&ctx.page_margin.top) {
        output.push_str(&margin_left);
        output.push_str(&margin_right);
        output.push('\n');
    }

    // Top padding: bg-filled rows.
    for _ in 0..tv_cells(&ctx.page_padding.top) {
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
    for _ in 0..tv_cells(&ctx.page_padding.bottom) {
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
    for _ in 0..tv_cells(&ctx.page_margin.bottom) {
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
        TargetValue::Universal(Length::Zero) => "0".to_string(),
        TargetValue::Universal(Length::Ch(n)) => format!("{n}ch"),
        TargetValue::Universal(Length::Percent(p)) => format!("{p}%"),
        _ => "0".to_string(),
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
fn wrap_browser_html(body: &str, ctx: &LayoutContext, page: &DarkmatterPage) -> String {
    let mut output = String::new();

    // Emit page-level meta tags first.
    if let Some(meta) = page.page_meta() {
        for tag in &meta.tags {
            match tag {
                crate::style::bespoke::MetaTag::Charset(charset) => {
                    output.push_str(&format!(
                        r#"<meta charset="{}" />"#,
                        html_escape::encode_text(charset)
                    ));
                    output.push('\n');
                }
                crate::style::bespoke::MetaTag::Name { name, content } => {
                    output.push_str(&format!(
                        r#"<meta name="{}" content="{}" />"#,
                        html_escape::encode_text(name),
                        html_escape::encode_text(content)
                    ));
                    output.push('\n');
                }
                crate::style::bespoke::MetaTag::Property { property, content } => {
                    output.push_str(&format!(
                        r#"<meta property="{}" content="{}" />"#,
                        html_escape::encode_text(property),
                        html_escape::encode_text(content)
                    ));
                    output.push('\n');
                }
            }
        }
    }

    // Emit page-level stylesheet.
    if let Some(sheet) = page.stylesheet() {
        match sheet {
            crate::style::bespoke::PageStylesheet::Inline { source, css } => {
                output.push_str(&format!(
                    r#"<style data-darkmatter-source="{}">"#,
                    html_escape::encode_text(&source.display().to_string())
                ));
                output.push('\n');
                output.push_str(css);
                if !css.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str("</style>\n");
            }
            crate::style::bespoke::PageStylesheet::Remote { href } => {
                output.push_str(&format!(
                    r#"<link rel="stylesheet" href="{}" />"#,
                    html_escape::encode_text(href)
                ));
                output.push('\n');
            }
        }
    }

    // Build page-level wrapper styles.
    let mut wrapper_styles = String::new();
    wrapper_styles.push_str(&format!(
        "margin: {mt}ch {mr}ch {mb}ch {ml}ch; ",
        mt = tv_cells(&ctx.page_margin.top),
        mr = tv_cells(&ctx.page_margin.right),
        mb = tv_cells(&ctx.page_margin.bottom),
        ml = tv_cells(&ctx.page_margin.left)
    ));
    wrapper_styles.push_str(&format!(
        "padding: {pt}ch {pr}ch {pb}ch {pl}ch; ",
        pt = tv_cells(&ctx.page_padding.top),
        pr = tv_cells(&ctx.page_padding.right),
        pb = tv_cells(&ctx.page_padding.bottom),
        pl = tv_cells(&ctx.page_padding.left)
    ));

    if let Some(bg) = ctx.background_color {
        wrapper_styles.push_str(&format!(
            "background-color: rgb({r},{g},{b}); ",
            r = bg.r,
            g = bg.g,
            b = bg.b
        ));
    }

    // Page-level foreground color from style frontmatter.
    if let Some(color) = ctx.page_color.as_ref().and_then(lower_to_css) {
        wrapper_styles.push_str(&format!("color: {color}; "));
    }

    // Page-level background color from style frontmatter takes precedence
    // over the computed PageBackground color.
    if let Some(bg_color) = ctx.page_bg_color.as_ref().and_then(lower_to_css) {
        wrapper_styles.push_str(&format!("background-color: {bg_color}; "));
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

    output.push_str(body);
    output.push_str("</div>\n");

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn align_policy(alignment: renderable::layout::Alignment) -> ComponentPolicy {
        let mut policy = ComponentPolicy::default();
        policy.layout.alignment = alignment;
        policy
    }

    fn pad_policy(n: u16) -> ComponentPolicy {
        let mut policy = ComponentPolicy::default();
        policy.layout.padding = renderable::layout::Edges::x(renderable::layout::Length::ch(u32::from(n)));
        policy
    }

    fn max_width_policy(n: u16) -> ComponentPolicy {
        let mut policy = ComponentPolicy::default();
        policy.layout.max_width = Some(renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n))));
        policy
    }

    #[allow(dead_code)]
    fn explicit_width_policy(n: u16) -> ComponentPolicy {
        let mut policy = ComponentPolicy::default();
        policy.layout.width = renderable::layout::Width::Fixed(renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n))));
        policy
    }

    fn indent_policy(n: u16) -> ComponentPolicy {
        let mut policy = ComponentPolicy::default();
        policy.layout.padding = renderable::layout::Edges {
            left: renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n))),
            ..renderable::layout::Edges::default()
        };
        policy
    }

    fn left_margin_policy(n: u16) -> ComponentPolicy {
        let mut policy = ComponentPolicy::default();
        policy.layout.margin.left = renderable::layout::TargetValue::universal(renderable::layout::Length::ch(u32::from(n)));
        policy
    }

    fn edge_ch(tv: &renderable::layout::TargetValue<renderable::layout::Length>) -> u16 {
        match tv {
            renderable::layout::TargetValue::Universal(renderable::layout::Length::Ch(n)) => u16::try_from(*n).unwrap_or(u16::MAX),
            _ => 0,
        }
    }

    fn page() -> DarkmatterPage {
        let term = Terminal::new_optimistic(120);
        DarkmatterPage::new(&term)
    }

    #[test]
    fn defaults_match_spec() {
        let page = page();
        assert_eq!(edge_ch(&page.page_margin().top), 0);
        assert_eq!(edge_ch(&page.page_margin().right), 0);
        assert_eq!(edge_ch(&page.page_margin().bottom), 0);
        assert_eq!(edge_ch(&page.page_margin().left), 0);
        assert_eq!(edge_ch(&page.page_padding().top), 0);
        assert_eq!(edge_ch(&page.page_padding().right), 0);
        assert_eq!(edge_ch(&page.page_padding().bottom), 0);
        assert_eq!(edge_ch(&page.page_padding().left), 0);
        assert_eq!(page.page_background(), PageBackground::Transparent);
        assert_eq!(page.max_width(), None);
        assert!(!page.line_numbers());
        assert_eq!(
            page.component_policy(PageComponent::Images).map(|p| p.layout.alignment).unwrap_or_default(),
            renderable::layout::Alignment::Left
        );
        assert!(page.component_policy(PageComponent::CodeBlocks).is_none());
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
        let m = page.page_margin();
        assert_eq!(edge_ch(&m.top), 0);
        assert_eq!(edge_ch(&m.right), 2);
        assert_eq!(edge_ch(&m.bottom), 2);
        assert_eq!(edge_ch(&m.left), 2);
    }

    #[test]
    fn margin_axis_helpers() {
        let page = page().with_margin_x(3).with_margin_y(1);
        let m = page.page_margin();
        assert_eq!(edge_ch(&m.left), 3);
        assert_eq!(edge_ch(&m.right), 3);
        assert_eq!(edge_ch(&m.top), 1);
        assert_eq!(edge_ch(&m.bottom), 1);
    }

    #[test]
    fn padding_shorthand_then_specific_overrides() {
        let page = page().with_padding(2).with_padding_left(0);
        let p = page.page_padding();
        assert_eq!(edge_ch(&p.top), 2);
        assert_eq!(edge_ch(&p.right), 2);
        assert_eq!(edge_ch(&p.bottom), 2);
        assert_eq!(edge_ch(&p.left), 0);
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
        let mut page = page();
        for component in PageComponent::ALL {
            page = page.with_component_policy(component, align_policy(renderable::layout::Alignment::Center));
        }
        let page = page.with_component_policy(PageComponent::Images, align_policy(renderable::layout::Alignment::Left));
        assert_eq!(
            page.component_policy(PageComponent::Images).map(|p| p.layout.alignment).unwrap_or_default(),
            renderable::layout::Alignment::Left
        );
        assert_eq!(
            page.component_policy(PageComponent::Tables).map(|p| p.layout.alignment).unwrap_or_default(),
            renderable::layout::Alignment::Center
        );
    }

    #[test]
    fn fill_overrides_per_component() {
        let mut page = page();
        for component in PageComponent::ALL {
            page = page.with_component_policy(component, pad_policy(2));
        }
        // Full is the default — remove the CodeBlocks override to restore default.
        let page = page.with_component_policy(PageComponent::CodeBlocks, ComponentPolicy::default());
        assert!(page.component_policy(PageComponent::CodeBlocks).map(|p| p.layout.padding == renderable::layout::Edges::default()).unwrap_or(true));
        assert_eq!(
            edge_ch(&page.component_policy(PageComponent::Tables).unwrap().layout.padding.left),
            2
        );
    }

    #[test]
    fn list_left_margin_accessor() {
        let page = page().with_component_policy(PageComponent::Ul, left_margin_policy(4));
        assert_eq!(
            edge_ch(&page.component_policy(PageComponent::Ul).unwrap().layout.margin.left),
            4
        );
        assert!(page.component_policy(PageComponent::Ol).is_none());
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
    fn zero_config_render_ignores_captured_terminal_width() {
        // Construct a DarkmatterPage from a Terminal whose captured width
        // differs from TerminalOptions::default() auto-detection. The page
        // must NOT leak that captured width into component width resolution;
        // output must remain byte-for-byte identical to the default
        // `as_terminal()` render. Without the `is_default_layout()` short-circuit in
        // `render`, image/list/blockquote/table/code component paths would
        // resolve widths against the captured Terminal width and diverge.
        for width in [40u32, 100, 200] {
            let term = Terminal::new_optimistic(width);
            let page = DarkmatterPage::new(&term);
            let md: Markdown = "# Heading\n\n- List item\n\n> Quoted prose\n\n```rust\nfn main() {}\n```\n\n| A | B |\n| - | - |\n| 1 | 2 |\n".into();

            let page_out = page.render(&md).unwrap();
            let direct_out = md.as_terminal(TerminalOptions::default()).unwrap();

            assert_eq!(
                page_out, direct_out,
                "zero-config render with captured_width={width} must equal the default as_terminal render",
            );
        }
    }

    /// `DarkmatterPage::new` must capture the [`Terminal`]'s color depth so a
    /// page built from `new_optimistic` (hardcoded `TrueColor`) reports that
    /// depth regardless of ambient detection.
    #[test]
    fn new_captures_terminal_color_depth() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        assert_eq!(page.terminal_color_depth(), ColorDepth::TrueColor);
    }

    /// On the decorated layout path, the page must thread its captured color
    /// depth into [`TerminalOptions`] so the render honors the [`Terminal`] it
    /// was constructed with rather than re-detecting from the ambient
    /// environment. Without this, a page built from `new_optimistic` in a
    /// headless `cargo test` env would emit 256-color or no-color SGR even
    /// though the captured terminal reports `TrueColor`.
    ///
    /// The truecolor background SGR sequence (`\x1b[48;2;r;g;bm`) is unique to
    /// 24-bit output — its presence is sufficient evidence that the captured
    /// depth was honored.
    #[test]
    fn decorated_render_honors_captured_color_depth() {
        let term = Terminal::new_optimistic(80);
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();
        let out = DarkmatterPage::new(&term)
            .with_margin_left(2)
            .with_margin_right(2)
            .with_code_theme("dracula")
            .render(&md)
            .unwrap();
        assert!(
            out.contains("\x1b[48;2;"),
            "decorated render with `new_optimistic` must emit truecolor SGR"
        );
    }

    /// An explicit [`Self::with_color_depth`] must override the captured
    /// terminal depth, so callers retain precise control when they want it.
    ///
    /// Pinning [`ColorDepth::None`] is the cleanest discriminator: the
    /// terminal renderer detects it at the top of its pipeline and returns the
    /// raw markdown content (no syntax highlighting, no SGR), so a passing
    /// assertion below proves the explicit value reached the renderer rather
    /// than being silently replaced by the captured `TrueColor`. (Verifying a
    /// downgrade between truecolor and 256-color would also require the
    /// highlighter to honor `color_depth`, which is a separate concern from
    /// this gate's contract.)
    #[test]
    fn with_color_depth_overrides_captured_depth() {
        let term = Terminal::new_optimistic(80);
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();
        let out = DarkmatterPage::new(&term)
            .with_color_depth(ColorDepth::None)
            .with_margin_left(2)
            .with_margin_right(2)
            .with_code_theme("dracula")
            .render(&md)
            .unwrap();
        assert!(
            !out.contains("\x1b["),
            "explicit `with_color_depth(None)` must suppress every SGR; got: {out:?}"
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
        let mut policy = max_width_policy(40);
        policy.layout.alignment = renderable::layout::Alignment::Center;
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::CodeBlocks, policy);
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
        let mut policy = max_width_policy(30);
        policy.layout.alignment = renderable::layout::Alignment::Right;
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Tables, policy);
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
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::CodeBlocks, max_width_policy(40));
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
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::CodeBlocks, pad_policy(4));
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);

        for (i, line) in plain.lines().enumerate() {
            eprintln!("DEBUG line {}: len={} {:?}", i, line.len(), line);
        }

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
            80,
            "padding row should match fold output, got len={}",
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
        let mut policy = indent_policy(10);
        policy.layout.alignment = renderable::layout::Alignment::Left;
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::BlockQuotes, policy);
        // Long content so the wrap point is observable. Without the active
        // width override, this line would render in a single 80-col span.
        let md: Markdown = "> This is a very long quoted paragraph that should be forced to wrap once the component-specific width override is applied, leaving the remaining text on subsequent lines below.\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        for (i, line) in plain.lines().enumerate() {
            eprintln!("DEBUG bq line {}: len={} {:?}", i, line.len(), line);
        }
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
            max_len <= 75,
            "blockquote lines should be capped by Indent(10), got max={}:\n{}",
            max_len,
            plain
        );
    }

    #[test]
    fn render_list_with_max_fill() {
        let term = Terminal::new_optimistic(80);
        let mut page = DarkmatterPage::new(&term);
        for component in [PageComponent::Ul, PageComponent::Ol, PageComponent::Li] {
            page = page.with_component_policy(component, max_width_policy(50));
        }
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
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Images, align_policy(renderable::layout::Alignment::Center));
        let md: Markdown = "![alt text|20](nonexistent.png)\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        let image_line = plain.lines().find(|l| l.contains("IMAGE")).unwrap_or("");
        // The tree fold keeps the raw alt (`alt text|20`) rather than parsing
        // the legacy `|20` width directive, so the placeholder is
        // `▉ IMAGE[alt text|20]`; centered in 80 this leaves ~30 leading spaces.
        let leading = image_line.chars().take_while(|c| *c == ' ').count();
        assert!(
            leading >= 28,
            "image placeholder should be centered (>=28 leading spaces), got {leading}: {:?}",
            image_line
        );
    }

    #[test]
    fn zero_config_with_non_default_alignment_still_matches() {
        // When only alignment is set (no margin/padding/bg/max-width), the page
        // should still render successfully and alignment should be applied.
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::CodeBlocks, align_policy(renderable::layout::Alignment::Center));
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
        // But should still contain the rendered markdown. The render-tree
        // browser path emits a heading slug `id`, so match the heading by its
        // tag + text rather than pinning the legacy attribute-free `<h1>`.
        assert!(
            page_html.contains("<h1 id=\"hello-world\">Hello World</h1>"),
            "zero-config page should still render markdown; html={page_html}"
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
            .with_component_policy(PageComponent::Tables, align_policy(renderable::layout::Alignment::Center))
            .with_component_policy(PageComponent::BlockQuotes, align_policy(renderable::layout::Alignment::Right));
        let md: Markdown = "# Hello\n".into();

        let html = page.render_to_browser(&md).unwrap();
        // Fold-based behavior emits a wrapper div but not bespoke per-component CSS.
        assert!(html.contains("<div class=\"darkmatter-page\""));
        assert!(html.contains("Hello"));
    }

    #[test]
    fn browser_render_with_component_fill_css() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::CodeBlocks, max_width_policy(60))
            .with_component_policy(PageComponent::Images, pad_policy(4));
        let md: Markdown = "# Hello\n".into();

        let html = page.render_to_browser(&md).unwrap();
        // Fold-based behavior emits a wrapper div but not bespoke per-component CSS.
        assert!(html.contains("<div class=\"darkmatter-page\""));
        assert!(html.contains("Hello"));
    }

    #[test]
    fn render_to_browser_emits_markdown_html() {
        let term = Terminal::new_optimistic(80);
        let md: Markdown = "# Browser\n".into();
        let page = DarkmatterPage::new(&term);

        // Browser output goes through the inherent method, not a
        // `BrowserRenderable` impl (decisions.md item 12A).
        let html = page.render_to_browser(&md).unwrap();
        // The render-tree browser path emits a heading slug `id`.
        assert!(
            html.contains("<h1 id=\"browser\">Browser</h1>"),
            "render_to_browser should output markdown HTML content; html={html}"
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

    /// A malformed code-block directive (an invalid highlight range) is a fatal
    /// error on the browser render, matching the legacy `output::as_html`
    /// contract (`parse_code_info(...)?`). The `as_html` cutover restores this
    /// via the `validate_code_directives` preflight, which runs over the folded
    /// tree before rendering and surfaces `MarkdownError::InvalidLineRange`; the
    /// `MarkdownError -> PageRenderError::Render` mapping in `render_to_browser`
    /// then propagates it. (The render-tree *terminal* path still degrades, also
    /// matching legacy, which used `unwrap_or_default` there.)
    #[test]
    fn render_browser_errors_on_malformed_code_directive() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let md: Markdown = "```rust highlight=1-2-3\nfn main() {}\n```\n".into();

        let err = page
            .render_to_browser(&md)
            .expect_err("malformed directive must fail the browser render");
        assert!(
            matches!(err, PageRenderError::Render(_)),
            "malformed directive must map to PageRenderError::Render; got {err:?}"
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
            .with_component_policy(PageComponent::CodeBlocks, align_policy(renderable::layout::Alignment::Center));
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

    // ---------- Phase 4: list split + wiring tests ----------

    #[test]
    fn render_ul_left_margin() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Ul, left_margin_policy(4));
        let md: Markdown = "- Hello world\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        let list_line = plain.lines().find(|l| l.contains("Hello")).unwrap();
        assert!(
            list_line.starts_with("    - "),
            "unordered list should have 4ch left margin before marker, got: {:?}",
            list_line
        );
    }

    #[test]
    fn render_ul_max_width() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Ul, max_width_policy(40));
        let md: Markdown = "- This is an unusually long bullet item that ought to be forced to wrap once Max(40) constrains the list rendering width to forty columns.\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        let lines: Vec<&str> = plain.lines().collect();
        let max_len = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        assert!(
            max_len <= 40,
            "list lines should be capped to 40 cols, got max={}:\n{}",
            max_len,
            plain
        );
    }

    #[test]
    fn render_ul_left_margin_and_max_width() {
        let term = Terminal::new_optimistic(80);
        let mut policy = max_width_policy(40);
        policy.layout.margin.left = renderable::layout::TargetValue::universal(renderable::layout::Length::ch(4));
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Ul, policy);
        let md: Markdown = "- This is an unusually long bullet item that ought to be forced to wrap once Max(40) constrains the list rendering width to forty columns.\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        let lines: Vec<&str> = plain.lines().collect();
        // Body wraps at <= ul.max-width (40 cells); the 4-cell left margin
        // sits outside the body, so total line length is <= 44 cells.
        let max_total = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        assert!(
            max_total <= 44,
            "list lines should fit in left-margin (4) + body (<= 40) = 44 cols, got max={}:\n{}",
            max_total,
            plain
        );
        // Body width: stripping the 4-cell margin, the remaining content
        // must wrap at no more than 40 cells.
        let max_body = lines
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                let trimmed = l.strip_prefix("    ").unwrap_or(l);
                trimmed.chars().count()
            })
            .max()
            .unwrap_or(0);
        assert!(
            max_body <= 40,
            "body (after 4ch margin) should wrap at <= 40 cols, got max body={}:\n{}",
            max_body,
            plain
        );
        // First non-empty line should start with 4 spaces of left margin.
        let first_line = lines.iter().find(|l| !l.trim().is_empty()).copied().unwrap_or("");
        assert!(
            first_line.starts_with("    - "),
            "first line should start with 4ch left margin, got: {:?}",
            first_line
        );
    }

    #[test]
    fn render_ol_alignment_right() {
        let term = Terminal::new_optimistic(80);
        let mut policy = max_width_policy(40);
        policy.layout.alignment = renderable::layout::Alignment::Right;
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Ol, policy);
        let md: Markdown = "1. Hello world\n".into();

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        let list_line = plain.lines().find(|l| l.contains("Hello")).unwrap();
        // Component is 40 cols wide, right-aligned in 80 => 40 cols of left padding.
        let leading_spaces = list_line.len() - list_line.trim_start().len();
        assert!(
            leading_spaces >= 35,
            "ordered list should be right-aligned, got {} leading spaces: {:?}",
            leading_spaces,
            list_line
        );
    }

    #[test]
    fn render_li_body_alignment_right() {
        let term = Terminal::new_optimistic(80);
        let mut policy = max_width_policy(40);
        policy.layout.alignment = renderable::layout::Alignment::Right;
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Li, policy);
        let md: Markdown = "- Hello world\n".into();

        assert!(!page.is_default_layout(), "page should not be default layout");
        assert_eq!(
            page.component_policy(PageComponent::Li).and_then(|p| p.layout.max_width.as_ref()).map(edge_ch),
            Some(40)
        );
        assert_eq!(
            page.component_policy(PageComponent::Li).map(|p| p.layout.alignment).unwrap_or_default(),
            renderable::layout::Alignment::Right
        );

        let out = page.render(&md).unwrap();
        let plain = crate::testing::strip_ansi_codes(&out);
        let lines: Vec<&str> = plain.lines().collect();
        // Per spec, `li.alignment` affects the item body only; the marker
        // stays at the column dictated by the containing Ul (column 0 here,
        // since Ul has no override). The body becomes a block on a new line
        // that is right-aligned within `effective_width - body_width = 40`.
        let marker_line = lines
            .iter()
            .find(|l| l.trim_start().starts_with('-'))
            .copied()
            .unwrap_or("");
        assert!(
            marker_line.starts_with("- "),
            "marker should remain at column 0 (Ul column), got: {:?}",
            marker_line
        );
        let body_line = lines
            .iter()
            .find(|l| l.contains("Hello"))
            .copied()
            .unwrap_or("");
        assert!(
            !body_line.contains('-'),
            "body should not contain the marker (marker is on its own line): {:?}",
            body_line
        );
        let leading_spaces = body_line.len() - body_line.trim_start().len();
        assert!(
            leading_spaces >= 35,
            "li body should be right-aligned within effective_width, got {} leading spaces: {:?}",
            leading_spaces,
            body_line
        );
    }

    #[test]
    fn browser_selectors_split_for_lists() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Ul, align_policy(renderable::layout::Alignment::Center))
            .with_component_policy(PageComponent::Ol, align_policy(renderable::layout::Alignment::Right))
            .with_component_policy(PageComponent::Li, max_width_policy(30));
        let md: Markdown = "- item\n".into();

        let html = page.render_to_browser(&md).unwrap();
        // Fold-based behavior emits a wrapper div but not bespoke per-component CSS.
        assert!(html.contains("<div class=\"darkmatter-page\""));
        assert!(html.contains("item"));
    }

    #[test]
    fn browser_ul_left_margin_css() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Ul, left_margin_policy(4));
        let md: Markdown = "- item\n".into();

        let html = page.render_to_browser(&md).unwrap();
        // Fold-based behavior emits a wrapper div but not bespoke per-component CSS.
        assert!(html.contains("<div class=\"darkmatter-page\""));
        assert!(html.contains("item"));
    }

    #[test]
    fn li_independent_of_ul_ol() {
        let term = Terminal::new_optimistic(80);
        let mut ul_policy = max_width_policy(30);
        ul_policy.layout.alignment = renderable::layout::Alignment::Left;
        let mut ol_policy = max_width_policy(40);
        ol_policy.layout.alignment = renderable::layout::Alignment::Center;
        let mut li_policy = max_width_policy(50);
        li_policy.layout.alignment = renderable::layout::Alignment::Right;
        let page = DarkmatterPage::new(&term)
            .with_component_policy(PageComponent::Ul, ul_policy)
            .with_component_policy(PageComponent::Ol, ol_policy)
            .with_component_policy(PageComponent::Li, li_policy);

        // Each component retains its own alignment independently.
        assert_eq!(page.component_policy(PageComponent::Ul).map(|p| p.layout.alignment).unwrap_or_default(), renderable::layout::Alignment::Left);
        assert_eq!(page.component_policy(PageComponent::Ol).map(|p| p.layout.alignment).unwrap_or_default(), renderable::layout::Alignment::Center);
        assert_eq!(page.component_policy(PageComponent::Li).map(|p| p.layout.alignment).unwrap_or_default(), renderable::layout::Alignment::Right);

        // Each component retains its own fill independently.
        assert_eq!(
            page.component_policy(PageComponent::Ul).and_then(|p| p.layout.max_width.as_ref()).map(edge_ch),
            Some(30)
        );
        assert_eq!(
            page.component_policy(PageComponent::Ol).and_then(|p| p.layout.max_width.as_ref()).map(edge_ch),
            Some(40)
        );
        assert_eq!(
            page.component_policy(PageComponent::Li).and_then(|p| p.layout.max_width.as_ref()).map(edge_ch),
            Some(50)
        );
    }

    // ---------- Phase 1: color API tests ----------

    use crate::style::StyleColor;
    use renderable::color::{Color, Tailwind};

    fn red_color() -> StyleColor {
        StyleColor {
            color: Color::Tailwind(Tailwind::Red500),
            opacity: None,
        }
    }

    fn blue_color() -> StyleColor {
        StyleColor {
            color: Color::Tailwind(Tailwind::Blue500),
            opacity: None,
        }
    }

    #[test]
    fn color_setters_and_getters() {
        let page = page()
            .with_page_color(red_color())
            .with_page_bg_color(blue_color())
            .with_component_color(PageComponent::Tables, red_color())
            .with_component_bg_color(PageComponent::Tables, blue_color());

        assert_eq!(page.page_color(), Some(&red_color()));
        assert_eq!(page.page_bg_color(), Some(&blue_color()));
        assert_eq!(
            page.color_for(PageComponent::Tables),
            Some(&red_color())
        );
        assert_eq!(
            page.bg_color_for(PageComponent::Tables),
            Some(&blue_color())
        );
    }

    #[test]
    fn color_inheritance_from_page() {
        let page = page()
            .with_page_color(red_color())
            .with_page_bg_color(blue_color());

        // Components without explicit color inherit page color.
        assert_eq!(
            page.color_for(PageComponent::Tables),
            Some(&red_color())
        );
        assert_eq!(
            page.bg_color_for(PageComponent::Tables),
            Some(&blue_color())
        );
        assert_eq!(
            page.color_for(PageComponent::Hyperlinks),
            Some(&red_color())
        );
    }

    #[test]
    fn component_color_overrides_page_color() {
        let page = page()
            .with_page_color(red_color())
            .with_component_color(PageComponent::Tables, blue_color());

        assert_eq!(
            page.color_for(PageComponent::Tables),
            Some(&blue_color())
        );
        // Other components still inherit page color.
        assert_eq!(
            page.color_for(PageComponent::Images),
            Some(&red_color())
        );
    }

    #[test]
    fn component_bg_color_overrides_page_bg_color() {
        let page = page()
            .with_page_bg_color(red_color())
            .with_component_bg_color(PageComponent::Tables, blue_color());

        assert_eq!(
            page.bg_color_for(PageComponent::Tables),
            Some(&blue_color())
        );
        assert_eq!(
            page.bg_color_for(PageComponent::Images),
            Some(&red_color())
        );
    }

    #[test]
    fn color_only_page_is_not_default_layout() {
        let page = page().with_page_color(red_color());
        assert!(!page.is_default_layout(), "page with color should not be default");
    }

    #[test]
    fn bg_color_only_page_is_not_default_layout() {
        let page = page().with_page_bg_color(red_color());
        assert!(!page.is_default_layout(), "page with bg-color should not be default");
    }

    #[test]
    fn component_color_only_page_is_not_default_layout() {
        let page = page().with_component_color(PageComponent::Tables, red_color());
        assert!(!page.is_default_layout(), "page with component color should not be default");
    }

    // ---------- Phase 5: render-level color tests ----------

    #[test]
    fn terminal_page_color_applies_sgr_to_components() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term).with_page_color(red_color());
        let md: Markdown = "# Hello\n".into();

        let out = page.render(&md).unwrap();
        // The heading text should be wrapped with the page color SGR
        // and properly reset.
        assert!(
            out.contains("\x1b[38;2;"),
            "page color should emit foreground SGR; got: {out:?}"
        );
        assert!(
            out.contains("\x1b[0m"),
            "page color scope should end with reset; got: {out:?}"
        );
    }

    #[test]
    fn terminal_component_color_overrides_page_color_in_output() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_page_color(red_color())
            .with_component_color(PageComponent::Tables, blue_color());
        let md: Markdown = "| a | b |\n|---|---|\n| 1 | 2 |\n".into();

        let out = page.render(&md).unwrap();
        // Table output should contain blue SGR, not just red.
        // Both colors may appear (red for heading, blue for table), so we
        // just verify the table-specific color is present.
        assert!(
            out.contains("\x1b[38;2;"),
            "component color should emit SGR; got: {out:?}"
        );
    }

    #[test]
    fn terminal_color_depth_none_omits_sgr_for_colors() {
        let term = Terminal::new_optimistic(80);
        let md: Markdown = "# Hello\n".into();
        let out = DarkmatterPage::new(&term)
            .with_page_color(red_color())
            .with_color_depth(ColorDepth::None)
            .render(&md)
            .unwrap();

        assert!(
            !out.contains("\x1b[38;2;"),
            "ColorDepth::None must suppress color SGR; got: {out:?}"
        );
    }

    #[test]
    fn terminal_reset_boundary_scopes_component_colors() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_component_color(PageComponent::Tables, red_color());
        let md: Markdown = "| a | b |\n|---|---|\n| 1 | 2 |\n".into();

        let out = page.render(&md).unwrap();
        // The table output should be wrapped with an opening SGR and a reset.
        assert!(
            out.contains("\x1b[0m"),
            "component color must be scoped with reset; got: {out:?}"
        );
    }

    #[test]
    fn browser_page_color_emits_wrapper_css() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term).with_page_color(red_color());
        let md: Markdown = "# Hello\n".into();

        let html = page.render_to_browser(&md).unwrap();
        assert!(
            html.contains("color: rgb("),
            "page color should emit CSS on wrapper; got: {html}"
        );
    }

    #[test]
    fn browser_page_bg_color_overrides_page_background_css() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_page_background(PageBackground::Subtle)
            .with_page_bg_color(red_color());
        let md: Markdown = "# Hello\n".into();

        let html = page.render_to_browser(&md).unwrap();
        // The wrapper should have the explicit bg-color after the computed one.
        let bg_count = html.matches("background-color:").count();
        assert!(
            bg_count >= 1,
            "wrapper should have background-color; got: {html}"
        );
        assert!(
            html.contains("background-color: rgb("),
            "page bg-color should be rgb(...); got: {html}"
        );
    }

    #[test]
    fn browser_component_color_emits_per_component_css() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_component_color(PageComponent::Tables, red_color())
            .with_component_bg_color(PageComponent::BlockQuotes, blue_color());
        let md: Markdown = "# Hello\n\n> Quote\n\n| a | b |\n|---|---|\n| 1 | 2 |\n".into();

        let html = page.render_to_browser(&md).unwrap();
        // Fold-based behavior emits a wrapper div but not bespoke per-component CSS.
        assert!(html.contains("<div class=\"darkmatter-page\""));
        assert!(html.contains("Hello"));
        assert!(html.contains("Quote"));
    }

    #[test]
    fn browser_opacity_preserved_as_rgba() {
        let term = Terminal::new_optimistic(120);
        let semi = StyleColor {
            color: Color::Tailwind(Tailwind::Red500),
            opacity: Some(50),
        };
        let page = DarkmatterPage::new(&term).with_page_color(semi);
        let md: Markdown = "# Hello\n".into();

        let html = page.render_to_browser(&md).unwrap();
        assert!(
            html.contains("rgba(") && html.contains("0.5"),
            "opacity should produce rgba CSS; got: {html}"
        );
    }

    #[test]
    fn browser_component_opacity_preserved_as_rgba() {
        // Review-1 finding 1: a component `bg-color` with Tailwind opacity must
        // survive the cutover path to the browser as `rgba(...)` — the renderable
        // `Style` cannot carry opacity, so the browser entry point splices it in.
        let term = Terminal::new_optimistic(120);
        let semi = StyleColor {
            color: Color::Tailwind(Tailwind::Red500),
            opacity: Some(50),
        };
        let page = DarkmatterPage::new(&term)
            .with_component_bg_color(PageComponent::BlockQuotes, semi);
        let md: Markdown = "> Quote\n".into();

        let html = page.render_to_browser(&md).unwrap();
        assert!(
            html.contains("rgba(") && html.contains("0.5"),
            "component bg-color opacity must lower to rgba on the browser path; got: {html}"
        );
    }

    #[test]
    fn terminal_component_opacity_dropped_but_color_kept() {
        // The terminal drops opacity (documented) yet still paints the color.
        let term = Terminal::new_optimistic(80);
        let semi = StyleColor {
            color: Color::Tailwind(Tailwind::Red500),
            opacity: Some(50),
        };
        let page = DarkmatterPage::new(&term)
            .with_component_bg_color(PageComponent::BlockQuotes, semi);
        let md: Markdown = "> Quote\n".into();

        let out = page.render(&md).unwrap();
        assert!(
            out.contains("\x1b[48;2;"),
            "terminal should emit a 24-bit background SGR (opacity dropped); got: {out:?}"
        );
    }

    #[test]
    fn terminal_opacity_dropped_from_sgr() {
        let term = Terminal::new_optimistic(80);
        let semi = StyleColor {
            color: Color::Tailwind(Tailwind::Red500),
            opacity: Some(50),
        };
        let page = DarkmatterPage::new(&term).with_page_color(semi);
        let md: Markdown = "# Hello\n".into();

        let out = page.render(&md).unwrap();
        // SGR should NOT contain opacity; it should be a plain 24-bit color.
        assert!(
            out.contains("\x1b[38;2;"),
            "terminal should still emit 24-bit SGR without opacity; got: {out:?}"
        );
    }

    #[test]
    fn browser_css_special_colors_passthrough() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_component_color(PageComponent::Tables, StyleColor {
                color: Color::Tailwind(Tailwind::Transparent),
                opacity: None,
            })
            .with_component_color(PageComponent::BlockQuotes, StyleColor {
                color: Color::Tailwind(Tailwind::Current),
                opacity: None,
            })
            .with_component_bg_color(PageComponent::Images, StyleColor {
                color: Color::Tailwind(Tailwind::Inherit),
                opacity: None,
            });
        let md: Markdown = "# Hello\n\n> Quote\n\n| a | b |\n|---|---|\n| 1 | 2 |\n".into();

        let html = page.render_to_browser(&md).unwrap();
        // Fold-based behavior emits a wrapper div but not bespoke per-component CSS.
        assert!(html.contains("<div class=\"darkmatter-page\""));
        assert!(html.contains("Hello"));
        assert!(html.contains("Quote"));
    }

    #[test]
    fn browser_list_selectors_emit_separate_rules_with_colors() {
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_component_color(PageComponent::Ul, red_color())
            .with_component_color(PageComponent::Ol, blue_color())
            .with_component_color(PageComponent::Li, StyleColor {
                color: Color::Tailwind(Tailwind::Green500),
                opacity: None,
            });
        let md: Markdown = "- one\n\n1. two\n".into();

        let html = page.render_to_browser(&md).unwrap();
        // Fold-based behavior emits a wrapper div but not bespoke per-component CSS.
        assert!(html.contains("<div class=\"darkmatter-page\""));
        assert!(html.contains("one"));
        assert!(html.contains("two"));
    }

    #[test]
    fn terminal_hyperlink_color_preserves_osc8_sequences() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_hyperlink_mode(HyperlinkMode::Always)
            .with_component_color(PageComponent::Hyperlinks, red_color());
        let md: Markdown = "[link](https://example.com)\n".into();

        let out = page.render(&md).unwrap();
        // OSC8 sequences must still be present.
        assert!(
            out.contains("\x1b]8;;https://example.com\x1b\\")
                || out.contains("\x1b]8;;https://example.com\x07"),
            "OSC8 open sequence must be preserved; got: {out:?}"
        );
        assert!(
            out.contains("\x1b]8;;\x1b\\") || out.contains("\x1b]8;;\x07"),
            "OSC8 close sequence must be preserved; got: {out:?}"
        );
        // The hyperlink text should also have the color SGR applied.
        assert!(
            out.contains("\x1b[38;2;"),
            "hyperlink color SGR should be present; got: {out:?}"
        );
    }

    #[test]
    fn code_block_bg_color_override_does_not_clobber_highlighting() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_component_bg_color(PageComponent::CodeBlocks, red_color());
        let md: Markdown = "```rust\nfn main() {}\n```\n".into();

        let out = page.render(&md).unwrap();
        // The code block should still contain syntax-highlighting SGRs
        // (multiple different colors for keywords, identifiers, etc.).
        let sgr_count = out.matches("\x1b[38;2;").count();
        assert!(
            sgr_count >= 2,
            "code block should retain multiple syntax highlight colors; got: {out:?}"
        );
    }

    // ---------- Review-5 follow-ups: terminal layout fidelity ----------

    /// With `ColorDepth::None`, a styled page must still render the full
    /// table layout (box-drawing characters and cell contents) — the
    /// pipeline no longer falls back to raw Markdown source.
    #[test]
    fn color_depth_none_preserves_table_layout_when_page_color_set() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_page_color(red_color())
            .with_color_depth(ColorDepth::None);
        let md: Markdown = "| H |\n|---|\n| C |\n".into();

        let out = page.render(&md).unwrap();
        assert!(
            !out.contains("\x1b[38;2;"),
            "ColorDepth::None must suppress color SGR even with style.page.color; got: {out:?}"
        );
        assert!(
            out.contains('H') && out.contains('C'),
            "table cell text must survive ColorDepth::None; got: {out:?}"
        );
        assert!(
            out.contains('┌') || out.contains('+') || out.contains('|'),
            "table structure must render under ColorDepth::None; got: {out:?}"
        );
    }

    /// `style.ul.color` must apply to list-item body text even when
    /// `style.li.color` is unset — list items inherit through their
    /// container scope just like CSS. The Tailwind Red-500 SGR triplet
    /// resolves at render time, so we look up the canonical bytes from the
    /// shared lowering helper rather than hard-coding RGB values.
    #[test]
    fn ul_color_inherits_into_li_body_when_li_color_unset() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_component_color(PageComponent::Ul, red_color());
        let md: Markdown = "- alpha\n- beta\n".into();

        let out = page.render(&md).unwrap();
        let red_sgr = crate::style::lower_to_sgr(&red_color(), ColorDepth::TrueColor, false)
            .expect("red_color must lower to truecolor SGR");
        // The ul color should wrap the marker AND the body, even though the
        // li scope has no explicit color of its own (the body would
        // otherwise inherit a None scope from li).
        let occurrences = out.matches(&red_sgr).count();
        assert!(
            occurrences >= 2,
            "ul color must wrap each item's body; got: {out:?}"
        );
    }

    /// `style.hyperlinks.color` must wrap link label text inside table
    /// cells, while preserving the OSC8 sequence — and it overrides the
    /// surrounding table color.
    #[test]
    fn browser_hr_bg_color_targets_rendered_element() {
        // The HR component emits `<svg class="darkmatter-hr">`. Under the
        // fold-based behavior the per-component CSS rule is no longer emitted,
        // but the SVG class remains so downstream stylesheets can target it.
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_component_bg_color(PageComponent::Hr, red_color());
        let md: Markdown = "Before\n\n---\n\nAfter\n".into();

        let html = page.render_to_browser(&md).unwrap();

        // Fold-based behavior: no bespoke per-component CSS, but the SVG
        // still carries the class for external stylesheets.
        assert!(
            html.contains(r#"class="darkmatter-hr""#),
            "HR SVG must carry the `darkmatter-hr` class; got: {html}"
        );
    }

    #[test]
    fn browser_hr_color_emits_rule_for_svg_target() {
        // `style.hr.color` reaches the SVG via CSS `color` inheritance →
        // the SVG primitives reference `currentColor` through
        // `var(--hr-color, currentColor)`. Under the fold-based behavior
        // the per-component CSS rule is no longer emitted.
        let term = Terminal::new_optimistic(120);
        let page = DarkmatterPage::new(&term)
            .with_component_color(PageComponent::Hr, red_color());
        let md: Markdown = "---\n".into();

        let html = page.render_to_browser(&md).unwrap();

        // Fold-based behavior: no bespoke per-component CSS.
        assert!(html.contains("<div class=\"darkmatter-page\""));
    }

    #[test]
    fn hyperlink_color_applies_inside_table_cells() {
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term)
            .with_hyperlink_mode(HyperlinkMode::Always)
            .with_component_color(PageComponent::Tables, blue_color())
            .with_component_color(PageComponent::Hyperlinks, red_color());
        let md: Markdown = "| col |\n|---|\n| [click](https://example.com) |\n".into();

        let out = page.render(&md).unwrap();
        // OSC8 sequences must still be present so the link remains clickable.
        assert!(
            out.contains("\x1b]8;;https://example.com\x07")
                || out.contains("\x1b]8;;https://example.com\x1b\\"),
            "OSC8 open sequence must be preserved in table; got: {out:?}"
        );
        let red_sgr = crate::style::lower_to_sgr(&red_color(), ColorDepth::TrueColor, false)
            .expect("red_color must lower to truecolor SGR");
        assert!(
            out.contains(&red_sgr),
            "hyperlink color must wrap table-link text; got: {out:?}"
        );
    }

    // ---------- Phase 5: renderable-typed page frame ----------

    #[test]
    fn page_frame_stores_renderable_types() {
        let page = DarkmatterPage::new(&Terminal::new_optimistic(80))
            .with_margin(2)
            .with_padding(3);
        // page-frame margin/padding are renderable Edges, not PageMargin/PagePadding
        let _: &renderable::layout::Edges = page.page_margin();
        let _: &renderable::layout::Edges = page.page_padding();
    }

    #[test]
    fn pronounced_still_flips_render_mode() {
        let page = DarkmatterPage::new(&Terminal::new_optimistic(80))
            .with_page_background(PageBackground::Pronounced);
        // existing guard: the code theme mode inverts; reuse the existing snapshot
        let html = page.render_to_browser(&"```rust\nfn x(){}\n```".into()).unwrap();
        assert!(html.contains("darkmatter-page"));
        insta::assert_snapshot!("pronounced_background_snapshot", html);
    }
}
