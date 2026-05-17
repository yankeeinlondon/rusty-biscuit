//! Render context and options for the terminal render-tree renderer.
//!
//! [`TerminalRenderContext`] bundles the terminal capabilities the renderer
//! consults; [`TerminalRenderOptions`] pairs that context with a
//! [`RenderStrictness`] policy.

use std::rc::Rc;

use renderable::tree::{CodeRenderer, LayoutHints, RenderStrictness};

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
/// assert_eq!(ctx.available_width, 80);
/// assert_eq!(ctx.current_indent, 0);
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
    /// The available rendering width after margins and indentation.
    ///
    /// Initially equal to `width`, this tracks the current renderable width
    /// as the renderer descends into nested structures that consume horizontal
    /// space.
    pub available_width: u32,
    /// Current indentation level in characters.
    ///
    /// Tracks how far from the left margin the current content should be
    /// rendered. Starts at 0 and increases as the renderer enters nested
    /// structures like block quotes or list items.
    pub current_indent: u32,
    /// Active layout hints from the current tree context.
    ///
    /// When a [`TreeRenderable`] provides layout hints, they are propagated
    /// here so child renderers can consult them.
    ///
    /// [`TreeRenderable`]: renderable::tree::TreeRenderable
    pub active_layout_hints: Option<LayoutHints>,
}

