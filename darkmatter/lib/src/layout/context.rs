//! Internal layout context computed at render time from [`DarkmatterPage`](super::DarkmatterPage)
//! state and the captured terminal dimensions.

use super::PageRenderError;
use super::page::{ComponentPolicy, length_is_zero, length_to_cells};
use super::types::{
    PageBackground, PageComponent,
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
    /// Page margin (renderable [`Edges`](renderable::layout::Edges)).
    pub page_margin: renderable::layout::Edges,
    /// Page padding (renderable [`Edges`](renderable::layout::Edges)).
    pub page_padding: renderable::layout::Edges,
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
    /// Page foreground color.
    pub page_color: Option<StyleColor>,
    /// Page background color.
    pub page_bg_color: Option<StyleColor>,
    /// Per-component renderable policies from `style:` frontmatter — the single
    /// source of truth for per-component layout **and** colors (including
    /// opacity).
    pub component_policies: std::collections::HashMap<PageComponent, ComponentPolicy>,
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

/// Whether every side of `edges` contributes no space (see
/// [`length_is_zero`]).
fn edges_is_zero(edges: &renderable::layout::Edges) -> bool {
    length_is_zero(&edges.top)
        && length_is_zero(&edges.right)
        && length_is_zero(&edges.bottom)
        && length_is_zero(&edges.left)
}

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
        page_margin: renderable::layout::Edges,
        page_padding: renderable::layout::Edges,
        page_background: PageBackground,
        page_max_width: Option<renderable::layout::TargetValue<renderable::layout::Length>>,
        terminal_color_mode: &biscuit_terminal::discovery::detection::ColorMode,
        options_color_mode: ColorMode,
        page_color: Option<StyleColor>,
        page_bg_color: Option<StyleColor>,
        component_policies: std::collections::HashMap<PageComponent, ComponentPolicy>,
        hyperlink_style: Option<CommonStyle>,
        local_hyperlink_style: Option<CommonStyle>,
        local_image_style: Option<CommonStyle>,
    ) -> Result<Self, PageRenderError> {
        let margin_x = length_to_cells(&page_margin.left, terminal_width)
            .saturating_add(length_to_cells(&page_margin.right, terminal_width));
        let padding_x = length_to_cells(&page_padding.left, terminal_width)
            .saturating_add(length_to_cells(&page_padding.right, terminal_width));
        let required = margin_x.saturating_add(padding_x);
        if required >= terminal_width {
            return Err(PageRenderError::MarginsExceedTerminalWidth {
                terminal_width,
                required,
            });
        }

        let content_width = terminal_width.saturating_sub(required);
        // A percentage `max-width` resolves against the content width.
        let effective_width = match page_max_width.as_ref().map(|tv| length_to_cells(tv, content_width)) {
            Some(0) => return Err(PageRenderError::MaxWidthZero),
            Some(mw) => content_width.min(mw),
            None => content_width,
        };

        let has_layout = !edges_is_zero(&page_margin)
            || !edges_is_zero(&page_padding)
            || page_background != PageBackground::Transparent
            || page_max_width.is_some()
            || !component_policies.is_empty()
            || page_color.is_some()
            || page_bg_color.is_some();

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
            page_margin,
            page_padding,
            content_width,
            effective_width,
            has_layout,
            background_color,
            render_color_mode,
            page_color,
            page_bg_color,
            component_policies,
            hyperlink_style,
            local_hyperlink_style,
            local_image_style,
        })
    }

    /// Whether row decoration should be applied.
    pub fn needs_decoration(&self) -> bool {
        self.has_layout
    }

    /// Resolve effective foreground color for a component.
    ///
    /// Returns the component-specific color when set, otherwise falls back
    /// to the page-level color.
    pub fn component_color(&self, component: PageComponent) -> Option<&StyleColor> {
        self.component_policies
            .get(&component)
            .and_then(|p| p.color.as_ref())
            .or(self.page_color.as_ref())
    }

    /// Resolve effective background color for a component.
    ///
    /// Returns the component-specific color when set, otherwise falls back
    /// to the page-level color.
    pub fn component_bg_color(&self, component: PageComponent) -> Option<&StyleColor> {
        self.component_policies
            .get(&component)
            .and_then(|p| p.bg_color.as_ref())
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
            .or_else(|| self.policy_color(PageComponent::Hyperlinks))
            .or_else(|| self.page_color.clone());
        let bg = merged
            .as_ref()
            .and_then(|s| s.bg_color.clone())
            .or_else(|| self.policy_bg_color(PageComponent::Hyperlinks))
            .or_else(|| self.page_bg_color.clone());

        (fg, bg)
    }

    /// The component's own foreground color from its [`ComponentPolicy`] (no
    /// page-level fallback), cloned.
    fn policy_color(&self, component: PageComponent) -> Option<StyleColor> {
        self.component_policies
            .get(&component)
            .and_then(|p| p.color.clone())
    }

    /// The component's own background color from its [`ComponentPolicy`] (no
    /// page-level fallback), cloned.
    fn policy_bg_color(&self, component: PageComponent) -> Option<StyleColor> {
        self.component_policies
            .get(&component)
            .and_then(|p| p.bg_color.clone())
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
                    color: self.policy_color(PageComponent::Images),
                    bg_color: self.policy_bg_color(PageComponent::Images),
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
            .or_else(|| self.policy_color(PageComponent::Images))
            .or_else(|| self.page_color.clone());
        let bg = merged
            .as_ref()
            .and_then(|s| s.bg_color.clone())
            .or_else(|| self.policy_bg_color(PageComponent::Images))
            .or_else(|| self.page_bg_color.clone());

        (fg, bg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn ctx(content_width: u16, effective_width: u16) -> LayoutContext {
        LayoutContext {
            terminal_width: effective_width,
            page_margin: renderable::layout::Edges::default(),
            page_padding: renderable::layout::Edges::default(),
            content_width,
            effective_width,
            has_layout: true,
            background_color: None,
            render_color_mode: ColorMode::Dark,
            page_color: None,
            page_bg_color: None,
            component_policies: HashMap::new(),
            hyperlink_style: None,
            local_hyperlink_style: None,
            local_image_style: None,
        }
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

    fn color_policy(color: StyleColor) -> ComponentPolicy {
        ComponentPolicy {
            color: Some(color),
            ..ComponentPolicy::default()
        }
    }

    fn bg_color_policy(color: StyleColor) -> ComponentPolicy {
        ComponentPolicy {
            bg_color: Some(color),
            ..ComponentPolicy::default()
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
        c.component_policies
            .insert(PageComponent::Tables, color_policy(red_style_color()));
        assert!(c.needs_decoration());
    }

    #[test]
    fn component_color_inherits_from_page() {
        let mut c = ctx(100, 80);
        c.page_color = Some(red_style_color());
        c.component_policies
            .insert(PageComponent::Tables, color_policy(blue_style_color()));

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
        c.component_policies
            .insert(PageComponent::Tables, bg_color_policy(blue_style_color()));

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
