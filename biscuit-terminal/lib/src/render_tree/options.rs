//! Render context and options for the terminal render-tree renderer.
//!
//! [`TerminalRenderContext`] bundles the terminal capabilities the renderer
//! consults; [`TerminalRenderOptions`] pairs that context with a
//! [`RenderStrictness`] policy and the optional render hooks
//! ([`CodeRenderer`], [`DiagramRenderer`]).

use std::path::PathBuf;
use std::rc::Rc;

use renderable::layout::Layout as RenderableLayout;
use renderable::output::{OutputOptions, TerminalProfile, Width};
use renderable::tree::{
    CodeRenderer, GraphicsMode, NodeAttrs, RenderNode, RenderStrictness, TerminalMermaidMode,
};

use crate::discovery::detection::{ColorDepth, ColorMode, GraphicsRoute, ImageSupport, UnderlineSupport};
use crate::terminal::Terminal;

const NO_UNDERLINE: UnderlineSupport = UnderlineSupport {
    straight: false,
    double: false,
    curly: false,
    dotted: false,
    dashed: false,
    colored: false,
};
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
/// assert_eq!(ctx.width, Width::Columns(80));
/// assert_eq!(ctx.available_width, Width::Columns(80));
/// assert_eq!(ctx.current_indent, 0);
/// # use renderable::output::Width;
/// ```
#[derive(Debug, Clone)]
pub struct TerminalRenderContext {
    /// The output width of the whole render.
    ///
    /// Under [`Width::Columns`] prose wraps and fills (rules, code bands,
    /// table plans) reach that many columns; under [`Width::Unlimited`] and
    /// [`Width::Source`] nothing wraps and every fill renders at its natural
    /// width.
    pub width: Width,
    /// The color depth the terminal advertises.
    pub color_depth: ColorDepth,
    /// The terminal light/dark mode.
    pub color_mode: ColorMode,
    /// The detected terminal foreground/text color, when available.
    pub page_text_color: Option<(u8, u8, u8)>,
    /// The detected terminal background color, when available.
    pub page_background_color: Option<(u8, u8, u8)>,
    /// Theme-resolved foreground for inline code spans, when supplied by the
    /// caller. Set by the darkmatter terminal entry point from the prose theme;
    /// `None` for callers that build the context directly.
    pub inline_code_color: Option<(u8, u8, u8)>,
    /// Theme-resolved background for inline code spans, when supplied by the
    /// caller. When set, inline code renders on this band instead of
    /// reverse-video; `None` falls back to a dim run.
    pub inline_code_background: Option<(u8, u8, u8)>,
    /// Whether the terminal renders OSC8 hyperlinks.
    pub hyperlinks: bool,
    /// Whether the terminal renders Unicode glyphs correctly.
    pub supports_unicode: bool,
    /// The base layout applied to block-level output.
    pub layout: Layout,
    /// The detected [`Terminal`], retained so components can render with
    /// faithful capabilities.
    ///
    /// Its `fixed_width` mirrors [`available_width`](Self::available_width)
    /// only under [`Width::Columns`]; under an unbounded width the
    /// render-tree paths read `available_width` and never size output from
    /// this snapshot's width.
    pub terminal: Terminal,
    /// The available rendering width after margins and indentation.
    ///
    /// Initially equal to `width`, this tracks the current renderable width
    /// as the renderer descends into nested structures that consume horizontal
    /// space. An unbounded width stays unbounded.
    pub available_width: Width,
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
    /// fidelity ceiling: a Mermaid fence is promoted only when this opt-in
    /// and the ceiling both allow it (see [`DiagramRenderer`]).
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
    /// The global `-v` count. Above zero, a link or image that cannot be
    /// clickable also prints its destination.
    pub verbosity: u8,
    /// Whether this render is pinned to the plain profile.
    ///
    /// Set by [`from_profile`](Self::from_profile) for a plain profile and by
    /// [`pin_plain`](Self::pin_plain). A plain render writes no escape byte
    /// and no trailing whitespace, and prints structure as text: block quotes
    /// as `> `, rules as `---`, headings without `#` markers, and disclosure
    /// bodies as ordinary blocks.
    pub plain: bool,
}

