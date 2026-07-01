//! Graph expression rendering for terminals.
//!
//! This module provides a `TerminalRenderable` component that renders graph diagrams
//! in the terminal. It delegates graph layout to `biscuit-visualized` and
//! uses `TerminalImage` for inline image display.
//!
//! ## Examples
//!
//! ```rust,no_run
//! use biscuit_terminal::components::graph_expression::{GraphExpression, GraphInputSyntax};
//! use biscuit_terminal::components::renderable::TerminalRenderable;
//! use biscuit_terminal::terminal::Terminal;
//!
//! fn example() -> Result<(), biscuit_terminal::components::graph_expression::GraphRenderError> {
//!     let graph = GraphExpression::for_terminal("a -> b -> c", GraphInputSyntax::Auto)?;
//!     let term = Terminal::new();
//!     println!("{}", graph.render(&term));
//!     Ok(())
//! }
//! ```

use std::any::Any;
use std::path::PathBuf;

use thiserror::Error;

use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::tree::{RenderNode, TreeRenderable};

use crate::components::renderable::{BrowserRenderable, TerminalRenderable};
use crate::components::terminal_image::{ImageWidth, TerminalImage};
use crate::terminal::Terminal;
use crate::utils::layout::{Layout, LayoutTerminalExt};

// Re-export types from biscuit-visualized
pub use biscuit_visualized::graph::{GraphColorTheme, GraphInputSyntax, GraphOrientation};
pub use biscuit_visualized::raster::available_font_families;

/// Errors that can occur during terminal rendering of graph diagrams.
#[derive(Error, Debug)]
pub enum GraphRenderError {
    /// Visualization error from biscuit-visualized
    #[error(transparent)]
    Visualization(#[from] biscuit_visualized::graph::GraphError),
    /// Terminal does not support inline images
    #[error("Terminal does not support inline images")]
    NoImageSupport,
    /// Image display failed
    #[error("Image display failed: {0}")]
    DisplayError(String),
}

/// A graph expression component for terminal output.
///
/// Implements `TerminalRenderable` so it integrates with the standard layout system
/// (margins, alignment) and can be composed with other components.
///
/// ## Examples
///
/// ```rust,no_run
/// use biscuit_terminal::components::graph_expression::{
///     GraphExpression, GraphInputSyntax, GraphOrientation,
/// };
/// use biscuit_terminal::components::renderable::TerminalRenderable;
/// use biscuit_terminal::utils::layout::{Length, TargetValue};
///
/// let graph = GraphExpression::for_terminal("a -> b -> c", GraphInputSyntax::Auto)?
///     .with_orientation(GraphOrientation::LeftToRight)
///     .with_title("My Graph")
///     .left_margin(TargetValue::universal(Length::ch(4)));
///
/// # Ok::<(), biscuit_terminal::components::graph_expression::GraphRenderError>(())
/// ```
///
/// ## Layout & Style Contract
///
/// `GraphExpression` is a bespoke image component (spec C5): the rendered
/// graph is rasterized by `biscuit-visualized` and emitted through a
/// terminal image protocol (Kitty / iTerm2 / Sixel). It does not project to
/// a `RenderNode`; the [`TerminalRenderable`] impl applies the configured
/// [`Layout`] via `apply_block_layout` and the inner
/// [`TerminalImage`] resolves image dimensions from [`ImageWidth`].
///
/// - **Slack sink** (spec D2): the rendered graph canvas. The image
///   dimensions are resolved by [`TerminalImage::resolve_dimensions_for`]
///   from [`ImageWidth`] (Fill / Percent / Characters):
///   - `Fill` ⇒ the image canvas absorbs **all** the slack.
///   - `Percent(p)` ⇒ the image canvas is `p * term_width` (basis matches
///     `Length::Percent` semantics), then clamped under the available width.
///   - `Characters(n)` ⇒ an explicit cell count, clamped under the
///     available width.
/// - [`Layout::margin`] is honored: it reduces the available width before
///   the image width is resolved, so a wider margin shrinks the image
///   canvas.
/// - [`Layout::alignment`] is honored: the image's horizontal offset
///   reflects left / center / right alignment against the slack.
/// - `Layout::width` is **N/A** for the image canvas: [`ImageWidth`] is the
///   explicit contract that selects Fill / Percent / Characters, and
///   reusing `Layout::width` would create two competing width controls.
///   This is the documented GraphExpression-specific carve-out — it is not
///   a silent no-op because `Layout::width` is never read by the image
///   resolver, and `Layout::margin` / `Layout::alignment` ARE honored.
/// - A fractional `ImageWidth::Percent(p)` is resolved exactly once against
///   `term_width` and then clamped under the available width; it never
///   re-resolves the percentage against the margin-narrowed width (the
///   double-application bug).
///
/// [`Layout::margin`]: crate::utils::layout::Layout::margin
/// [`Layout::alignment`]: crate::utils::layout::Layout::alignment
pub struct GraphExpression {
    /// The inner diagram from biscuit-visualized
    diagram: biscuit_visualized::graph::GraphDiagram,
    /// Scale factor for output resolution (default: 2)
    scale: u32,
    /// Use transparent background
    transparent_background: bool,
    /// Image width specification for terminal display
    width: ImageWidth,
    /// Layout configuration (margins, alignment)
    layout: Layout,
}

impl std::fmt::Debug for GraphExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphExpression")
            .field("scale", &self.scale)
            .field("transparent_background", &self.transparent_background)
            .field("width", &self.width)
            .finish()
    }
}

