//! Internal layout context computed at render time from [`DarkmatterPage`](super::DarkmatterPage)
//! state and the captured terminal dimensions.
//!
//! As of the context-aware fold cutover, `LayoutContext` carries only
//! page-frame state: terminal/content dimensions, margins, padding,
//! background color, and page-level foreground/background colors for the
//! wrapper CSS. Per-component policies, hyperlink styles, and image styles
//! live on [`TreeBuildContext`](crate::markdown::render_tree::build_context::TreeBuildContext)
//! and are baked into the render tree during the fold.

use super::PageRenderError;
use super::page::{length_to_cells};
use super::types::PageBackground;
use crate::markdown::highlighting::ColorMode;

/// Resolved page-frame layout state used during rendering.
///
/// Created from a [`DarkmatterPage`](super::DarkmatterPage) at render time so
/// downstream code does not recompute widths on every row. Component-level
/// layout and colors are handled by the context-aware fold
/// ([`TreeBuildContext`](crate::markdown::render_tree::build_context::TreeBuildContext));
/// this type carries only the page-frame geometry and background.
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
    /// Whether any page-frame layout settings are non-default (triggers row
    /// decoration).
    pub has_layout: bool,
    /// Resolved page background color, if any.
    pub background_color: Option<BackgroundColor>,
    /// Color mode passed to the markdown renderer (may be inverted for Pronounced).
    pub render_color_mode: ColorMode,
    /// Page-level background color. Painted by the page frame (browser wrapper /
    /// terminal row decoration); the page foreground is not held here — it rides
    /// the render tree's root node and inherits from there.
    pub page_bg_color: Option<renderable::style::PaintColor>,
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

use super::page::{length_is_zero};

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
        page_color: Option<renderable::style::PaintColor>,
        page_bg_color: Option<renderable::style::PaintColor>,
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

        // An explicit page `bg-color` takes precedence over the computed
        // `PageBackground` and paints the terminal page frame. The page
        // background no longer rides the (non-inheriting) root node style, so the
        // frame is where it is applied — mirroring the browser page wrapper.
        let background_color = page_bg_color
            .and_then(|paint| paint.color.to_rgb())
            .map(|(r, g, b)| BackgroundColor { r, g, b })
            .or(background_color);

        Ok(Self {
            terminal_width,
            page_margin,
            page_padding,
            content_width,
            effective_width,
            has_layout,
            background_color,
            render_color_mode,
            page_bg_color,
        })
    }

    /// Whether row decoration should be applied.
    pub fn needs_decoration(&self) -> bool {
        self.has_layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            page_bg_color: None,
        }
    }

    use renderable::color::{Color, Tailwind};
    use renderable::style::PaintColor;

    fn red_paint() -> PaintColor {
        PaintColor::new(Color::Tailwind(Tailwind::Red500))
    }

    #[test]
    fn needs_decoration_true_when_page_color_set() {
        // A page foreground forces the decorated path (so the page frame wraps the
        // body) even though the color itself now rides the tree root.
        use biscuit_terminal::discovery::detection::ColorMode as DetectMode;
        let c = LayoutContext::from_page(
            80,
            renderable::layout::Edges::default(),
            renderable::layout::Edges::default(),
            PageBackground::Transparent,
            None,
            &DetectMode::Dark,
            ColorMode::Dark,
            Some(red_paint()),
            None,
        )
        .expect("from_page");
        assert!(c.needs_decoration());
    }

    #[test]
    fn needs_decoration_true_when_page_bg_color_set() {
        let mut c = ctx(100, 80);
        c.page_bg_color = Some(red_paint());
        assert!(c.needs_decoration());
    }
}