impl TerminalRenderContext {
    /// Builds a context from an already-detected [`Terminal`].
    ///
    /// Width, color depth, hyperlink support, and Unicode
    /// support are copied from the terminal; the layout defaults to
    /// [`Layout::default`]. The `available_width` is initialized to the
    /// terminal width and `current_indent` to 0.
    #[must_use]
    pub fn from_terminal(term: &Terminal) -> Self {
        let width = Width::Columns(term.width().max(1));
        Self {
            width,
            color_depth: term.color_depth,
            color_mode: term.color_mode,
            page_text_color: term.text_color.map(|c| (c.r, c.g, c.b)),
            page_background_color: term.background_color.map(|c| (c.r, c.g, c.b)),
            inline_code_color: None,
            inline_code_background: None,
            hyperlinks: term.osc_link_support,
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
            verbosity: 0,
            plain: false,
        }
    }

    /// Builds a context from an output profile and width, with no ambient
    /// detection.
    ///
    /// A plain profile ([`TerminalProfile::is_plain`]) pins the context with
    /// [`pin_plain`](Self::pin_plain).
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::render_tree::TerminalRenderContext;
    /// use renderable::output::{TerminalProfile, Width};
    ///
    /// let ctx = TerminalRenderContext::from_profile(&TerminalProfile::plain(), Width::Unlimited);
    /// assert!(ctx.plain);
    /// assert_eq!(ctx.available_width, Width::Unlimited);
    /// assert!(!ctx.hyperlinks);
    /// ```
    #[must_use]
    pub fn from_profile(profile: &TerminalProfile, width: Width) -> Self {
        let mut term = Terminal::new_optimistic(width.as_columns().unwrap_or(80));
        term.color_depth = profile.color_depth.into();
        term.color_mode = profile.color_mode.into();
        term.text_color = profile.text_color.map(|(r, g, b)| crate::discovery::osc_queries::RgbValue { r, g, b });
        term.background_color =
            profile.background_color.map(|(r, g, b)| crate::discovery::osc_queries::RgbValue { r, g, b });
        term.is_tty = profile.is_tty;
        term.osc_link_support = profile.hyperlinks;
        term.supports_italic = profile.italics;
        if !profile.underline {
            term.underline_support = NO_UNDERLINE;
        }
        if !profile.graphics {
            term.graphics_route = GraphicsRoute::from(ImageSupport::None);
        }
        term.supports_unicode = profile.unicode;

        let mut ctx = Self::from_terminal(&term);
        ctx.width = width;
        ctx.available_width = width;
        if profile.is_plain() {
            ctx.pin_plain();
        }
        ctx
    }

    /// Builds a context from the options every output path receives: the
    /// profile, width, and verbosity.
    #[must_use]
    pub fn from_output_options(options: &OutputOptions) -> Self {
        let mut ctx = Self::from_profile(&options.terminal_profile, options.width);
        ctx.verbosity = options.verbosity;
        ctx
    }

    /// The profile this context renders with.
    #[must_use]
    pub fn profile(&self) -> TerminalProfile {
        if self.plain {
            return TerminalProfile::plain();
        }
        let underline = &self.terminal.underline_support;
        TerminalProfile {
            color_depth: self.color_depth.into(),
            color_mode: self.color_mode.into(),
            text_color: self.page_text_color,
            background_color: self.page_background_color,
            is_tty: self.terminal.is_tty,
            hyperlinks: self.hyperlinks,
            italics: self.terminal.supports_italic,
            underline: underline.straight
                || underline.double
                || underline.curly
                || underline.dotted
                || underline.dashed,
            graphics: self.terminal.image_support() != ImageSupport::None,
            unicode: self.supports_unicode,
        }
    }