impl GraphExpression {
    /// Creates a new `GraphExpression` by parsing the given source.
    ///
    /// Uses default settings: scale 2, opaque background, 50% width.
    ///
    /// ## Errors
    ///
    /// Returns `GraphRenderError` if parsing fails.
    pub fn parse(
        source: impl Into<String>,
        syntax: GraphInputSyntax,
    ) -> Result<Self, GraphRenderError> {
        let diagram = biscuit_visualized::graph::GraphDiagram::parse(source, syntax)?;

        Ok(Self {
            diagram,
            scale: 2,
            transparent_background: false,
            width: ImageWidth::Percent(0.5),
            layout: Layout::default(),
        })
    }

    /// Creates a `GraphExpression` configured for the current terminal.
    ///
    /// Automatically detects dark/light mode and sets appropriate colors
    /// and transparent background.
    ///
    /// ## Errors
    ///
    /// Returns `GraphRenderError` if parsing fails.
    pub fn for_terminal(
        source: impl Into<String>,
        syntax: GraphInputSyntax,
    ) -> Result<Self, GraphRenderError> {
        Self::for_terminal_mode(source, syntax, false)
    }

    /// Creates a `GraphExpression` with an explicit inverse theme.
    ///
    /// The inverse theme uses an opaque surface and the opposite terminal color
    /// palette so diagrams remain legible on both light and dark terminals.
    ///
    /// ## Errors
    ///
    /// Returns `GraphRenderError` if parsing fails.
    pub fn inverted_for_terminal(
        source: impl Into<String>,
        syntax: GraphInputSyntax,
    ) -> Result<Self, GraphRenderError> {
        Self::for_terminal_mode(source, syntax, true)
    }