impl TerminalRenderContext {
    /// Builds a context from an already-detected [`Terminal`].
    ///
    /// Width, color depth, hyperlink support, image support, and Unicode
    /// support are copied from the terminal; the layout defaults to
    /// [`Layout::default`]. The `available_width` is initialized to the
    /// terminal width and `current_indent` to 0.
    #[must_use]
    pub fn from_terminal(term: &Terminal) -> Self {
        let width = term.width();
        Self {
            width,
            color_depth: term.color_depth.clone(),
            color_mode: term.color_mode.clone(),
            hyperlinks: term.osc_link_support,
            image_support: term.image_support.clone(),
            supports_unicode: term.supports_unicode,
            layout: Layout::default(),
            terminal: Terminal::from(term),
            available_width: width,
            current_indent: 0,
            active_layout_hints: None,
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

    /// Creates a child context with adjusted indent and width.
    ///
    /// Used when rendering nested content that needs reduced width.
    /// The `indent_delta` is added to `current_indent` and both
    /// `indent_delta` and `width_delta` are subtracted from `available_width`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::render_tree::TerminalRenderContext;
    ///
    /// let ctx = TerminalRenderContext::fallback();
    /// assert_eq!(ctx.available_width, 80);
    /// assert_eq!(ctx.current_indent, 0);
    ///
    /// let child = ctx.for_child(4, 0);
    /// assert_eq!(child.available_width, 76);
    /// assert_eq!(child.current_indent, 4);
    ///
    /// let grandchild = child.for_child(2, 2);
    /// assert_eq!(grandchild.available_width, 72);
    /// assert_eq!(grandchild.current_indent, 6);
    /// ```
    #[must_use]
    pub fn for_child(&self, indent_delta: u32, width_delta: u32) -> Self {
        Self {
            available_width: self
                .available_width
                .saturating_sub(indent_delta + width_delta),
            current_indent: self.current_indent + indent_delta,
            ..self.clone()
        }
    }

    /// Creates a context with a specific available width.
    ///
    /// Useful when a component knows the exact width it should render within,
    /// such as a table cell or column in a two-column layout.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::render_tree::TerminalRenderContext;
    ///
    /// let ctx = TerminalRenderContext::fallback();
    /// let narrow = ctx.with_width(40);
    /// assert_eq!(narrow.available_width, 40);
    /// assert_eq!(narrow.width, 80); // root width unchanged
    /// ```
    #[must_use]
    pub fn with_width(&self, available_width: u32) -> Self {
        Self {
            available_width,
            ..self.clone()
        }
    }

    /// Creates a context with specific layout hints.
    ///
    /// Used when a [`TreeRenderable`] provides layout hints that should be
    /// available to child renderers.
    ///
    /// [`TreeRenderable`]: renderable::tree::TreeRenderable
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::render_tree::TerminalRenderContext;
    /// use renderable::tree::LayoutHints;
    ///
    /// let ctx = TerminalRenderContext::fallback();
    /// assert!(ctx.active_layout_hints.is_none());
    ///
    /// let hints = LayoutHints {
    ///     top_margin: Some(1),
    ///     ..Default::default()
    /// };
    /// let with_hints = ctx.with_layout(hints.clone());
    /// assert_eq!(with_hints.active_layout_hints, Some(hints));
    /// ```
    #[must_use]
    pub fn with_layout(&self, hints: LayoutHints) -> Self {
        Self {
            active_layout_hints: Some(hints),
            ..self.clone()
        }
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
#[derive(Clone, Default)]
pub struct TerminalRenderOptions {
    /// The terminal state the renderer consults.
    pub context: TerminalRenderContext,
    /// How strictly lossy or unsupported content is treated.
    pub strictness: RenderStrictness,
    /// An optional hook for custom code-block rendering.
    ///
    /// When set, the renderer consults this hook for every
    /// [`NodeKind::Code`](renderable::tree::NodeKind::Code) node before
    /// falling back to its built-in plain code-block rendering.
    pub code_renderer: Option<Rc<dyn CodeRenderer>>,
}

impl std::fmt::Debug for TerminalRenderOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `dyn CodeRenderer` is not `Debug`; report only whether one is set.
        f.debug_struct("TerminalRenderOptions")
            .field("context", &self.context)
            .field("strictness", &self.strictness)
            .field("code_renderer", &self.code_renderer.is_some())
            .finish()
    }
}

impl TerminalRenderOptions {
    /// Builds options for the given terminal and strictness.
    #[must_use]
    pub fn new(term: &Terminal, strictness: RenderStrictness) -> Self {
        Self {
            context: TerminalRenderContext::from_terminal(term),
            strictness,
            code_renderer: None,
        }
    }

    /// Builds options from an explicit context and strictness.
    ///
    /// Useful when you need to override specific context fields (e.g., for
    /// testing that `available_width` differs from root width).
    #[must_use]
    pub fn from_context(context: TerminalRenderContext, strictness: RenderStrictness) -> Self {
        Self {
            context,
            strictness,
            code_renderer: None,
        }
    }

    /// Sets the optional code-block render hook.
    #[must_use]
    pub fn with_code_renderer(mut self, code_renderer: Rc<dyn CodeRenderer>) -> Self {
        self.code_renderer = Some(code_renderer);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_terminal_initializes_available_width_to_width() {
        let term = Terminal::new_optimistic(120);
        let ctx = TerminalRenderContext::from_terminal(&term);

        assert_eq!(ctx.width, 120);
        assert_eq!(ctx.available_width, 120);
    }

    #[test]
    fn from_terminal_initializes_current_indent_to_zero() {
        let term = Terminal::new_optimistic(80);
        let ctx = TerminalRenderContext::from_terminal(&term);

        assert_eq!(ctx.current_indent, 0);
    }

    #[test]
    fn from_terminal_initializes_layout_hints_to_none() {
        let term = Terminal::new_optimistic(80);
        let ctx = TerminalRenderContext::from_terminal(&term);

        assert!(ctx.active_layout_hints.is_none());
    }

    #[test]
    fn fallback_initializes_new_fields_correctly() {
        let ctx = TerminalRenderContext::fallback();

        assert_eq!(ctx.width, 80);
        assert_eq!(ctx.available_width, 80);
        assert_eq!(ctx.current_indent, 0);
        assert!(ctx.active_layout_hints.is_none());
    }

    #[test]
    fn for_child_reduces_available_width_by_indent_plus_width_delta() {
        let ctx = TerminalRenderContext::fallback();
        let child = ctx.for_child(4, 2);

        assert_eq!(child.available_width, 80 - 4 - 2);
    }

    #[test]
    fn for_child_increases_current_indent_by_indent_delta() {
        let ctx = TerminalRenderContext::fallback();
        let child = ctx.for_child(4, 0);

        assert_eq!(child.current_indent, 4);

        let grandchild = child.for_child(2, 0);
        assert_eq!(grandchild.current_indent, 6);
    }

    #[test]
    fn for_child_preserves_root_width() {
        let ctx = TerminalRenderContext::fallback();
        let child = ctx.for_child(10, 5);

        assert_eq!(child.width, 80);
    }

    #[test]
    fn for_child_saturates_at_zero_width() {
        let ctx = TerminalRenderContext::fallback();
        let child = ctx.for_child(100, 100);

        assert_eq!(child.available_width, 0);
        assert_eq!(child.current_indent, 100);
    }

    #[test]
    fn for_child_preserves_layout_hints() {
        let ctx = TerminalRenderContext::fallback();
        let hints = LayoutHints {
            top_margin: Some(2),
            ..Default::default()
        };
        let with_hints = ctx.with_layout(hints.clone());
        let child = with_hints.for_child(4, 0);

        assert_eq!(child.active_layout_hints, Some(hints));
    }

    #[test]
    fn with_width_sets_specific_width() {
        let ctx = TerminalRenderContext::fallback();
        let narrow = ctx.with_width(40);

        assert_eq!(narrow.available_width, 40);
    }

    #[test]
    fn with_width_preserves_root_width() {
        let ctx = TerminalRenderContext::fallback();
        let narrow = ctx.with_width(40);

        assert_eq!(narrow.width, 80);
    }

    #[test]
    fn with_width_preserves_current_indent() {
        let ctx = TerminalRenderContext::fallback();
        let indented = ctx.for_child(5, 0);
        let narrow = indented.with_width(30);

        assert_eq!(narrow.current_indent, 5);
        assert_eq!(narrow.available_width, 30);
    }

    #[test]
    fn with_layout_sets_hints() {
        let ctx = TerminalRenderContext::fallback();
        let hints = LayoutHints {
            top_margin: Some(1),
            bottom_margin: Some(2),
            left_margin: Some(3),
            right_margin: Some(4),
        };
        let with_hints = ctx.with_layout(hints.clone());

        assert_eq!(with_hints.active_layout_hints, Some(hints));
    }

    #[test]
    fn with_layout_replaces_existing_hints() {
        let ctx = TerminalRenderContext::fallback();
        let hints1 = LayoutHints {
            top_margin: Some(1),
            ..Default::default()
        };
        let hints2 = LayoutHints {
            top_margin: Some(5),
            bottom_margin: Some(10),
            ..Default::default()
        };

        let with_hints1 = ctx.with_layout(hints1);
        let with_hints2 = with_hints1.with_layout(hints2.clone());

        assert_eq!(with_hints2.active_layout_hints, Some(hints2));
    }

    #[test]
    fn with_layout_preserves_width_and_indent() {
        let ctx = TerminalRenderContext::fallback();
        let indented = ctx.for_child(5, 3);
        let hints = LayoutHints {
            top_margin: Some(1),
            ..Default::default()
        };
        let with_hints = indented.with_layout(hints);

        assert_eq!(with_hints.width, 80);
        assert_eq!(with_hints.available_width, 72);
        assert_eq!(with_hints.current_indent, 5);
    }
}
