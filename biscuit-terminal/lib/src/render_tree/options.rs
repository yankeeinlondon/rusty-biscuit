//! Render context and options for the terminal render-tree renderer.
//!
//! [`TerminalRenderContext`] bundles the terminal capabilities the renderer
//! consults; [`TerminalRenderOptions`] pairs that context with a
//! [`RenderStrictness`] policy.

use renderable::tree::RenderStrictness;

use crate::discovery::detection::{ColorDepth, ColorMode, ImageSupport};
use crate::terminal::Terminal;
use crate::utils::layout::Layout;

/// The terminal state the render-tree renderer consults.
///
/// This is a thin, owned snapshot of the parts of a [`Terminal`] that affect
/// structural rendering decisions: usable width, color depth, OSC8 hyperlink
/// support, image-protocol support, light/dark mode, and a base [`Layout`].
///
/// Terminal capability detection is *not* reimplemented here — a context is
/// built from an already-detected [`Terminal`] via
/// [`TerminalRenderContext::from_terminal`].
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::render_tree::TerminalRenderContext;
/// use biscuit_terminal::terminal::Terminal;
///
/// let term = Terminal::new_optimistic(80);
/// let ctx = TerminalRenderContext::from_terminal(&term);
/// assert_eq!(ctx.width, 80);
/// ```
#[derive(Debug, Clone)]
pub struct TerminalRenderContext {
    /// The usable terminal width in columns.
    pub width: u32,
    /// The color depth the terminal advertises.
    pub color_depth: ColorDepth,
    /// The terminal light/dark mode.
    pub color_mode: ColorMode,
    /// Whether the terminal renders OSC8 hyperlinks.
    pub hyperlinks: bool,
    /// The terminal image-protocol support level.
    pub image_support: ImageSupport,
    /// Whether the terminal renders Unicode glyphs correctly.
    pub supports_unicode: bool,
    /// The base layout applied to block-level output.
    pub layout: Layout,
    /// The detected [`Terminal`], retained so components can render with
    /// faithful capabilities.
    pub terminal: Terminal,
}

impl TerminalRenderContext {
    /// Builds a context from an already-detected [`Terminal`].
    ///
    /// Width, color depth, hyperlink support, image support, and Unicode
    /// support are copied from the terminal; the layout defaults to
    /// [`Layout::default`].
    #[must_use]
    pub fn from_terminal(term: &Terminal) -> Self {
        Self {
            width: term.width(),
            color_depth: term.color_depth.clone(),
            color_mode: term.color_mode.clone(),
            hyperlinks: term.osc_link_support,
            image_support: term.image_support.clone(),
            supports_unicode: term.supports_unicode,
            layout: Layout::default(),
            terminal: Terminal::from(term),
        }
    }

    /// Builds a fallback context for an 80-column modern terminal.
    ///
    /// Unlike [`TerminalRenderContext::default`], this performs no
    /// environment detection. It is useful for tests and for rendering
    /// intended for a generic modern terminal.
    #[must_use]
    pub fn fallback() -> Self {
        Self::from_terminal(&Terminal::new_optimistic(80))
    }
}

impl Default for TerminalRenderContext {
    /// Builds a context from a freshly detected [`Terminal`].
    fn default() -> Self {
        Self::from_terminal(&Terminal::default())
    }
}

/// Options controlling a terminal render-tree render.
///
/// The [`Default`] uses a detected [`TerminalRenderContext`] and
/// [`RenderStrictness::Warn`].
#[derive(Debug, Clone, Default)]
pub struct TerminalRenderOptions {
    /// The terminal state the renderer consults.
    pub context: TerminalRenderContext,
    /// How strictly lossy or unsupported content is treated.
    pub strictness: RenderStrictness,
}

impl TerminalRenderOptions {
    /// Builds options for the given terminal and strictness.
    #[must_use]
    pub fn new(term: &Terminal, strictness: RenderStrictness) -> Self {
        Self {
            context: TerminalRenderContext::from_terminal(term),
            strictness,
        }
    }
}