    /// Sets the title for the diagram.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.diagram = self.diagram.with_title(title);
        self
    }

    /// Sets the graph orientation.
    pub fn with_orientation(mut self, orientation: GraphOrientation) -> Self {
        self.diagram = self.diagram.with_orientation(orientation);
        self
    }

    /// Overrides the font family for diagram labels.
    ///
    /// Accepts any CSS font-family value (e.g., `"Courier"`, `"Georgia, serif"`).
    pub fn with_font_family(mut self, font_family: impl Into<String>) -> Self {
        self.diagram = self.diagram.with_font_family(font_family);
        self
    }

    /// Enables transparent background for better terminal integration.
    pub fn with_transparent_background(mut self, transparent: bool) -> Self {
        self.transparent_background = transparent;
        self
    }

    /// Sets the scale factor for output resolution.
    ///
    /// Higher values produce sharper images but larger files.
    /// Default is 2 (good for most modern displays).
    pub fn with_scale(mut self, scale: u32) -> Self {
        self.scale = scale.max(1);
        self
    }

    /// Sets the display width for the rendered image.
    ///
    /// Default is 50% of available terminal width.
    pub fn with_width(mut self, width: ImageWidth) -> Self {
        self.width = width;
        self
    }

    /// Uses inverted colors (solid background with the opposite terminal theme).
    pub fn inverted(mut self, is_dark_mode: bool) -> Self {
        let theme = if is_dark_mode {
            GraphColorTheme::light()
        } else {
            GraphColorTheme::dark()
        };
        self.diagram = self.diagram.with_color_theme(theme);
        self.transparent_background = false;
        self
    }

    /// Renders the graph to a terminal-displayable string.
    ///
    /// This is the fallible render path. Use it when callers need the rendered
    /// output and cache metadata instead of silent fallback behavior.
    pub fn try_render(&self, term: &Terminal) -> Result<GraphRenderResult, GraphRenderError> {
        let (output, png_path, cache_hit) = self.render_to_image(term)?;
        // Graphs are block-cohesive (image escapes or expression fallback) —
        // align as a block.
        let output = self.layout.apply_block_layout(&output, term.width());

        Ok(GraphRenderResult {
            output,
            png_path,
            cache_hit,
        })
    }

    /// Renders the diagram to a cached PNG file, returning the path and cache hit status.
    ///
    /// Uses the legacy scale-based rasterization (`scale × svg_native`). When you
    /// know the *target pixel width* — e.g. for terminal display — prefer
    /// [`render_to_cached_png_at_width`](Self::render_to_cached_png_at_width)
    /// so the SVG is rasterized once at the display resolution instead of
    /// being oversampled and downscaled.
    ///
    /// ## Errors
    ///
    /// Returns error if diagram rendering or file I/O fails.
    #[tracing::instrument(skip(self))]
    pub fn render_to_cached_png(&self) -> Result<(PathBuf, bool), GraphRenderError> {
        let request = biscuit_visualized::artifact::RenderRequest {
            format: biscuit_visualized::artifact::OutputFormat::Png,
            scale: self.scale,
            target_width: None,
            transparent_background: self.transparent_background,
        };

        let artifact = self.diagram.render(&request)?;

        tracing::info!(
            path = ?artifact.path,
            cache_hit = artifact.cache_hit,
            "Rendered graph diagram to PNG"
        );

        Ok((artifact.path, artifact.cache_hit))
    }

    /// Renders the diagram to a cached PNG file at the specified target
    /// pixel width.
    ///
    /// Height is derived from the SVG's aspect ratio. The PNG comes out of
    /// `resvg` rendered fresh at `target_width` pixels — no intermediate
    /// bitmap, no downstream downscaling — so text and shapes are rasterized
    /// at the display resolution rather than resampled from an oversized
    /// source.
    ///
    /// ## Errors
    ///
    /// Returns error if diagram rendering or file I/O fails.
    #[tracing::instrument(skip(self))]
    pub fn render_to_cached_png_at_width(
        &self,
        target_width: u32,
    ) -> Result<(PathBuf, bool), GraphRenderError> {
        let request = biscuit_visualized::artifact::RenderRequest {
            format: biscuit_visualized::artifact::OutputFormat::Png,
            scale: self.scale,
            target_width: Some(target_width),
            transparent_background: self.transparent_background,
        };

        let artifact = self.diagram.render(&request)?;

        tracing::info!(
            path = ?artifact.path,
            cache_hit = artifact.cache_hit,
            target_width,
            "Rendered graph diagram to PNG at target width"
        );

        Ok((artifact.path, artifact.cache_hit))
    }

    /// Checks if the current terminal supports image rendering.
    pub fn terminal_supports_images() -> bool {
        use crate::discovery::detection::ImageSupport;

        let term = Terminal::new();
        !matches!(term.image_support, ImageSupport::None)
    }

    /// Returns a fallback code block string for the diagram.
    pub fn fallback_code_block(&self) -> String {
        self.diagram.fallback_code_block()
    }

    fn render_to_image(
        &self,
        term: &Terminal,
    ) -> Result<(String, PathBuf, bool), GraphRenderError> {
        // Compute the terminal cell area we'll display the image into, then
        // translate that to pixels using the terminal's detected cell size.
        // The resulting `target_width_px` is fed to the rasterizer so the PNG
        // is rendered once at exactly the display resolution — no oversampling,
        // no downscaling, text rasterised fresh at the terminal's pixel density.
        let dims = TerminalImage::resolve_dimensions_for(&self.width, &self.layout, term.width());
        let cell_pixel_width = term.cell_size().map(|cs| cs.width.max(1)).unwrap_or(8u32);
        let target_width_px = (dims.image_width.max(1)) * cell_pixel_width;

        let (png_path, cache_hit) = self.render_to_cached_png_at_width(target_width_px)?;

        let term_image = TerminalImage::new(&png_path)
            .map_err(|e| GraphRenderError::DisplayError(e.to_string()))?
            .with_width(self.width.clone());

        let output = term_image.render(term);
        if output.is_empty() {
            return Err(GraphRenderError::NoImageSupport);
        }

        Ok((output, png_path, cache_hit))
    }

    fn render_raw(&self, term: &Terminal) -> String {
        match self.render_to_image(term) {
            Ok((output, _, _)) => output,
            Err(_) => self.fallback_code_block(),
        }
    }

    fn for_terminal_mode(
        source: impl Into<String>,
        syntax: GraphInputSyntax,
        inverse: bool,
    ) -> Result<Self, GraphRenderError> {
        let color_mode = crate::discovery::detection::color_mode();
        let is_dark = matches!(
            color_mode,
            crate::discovery::detection::ColorMode::Dark
                | crate::discovery::detection::ColorMode::Unknown
        );

        let theme = if inverse {
            if is_dark {
                GraphColorTheme::light()
            } else {
                GraphColorTheme::dark()
            }
        } else if is_dark {
            GraphColorTheme::dark()
        } else {
            GraphColorTheme::light()
        };

        let diagram =
            biscuit_visualized::graph::GraphDiagram::parse(source, syntax)?.with_color_theme(theme);

        Ok(Self {
            diagram,
            scale: 2,
            transparent_background: !inverse,
            width: ImageWidth::Percent(0.5),
            layout: Layout::default(),
        })
    }
}