    /// Pins this context to the plain profile.
    ///
    /// Call it after every other option has been applied: it turns off
    /// color, OSC 8 links, italics, underline, and images on both the context
    /// and its terminal snapshot, so no later capability check can emit an
    /// escape sequence. Width, verbosity, and layout are kept.
    pub fn pin_plain(&mut self) {
        self.plain = true;
        self.color_depth = ColorDepth::None;
        self.hyperlinks = false;
        self.inline_code_color = None;
        self.inline_code_background = None;
        self.graphics_mode = GraphicsMode::Off;
        self.force_graphics = false;
        self.mermaid_mode = TerminalMermaidMode::Code;
        self.terminal.color_depth = ColorDepth::None;
        self.terminal.osc_link_support = false;
        self.terminal.supports_italic = false;
        self.terminal.underline_support = NO_UNDERLINE;
        self.terminal.graphics_route = GraphicsRoute::from(ImageSupport::None);
        self.terminal.is_tty = false;
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
    /// `indent_delta` and `width_delta` are subtracted from a bounded
    /// `available_width` (never below one column); an unbounded width stays
    /// unbounded.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::render_tree::TerminalRenderContext;
    ///
    /// let ctx = TerminalRenderContext::fallback();
    /// assert_eq!(ctx.available_width, Width::Columns(80));
    /// assert_eq!(ctx.current_indent, 0);
    ///
    /// let child = ctx.for_child(4, 0);
    /// assert_eq!(child.available_width, Width::Columns(76));
    /// assert_eq!(child.current_indent, 4);
    ///
    /// let grandchild = child.for_child(2, 2);
    /// assert_eq!(grandchild.available_width, Width::Columns(72));
    /// assert_eq!(grandchild.current_indent, 6);
    /// # use renderable::output::Width;
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
    /// let narrow = ctx.with_width(Width::Columns(40));
    /// assert_eq!(narrow.available_width, Width::Columns(40));
    /// assert_eq!(narrow.width, Width::Columns(80)); // root width unchanged
    /// # use renderable::output::Width;
    /// ```
    #[must_use]
    pub fn with_width(&self, available_width: Width) -> Self {
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
    /// An optional hook that draws diagram code fences.
    ///
    /// Without one a Mermaid fence always renders as a code block, whatever
    /// [`TerminalRenderContext::mermaid_mode`] asks for. See
    /// [`DiagramRenderer`] for when it is consulted.
    pub diagram_renderer: Option<Rc<dyn DiagramRenderer>>,
}

impl std::fmt::Debug for TerminalRenderOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The hooks are not `Debug`; report only whether each is set.
        f.debug_struct("TerminalRenderOptions")
            .field("context", &self.context)
            .field("strictness", &self.strictness)
            .field("code_renderer", &self.code_renderer.is_some())
            .field("diagram_renderer", &self.diagram_renderer.is_some())
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
            diagram_renderer: None,
        }
    }

    /// Builds options for an output profile and width (see
    /// [`TerminalRenderContext::from_output_options`]).
    #[must_use]
    pub fn from_output_options(options: &OutputOptions, strictness: RenderStrictness) -> Self {
        Self {
            context: TerminalRenderContext::from_output_options(options),
            strictness,
            code_renderer: None,
            diagram_renderer: None,
        }
    }

    /// Pins the render to the plain profile (see
    /// [`TerminalRenderContext::pin_plain`]). Installed hooks stay installed
    /// but are handed a colorless context.
    pub fn pin_plain(&mut self) {
        self.context.pin_plain();
    }

    /// Sets the optional code-block render hook.
    #[must_use]
    pub fn with_code_renderer(mut self, code_renderer: Rc<dyn CodeRenderer>) -> Self {
        self.code_renderer = Some(code_renderer);
        self
    }

    /// Sets the optional diagram render hook.
    #[must_use]
    pub fn with_diagram_renderer(mut self, diagram_renderer: Rc<dyn DiagramRenderer>) -> Self {
        self.diagram_renderer = Some(diagram_renderer);
        self
    }
}

