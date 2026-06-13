//! Render context and options for the terminal render-tree renderer.
//!
//! [`TerminalRenderContext`] bundles the terminal capabilities the renderer
//! consults; [`TerminalRenderOptions`] pairs that context with a
//! [`RenderStrictness`] policy.

use std::path::PathBuf;
use std::rc::Rc;

use renderable::layout::Layout as RenderableLayout;
use renderable::tree::{CodeRenderer, GraphicsMode, RenderStrictness, TerminalMermaidMode};

/// How an image renders when no raster form is emitted (graphics off / vector,
/// or a failed protocol render).
///
/// The default [`Bracket`](Self::Bracket) form (`[alt]`) is the generic
/// renderable fallback. The darkmatter decorated document pipeline opts into
/// [`Block`](Self::Block) (`▉ IMAGE[alt]`) to match its legacy
/// `for_terminal_with_layout` placeholder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImagePlaceholder {
    /// `[alt]` — the generic inline fallback.
    #[default]
    Bracket,
    /// `▉ IMAGE[alt]` — the darkmatter legacy block placeholder.
    Block,
}

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
    /// The detected terminal foreground/text color, when available.
    pub page_text_color: Option<(u8, u8, u8)>,
    /// The detected terminal background color, when available.
    pub page_background_color: Option<(u8, u8, u8)>,
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
    /// Active layout carried on the context, set via [`with_layout`].
    ///
    /// This field is not consulted by the tree renderer today: it applies a
    /// node's layout directly from
    /// [`NodeAttrs::layout`](renderable::tree::NodeAttrs::layout). The field is
    /// retained for the [`with_layout`] API shape and for potential future use
    /// by child renderers.
    ///
    /// [`with_layout`]: TerminalRenderContext::with_layout
    pub active_layout: Option<RenderableLayout>,
    /// The graphics fidelity tier for this render.
    pub graphics_mode: GraphicsMode,
    /// Whether to force graphics rendering even when the terminal
    /// capability detection suggests the feature is unavailable.
    pub force_graphics: bool,
    /// Whether Mermaid code fences are promoted to diagrams.
    ///
    /// Orthogonal to [`graphics_mode`](Self::graphics_mode), which is the
    /// fidelity ceiling: a Mermaid fence is promoted only when this opt-in is
    /// [`TerminalMermaidMode::Image`] *and* the tier permits rasterization.
    pub mermaid_mode: TerminalMermaidMode,
    /// Base path for resolving relative image paths at
    /// [`GraphicsMode::Rich`]. When `None`, the current working directory is
    /// used (matching the legacy terminal image renderer).
    pub image_base_path: Option<PathBuf>,
    /// Whether top-level paragraphs are word-wrapped to
    /// [`available_width`](Self::available_width).
    ///
    /// Off by default: component callers (e.g. [`Prose`](crate::components::prose::Prose))
    /// apply their own post-render wrapping, so the tree paragraph path must
    /// emit unwrapped lines for them. The darkmatter Markdown document pipeline
    /// turns this on so top-level prose flows to the content width, matching the
    /// legacy `for_terminal` renderer's line wrapping.
    pub wrap_prose: bool,
    /// Optional code-theme name forwarded to the [`CodeRenderer`] hook via
    /// [`TerminalCodeContext`](renderable::color::TerminalCodeContext).
    ///
    /// A plain string (e.g. `"github"`) so the renderer stays free of any
    /// consumer-specific theme types. When `None`, the code renderer uses its
    /// own default theme. The darkmatter terminal entry point sets this from
    /// `TerminalOptions::code_theme` so a caller's `with_code_theme(...)`
    /// reaches the highlighted code panel.
    ///
    /// [`CodeRenderer`]: renderable::tree::CodeRenderer
    pub code_theme: Option<String>,
    /// Whether fenced code blocks render a line-number gutter, forwarded to the
    /// [`CodeRenderer`](renderable::tree::CodeRenderer) hook.
    ///
    /// Off by default. The darkmatter document pipeline sets this from
    /// `TerminalOptions::include_line_numbers` so the page line-number toggle
    /// reaches the highlighted code panel.
    pub line_numbers: bool,
    /// Optional left-border prefix for unstyled block quotes.
    ///
    /// When `None` (the default), an unstyled [`NodeKind::Block­Quote`](renderable::tree::NodeKind::BlockQuote)
    /// renders through the shared [`BlockQuote`](crate::components::block_quote::BlockQuote)
    /// component with its canonical `│ ` border. When set, each quoted line is
    /// prefixed with this string instead, and nested quotes repeat it per level.
    /// The darkmatter Markdown document pipeline sets `"▐   "` so block quotes
    /// match the legacy `for_terminal` renderer's quarter-block bar.
    pub blockquote_prefix: Option<String>,
    /// How an image renders when no raster form is emitted.
    ///
    /// [`ImagePlaceholder::Bracket`] (the default) emits `[alt]`. The darkmatter
    /// decorated document pipeline sets [`ImagePlaceholder::Block`] so the
    /// fallback matches the legacy `▉ IMAGE[alt]` block placeholder.
    pub image_placeholder: ImagePlaceholder,
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
            color_depth: term.color_depth,
            color_mode: term.color_mode,
            page_text_color: term.text_color.map(|c| (c.r, c.g, c.b)),
            page_background_color: term.background_color.map(|c| (c.r, c.g, c.b)),
            hyperlinks: term.osc_link_support,
            image_support: term.image_support.clone(),
            supports_unicode: term.supports_unicode,
            layout: Layout::default(),
            terminal: Terminal::from(term),
            available_width: width,
            current_indent: 0,
            active_layout: None,
            graphics_mode: GraphicsMode::default(),
            force_graphics: false,
            mermaid_mode: TerminalMermaidMode::default(),
            image_base_path: None,
            wrap_prose: false,
            code_theme: None,
            line_numbers: false,
            blockquote_prefix: None,
            image_placeholder: ImagePlaceholder::default(),
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

    /// Creates a context with a specific active [`Layout`].
    ///
    /// Used when a [`TreeRenderable`] provides a layout that should be
    /// available to child renderers.
    ///
    /// [`TreeRenderable`]: renderable::tree::TreeRenderable
    /// [`Layout`]: renderable::layout::Layout
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::render_tree::TerminalRenderContext;
    /// use renderable::layout::{Layout, Length, Edges};
    ///
    /// let ctx = TerminalRenderContext::fallback();
    /// assert!(ctx.active_layout.is_none());
    ///
    /// let layout = Layout {
    ///     margin: Edges::y(Length::ch(1)),
    ///     ..Layout::default()
    /// };
    /// let with_layout = ctx.with_layout(Some(layout.clone()));
    /// assert_eq!(with_layout.active_layout, Some(layout));
    /// ```
    #[must_use]
    pub fn with_layout(&self, layout: Option<RenderableLayout>) -> Self {
        Self {
            active_layout: layout,
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
    fn from_terminal_initializes_layout_to_none() {
        let term = Terminal::new_optimistic(80);
        let ctx = TerminalRenderContext::from_terminal(&term);

        assert!(ctx.active_layout.is_none());
    }

    #[test]
    fn from_terminal_initializes_graphics_mode_to_rich() {
        let term = Terminal::new_optimistic(80);
        let ctx = TerminalRenderContext::from_terminal(&term);

        assert_eq!(ctx.graphics_mode, GraphicsMode::Rich);
    }

    #[test]
    fn from_terminal_initializes_force_graphics_to_false() {
        let term = Terminal::new_optimistic(80);
        let ctx = TerminalRenderContext::from_terminal(&term);

        assert!(!ctx.force_graphics);
    }

    #[test]
    fn fallback_initializes_new_fields_correctly() {
        let ctx = TerminalRenderContext::fallback();

        assert_eq!(ctx.width, 80);
        assert_eq!(ctx.available_width, 80);
        assert_eq!(ctx.current_indent, 0);
        assert!(ctx.active_layout.is_none());
        assert_eq!(ctx.graphics_mode, GraphicsMode::Rich);
        assert!(!ctx.force_graphics);
        assert_eq!(ctx.mermaid_mode, TerminalMermaidMode::Code);
        assert!(ctx.image_base_path.is_none());
        assert!(!ctx.wrap_prose);
        assert!(ctx.code_theme.is_none());
        assert!(!ctx.line_numbers);
        assert!(ctx.blockquote_prefix.is_none());
        assert_eq!(ctx.image_placeholder, ImagePlaceholder::Bracket);
    }

    #[test]
    fn from_terminal_keeps_mermaid_promotion_off_by_default() {
        // Public default: a Mermaid fence stays a code block unless the caller
        // opts in. `GraphicsMode::Rich` is only the ceiling, not a request.
        let ctx = TerminalRenderContext::from_terminal(&Terminal::new_optimistic(80));
        assert_eq!(ctx.mermaid_mode, TerminalMermaidMode::Code);
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
    fn for_child_preserves_layout() {
        use renderable::layout::{Layout, Length, Edges};

        let ctx = TerminalRenderContext::fallback();
        let layout = Layout {
            margin: Edges::y(Length::ch(2)),
            ..Layout::default()
        };
        let with_layout = ctx.with_layout(Some(layout.clone()));
        let child = with_layout.for_child(4, 0);

        assert_eq!(child.active_layout, Some(layout));
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
    fn with_layout_sets_layout() {
        use renderable::layout::{Layout, Length, Edges};

        let ctx = TerminalRenderContext::fallback();
        let layout = Layout {
            margin: Edges::x(Length::ch(3)),
            ..Layout::default()
        };
        let with_layout = ctx.with_layout(Some(layout.clone()));

        assert_eq!(with_layout.active_layout, Some(layout));
    }

    #[test]
    fn with_layout_replaces_existing_layout() {
        use renderable::layout::{Layout, Length, Edges};

        let ctx = TerminalRenderContext::fallback();
        let layout1 = Layout {
            margin: Edges::y(Length::ch(1)),
            ..Layout::default()
        };
        let layout2 = Layout {
            margin: Edges::y(Length::ch(5)),
            ..Layout::default()
        };

        let with_layout1 = ctx.with_layout(Some(layout1));
        let with_layout2 = with_layout1.with_layout(Some(layout2.clone()));

        assert_eq!(with_layout2.active_layout, Some(layout2));
    }

    #[test]
    fn with_layout_preserves_width_and_indent() {
        use renderable::layout::{Layout, Length, Edges};

        let ctx = TerminalRenderContext::fallback();
        let indented = ctx.for_child(5, 3);
        let layout = Layout {
            margin: Edges::y(Length::ch(1)),
            ..Layout::default()
        };
        let with_layout = indented.with_layout(Some(layout));

        assert_eq!(with_layout.width, 80);
        assert_eq!(with_layout.available_width, 72);
        assert_eq!(with_layout.current_indent, 5);
    }
}