impl TerminalRenderable for GraphExpression {
    fn render(&self, term: &Terminal) -> String {
        let content = self.render_raw(term);
        self.layout.apply_block_layout(&content, term.width())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }
}

impl TreeRenderable for GraphExpression {
    /// Projects the rendered graph into a [`NodeKind::Paragraph`] wrapping an
    /// inline [`NodeKind::Image`] and carrying the component's outer-box
    /// [`Layout`] (margin, alignment).
    ///
    /// The rasterized graph canvas is irreducible (spec C5/D5); the structural
    /// image node carries placement and alt text describing the graph source,
    /// so Browser/Markdown targets degrade to a readable placeholder. The
    /// `url` is left empty because the rasterized PNG is produced on demand
    /// and has no stable address until rendered to cache.
    ///
    /// [`NodeKind::Image`]: renderable::tree::NodeKind::Image
    /// [`NodeKind::Paragraph`]: renderable::tree::NodeKind::Paragraph
    fn render_tree(&self) -> RenderNode {
        let alt = self.fallback_code_block();
        let image = RenderNode::image("", None, &alt);
        let mut node = RenderNode::paragraph(vec![image]);
        node.attrs.set_layout(&self.layout);
        node
    }
}

impl BrowserRenderable for GraphExpression {
    /// Wraps the graph's SVG as a [`ComposableNode::RawHtml`] island.
    ///
    /// The SVG is caller-owned, already-formed markup; emitting it as a
    /// `RawHtml` node tells the renderer to pass it through verbatim
    /// rather than HTML-escaping it.
    ///
    /// [`ComposableNode::RawHtml`]: renderable::browser::fragment::ComposableNode::RawHtml
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        BrowserFragment::new()
            .define_as_raw_html(self.render_browser_svg())
            .finalize()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl GraphExpression {
    /// Render the graph as a raw inline SVG element suitable for embedding
    /// directly into an HTML document.
    ///
    /// Returns the SVG source from `biscuit-visualized`'s caching renderer
    /// (orientation, theme, title, and post-processing all applied). The
    /// SVG carries its own `width`/`height` attributes — wrap in a styled
    /// container if you need to constrain or override sizing.
    ///
    /// ## Failure mode
    ///
    /// On render failure, returns the fallback code block wrapped in
    /// `<pre><code class="language-dot">…</code></pre>` so the source is
    /// still visible to the reader.
    fn render_browser_svg(&self) -> String {
        let request = biscuit_visualized::artifact::RenderRequest {
            format: biscuit_visualized::artifact::OutputFormat::Svg,
            scale: self.scale,
            // SVG is vector — there's no rasterization step that would benefit
            // from a pixel target. Leave `target_width` unset.
            target_width: None,
            transparent_background: self.transparent_background,
        };

        match self.diagram.render(&request) {
            Ok(artifact) => match std::fs::read_to_string(&artifact.path) {
                Ok(svg) => svg,
                Err(e) => self.browser_fallback(&format!("read SVG artifact: {e}")),
            },
            Err(e) => self.browser_fallback(&format!("render graph: {e}")),
        }
    }

    fn browser_fallback(&self, error: &str) -> String {
        let escaped = html_escape(&self.fallback_code_block());
        format!(
            "<!-- biscuit-terminal graph render failed: {} -->\n<pre><code class=\"language-dot\">{}</code></pre>",
            html_escape(error),
            escaped,
        )
    }
}

fn html_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

impl TryFrom<String> for GraphExpression {
    type Error = GraphRenderError;

    fn try_from(source: String) -> Result<Self, Self::Error> {
        Self::parse(source, GraphInputSyntax::Auto)
    }
}

impl TryFrom<&str> for GraphExpression {
    type Error = GraphRenderError;