/// Draws a diagram code fence for the terminal render-tree renderer.
///
/// This crate does not know about diagrams. A caller that wants Mermaid
/// fences drawn installs an implementation on
/// [`TerminalRenderOptions::diagram_renderer`]; `biscuit-visualized` provides
/// one behind its `terminal` feature.
///
/// The renderer consults the hook for a `mermaid` fence, before the
/// [`CodeRenderer`] hook, when [`TerminalRenderContext::mermaid_mode`] and
/// [`TerminalRenderContext::graphics_mode`] allow it:
///
/// | `mermaid_mode` | `Rich` | `Vector` | `Off` |
/// | --- | --- | --- | --- |
/// | [`Code`](TerminalMermaidMode::Code) | no | no | no |
/// | [`Image`](TerminalMermaidMode::Image) | yes | no | no |
/// | [`Automatic`](TerminalMermaidMode::Automatic) | yes | yes | no |
///
/// The hook must not exceed [`DiagramRenderRequest::graphics_mode`]: at
/// `Vector` it may answer with text but never an image. An `Err` is handled by
/// the render's [`RenderStrictness`]: `Strict` rejects the render, `Warn`
/// records a lossy diagnostic, and both `Warn` and `Lossy` fall back to code
/// rendering. [`DiagramOutcome::Declined`] renders the code block with no
/// diagnostic under every strictness, and so does a
/// [`DiagramOutcome::Nodes`] fragment whose finished rows do not fit the
/// fence's columns. A [`DiagramOutcome::Rendered`] answer is never measured.
pub trait DiagramRenderer {
    /// Renders `request.source` for the terminal.
    ///
    /// ## Errors
    ///
    /// Returns a [`DiagramRenderError`] when the diagram cannot be drawn for
    /// this terminal (invalid source, no usable graphics route, a rendering
    /// failure).
    fn render_terminal_diagram(
        &self,
        request: &DiagramRenderRequest<'_>,
    ) -> Result<DiagramOutcome, DiagramRenderError>;
}

/// A [`DiagramRenderer`]'s successful answer.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DiagramOutcome {
    /// Terminal output the hook already formatted (for example an image
    /// escape sequence), printed verbatim in place of the fence.
    Rendered(String),
    /// A render-tree fragment laid out in place of the fence, at the fence's
    /// indent and available width. Code nodes in it go through the installed
    /// [`CodeRenderer`]; a `mermaid` code node in it is never offered to the
    /// diagram hook again. If any finished row (measured in display cells,
    /// escape sequences excluded) is wider than
    /// [`DiagramRenderRequest::available_width`], for instance because the
    /// code renderer added a line-number gutter, the fragment is discarded
    /// and the outcome is treated as [`Declined`](Self::Declined).
    Nodes(Vec<RenderNode>),
    /// The hook chose not to draw this fence. It renders as the ordinary code
    /// block, with no diagnostic, exactly as if the hook had not been called.
    Declined,
}

/// What the render-tree renderer hands a [`DiagramRenderer`].
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct DiagramRenderRequest<'a> {
    /// The fence body: the diagram source.
    pub source: &'a str,
    /// The code node's attributes; its layout is `attrs.layout`.
    pub attrs: &'a NodeAttrs,
    /// The capability snapshot to render against, with
    /// [`TerminalRenderContext::force_graphics`] already applied.
    pub terminal: &'a Terminal,
    /// The width available at the fence's position. A text answer laid out
    /// under an unbounded width is never measured against it.
    pub available_width: Width,
    /// The render's fidelity ceiling: `Rich` or `Vector`, never `Off`.
    pub graphics_mode: GraphicsMode,
    /// The promotion mode that offered the fence: `Image` (only at `Rich`)
    /// or `Automatic`, never `Code`. An `Image` request asks for an image
    /// and nothing else; an `Automatic` one may also be answered with text.
    pub mermaid_mode: TerminalMermaidMode,
}

/// Why a [`DiagramRenderer`] could not draw a diagram.
///
/// Displays as the underlying error, so diagnostics read the same whichever
/// hook produced them.
#[derive(Debug)]
pub struct DiagramRenderError(Box<dyn std::error::Error>);

impl DiagramRenderError {
    /// Wraps the hook's own error.
    pub fn new(error: impl Into<Box<dyn std::error::Error>>) -> Self {
        Self(error.into())
    }
}

impl std::fmt::Display for DiagramRenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for DiagramRenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_terminal_initializes_available_width_to_width() {
        let term = Terminal::new_optimistic(120);
        let ctx = TerminalRenderContext::from_terminal(&term);

