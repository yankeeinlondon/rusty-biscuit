//! Graph expression rendering for terminals.
//!
//! This module provides a `Renderable` component that renders graph diagrams
//! in the terminal. It delegates graph layout to `biscuit-visualized` and
//! uses `TerminalImage` for inline image display.
//!
//! ## Examples
//!
//! ```rust,no_run
//! use biscuit_terminal::components::graph_expression::{GraphExpression, GraphInputSyntax};
//! use biscuit_terminal::components::renderable::Renderable;
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

use crate::components::renderable::Renderable;
use crate::components::terminal_image::{ImageWidth, TerminalImage};
use crate::terminal::Terminal;
use crate::utils::layout::Layout;

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
/// Implements `Renderable` so it integrates with the standard layout system
/// (margins, alignment) and can be composed with other components.
///
/// ## Examples
///
/// ```rust,no_run
/// use biscuit_terminal::components::graph_expression::{
///     GraphExpression, GraphInputSyntax, GraphOrientation,
/// };
/// use biscuit_terminal::components::renderable::Renderable;
/// use biscuit_terminal::utils::layout::Margin;
///
/// let graph = GraphExpression::for_terminal("a -> b -> c", GraphInputSyntax::Auto)?
///     .with_orientation(GraphOrientation::LeftToRight)
///     .with_title("My Graph")
///     .left_margin(Margin::Chars(4));
///
/// # Ok::<(), biscuit_terminal::components::graph_expression::GraphRenderError>(())
/// ```
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
        let output = self.layout.apply_layout(&output, term.width());

        Ok(GraphRenderResult {
            output,
            png_path,
            cache_hit,
        })
    }

    /// Renders the diagram to a cached PNG file, returning the path and cache hit status.
    ///
    /// ## Errors
    ///
    /// Returns error if diagram rendering or file I/O fails.
    #[tracing::instrument(skip(self))]
    pub fn render_to_cached_png(&self) -> Result<(PathBuf, bool), GraphRenderError> {
        let request = biscuit_visualized::artifact::RenderRequest {
            format: biscuit_visualized::artifact::OutputFormat::Png,
            scale: self.scale,
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
        let (png_path, cache_hit) = self.render_to_cached_png()?;

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
        let color_mode = Terminal::color_mode();
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

impl Renderable for GraphExpression {
    fn render(&self, term: &Terminal) -> String {
        let content = self.render_raw(term);
        self.layout.apply_layout(&content, term.width())
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
        // Verify it has layout methods from Renderable
        let _layout = graph.layout();
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