    fn try_from(source: &str) -> Result<Self, Self::Error> {
        Self::parse(source, GraphInputSyntax::Auto)
    }
}

/// Successful render result from [`GraphExpression::try_render()`].
#[derive(Debug, Clone)]
pub struct GraphRenderResult {
    /// The rendered terminal output string.
    pub output: String,
    /// Path to the cached PNG file.
    pub png_path: PathBuf,
    /// Whether the PNG was served from cache.
    pub cache_hit: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    use biscuit_visualized::artifact::{OutputFormat, RenderRequest};

    #[test]
    fn test_parse() {
        let graph = GraphExpression::parse("a -> b -> c", GraphInputSyntax::Auto);
        assert!(graph.is_ok());
    }

    #[test]
    fn test_with_title() {
        let graph = GraphExpression::parse("a -> b", GraphInputSyntax::Auto)
            .unwrap()
            .with_title("Test Graph");
        let dot = graph.diagram.source_as_dot();
        assert!(dot.contains("label=\"Test Graph\""));
    }

    #[test]
    fn test_from_string() {
        let source = String::from("a -> b -> c");
        let graph = GraphExpression::try_from(source).unwrap();
        let dot = graph.diagram.source_as_dot();
        assert!(dot.contains("digraph"));
    }

    #[test]
    fn test_from_str() {
        let graph = GraphExpression::try_from("a -> b -> c").unwrap();
        let dot = graph.diagram.source_as_dot();
        assert!(dot.contains("digraph"));
    }

    #[test]
    fn test_fallback_code_block() {
        let graph = GraphExpression::parse("a -> b", GraphInputSyntax::Auto).unwrap();
        let output = graph.fallback_code_block();
        assert!(output.starts_with("```graph-expression\n"));
        assert!(output.ends_with("\n```"));
        assert!(output.contains("a -> b"));
    }

    #[test]
    fn test_fallback_code_block_uses_dot_info_string_for_dot_input() {
        let graph = GraphExpression::parse("digraph { A -> B; }", GraphInputSyntax::Dot).unwrap();
        let output = graph.fallback_code_block();
        assert!(output.starts_with("```dot\n"));
        assert!(output.contains("digraph { A -> B; }"));
    }

    #[test]
    fn test_implements_renderable() {
        let graph = GraphExpression::parse("a -> b", GraphInputSyntax::Auto).unwrap();
        // Verify it has layout methods from TerminalRenderable
        let _layout = graph.layout();
    }

    #[test]
    fn browser_render_emits_svg() {
        let graph = GraphExpression::parse("a -> b -> c", GraphInputSyntax::Auto).unwrap();
        let html = graph.render_browser_svg();
        assert!(
            html.contains("<svg"),
            "expected raw SVG output, got: {html}"
        );
        assert!(html.contains("</svg>"));
        assert!(
            !html.contains("base64"),
            "browser render must not embed PNG data: {html}"
        );
    }

    #[test]
    fn browser_render_dot_input_emits_svg() {
        let graph =
            GraphExpression::parse("digraph G { A -> B; B -> C; }", GraphInputSyntax::Dot).unwrap();
        let html = graph.render_browser_svg();
        assert!(html.contains("<svg"));
        assert!(html.contains("</svg>"));
    }

    #[test]
    fn test_inverted_uses_opposite_theme_surface_color() {
        // Dark mode → inverted uses light theme
        let graph = GraphExpression::parse("a -> b", GraphInputSyntax::Auto)
            .unwrap()
            .inverted(true);
        assert!(!graph.transparent_background);

        let artifact = graph
            .diagram
            .render(&RenderRequest {
                format: OutputFormat::Svg,
                scale: 1,
                target_width: None,
                transparent_background: graph.transparent_background,
            })
            .unwrap();
        let svg = std::fs::read_to_string(artifact.path).unwrap();

        assert!(svg.contains(&format!(
            "fill=\"{}\"",
            GraphColorTheme::light().surface_color
        )));

        // Light mode → inverted uses dark theme
        let graph = GraphExpression::parse("a -> b", GraphInputSyntax::Auto)
            .unwrap()
            .inverted(false);
        assert!(!graph.transparent_background);

        let artifact = graph
            .diagram
            .render(&RenderRequest {
                format: OutputFormat::Svg,
                scale: 1,
                target_width: None,
                transparent_background: graph.transparent_background,
            })
            .unwrap();
        let svg = std::fs::read_to_string(artifact.path).unwrap();

        assert!(svg.contains(&format!(
            "fill=\"{}\"",
            GraphColorTheme::dark().surface_color
        )));
    }
}