        assert_eq!(ctx.width, Width::Columns(120));
        assert_eq!(ctx.available_width, Width::Columns(120));
    }

    #[test]
    fn for_child_keeps_an_unbounded_width_unbounded() {
        let mut ctx = TerminalRenderContext::fallback();
        ctx.available_width = Width::Unlimited;
        assert_eq!(ctx.for_child(10, 5).available_width, Width::Unlimited);
    }

    #[test]
    fn from_profile_maps_capabilities_and_width() {
        use renderable::color::ColorDepth as ProfileDepth;

        let mut profile = TerminalProfile::plain();
        profile.color_depth = ProfileDepth::Basic;
        profile.hyperlinks = true;
        profile.is_tty = true;
        let ctx = TerminalRenderContext::from_profile(&profile, Width::Columns(40));
        assert!(!ctx.plain);
        assert_eq!(ctx.color_depth, ColorDepth::Basic);
        assert!(ctx.hyperlinks && ctx.terminal.osc_link_support);
        assert_eq!(ctx.width, Width::Columns(40));
        assert_eq!(ctx.terminal.fixed_width, Some(40));
        assert_eq!(ctx.profile(), profile);
    }

    #[test]
    fn a_plain_profile_pins_the_context() {
        let ctx = TerminalRenderContext::from_profile(&TerminalProfile::plain(), Width::Source);
        assert!(ctx.plain);
        assert_eq!(ctx.color_depth, ColorDepth::None);
        assert_eq!(ctx.terminal.color_depth, ColorDepth::None);
        assert!(!ctx.hyperlinks && !ctx.terminal.osc_link_support);
        assert_eq!(ctx.profile(), TerminalProfile::plain());
        assert_eq!(ctx.available_width, Width::Source);
    }

    #[test]
    fn pin_plain_overrides_capabilities_set_earlier() {
        let mut options = TerminalRenderOptions::new(&Terminal::new_optimistic(80), RenderStrictness::Warn);
        options.context.hyperlinks = true;
        options.context.inline_code_background = Some((1, 2, 3));
        options.context.graphics_mode = GraphicsMode::Rich;
        options.pin_plain();
        assert!(options.context.plain);
        assert!(!options.context.hyperlinks);
        assert!(options.context.inline_code_background.is_none());
        assert_eq!(options.context.graphics_mode, GraphicsMode::Off);
        assert!(!options.context.terminal.supports_italic);
    }

    #[test]
    fn from_output_options_carries_verbosity() {
        let options = OutputOptions::new(Width::Unlimited, TerminalProfile::plain()).with_verbosity(2);
        let ctx = TerminalRenderContext::from_output_options(&options);
        assert_eq!(ctx.verbosity, 2);
        assert_eq!(ctx.width, Width::Unlimited);
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

        assert_eq!(ctx.width, Width::Columns(80));
        assert_eq!(ctx.available_width, Width::Columns(80));
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
        assert_eq!(ctx.verbosity, 0);
        assert!(!ctx.plain);
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

        assert_eq!(child.available_width, Width::Columns(80 - 4 - 2));
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

        assert_eq!(child.width, Width::Columns(80));
    }

    #[test]
    fn for_child_saturates_at_one_column() {
        let ctx = TerminalRenderContext::fallback();
        let child = ctx.for_child(100, 100);

        assert_eq!(child.available_width, Width::Columns(1));
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
        let narrow = ctx.with_width(Width::Columns(40));

        assert_eq!(narrow.available_width, Width::Columns(40));
    }

    #[test]
    fn with_width_preserves_root_width() {
        let ctx = TerminalRenderContext::fallback();
        let narrow = ctx.with_width(Width::Columns(40));

        assert_eq!(narrow.width, Width::Columns(80));
    }

    #[test]
    fn with_width_preserves_current_indent() {
        let ctx = TerminalRenderContext::fallback();
        let indented = ctx.for_child(5, 0);
        let narrow = indented.with_width(Width::Columns(30));

        assert_eq!(narrow.current_indent, 5);
        assert_eq!(narrow.available_width, Width::Columns(30));
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

        assert_eq!(with_layout.width, Width::Columns(80));
        assert_eq!(with_layout.available_width, Width::Columns(72));
        assert_eq!(with_layout.current_indent, 5);
    }
}
